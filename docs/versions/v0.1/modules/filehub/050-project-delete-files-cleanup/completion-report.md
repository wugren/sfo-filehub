# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/050-project-delete-files-cleanup.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - `SqliteProjectService` 注入文件存储服务 `FileStore`（`ProjectModule::init` /
    `AppState::assemble` 装配）；`delete()` 在既有单事务内、删除关联行之前先
    收集本项目 versions 引用的 file_id，事务提交后逐个 `files.discard` 即时
    删除 `files` 行并 best-effort 删除物理归档；失败仅记 warn、残留由启动 GC
    兜底。404/403 语义与权限校验顺序不变，事务回滚时不执行 discard。
  - 测试语义同步：API 集成断言从「文件保留到启动 GC」改为「删除即 files 计数 0
    且 data_dir 无 `.tar.gz`、startup GC 无残留」；DV 改为验证两个已发布文件的
    行/归档立即消失 + 单独孤儿文件仍由启动 GC 兜底回收；单元新增
    `delete_project_reclaims_files_immediately`（先 red 后 green）。
- Handoff: `cargo test -p filehub-server -- --test-threads=1` 全量 84 项通过
  （20 api_integration + 2 dv + 62 unit）；clippy 输出无本任务新增告警；
  rustfmt `--check` 仅报告其它在制任务 hunk，本次新增代码段格式清洁，未做
  仓库级格式化（共享工作树规则）。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-delete-files-discard | `SqliteProjectService` 注入文件存储服务 `FileStore`；`delete()` 事务内先收集本项目 versions 引用的 file_id，提交后逐个 `files.discard` 即时清理 files 行与物理归档 | proposal.md P-001 | service.rs 收集查询 + 提交后 discard + warn 兜底；mod.rs/http.rs 装配注入；404/403 路径未触碰 | 匹配 | pass |
| fh-delete-files-tests | 既有 API/DV 测试改为断言「删除即清理」，并新增/保留 GC 兜底与立即回收回归覆盖 | proposal.md P-002 | api_integration/dv_tests 断言改写通过；unit 新增用例先 red 后 green；storage/versions 既有 GC 用例不回归 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | 收集 SQL（version_apps JOIN versions 按 project_id）、收集发生在删除 version_apps/projects 之前、提交后循环 discard、`FileStore::discard` 的引用计数先检与 files 行删除顺序 | 代入「无 app 的版本」（空 file_ids 仍正常销毁）、「files 行缺失」（discard 返回 Ok）、「文件仍被引用」（Conflict → warn 跳过，GC 保留引用内文件）、「项目不存在/无权限」（事务提交前返回，不执行 discard） | 各分支行为正确：不会误删仍在引用的文件；不会在没有删除成功时执行清理；收集先于删除，避免引用行先消失导致漏收 | pass |
| boundaries-and-failure-paths | 事务 rollback 语义、discard 失败仅 warn、startup_gc 兜底路径（`server/src/http/mod.rs` kept set 与 `gc_orphans`）、版本 `UNIQUE(file_id)` schema | 代入「commit 后 discard 失败/进程崩溃」（残留由 startup GC 兜底）、「并发发布插入新 file_id」（晚于收集点的新文件由级联删除 + GC 兜底）、「物理 remove_file 失败」（best-effort，file 行已删，属 031/033 既有边界） | 失败路径均不破坏项目删除成功语义，也不产生持久性泄漏超出既有 GC 兜底窗口；并发窗口与 039 分工后无新增永久孤儿路径 | pass |
| regression-and-side-effects | 全量串行测试 84 项、定向三条用例、clippy 输出、rustfmt --check 差异、git 变更范围（仅本项目 6 个文件 + 任务文档） | 检查 033 的 update/delete_app 即时回收与自引用用例是否受影响（均通过）、startup GC 引用保留用例是否受影响（unit storage/versions 通过）、http/mod.rs 装配是否引入 clippy/编译问题、并行全量出现的失败是否与本次改动有关（单独复跑均绿） | 无回归：84 项全绿；clippy 无本任务新增告警；rustfmt 差异与本文无关；并行全量的 login 502/限流抖动在单独与串行复跑均绿，属既有时序问题（见 F-1） | pass |

## Verification

- Targeted check: `delete_project_reclaims_files_immediately` 临时回退修复后先
  red（files 行残留 1 条，`left: 1 / right: 0`），恢复修复后 green；
  `project_delete_cascades_versions_apps_and_files` 与
  `dv_full_workflow_with_tokens_and_gc` 定向跑通过；`cargo test -p
  filehub-server -- --test-threads=1` 全量 84 项通过；`cargo clippy -p
  filehub-server --tests --message-format short` 无本任务新增告警
- Result: pass
- Exception reason: not-applicable（无需例外；在制工作树格式化差异按共享工作树
  规则保留，不归本任务）

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 两次并行全量各出现 1-3 个与本次改动无关的失败（login 502、限流窗口 10 vs 11、upload 连接抖动）；失败用例单独复跑均通过，串行全量 84 项全绿 | 既有并行运行时序干扰（多测试实例同时登录/固定窗口跨分钟边界），与 050 无关，未在本任务修复（先例：039/042 记录过同类并行抖动） | no |
| F-2 | low | `discard` 的 `remove_physical` 为 best-effort；commit 后 discard 前存在崩溃窗口 | 物理字节残留或孤儿行依赖 startup GC 兜底；模式与 031/033 已记录边界一致，不阻塞交付 | no |
| F-3 | low | `rustfmt --check` 报告 service.rs/mod.rs/projects.rs 等行差异 | 均为其它在制任务未格式化 hunk（含级联涉及的 http.rs），非 050 新增；本任务新增代码段格式清洁 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: P-001/P-002 全部落地：项目删除提交后立即回收其版本引用的 `files` 行
  与物理归档，startup GC 保留为异常兜底；404/403 与权限语义不变；单元回归先
  red 后 green、定向与串行全量（84 项）全绿，clippy 无本任务新增告警；F-1～
  F-3 均为非阻塞低危记录。
