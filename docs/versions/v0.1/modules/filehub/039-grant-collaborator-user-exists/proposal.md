---
task_manifest: task.yaml
status: approved
---

# 协作者授权必须校验目标用户存在（project_grants 增加 users(id) 外键）

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Proposal and tier confirmation: 用户 2026-08-25 回复「确认」，确认采纳本提案
  （grant_collaborator 写入前校验目标用户存在、project_grants 增加 users(id)
  外键、补不存在用户回归测试）并接受建议的 standard 层级。
- Tier rationale / triggered boundaries:
  - 这是生产授权边界的数据完整性/安全 bugfix：`grant_collaborator` 当前可向
    任意不存在（含负数、尚未创建的正数）`user_id` 写入授权，且
    `project_grants.user_id` 无外键。修复需要同时改权限写入路径与持久化 schema，
    并新增不存在用户回归测试，不满足 trivial 对「无 security/数据 schema 实质
    影响、无迁移」的界定；
  - 未触发 high-risk：不新增/不改变公开 API 路由、请求字段与成功响应形状；
    不存在的目标用户授权失败走契约既有 `404 not_found`（「不存在或对该身份
    不可见」）；不新增依赖、不改变登录/会话/token 签名机制；`0003_roles_grants.sql`
    在本工作树中仍是未提交（git status 为 M）的 greenfield schema，没有已发布
    生产库与存量迁移/回滚协调需求；修复方向是收紧现状（不再允许向不存在用户
    写授权），且验证面只有既有协作者/权限测试框架。

## Approval Record

- approver: 用户
- approval_date: 2026-08-25
- user_statement: 用户 2026-08-25 回复「确认」，确认采纳本提案（协作者授权必须
  验证目标用户存在，含 users(id) 外键与不存在用户回归测试）并接受 standard 层级。

## Background and Goal

- 现象（评审第 5 项，中危）：`SqlitePermissionChecker::grant_collaborator`
  （`server/src/permissions/checker.rs:206`）只校验操作者是否具备
  `administration`、以及目标不是项目 owner，随后直接把传入的 `user_id` 写入
  `project_grants`；`server/migrations/0003_roles_grants.sql:3` 的
  `project_grants.user_id` 没有 `users(id)` 外键。
- 后果：可以对任意负数或尚未创建的正整数用户 ID 预授权。未来新增账号按
  `AUTOINCREMENT` 拿到该 ID 时，历史授权会自动生效——即使当初授权的管理员
  已被移除，也无法撤销此历史数据。
- 目标（用户确定的口径）：协作者授权前必须验证目标用户存在；`project_grants`
  增加 `users(id)` 外键作为数据库层第二道防线；并为不存在用户补回归测试。

## Scope

### In scope

- `server/src/permissions/checker.rs`：`grant_collaborator` 在通过操作者
  `administration` 与 owner 判定后、写入前，先查询 `users` 表确认
  `user_id` 存在；不存在返回 `PermissionError::not_found`（HTTP 404，
  符合 `docs/api/v1-contract.md` 通用错误语义）；
- `server/migrations/0003_roles_grants.sql`：`project_grants.user_id` 增加
  `REFERENCES users(id)`；`connect_pool` 已启用 `foreign_keys(true)`
  （`server/src/account/store.rs:194`），外键在文件库/测试库中实际生效；
- `server/tests/unit/permissions.rs`：新增回归用例——向负数用户 ID、大于当前
  最大 ID 的未创建正整数用户 ID 授权均被拒绝；已有合法授权（alice/bob）不受
  影响；并直接验证数据库层外键（绕过 checker 直插不存在 `user_id` 报错）；
- `server/tests/api_integration.rs`：协作者流程追加 API 层回归——PUT
  不存在 `user_id` 返回 404，合法授权仍返回 200。

### Out of scope

- 不修改 `update_collaborator` / `remove_collaborator` 对不存在用户的既有语义
  （UPDATE 零行静默 no-op、DELETE 幂等 204 继续保留；它们不会新增授权，不存在
  「未来账号接管历史授权」风险；如需统一 404 可后续单独提任务）；
- 不顺带修复 `projects.owner_id`、`tokens.owner_id` 等其他缺失外键；
- 不修改 `docs/api/v1-contract.md` 路由表（404 已属契约既有通用错误）与
  admin-web/CLI 前端行为；
- 不触碰 025-038 等在制未提交任务改动；不运行仓库级格式化。

### Boundary with neighboring modules

