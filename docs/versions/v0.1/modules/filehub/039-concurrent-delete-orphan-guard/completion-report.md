# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/039-concurrent-delete-orphan-guard.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - 新库 schema：`server/migrations/0003_roles_grants.sql` 与 `0006_versions.sql`
    分别为 `project_grants.project_id`、`versions.project_id` 增加
    `REFERENCES projects(id) ON DELETE CASCADE`；`server/src/account/store.rs`
    的内存库分支补开 `PRAGMA foreign_keys`；
  - 事务边界：`PermissionChecker` 新增事务连接版 `can_access_tx`（与池版共用
    纯判定函数 `decide_project_access`/`feature_allowed`）；`create_version`
    与 `grant_collaborator` 均在 `BEGIN IMMEDIATE` 事务内完成项目存在性/权限/
    owner/用户存在性确认与写入，外键失败映射为项目不存在（404 语义）；
  - 回归：`server/tests/unit/projects.rs` 新增 FK 级联与违反用例、并发删除+
    创建/授权不变量用例。
- Handoff: `cargo test -p filehub-server` 全量通过（46 unit + 2 dv + 6
  api_integration，共 54 项）；`cargo check -p filehub-server --tests`、
  `cargo clippy -p filehub-server --tests` 通过且无新增告警。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-orphan-fk-cascade | 新库 `versions.project_id` 与 `project_grants.project_id` 增加 `REFERENCES projects(id) ON DELETE CASCADE`；内存库亦开启 FK 约束 | proposal.md P-001 | 0003/0006 迁移定义 + account/store.rs 内存库 `foreign_keys(true)`；新库直删项目行级联清零、直插不存在项目被拒（unit 用例） | 匹配 | pass |
| fh-orphan-tx-boundary | `create_version` 与 `grant_collaborator` 在 `BEGIN IMMEDIATE` 事务内完成项目存在性/权限确认与写入 | proposal.md P-002 | versions/service.rs 与 permissions/checker.rs 的单事务实现；`can_access_tx` 与池版共享判定；并发用例孤儿计数 0 | 匹配 | pass |
| fh-orphan-concurrency-tests | 回归：FK 级联/违反 + 并发不变量可机器验证 | proposal.md P-003 | `project_fk_cascade_prevents_orphan_versions_and_grants` 与 `concurrent_project_delete_and_child_creates_leave_no_orphans` 均通过 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | `create_version`/`grant_collaborator` 事务全流程、`can_access_tx` 与池版判定函数、FK/唯一错误映射、owner 空值分支 | 反向推演四种交错：删除先提交（确认失败）、删除在本事务写锁前阻塞、删除在提交后级联清除、外键兜底直写；逐主体验证 anonymous/user/token 判定分支不因抽取而漂移 | 无绕过：所有交错都不会残留孤儿行；`can_access` 语义由 46 项 unit 全量回归确认未回归 | pass |
| boundaries-and-failure-paths | 连接池单连接行为、`BEGIN IMMEDIATE` 的 busy 语义、事务内早退（rollback）与提交后归还、内存库 FK pragma | 构造“事务提交后立即聚合查询”路径，发现事务作用域未及时归还连接导致池超时（已修复为块作用域）；再检查早退路径 drop 自动回滚；验证旧 DDL 与新 DDL 直删后的孤儿计数（1,1）→（0,0） | 池超时实现期已修且全量复跑通过；早退路径依赖 sqlx 事务 Drop 回滚，行为正确；无 off-by-one/约束遗漏 | pass |
| regression-and-side-effects | 既有项目删除清理用例、权限/版本全量用例、token scope 与公开可见性判定、批量并发任务的孤儿计数 | 排查是否误改其它 can_access 调用点（未改，另加仅两处 tx 入口）；`decide_project_access` 分支顺序与旧实现逐条对比；`project_owner` 仍供 update/remove 使用不受影响；任务未触碰 025-038 在制改动 | 既有 54 项全量测试通过；无新增 clippy 告警；`publish_app` 等其它路径保持原事务语义未动 | pass |

## Verification

- Targeted check: `cargo test -p filehub-server` 全量 54 项通过（46 unit
  + 2 dv + 6 api_integration）；两个新增回归用例单独复跑通过
- Result: pass
- Exception reason: 无

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 0003/0006 为 `CREATE TABLE IF NOT EXISTS`，存量库不受影响 | 存量开发库不会获得外键约束；孤儿防护依赖 032 显式清理与本次事务层修复，若未来需要数据库级兜底需表重建迁移（已列入提案 Non-Goal） | no |
| F-2 | low | `connect_pool(":memory:")` 原先未开 FK pragma，本次已补齐 | 内存库连接数被强制为单连接，`BEGIN IMMEDIATE` 场景下并发写会串行等待；本仓库无生产内存库用例，属既有约束非本次引入 | no |
| F-3 | low | 并发用例为终态不变量断言 | 竞态窗口依赖任务交错，旧码未必稳定必红；已由确定性 FK 用例与 schema red/green 对比兜底 | no |
| F-4 | low | `BEGIN IMMEDIATE` 创建版本/授权时对写锁有短暂独占 | 高并发争锁下部分请求等待（sqlx 池 30s acquire/busy 超时策略）后返回 Db 错误，客户端可重试；不产生静默孤儿数据 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: P-001/P-002/P-003 全部落地：新库 versions/project_grants 外键级联
  生效（含内存库 FK pragma），`create_version`/`grant_collaborator` 确认与
  写入处于同一 `BEGIN IMMEDIATE` 事务且与池版权限判定共用逻辑；确定性 FK 用例
  与并发不变量用例通过，全量 54 项测试与 clippy 无回归；独立缺陷发现覆盖
  逻辑/边界/回归，F-1~F-4 均为非阻塞低危记录。
