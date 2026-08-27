# 更新/删除 app 后立即回收旧物理文件

- Status: complete
- Owner module: filehub（filehub-server versions 子模块）
- Task manifest: `docs/versions/v0.1/modules/filehub/033-version-app-file-cleanup/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/033-version-app-file-cleanup/proposal.md`
- Affected paths: `server/src/versions/service.rs`、`server/src/versions/mod.rs`、
  `server/src/http/mod.rs`、`server/tests/unit/versions.rs`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 缺陷定位：`publish_app` 用
  `INSERT ... ON CONFLICT(version_id, app) DO UPDATE SET file_id = excluded.file_id`
  直接覆盖版本 app 的引用，`delete_app` 直接 `DELETE` 引用行；两条路径都不回收
  原文件，被替换/删除的旧引用只能等服务重启后的 startup GC 清理，长时间运行磁盘
  占用持续增长。
- 最小修复：`SqliteVersionService` 注入 `Arc<dyn FileStore>`（由
  `VersionModule::init` / `AppState::assemble` 装配，复用同一 db 连接池），复用既有
  `FileStore::discard` 的清理语义——该原语在 `version_apps` 中不再有引用时删除
  `files` 行并 best-effort 删除物理文件：
  - `publish_app`：事务内把 existing 探测改为 `SELECT file_id`，捕获旧 `file_id`；
    事务提交后当旧 id 与新 id 不同时调用 `files.discard(old)`；重复发布同一
    `file_id`（自引用更新）跳过回收，不误删仍在使用的文件；
  - `delete_app`：select 被删行 `file_id` 与 `DELETE` 放入同一事务（保留
    `rows_affected == 0` 的 404 语义），提交后调用 `files.discard(old)`；
  - 两处清理失败都只记 warn 日志，不回滚已提交的引用变更；残留孤儿仍由 startup
    GC 兜底，接口成功/失败语义与响应码不变。
- 正确性依据：`version_apps.file_id` 有 `NOT NULL UNIQUE`（`0006_versions.sql`），
  被本路径替换/删除的旧文件不可能再被其它版本 app 行引用，discard 的 Conflict 分支
  在现 schema 下只会命中自引用（已跳过）。
- 回归测试：`server/tests/unit/versions.rs` 新增两条用例——更新 app 后旧文件
  `files` 行与 data_dir 物理文件立即消失（修复前 red：旧行残留 1 条）、删除 app 后
  文件同样立即消失且 startup GC 无残留；自引用重复发布不误删且引用集仍包含该文件。

## Risk Screen

- Public contract, protocol, or CLI change: no（HTTP 语义与响应码不变）
- Persistent data, schema, or migration change: no（无 schema/迁移改动；复用
  `files.discard` 现有清理语义）
- Security, privacy, or trust-boundary change: no（权限校验与顺序未动；旧文件按与
  启动 GC 相同的“不再被引用即回收”规则即时释放）
- Concurrency, lifecycle, or runtime integration change: yes——本次修复目标正是运行时
  文件生命周期：更新/删除提交后即时回收旧文件；无新增后台任务/并发机制，清理失败
  降级为既有 startup GC 兜底
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: 新增
  `update_and_delete_app_reclaim_old_files_immediately` 先 red（更新后旧文件
  `files` 行残留 1 条）后 green（files 行与物理文件均消失、startup GC 无残留）；
  `republish_same_file_keeps_reference` 自引用不误删；既有
  `delete_app_and_missing_targets`（404 语义）保持通过
- Result: pass
- Residual risk or follow-up: `discard` 的物理删除本身是 best-effort——Windows
  等环境下文件句柄被占用时 `remove_file` 失败，`files` 行已删后该字节不再被 startup
  GC 命中；此为存储层既有行为（031 已沿用 discard），属相邻边界非本次引入；另
  `upload_security_boundaries` 在个别全量并行运行中出现过 1 次 `Connection reset by
  peer` 瞬时失败（匿名上传鉴权前置关闭连接与 reqwest 写 body 的既有竞态），单独与
  连续全量复跑均通过，与本次改动无关
