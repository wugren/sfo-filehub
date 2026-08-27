---
task_manifest: task.yaml
status: approved
---

# 删除项目时立即回收关联文件（files 行 + 物理归档）

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Proposal and tier confirmation: 用户 2026-08-26 回复「确认」，采用 standard
  层级与上述 In scope 范围继续执行。
- Tier rationale / triggered boundaries:
  - 修改点是 filehub-server 单个交付面内 projects 子模块的行为修复，并注入复用
    既有 `FileStore` 清理原语；需要联动装配（ProjectModule/AppState）与
    API/DV 回归测试语义调整，属于有界修复；
  - 不满足 trivial：涉及跨 projects/storage 子模块装配与运行期文件生命周期语义
    变更，且要改写既有「文件保留到启动 GC」的集成测试断言，需要 change record
    与完成报告记录设计取舍（与 032/033 同类的 standard 先例）；
  - 未触发 high-risk：仓库为 greenfield 无生产数据；无 schema/迁移改动、无公开
    HTTP/CLI 契约变更（DELETE 仍 204，404/403 语义不变）；物理清理复用已在
    033 验证的 `FileStore::discard` 语义（不再被引用才删除，引用冲突则跳过），
    不引入新的后台任务或外部依赖。

## Approval Record

- approver: 用户
- approval_date: 2026-08-26
- user_statement: 用户 2026-08-26 回复「确认」，采用 standard 层级继续执行。

## Background and Goal

- 现象（评审中危 #3）：`SqliteProjectService::delete`
  （`server/src/projects/service.rs`）只显式清理 projects、project_grants、
  version_apps、versions，没有收集并清理这些版本引用的 files 行及物理归档。
  归档只能等下一次启动时 `AppState::startup_gc`（`server/src/main.rs` 启动阶段
  调用）回收，长期不重启的实例持续占用磁盘，不符合「项目删除后相关数据清理
  干净」的生命周期语义。
- 现状佐证：`server/tests/api_integration.rs` 中
  `project_delete_cascades_versions_apps_and_files` 目前明确断言 files 行与
  `.tar.gz` 归档在删除后仍保留、由启动 GC 回收；该断言本身固化的是旧语义。
- 目标：项目删除成功后，其版本曾引用的 `files` 行与对应物理归档**立即**消失；
  startup GC 仅保留为 crash/竞态等异常残留的兜底防线，不再作为主清理路径。

## Scope

### In scope

- `SqliteProjectService` 注入文件存储服务 `FileStore`（由 `ProjectModule::init` 与
  `AppState::assemble` 装配，复用与 versions 子模块同一文件存储与连接池）；
- `delete()` 在既有单事务内、删除 version_apps 之前先收集本项目 versions 引用
  的 `file_id` 集合；事务提交后对每个 file_id 调用 `files.discard(file_id)` 即时
  回收（files 行 + best-effort 物理删除），失败仅记录 warn 日志并继续，残留由
  startup GC 兜底——与 033 已确立的「提交后 discard + 兜底」模式一致；
- 测试语义调整与回归：
  - `server/tests/api_integration.rs`：删除断言从「文件保留到启动 GC」改为
    「删除即 files 行与归档消失，随后 startup GC 无残留」；
  - `server/tests/dv_tests.rs`：`dv_full_workflow_with_tokens_and_gc` 中
    「删除项目后 GC 清理文件」改为验证删除即时清理 + 单独构造孤儿文件验证
    startup GC 兜底仍有效；
  - `server/tests/unit/projects.rs`：新增删除项目即时回收文件行的回归用例
    （files 行与物理文件均消失）；
- 生成 standard 层级的 `docs/changes/050-project-delete-files-cleanup.md` 与任务包
  `completion-report.md`（中文正文），经 lower-tier-check 校验。

### Out of scope

- 不修改 files/version_apps 的 schema 或引入迁移（`version_apps.file_id` 保持
  UNIQUE，discard 的引用计数检查足以防误删）；
- 不删除或弱化 startup GC 兜底能力；不修改 versions 子模块 publish/delete_app
  路径（033 已实现即时回收）；
- 不改变 DELETE 的 404/403、权限校验顺序、响应码与响应体；
- 不触碰工作树中其他在制任务的未提交改动（025-049 桌面/WEB/CLI 等），不运行
  仓库级格式化。

## Requirement Review

- 需求合理：文件在项目删除后不再可被任何接口访问（项目/版本/app 均消失），
  继续占用 files 行与磁盘只产生不可达资源；「删除即清理」与用户确立的
  「删除时要将相关数据都删除干净」口径一致。
- 方向选择：复用 `FileStore::discard` 而非新增 SQL——该原语先检查
  `version_apps` 引用计数，未引用时删除 files 行并 best-effort 删除物理文件；
  在事务提交后调用，避免磁盘 IO 失败回滚已成功的项目删除，也避免把物理清理
  语义复制到 projects 层。收集 file_id 必须在删除 version_apps 之前完成。
- 材料风险/权衡：commit 与 discard 之间存在极短窗口（进程崩溃会留下孤儿），
  由启动 GC 兜底；discard 的物理删除本身是 best-effort（032/033 既有边界），
  残留缺口与 033 记录一致。
- 待确认问题：无（实现路径、测试口径与层级均按现状确定）。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-delete-files-discard | `SqliteProjectService` 注入文件存储服务 `FileStore`；`delete()` 在事务内先收集本项目 versions 引用的 file_id，提交后逐个 `files.discard` 即时清理 files 行与物理归档 | 仅 projects 子模块删除路径与对应装配点；不新增/修改 public trait 之外的接口 | 与 033 相同：提交后清理，失败仅告警并由 startup GC 兜底 | 删除项目后 files 行计数为 0、data_dir 无对应 `.tar.gz`；404/403 语义保持 | 不改 schema、不改 startup GC、不改 publish/delete_app |
| P-002 | fh-delete-files-tests | 既有 API/DV 测试改为断言「删除即清理」，并新增/保留 GC 兜底与立即回收回归覆盖 | `server/tests/api_integration.rs`、`server/tests/dv_tests.rs`、`server/tests/unit/projects.rs` | 旧断言固化的是错误语义，必须改写；回归用例提供机器可验证证据 | 定向用例先 red 后 green；全量 filehub-server 测试通过；startup GC 兜底单独仍覆盖 | 不引入专用测试基建、不改其他测试语义 |

## Success Criteria

- owner（或具备项目级删除权限的身份）删除项目成功后，该项目版本引用的
  `files` 行与 data_dir 物理归档立即消失，无需重启；
- 删除后调用 `startup_gc` 无本任务路径产生的残留；人工构造的孤儿文件仍可被
  startup GC 回收（兜底不退化）；
- 不存在/无权限删除的 404/403 语义与既有权限校验顺序保持不变；
- `cargo test -p filehub-server`（或 task 定向测试入口）全量既有用例通过；
- 按 standard 流程产出 change record 与 completion-report，lower-tier-check
  completion 校验通过。

## Risks

- commit 后、discard 前的进程崩溃会让少量 files 行/归档残留到下次启动 GC——
  属设计内兜底窗口，与 033 的既有模式一致，不造成数据丢失；
- 并发场景：同一 file_id 不可能被新发布引用（ingest 每次生成新 UUID，且
  `version_apps.file_id` UNIQUE），discard 还会在删除前复检引用计数，冲突则跳过
  不误删；
- 工作树含大量在制未提交改动，全量测试可能受其他任务内容影响；本任务改动
  严格限定在提案文件列表，以定向验证为准并记录于完成报告。
