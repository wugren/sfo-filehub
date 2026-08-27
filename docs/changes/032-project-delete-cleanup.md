# 删除项目时清理版本与授权关联数据

- Status: complete
- Owner module: filehub（filehub-server projects 子模块）
- Task manifest: `docs/versions/v0.1/modules/filehub/032-project-delete-cleanup/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/032-project-delete-cleanup/proposal.md`
- Affected paths: `server/src/projects/service.rs`、`server/tests/unit/projects.rs`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 缺陷定位：`SqliteProjectService::delete` 只执行
  `DELETE FROM projects WHERE id = ?`；`versions.project_id` 与
  `project_grants.project_id` 在迁移 0003/0006 中均为无外键约束的普通
  INTEGER 列，`version_apps` 虽对 `versions` 有 `ON DELETE CASCADE`，但版本
  行本身不会随项目删除，因此版本、app 与协作者授权记录全部残留为僵尸数据。
- 最小修复：不依赖外键/pragma，在 `delete()` 内使用单个 SQLite 事务依次显式
  删除 `project_grants`、`version_apps`（以该项目的 versions 子查询定位）、
  `versions`，最后删除 `projects` 主行；主行不存在时返回 404（事务随 Err
  回滚）。任一删除失败都会整体回滚，不产生部分删除状态。
- 回归测试：`server/tests/unit/projects.rs` 新增
  `delete_project_removes_versions_apps_and_grants`——创建项目、创建并发布
  版本/app、授予协作者后删除项目，直接对 SQLite 计数断言 project/版本/app/
  授权四类行全部为 0。修复前该用例失败（versions/apps/grants 残留），修复后
  通过。

## Risk Screen

- Public contract, protocol, or CLI change: no（HTTP 语义不变：成功 200/204，
  不存在仍 404，权限校验顺序不变）
- Persistent data, schema, or migration change: no（无 schema/迁移改动；运行
  期删除项目时按需求清除关联行）
- Security, privacy, or trust-boundary change: no（协作者授权记录随项目删除，
  不会再指向已删项目；删除权限语义未变）
- Concurrency, lifecycle, or runtime integration change: no（单事务内聚合删除，
  无新增并发/后台任务；同一项目小数据集锁区间有限）
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: 回归测试先 red（修复前 versions/apps/grants 计数残留）后
  green（四类计数均 0）；`cargo test -p filehub-server` 全量 39 项通过
  （4 api_integration + 2 dv + 33 unit）
- Result: pass
- Residual risk or follow-up: token 的 `project_scope` 为文本列，删除项目不
  撤销/改写既有 token（相邻边界，记录于提案非目标）；`files` 表元数据与磁盘
  文件为全局资源，物理清理沿用 startup GC，不随项目删除即时清除。
