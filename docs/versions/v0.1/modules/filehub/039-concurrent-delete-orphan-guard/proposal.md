---
task_manifest: task.yaml
status: approved
---

# 修复项目删除与创建版本/协作者授权并发时的孤儿数据

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Proposal and tier confirmation: 用户 2026-08-25 指令「按建议修复」（评审第 4 项 +
  建议修复口径），即明确要求实施下方 In scope 内容；本次提案如实记录该口径，
  视同对提案范围的确认，并记录于批准记录。若用户后续希望换用 trivial/high-risk
  层级或调整范围，可按规则再路由。
- Tier rationale / triggered boundaries:
  - 修改点是 filehub-server 单个交付面内 permissions 与 versions 两个子模块的
    bugfix（竞态产生孤儿数据），并需要数据库级回归验证，属于有界修复；
  - 不满足 trivial：涉及 schema/迁移定义（新库外键）与并发/生命周期写入边界，
    trivial 明确排除 persistent data/schema/migration 与并发影响的变更；
  - 未触发 high-risk：仓库目前为 greenfield 阶段、无生产数据，迁移为
    `CREATE TABLE IF NOT EXISTS`，只对新建数据库生效、不改动存量库（存量库
    仍由 032 的显式级联清理兜底）；不改变公开协议/CLI、依赖图、发布/回滚/
    部署语义；权限判定只做逻辑抽取与事务化，不改变语义；与 030（bcrypt
    schema+迁移的 standard 先例）同类处理。

## Approval Record

- approver: 用户
- approval_date: 2026-08-25
- user_statement: 用户 2026-08-25 针对评审第 4 项回复「按建议修复」，明确采纳
  「新库补充 `REFERENCES projects(id) ON DELETE CASCADE` + 项目存在性/权限
  确认与写入处于一致事务边界」的建议口径。

## Background and Goal

- 现象（评审第 4 项，中危）：`create_version()` 在权限检查完成后单独执行
  INSERT（`server/src/versions/service.rs`），`versions.project_id` 无外键
  （`server/migrations/0006_versions.sql`）；若项目在权限检查后被删除，INSERT
  仍能成功，重新产生已删除项目的孤儿版本。`project_grants.project_id` 同样无
  外键（`server/migrations/0003_roles_grants.sql`），`grant_collaborator`
  的权限/owner 确认与 INSERT 之间也存在相同竞态。
- 目标（用户确定的口径）：a) 新库为 `versions.project_id` 与
  `project_grants.project_id` 补充 `REFERENCES projects(id) ON DELETE
  CASCADE`；b) 让「项目存在性/权限确认 + 写入」处于一致的事务边界内。

## Scope

### In scope

- `server/migrations/0003_roles_grants.sql`：`project_grants.project_id` 增加
  `REFERENCES projects(id) ON DELETE CASCADE`（新库生效）；
- `server/migrations/0006_versions.sql`：`versions.project_id` 增加
  `REFERENCES projects(id) ON DELETE CASCADE`（新库生效）；
- `server/src/account/store.rs`：`connect_pool` 对内存库分支同样开启
  `PRAGMA foreign_keys`，保证新库（含内存库）外键约束一致生效；
- `server/src/permissions/mod.rs` + `server/src/permissions/checker.rs`：
  `PermissionChecker` 增加在指定事务连接上执行与 `can_access` 同等判定的入口
  （`can_access_tx`）；`grant_collaborator` 改为 `BEGIN IMMEDIATE` 事务内
  「项目存在性/权限确认 → owner 校验 → INSERT → commit」；
- `server/src/versions/service.rs`：`create_version` 改为 `BEGIN IMMEDIATE`
  事务内「项目存在性/权限确认 → INSERT → commit」，唯一冲突与并发删除导致的外键
  失败分别映射为 Conflict 与项目不存在；
- `server/tests/unit/projects.rs` / `server/tests/unit/versions.rs`：新增
  FK 级联与违反回归、并发删除+创建/授权不产生孤儿数据的不变量用例。

### Out of scope

- 不新增 ALTER/表重建迁移去给存量数据库补外键（迁移为 `CREATE TABLE IF NOT
  EXISTS`，新库生效；存量库的孤儿防护继续依赖 032 显式清理）；