- 授权写入路径唯一收口在 `PermissionChecker::grant_collaborator`，外键约束同时
  覆盖未来任何绕过 checker 直写 `project_grants` 的代码路径；
- 校验错误作为 `PermissionErrorKind::NotFound` 走既有
  `api_error_response` 映射（404 `not_found`），错误结构不变；
- 模块初始化顺序仍为 account（先建 `users`）-> permissions（建/用
  `project_grants`），外键引用表在插入时必然已存在。

## Requirement Review

- 需求合理：授权关系必须以真实存在的账号为前提；双层防线（应用层存在性校验 +
  DB 外键）既给出精确的 404 错误，也兜底未来所有写入路径；
- 方向选择：在 `grant_collaborator` 内直接 `SELECT 1 FROM users WHERE id = ?`
  是最小改动，不引入 account 子模块依赖（权限核心本就持有 `db`）；外键只加
  `REFERENCES users(id)`，不加 `ON DELETE CASCADE`——当前没有用户删除 API，
  不扩大行为面；
- 材料风险/权衡：
  - `0003_roles_grants.sql` 是 `CREATE TABLE IF NOT EXISTS`，对修复前已建成的
    本地开发库不会回填外键；但该 migration 在本工作树未提交且无发布版本，
    通过重构表或删除本地开发库重建即可，不构成生产迁移（见 Risks）；
  - `update_collaborator` 对不存在用户仍是静默 no-op，与本任务漏洞场景
    （新增授权）不同，列为 non-goal 但不隐藏；
  - HTTP 语义变化仅限「此前错误地返回 200 的非法输入」变为 404，合法流程
    响应不变。
- 待确认问题：无。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-grant-user-exists | `grant_collaborator` 写入前校验 `users(id)` 存在，不存在返回 NotFound；`project_grants.user_id` 增加 `REFERENCES users(id)` | 仅 `server/src/permissions/checker.rs` 与 `0003_roles_grants.sql`；错误走既有 NotFound 404 映射 | 应用层校验 + 外键双层防线；不加 ON DELETE CASCADE | 负数、未创建正整数授权均报 NotFound；绕过 checker 直插不存在 user_id 触发外键约束失败 | 不改 update/remove 对不存在用户语义；不补其它表外键 |
| P-002 | fh-grant-user-exists-tests | 单元与 API 集成回归：不存在用户授权被拒、合法授权不受影响、外键直插失败 | `server/tests/unit/permissions.rs`、`server/tests/api_integration.rs` 协作者用例区 | 覆盖 checker 层与 HTTP 层两条路径，防未来重新放开 | 新增用例通过；既有协作者/权限用例不回归 | 不新增 mock 基建，不重写既有协作者流程 |

## Success Criteria

- `grant_collaborator` 对负数 `user_id`（如 -1）与大于当前最大 ID 的未创建
  正整数 `user_id` 均返回 `PermissionErrorKind::NotFound`；API 层 PUT 对应用例
  返回 404；
- `project_grants` DDL 含 `user_id ... REFERENCES users(id)`，且绕过 checker
  直接向 `project_grants` 插入不存在 `user_id` 触发 SQLite 外键约束失败
  （`foreign_keys(true)` 已在 `connect_pool` 开启）；
- 对已存在用户（alice/bob）的授权/列表/移除与 owner 保护行为保持不变，
  既有协作者管理、token 项目范围、public 可见性等权限用例不回归；
- 目标命令通过：`cargo test -p filehub-server` 相关单元/集成用例及编译检查
  （全量测试结果受 036 等在制任务工作树状态影响，在实现阶段实测后记录）；
- 按 standard 流程产出 `docs/changes/039-grant-collaborator-user-exists.md`
  与任务包 `completion-report.md`（中文正文），并经 lower-tier-check 校验。

## Risks

- 存量本地开发库：修复前按旧 `0003` 建库的文件库不会自动获得外键；文档与
  completion-report 中标注需删除本地库重建（数据可由配置/上传恢复），不写
  自动 ALTER 迁移，避免扩大 schema 变更面；
- 工作树存在大量未提交的用户改动（025-038 等在制内容），本任务只修改提案
  Scope 列出的文件；全量 `cargo test -p filehub-server` 可能因在制用例
  （如 036 上传用例）受影响，以提案列出的目标用例 + 编译/既有权限回归为准，
  并如实记录；
- 外键只约束 `project_grants.user_id`，`update_collaborator` 对不存在用户仍
  返回成功（零行更新），该边界已在 Out of scope 显式保留。
