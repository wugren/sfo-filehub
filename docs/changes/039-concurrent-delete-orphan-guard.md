# 项目删除与创建版本/协作者授权并发时防孤儿数据

- Status: complete
- Owner module: filehub（filehub-server permissions + versions 子模块）
- Task manifest: `docs/versions/v0.1/modules/filehub/039-concurrent-delete-orphan-guard/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/039-concurrent-delete-orphan-guard/proposal.md`
- Affected paths: `server/migrations/0003_roles_grants.sql`、
  `server/migrations/0006_versions.sql`、`server/src/account/store.rs`、
  `server/src/permissions/mod.rs`、`server/src/permissions/checker.rs`、
  `server/src/versions/service.rs`、`server/tests/unit/projects.rs`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 缺陷定位：`create_version()` 的权限检查（`SqlitePermissionChecker::can_access`
  池查询）与 `INSERT INTO versions` 之间无事务边界，`versions.project_id` 与
  `project_grants.project_id` 又无外键；项目在确认后被并发删除时，INSERT 仍可
  成功，重新产生孤儿版本/授权行。`grant_collaborator` 的确认与 INSERT 同样分离。
- 修复分两层：
  - schema 层（新库生效）：0003/0006 给 `project_grants.project_id`、
    `versions.project_id` 增加 `REFERENCES projects(id) ON DELETE CASCADE`；
    `connect_pool` 的内存库分支同样开启 `PRAGMA foreign_keys`，保证新建库
    （含内存库）外键级联语义一致；存量库不做 ALTER/表重建，继续由
    032 的服务层显式清理兜底；项目删除的显式清理逻辑保留；
  - 事务层：`create_version` 与 `grant_collaborator` 改为
    `BEGIN IMMEDIATE` 事务，权限/owner/用户存在性确认与 INSERT 全部在同一
    事务连接上完成（`PermissionChecker` 新增 `can_access_tx` 事务连接版入口，
    与池版 `can_access` 共用纯判定函数），SQLite 单写者串行化保证删除要么先
    提交（确认失败）、要么在写锁之后提交并被级联清除；INSERT 落入
    `FOREIGN KEY constraint failed` 时映射为项目不存在，不再静默成功。
- 一致性保障：`create_version` 的事务作用域在聚合查询前结束，连接及时归还
  连接池，避免池单连接场景下的二次取连接超时（实现期自查发现并修复）。

## Risk Screen

- Public contract, protocol, or CLI change: no（正常路径 HTTP 语义与错误码
  不变；并发删除场景不再出现“写成功但项目已删”；FK 兜底路径返回 404 语义）
- Persistent data, schema, or migration change: yes——0003/0006 为新库补充外键
  约束（`CREATE TABLE IF NOT EXISTS`，无 ALTER、不重建存量表），内存库开启 FK
  pragma；本仓库为 greenfield 无生产数据，存量开发库的孤儿防护继续依赖
  032 显式清理
- Security, privacy, or trust-boundary change: no（权限判定语义、校验顺序与
  错误文案不变，仅判定所在事务连接变化）
- Concurrency, lifecycle, or runtime integration change: yes——本次修复目标正是
  确认与写入的并发边界：两处写入改为 `BEGIN IMMEDIATE` 单事务，SQLite 单写者
  锁串行化“确认 → 写入”，无新增后台任务；争锁时等待（busy 超时后按 Db 错误
  返回，可重试），不产生孤儿数据
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: 新增 `project_fk_cascade_prevents_orphan_versions_and_grants`
  （直删项目行断言 versions/project_grants 级联清零，向不存在项目直插两表均被
  外键拒绝）与 `concurrent_project_delete_and_child_creates_leave_no_orphans`
  （删除/创建/授权三任务并发后孤儿计数为 0）均通过；schema 级 red/green 对比
  （旧 DDL 直删项目残留 1 版本 + 1 授权，新 DDL 全部级联清除）验证修复方向；
  `cargo test -p filehub-server` 全量 54 项通过（46 unit + 2 dv + 6
  api_integration），`cargo clippy -p filehub-server --tests` 无新增告警
- Result: pass
- Residual risk or follow-up: 存量库不会通过本次迁移获得外键，孤儿防护依赖
  服务层显式清理；如需给存量库补约束需表重建迁移（提案明确非目标）；并发用例
  是终态不变量断言，竞态窗口未必保证旧码必红，确定性由 FK 用例提供
