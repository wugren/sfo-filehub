task_manifest: task.yaml
status: approved
---

# 删除项目时清理版本与授权关联数据

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Approval Record

- approver: 用户
- approval_date: 2026-08-25
- user_statement: 用户回复「确认」，批准 standard 层级提案并按该层级完成
  实现、验证与收尾。

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 这是 filehub-server 单一 crate 的持久化删除语义缺陷修复（
    `server/src/projects/service.rs` 的 `delete()`），目标是删除项目时把
    `versions`（含 `version_apps`）与 `project_grants` 关联数据一并清干净；
  - 变更面受限于 projects 删除路径与一条回归测试，不改变公开 API 契约、
    不新增/修改数据库 schema 与迁移、不新增依赖；
  - 但删除的是既有持久数据（数据生命周期/失败路径语义），且需要回归测试
    证明关联行不再残留，不满足 trivial 对"无数据/schema 影响 + 极简验证"
    的定义；未发现 high-risk 触发边界（无 schema 迁移、无契约变化、无
    并发/运行时集成面/交叉模块影响），按 standard 默认 bounded bugfix 流程
    执行（pre-edit 基线 -> 实现 -> 验证 -> 变更记录 -> completion-report）。

## Background and Goal

- 现象（用户报告，中危）：删除项目只执行 `DELETE FROM projects WHERE id = ?`
  （`server/src/projects/service.rs:127`）；`versions.project_id`（
  `server/migrations/0006_versions.sql:2`）与 `project_grants.project_id`（
  `server/migrations/0003_roles_grants.sql:6`）均没有外键级联，导致删除项目后
  版本、app 与协作者记录成为僵尸数据。
- 目标：`ProjectService::delete` 成功删除项目时，在同一 SQLite 事务内把该
  项目的版本（含版本内 app）与项目协作者授权记录全部删除干净；未找到项目时
  保持现有 404 语义且不做任何删除。

## Scope

### In scope

- 修改 `server/src/projects/service.rs` 的 `SqliteProjectService::delete`：
  在单个 SQLite 事务内依次删除
  `project_grants WHERE project_id = ?`、
  `version_apps WHERE version_id IN (SELECT id FROM versions WHERE project_id = ?)`、
  `versions WHERE project_id = ?`、
  `projects WHERE id = ?`；
- 删除顺序与事务保证：先按项目 id 删除主行并检查
  `rows_affected == 0`（不存在则回滚并返回 404），再删除关联行后提交；
- 新增一条单元回归测试：创建项目 -> 创建版本并发布 app -> 授予协作者 ->
  删除项目，断言 `project_grants`/`versions`/`version_apps` 中该项目相关行
  计数为 0。

### Out of scope

- 不修改数据库迁移文件（0003/0006）或新增迁移；不依赖 SQLite
  `PRAGMA foreign_keys` 与 FK 级联来完成清理；
- 不改变删除权限语义（账号级 `projects:delete` + token 项目 scope/管理
  校验保持现状）；
- 不处理 `files` 表元数据与磁盘文件（files 是全局表、不含 project_id，
  物理清理沿用既有 startup GC 语义）；
- 不处理 token 的 `project_scope` 文本引用：删除项目不撤销/改写已有 token
  （既定设计现状，作为相邻边界记录）；
- 不动 admin-web/cli 与 API 契约文档。

### Boundary with neighboring modules

- 仅 filehub-server projects -> versions / permissions 的既有删除调用面变化：
  删除项目时额外清理由其它子模块持有的关联行；这两处子模块自身不动。

## Requirement Review

- 需求合理：与用户描述一致（仅 `projects` 表被删，子表无 FK 级联），
  删除项目应把版本与授权元数据一并清除；
- 材料风险/权衡：选用服务层事务内显式 DELETE，而不是"给迁移加
  ON DELETE CASCADE"——后者只对全新库生效、现有库不迁移就无效，且实际是否
  级联还取决于连接是否开启 `PRAGMA foreign_keys`；服务层显式删除对现有库与
  新库都成立，且不带来迁移/回滚面；
- 选择方向：最小改动位于 `delete()` 内部，事务保证全部或全部不删。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-project-delete-related-cleanup | 删除项目时事务内清除该项目全部 `project_grants`、`versions`、`version_apps` 行；项目不存在时回滚并保持 404 | 仅 `server/src/projects/service.rs` 的 delete 与 `server/tests/unit/projects.rs` 回归测试；不依赖 FK/pragma | 显式清理由 projects 服务负责，其它子模块保持不动；事务保证原子性 | 新增单测 red->green（修复前关联行残留、修复后计数 0）；`cargo test -p filehub-server` 全量通过 | 不改迁移/schema、不改删除权限、不清理 files/token scope |

## Success Criteria

- Concrete user-visible or system-visible result: 删除项目后，该项目的版本、
  app 与协作者授权记录不再残留；重复/不存在的项目删除仍返回 404。
- Required evidence:
  - 一条新增单元回归测试（修复前 red：删除后关联表仍有项目行；修复后
    green：三种关联表计数均 0）；
  - `cargo test -p filehub-server` 相关用例与全量测试全绿；
  - `docs/changes/032-project-delete-cleanup.md` 与任务包
    `completion-report.md` 记录变更与独立缺陷发现结论。
- Explicit non-goals: 不引入迁移/后台清理任务；不改变 API 契约；不改变
  文件与 token 生命周期语义。

## Risks

- 低：事务内新增 3 条 DELETE，删除失败会整体回滚（不会出现"项目删了但
  关联行残留"的部分状态）；一次性删除多行扩大锁区间，但均为同一项目内
  的小数据集；
- 相邻边界：token 的 `project_scope` 为文本列、非引用数据，项目删除后
  既有 token 仍按现状有效（scope 指向已删除项目时对应访问将失败），此行为
  不在本次范围内，如需要可后续单独评估 token 清理/撤销策略；
- 无 schema/迁移、无公开契约、无依赖变化；回归测试覆盖删除后的关联表状态。
- Proposal and tier confirmation: 2026-08-25 用户回复「确认」，以 standard
  终值批准，随后按 lower-tier 流程执行（pre-edit 基线 -> 实现 -> 验证 ->
  变更记录 -> completion-report）。
