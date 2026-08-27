# 删除项目时立即回收关联文件（files 行 + 物理归档）

- Status: complete
- Owner module: filehub（filehub-server projects 子模块）
- Task manifest: `docs/versions/v0.1/modules/filehub/050-project-delete-files-cleanup/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/050-project-delete-files-cleanup/proposal.md`
- Affected paths: `server/src/projects/service.rs`、`server/src/projects/mod.rs`、
  `server/src/http/mod.rs`、`server/tests/api_integration.rs`、
  `server/tests/dv_tests.rs`、`server/tests/unit/projects.rs`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 缺陷定位：`SqliteProjectService::delete` 只显式清理 projects/project_grants/
  version_apps/versions，没有收集这些版本引用的 file_id；`files` 行与 data_dir
  物理归档只能等下次启动 GC 回收，长期不重启的实例持续占用磁盘。
  `server/tests/api_integration.rs` 原有断言明确固化「文件保留到启动 GC」的旧
  语义，与「项目删除后相关数据清理干净」的生命周期口径冲突（评审中危 #3）。
- 最小修复：`SqliteProjectService` 注入文件存储服务 `FileStore`（由
  `ProjectModule::init` / `AppState::assemble` 装配，复用 033 已验证的同一
  `FileStore::discard` 清理原语）；`delete()` 在既有单事务内、删除
  version_apps/projects 之前先用
  `SELECT va.file_id FROM version_apps va JOIN versions v ON v.id = va.version_id
  WHERE v.project_id = ?` 收集 file_id，事务提交后逐个 `files.discard(file_id)`
  即时删除 `files` 行并 best-effort 删除物理归档；清理失败仅记 warn 日志，
  残留孤儿仍由启动 GC 兜底（与 033 的「提交后 discard + 兜底」模式一致）。
- 正确性依据：`version_apps.file_id` 为 `NOT NULL UNIQUE`，被删除项目引用的
  file_id 不可能被其它版本 app 行引用；`discard` 删除前还会复检
  `version_apps` 引用计数（Conflict 则跳过），不存在跨项目共享文件误删；
  404/删除无权限路径在提交前返回，不会误执行 discard。
- 测试调整与回归：
  - `api_integration.rs`：`project_delete_cascades_versions_apps_and_files`
    从「files 行与归档保留到启动 GC」改为断言删除后 files 计数 0、data_dir
    无 `.tar.gz`，随后 startup GC 无残留；
  - `dv_tests.rs`：`dv_full_workflow_with_tokens_and_gc` 改为断言 f1/f2 的
    files 行与物理归档在删除后立即消失；另构造一个未被版本引用的孤儿文件，
    验证 startup GC 兜底仍只回收孤儿、不把已即时清理的文件重新处理；
  - `unit/projects.rs`：新增 `delete_project_reclaims_files_immediately`
    回归用例（先 red：修复前删除后 files 行残留 1 条；后 green：行与归档均
    立即消失）。

## Risk Screen

- Public contract, protocol, or CLI change: no（DELETE 语义不变：成功 204、
  不存在 404、无权限 403，错误码与错误文案不动）
- Persistent data, schema, or migration change: no（无 schema/迁移改动；复用
  `files.discard` 现有清理语义）
- Security, privacy, or trust-boundary change: no（权限校验与顺序未动；按与启动
  GC 相同的「不再被引用即回收」规则即时释放，回收范围只限被删项目版本曾引用的
  文件）
- Concurrency, lifecycle, or runtime integration change: yes——本次修复目标正是
  运行期文件生命周期语义：项目删除提交后立即回收其版本引用的 files 行与物理
  归档；无新增后台任务/并发机制，清理失败降级为既有 startup GC 兜底
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no（`ProjectModule::init`
  装配点局限 filehub-server 内部，无外部 crate/API 变化）

## Verification

- Targeted check: 新增 `delete_project_reclaims_files_immediately` 先 red
  （修复前断言 files 行残留 1 条失败）后 green（files 行与物理归档均即时消失）；
  `project_delete_cascades_versions_apps_and_files`（API 集成）与
  `dv_full_workflow_with_tokens_and_gc`（DV）定向跑均通过；
  `cargo test -p filehub-server -- --test-threads=1` 全量 84 项通过
  （20 api_integration + 2 dv + 62 unit）；clippy 无本任务新增告警
- Result: pass
- Residual risk or follow-up: `discard` 的物理删除是 best-effort（Windows 等
  环境下文件句柄被占用时 `remove_file` 失败，files 行已删后该字节不再被 startup
  GC 命中，为 031/033 记录的存储层既有边界）；commit 与 discard 之间的进程崩溃
  窗口会残留孤儿文件，由下次启动 GC 兜底；并发发布与项目删除撞车的极端窗口下，
  晚于收集点插入的全新 file_id 由启动 GC 兜底回收（039 已覆盖版本/授权创建
  竞态，publish_app 竞态属既有相邻边界）
