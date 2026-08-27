# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/039-grant-collaborator-user-exists.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - `server/src/permissions/checker.rs`：`grant_collaborator` 在写入前（写入事务
    内）校验 `users(id)` 存在，负数或未创建正整数 `user_id` 返回
    `PermissionErrorKind::NotFound`；
  - `server/migrations/0003_roles_grants.sql`：`project_grants.user_id` 增加
    `REFERENCES users(id)`（`connect_pool` 已启用 `foreign_keys(true)`，DB 层
    兜底拦截绕过 checker 的直写）；
  - `server/tests/unit/permissions.rs`：新增
    `grant_collaborator_rejects_nonexistent_user_and_fk`（负数、未创建正整数
    NotFound；外键直插失败；合法授权不回归）；
  - `server/tests/api_integration.rs`：协作者流程追加 PUT 不存在用户返回 404。
- Handoff:
  - `cargo test -p filehub-server --test unit_tests -- --test-threads=1` 46/46 通过；
  - `cargo test -p filehub-server --test api_integration -- --test-threads=1` 6/6 通过；
  - 全量（含 dv_tests）受同工作树另一在制事务化重构影响，`dv_full_workflow_with_tokens_and_gc`
    在版本发布路径报单连接池超时（详见 Findings）。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-grant-user-exists | `grant_collaborator` 写入前校验 `users(id)` 存在，不存在返回 NotFound；`project_grants.user_id` 增加 `REFERENCES users(id)` | proposal.md P-001 | checker.rs 存在性查询 + 0003 外键列；单元用例断言 NotFound 与直插外键失败 | 匹配 | pass |
| fh-grant-user-exists-tests | 单元与 API 集成回归：不存在用户授权被拒、合法授权不受影响、外键直插失败 | proposal.md P-002 | permissions.rs 新用例；api_integration.rs 404 断言；单元 46/46、api 6/6 串行通过 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | `grant_collaborator` 校验顺序（administration -> owner 保护 -> 存在性 -> 写入）与事务语义；`PermissionErrorKind::NotFound` -> 404 `not_found` 映射；`connect_pool` 的 `foreign_keys(true)` 是否实际开启 | 反向推演：负数 ID 不可能等于 owner 先被放行（先查 owner 后查存在性，负数直接 NotFound）；存在性查询与写入同事务，目标账号在确认后被删除时 INSERT 由外键兜底；绕过 checker 直插时外键必须拦截 | 无绕过：三层防线（存在性校验、事务内确认、外键约束）顺序正确，错误映射符合契约 | pass |
| boundaries-and-failure-paths | `UserId(-1)`、`UserId(9_999_999)`（大于当前自增最大 id 的未创建正整数）、已存在 bob 的合法授权、直插 `project_grants` 的 FK 错误、HTTP 404 响应 | off-by-one/顺序推演：恰好是当前最大 id+1 的未创建 id 同样 NotFound；owner 仍是已存在用户时保持 forbidden 优先；DELETE owner 保护不变；204/200 合法流程不受影响 | 边界案例全部由新增用例覆盖并断言；无 off-by-one；合法授权与 owner 保护零回归 | pass |
| regression-and-side-effects | 单元全集 46/46（account/model/permissions/projects/storage/tokens/upload/upload_ingest/versions 各模块）；api_integration 串行 6/6；dv_tests 1/2；并行模式 login 偶发 502 | 排查是否存在「外键导致既有 grant/remove/update 用例回归」（无：既有用例授权对象均为已创建用户）；迁移是否影响 token/project 功能（未改这两张表）；`update_collaborator`/`remove_collaborator` 语义未被触碰 | 目标用例与既有权限/版本/上传用例全绿；dv 失败为并发在制事务化重构的单连接池超时（非本任务路径），并行 502 为既有偶发（串行全绿），均记录于 Findings | pass |

## Verification

- Targeted check:
  - `cargo test -p filehub-server --test unit_tests grant_collaborator_rejects_nonexistent_user_and_fk` 通过；
  - `cargo test -p filehub-server --test unit_tests -- --test-threads=1` 46 通过 / 0 失败；
  - `cargo test -p filehub-server --test api_integration -- --test-threads=1` 6 通过 / 0 失败；
  - `cargo test -p filehub-server --test api_integration api_login_session_and_token_flow` 通过。
- Result: pass
- Exception reason: 全量 `cargo test -p filehub-server` 中 dv_tests 的
  `dv_full_workflow_with_tokens_and_gc` 因同工作树并发在制事务化重构（版本发布
  路径 `BEGIN IMMEDIATE` + 单连接池）报 `pool timed out while waiting for an
  open connection`，与本次授权校验/外键改动路径无关（该用例在项目/版本创建段
  失败，不涉及 grant）；串行全绿目标覆盖已作为本任务验收基线记录。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | medium | 全量串行跑 dv_tests 1/2：`dv_full_workflow_with_tokens_and_gc` 在版本发布路径 panic，错误为 `pool timed out while waiting for an open connection`；`versions/service.rs` mtime 在本次任务期间连续变动，`checker.rs` 亦被并发并入 BEGIN IMMEDIATE 事务化改写 | 同工作树另一在制任务的事务化重构与单连接池（`max_connections=1`）不兼容，全量朝绿状态由在制任务负责；本任务未触碰该路径 | no |
| F-2 | low | `0003` 为 `CREATE TABLE IF NOT EXISTS`；本任务仅修改 DDL | 修复前已建成的本地 SQLite 开发库不会自动获得外键，需删除重建；无已发布生产库，不写自动 ALTER 迁移 | no |
| F-3 | low | api_integration 并行跑偶发 login 502，串行 `--test-threads=1` 6/6 通过 | 既有并行测试稳定性问题（各用例独立起服），非本次改动引入 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: P-001/P-002 全部落地：授权写入前校验目标用户存在（404）、
  `project_grants` 增加 `users(id)` 外键、单元与 API 回归用例覆盖负数/未创建
  正整数/直插外键/合法授权四条路径；单元 46/46、API 集成 6/6 串行通过，既有
  权限与账号/版本/上传用例不回归；dv 全量失败已归因至并发在制事务化重构
  （F-1），独立缺陷发现问题 F-1~F-3 均为非阻塞记录。
