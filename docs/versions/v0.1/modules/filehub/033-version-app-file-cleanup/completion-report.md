# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/033-version-app-file-cleanup.md

## Delivery Summary
- Outcome: 更新/删除 app 成功后立即回收旧文件（files 行 + 物理文件），清理失败仅
  warn 并由启动 GC 兜底，接口语义不变；详情：
  - `SqliteVersionService` 注入 `Arc<dyn FileStore>`（`server/src/versions/mod.rs`
    `VersionModule::init` 与 `server/src/http/mod.rs` `AppState::assemble` 完成装配，
    复用同一 db 连接池）；
  - `publish_app`：事务内 existing 探测改为读取旧 `file_id`，upsert 提交后当旧 id 与
    新 id 不同时调用 `files.discard(old)`；重复发布同一 file_id（自引用更新）跳过
    回收；
  - `delete_app`：select 被删行 `file_id` 与 `DELETE` 放入同一事务（保留
    `rows_affected == 0` 的 404 语义），提交后调用 `files.discard(old)`；
  - 两处清理失败只记 warn 日志、不影响已提交的引用变更与接口响应；残留孤儿由启动
    GC 兜底；`server/tests/unit/versions.rs` 新增两条回归测试
    `update_and_delete_app_reclaim_old_files_immediately` 与
    `republish_same_file_keeps_reference`。
- Handoff: `cargo test -p filehub-server` 全量 41 项（4 api_integration + 2 dv +
  35 unit）连续 3 次全绿；本任务实际改动限定于 `server/src/versions/service.rs`、
  `server/src/versions/mod.rs`、`server/src/http/mod.rs`、
  `server/tests/unit/versions.rs` 与任务包/变更记录文档（按 pre-edit 基线逐文件
  核对）；工作区存在 026 等并发在制任务，其编辑（authz/upload/max_archive_bytes 等）
  早于本任务起点、已在 pre-edit 基线快照中，completion manifest 差异归属详见下表
  regression 行。

## Proposal Consistency
| Change ID | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-version-app-immediate-file-cleanup | 更新/删除 app 成功后，对不再被引用（且非自引用重复发布）的旧 file_id 立即 `files.discard`；清理失败仅 warn、由启动 GC 兜底；不改 schema/迁移、存储层 discard/gc 语义与 HTTP 契约 | proposal.md P-001 + In scope | service.rs 注入 FileStore 并新增 post-commit discard；delete_app 事务化 select+delete；两条新单测 red->green；`version_lifecycle_*`/`delete_app_and_missing_targets` 等既有用例保持全绿 | 匹配 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | service.rs publish_app/delete_app 修订后全方法、迁移 0006 `version_apps.file_id NOT NULL UNIQUE`、store.rs `discard`/`gc_orphans` 实现、全仓 `version_apps` 写入点（仅 service.rs 两处 + 032 projects 删除路径） | 推演 UNIQUE 约束下替换/删除后旧文件必不再被其它行引用（该行是唯一引用者），discard 的 Conflict 分支只会命中自引用（已跳过）；推测并发双更新对同一旧文件时先 discard 删行、后者 files 行缺失返回 Ok，幂等无二次副作用；自引用更新（同 id 不同 sha256/size）不触发回收 | 修复覆盖更新与删除两条用户报告的泄漏路径，无遗漏写入面；自引用与并发幂等语义收敛 | pass |
| boundaries-and-failure-paths | 新单测、既有 `delete_app_and_missing_targets`/`version_lifecycle_create_publish_update_lock_delete`、`require_unlocked` 与权限校验顺序、http.rs 上传 handler 的 publish 失败 discard 语义 | 删除不存在 app 时事务内 select 无行即返回 404（事务回滚）；`rows_affected == 0` 防御分支保留原 404 语义；锁定版本在进入事务前即被拒，顺序不变；discard 的 Db/IO 失败走 warn 分支不回滚引用变更，接口响应码不变；清理与 startup_gc 目标集合一致（`referenced_file_ids` 为空时才删除） | 不存在/锁定/越权/404、清理失败降级、与 GC 的边界均收敛；HTTP 上传新文件失败路径的 discard 语义不受影响 | pass |
| regression-and-side-effects | pre-edit 基线、git diff、全量测试 run（多次）、`version_apps.file_id` UNIQUE schema、`FileStore` 模块依赖方向 | 全仓搜索 `SqliteVersionService::new`/`VersionModule::init` 仅两处调用，装配签名变更无遗漏；对比修复前后：更新/删除接口返回码与 body 不变，旧文件由「等重启 GC」变为「提交后即时回收」；无新增依赖/构建变化；新测试未触碰并发在制任务的 admin-web/storage 文件；项目删除路径（032）仍按既有记录留给启动 GC，未越界扩展 | 41 项全量测试连续 3 次全绿；首次全量运行时 api_integration 的 `upload_security_boundaries` 偶发 1 次 Connection reset（匿名上传鉴权前置关连接与 reqwest 写 body 的既有竞态），单独与后续全量复跑均通过，详见 F-3；completion manifest 中并发任务文件（authz/upload/http 装配等）经 baseline 对比确认早于本任务基线 | pass |

## Verification
- Targeted check: `cargo test -p filehub-server --test unit_tests update_and_delete_app_reclaim_old_files_immediately` red->green（修复前更新后旧文件 files 行残留 1 条、修复后 files 行与 data_dir 物理文件均消失且 startup GC 无残留）；`republish_same_file_keeps_reference`、`delete_app_and_missing_targets` 通过；`cargo test -p filehub-server` 全量 41 项（4 api_integration + 2 dv + 35 unit）连续 3 次全绿
- Result: pass
- Exception reason: not-applicable

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | store.rs `discard`/`gc_orphans` 先删 `files` 行、`remove_physical` best-effort 忽略失败 | Windows 等环境下文件句柄被占用时物理删除失败，files 行已删后该字节不再被 startup GC 命中；这是存储层既有设计（031 已沿用 discard），本次任务继承而未加重；如需可后续单独立项（例如 files 行保留至物理删除成功） | no |
| F-2 | low | 032-project-delete-cleanup 记录 + proposal 非目标 | 项目删除路径仍把 files/磁盘字节留给 startup GC，不随更新/删除-app 一并即时清理；属 032 明确边界，不在本任务范围 | no |
| F-3 | low | 首次全量 run 的 api_integration `upload_security_boundaries` 偶发 Connection reset by peer | 鉴权前置关闭连接（026 设计）与 reqwest 多部分 body 写入的瞬时竞态；单独重跑与后续连续 3 次全量复跑均通过，与本次改动无因果（本任务不触碰上传/鉴权路径） | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 提案 P-001 的两条即时回收路径（更新、删除）均已落地并通过 red->green 与
  全量 41 项测试（连续 3 次）验证；独立缺陷发现覆盖行为逻辑、边界与失败路径、回归
  副作用，SCP 与并发在制任务影响均收敛；F-1/F-2 为既有存储层设计边界与 032 非目标，
  F-3 为无关瞬时竞态，均不阻塞交付。
