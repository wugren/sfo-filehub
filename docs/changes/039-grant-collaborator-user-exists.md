# 协作者授权必须校验目标用户存在（含 project_grants users(id) 外键）

- Status: complete
- Owner module: filehub（filehub-server permissions 子模块 + 0003 迁移）
- Task manifest: `docs/versions/v0.1/modules/filehub/039-grant-collaborator-user-exists/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/039-grant-collaborator-user-exists/proposal.md`
- Affected paths: `server/src/permissions/checker.rs`、
  `server/migrations/0003_roles_grants.sql`、`server/tests/unit/permissions.rs`、
  `server/tests/api_integration.rs`、`docs/changes/039-grant-collaborator-user-exists.md`、
  `docs/versions/v0.1/modules/filehub/039-grant-collaborator-user-exists/completion-report.md`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 缺陷（评审第 5 项，中危）：`SqlitePermissionChecker::grant_collaborator` 只做
  操作者 `administration` 与「目标非 owner」校验就直接把传入的 `user_id` 写入
  `project_grants`；`0003_roles_grants.sql` 的 `user_id` 无 `users(id)` 外键，
  因此可向负数/未创建正整数 ID 预授权，未来新增账号拿到该自增 ID 时历史授权
  自动生效，原管理员被移除后也无法撤销。
- 修复（应用层，`checker.rs`）：授权写入前在事务内先
  `SELECT 1 FROM users WHERE id = ?` 确认目标用户存在，不存在返回
  `PermissionError::not_found`（HTTP 404 `not_found`，符合 v1 契约通用错误语义）。
  校验顺序保持 操作者权限 -> owner 保护 -> 用户存在性 -> 写入。
- 修复（schema，`0003_roles_grants.sql`）：`project_grants.user_id` 增加
  `REFERENCES users(id)`；`connect_pool` 已开启 `foreign_keys(true)`，数据库层
  兜底拦截所有绕过 checker 的直写。不追加 `ON DELETE CASCADE`（当前无用户删除
  API，保持最小行为面）。
- 测试：单元回归覆盖负数 ID、未创建正整数 ID 被拒（NotFound）、绕过 checker
  直插触发外键失败、已存在用户授权不受影响；API 集成在协作者流程追加 PUT
  不存在用户返回 404。

## Risk Screen

- Public contract, protocol, or CLI change: no（路由/请求/成功响应不变；404 属
  契约既有通用错误「不存在或对该身份不可见」）
- Persistent data, schema, or migration change: yes（`project_grants.user_id`
  新增 `REFERENCES users(id)`；`0003_roles_grants.sql` 为本工作树未提交的
  greenfield schema，无已发布生产库；修复前已建成的本地开发库需删除重建，
  不写自动 ALTER 迁移）
- Security, privacy, or trust-boundary change: yes（授权边界收紧：不存在用户无法
  获得预授权，消除未来账号接管历史授权的安全缺口；属已确认 standard 交付）
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check:
  - `cargo test -p filehub-server --test unit_tests grant_collaborator_rejects_nonexistent_user_and_fk`
    通过（负数/未创建正整数 NotFound、外键直插失败、合法授权不回归）；
  - `cargo test -p filehub-server --test unit_tests -- --test-threads=1` 46/46 通过；
  - `cargo test -p filehub-server --test api_integration -- --test-threads=1` 6/6 通过
    （含 `api_login_session_and_token_flow` 内新增的不存在用户 404 断言）。
- Result: pass
- Residual risk or follow-up:
  - 全量跑 `cargo test -p filehub-server`（非串行）时存在与本次改动无关的在制问题：
    `dv_full_workflow_with_tokens_and_gc` 在版本发布路径报 `pool timed out while
    waiting for an open connection`，且 api_integration 并行跑会偶发 login 502；
    二者串行 api 6/6、dv 之外的单元模块均通过。该现象来自同工作树另一在制
    事务化重构（`BEGIN IMMEDIATE` + 单连接池），属对方任务待修复范围，已如实记录；
  - 修复前已建成的本地 SQLite 开发库不会自动获得外键，需删除重建后再启动服务；
  - `update_collaborator`/`remove_collaborator` 对不存在用户的既有语义（零行
    no-op / 幂等 204）未改动，作为提案明确 non-goal。