- 不改 `tokens`/`token_scopes` 等其它表的引用关系；
- 不把全部 `can_access` 调用点改成事务式（仅两处「确认+写入」竞态路径）；
- 不修改 `SqliteProjectService::delete` 的显式级联清理、HTTP 契约或错误文案；
- 不触碰 025-038 等在制未提交任务改动；不运行仓库级格式化。

## Requirement Review

- 需求合理：评审指出的竞态窗口真实存在——无外键时 INSERT 不会感知项目行已消失；
  FK + 事务边界使「确认与写入」串行化，删除要么先完成（确认失败），要么在执行
  写锁之后完成并级联清除，孤儿数据不可再产生；
- 方向选择：SQLite `BEGIN IMMEDIATE` 在首个写锁请求前锁住写入序列；权限、
  owner 与 INSERT 全部在同一个事务连接上执行，避免池单连接下的死锁/窗口问题；
  FK 作为数据库级兜底，同时支持并发删除已提交后的直写尝试返回 404 语义；
- 材料风险/权衡：`PermissionChecker` 与 `SqlitePermissionChecker` 需要新增一个
  事务连接版判定方法，`can_access` 与 `can_access_tx` 共享纯判定函数，避免
  双份决策逻辑漂移；迁移文件改动对存量库无效（已在非目标说明）。
- 待确认问题：无。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-orphan-fk-cascade | 新库 `versions.project_id` 与 `project_grants.project_id` 增加 `REFERENCES projects(id) ON DELETE CASCADE` | 仅 0003/0006 两迁移；`CREATE TABLE IF NOT EXISTS` 且不重建存量表 | 数据库级兜底替代仅靠服务层删除清理 | 新库 assemble 后直删 projects 行级联清空 versions/grants；向不存在 project 直插 versions/grants 报外键错误 | 不改存量库 schema、不改其它表 |
| P-002 | fh-orphan-tx-boundary | `create_version` 与 `grant_collaborator` 在 `BEGIN IMMEDIATE` 事务内完成项目存在性/权限确认与写入 | 仅上述两个方法；事务连接上执行权限判定（`can_access_tx`） | 权限判定抽象增加事务版入口；每次创建多一个短写锁持有 | 并发删除与创建/授权互跑后无孤儿 versions/project_grants 行；权限与错误语义保持 | 不事务化全量 can_access 调用点 |
| P-003 | fh-orphan-concurrency-tests | 回归：FK 级联/违反 + 并发不变量可机器验证 | `server/tests/unit/projects.rs`、`server/tests/unit/versions.rs` | 并发用例为不变量断言（竞态窗口难以保证旧码必红） | FK 用例先 red 后 green；并发用例最终孤儿计数为 0；既有特例不回归 | 不引入专属并发测试基建 |

## Success Criteria

- 新建测试数据库装配后，`PRAGMA foreign_keys` 生效，直删项目行时
  versions/project_grants 关联行级联清除；向不存在的 project_id 直插
  versions/project_grants 被外键拒绝；
- `create_version`/`grant_collaborator` 的确认与写入在同一 `BEGIN IMMEDIATE`
  事务内完成；并发删除与创建/授权跑完后 versions/project_grants 无孤儿行；
- 冲突（重复版本）仍返回 Conflict；项目不存在/被并发删除时创建版本不成功且不
  产生孤儿行（错误语义不劣于现行为：不再出现“写成功了但项目已删”的状态）；
- 既有项目删除清理用例、权限/版本全量既有用例通过；
- 按 standard 流程产出 `docs/changes/039-concurrent-delete-orphan-guard.md`
  与任务包 `completion-report.md`（中文正文），并经 lower-tier-check 校验。

## Risks

- 存量数据库（含本地开发库）不会通过本次迁移获得外键约束；孤儿防护依赖 032
  显式清理与本次服务层事务，残余缺口仅存在于“存量库 + 绕过服务层直写”场景；
- `BEGIN IMMEDIATE` 会让创建版本/授予协作者持有短暂 SQLite 写锁，与被删除操作
  竞争时可能等待或返回 busy 错误（并入 Db 错误路径，客户端可重试），不产生
  静默成功；
- `can_access_tx` 与 `can_access` 共享判定函数，但事务版的项目读取绕过
  `ProjectAccess` 端口直接查 projects 表（在 permissions 边界内使用
  `row_to_project`），后续 schema 变更需同步两处查询；
- 工作树存在大量未提交的在制任务改动（025-038 等），本任务只修改提案列出文件，
  不运行仓库级格式化；全量测试状态受在制内容影响时以定向验证为准并记录。
