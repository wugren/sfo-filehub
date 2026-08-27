# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/032-project-delete-cleanup.md

## Delivery Summary
- Outcome:
  - `server/src/projects/service.rs` 的 `SqliteProjectService::delete`：在单个
    SQLite 事务内，先删除 `projects` 主行（`rows_affected == 0` 时返回 404），
    再显式删除该项目全部 `project_grants`、`version_apps`（按该项目 versions
    子查询定位）与 `versions` 行，最后提交；任一失败随事务整体回滚；
  - 不依赖 SQLite `PRAGMA foreign_keys` 与 0003/0006 的列约束，现有库与新库
    均不会在删除项目后残留版本、app 或协作者授权僵尸数据；
  - `server/tests/unit/projects.rs` 新增回归测试
    `delete_project_removes_versions_apps_and_grants`：创建项目 -> 发布版本/app
    -> 授予协作者 -> 删除项目，断言 `projects`/`versions`/`version_apps`/
    `project_grants` 四类行计数均为 0（修复前 red：关联行残留 1 条）。
- Handoff: `cargo test -p filehub-server` 全量通过（4 api_integration + 2 dv +
  33 unit = 39 项）；本任务实际改动限定于 `server/src/projects/service.rs` 与
  `server/tests/unit/projects.rs`（按 pre-edit 基线逐文件核对）；工作区存在
  `026` 等并发在制任务，其编辑（admin-web sha256 相关与 `storage.rs` 等）与本
  任务窗口重叠，因此 canonical changed-path manifest 除本任务 2 个文件外还列出
  并发改动，归属已在下表 regression 行注明，不属本任务交付。

## Proposal Consistency
| Change ID | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-project-delete-related-cleanup | 删除项目时事务内清理全部 project_grants/versions/version_apps；项目不存在回滚并 404；不改迁移/schema、不依赖 FK/pragma、不改删除权限 | proposal.md P-001 + In scope | service.rs delete 事务化四步 DELETE；新单测 red->green；既有 token 删除权限用例与 404 用例保持全绿 | 匹配 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | `service.rs` 全方法、迁移 0003/0006、`version_apps` FK 语义、`connect_pool` 的 `foreign_keys(true)` | 全仓搜索其它删除 projects 的路径（仅 service.rs 一处）；推演 FK 关闭时 `version_apps` 不级联的路径——子查询显式删除仍成立；锁定版本随项目删除属预期整体删除语义 | 修复覆盖项目删除全部关联写入面，无遗漏路径；不依赖 FK/pragma | pass |
| boundaries-and-failure-paths | 新单测与既有 `token_delete_requires_project_scope_and_project_admin`（含 `ProjectId(9999)` 404） | 主行不存在时事务随 Err 自动回滚且不执行关联删除；每个 DELETE 均绑定本项目 id，跨项目行不受影响；空集 DELETE 幂等无害；权限与 token scope 校验顺序未动 | 不存在/越权/404/跨项目隔离边界均收敛 | pass |
| regression-and-side-effects | pre-edit 基线、git diff、全量测试、canonical manifest、baseline 快照 | 对比修复前后行为：HTTP/权限语义不变；无新增依赖/构建变化；新测试不依赖连接层 pragma；逐文件核对本任务增量 | 39 项全绿；首次全量运行时 `unit::storage::ingest_discard_and_orphan_gc` 曾因并发在制任务中间状态（其 test 文件当时正被编辑）出现陈旧构建失败，重跑后 33/33 通过；completion manifest 出现的 admin-web/`storage.rs` 额外路径经 snapshot 对比确认属于并发任务在本任务窗口内的改动，本任务自身仅 2 个文件 | pass |

## Verification
- Targeted check: `cargo test -p filehub-server --test unit_tests
  delete_project_removes_versions_apps_and_grants` red->green（修复前
  versions/apps/grants 残留 1 条、修复后四类计数均 0）；
  `cargo test -p filehub-server` 全量 39 项通过
- Result: pass
- Exception reason: not-applicable

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | tokens.project_scope 为 TEXT 列、非引用行 | 删除项目不撤销/改写 scope 指向已删项目的既有 token；访问已删项目时权限判失败，但 token 记录本身仍存在。属提案明确非目标，可后续单独评估 token 清理/撤销策略 | no |
| F-2 | low | files 为全局表、不含 project_id | 项目删除不同步删除文件元数据与磁盘字节；沿用既有 startup GC 清理无引用文件，符合提案边界 | no |
| F-3 | low | 首次全量 run 的 storage 用例陈旧失败 | 另一在制任务编辑 `server/tests/unit/storage.rs` 期间（中间状态）陈旧指纹导致一次失败；随即重跑 33/33 全绿 | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 提案 P-001 的事务化显式清理与回归测试均已落地并经 red->green 与
  全量 39 项测试验证；独立缺陷发现覆盖行为逻辑、边界与失败路径、回归副作用，
  未发现阻塞性缺陷；F-1/F-2 为提案内非目标边界，F-3 为外部在制任务造成的
  临时构建噪声，均不阻塞交付。
