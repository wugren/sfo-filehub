task_manifest: task.yaml
status: approved
---

# 更新/删除 app 后立即回收旧物理文件

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Approval Record

- approver: 用户
- approval_date: 2026-08-25
- user_statement: 用户回复「确认」，批准 standard 层级提案并按该层级完成实现、验证与收尾。

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 这是 filehub-server 单一 crate 的生产代码缺陷修复（`server/src/versions/service.rs`
    `publish_app` upsert 与 `delete_app` 两条路径不回收被替换/删除引用指向的旧文件），
    目标是复用已有 `FileStore::discard` 语义，在引用变更提交后立即清理不再被引用的
    `files` 行与磁盘文件；
  - 变更面限于 versions 服务（注入 `FileStore`）与一条单测回归，不改变公开 API 契约、
    不修改数据库 schema/迁移、不新增依赖；
  - 但变更的是文件物理生命周期/运行时清理语义（更新与删除路径引入即时回收），并需要
    回归测试证明旧文件不再残留，不满足 trivial 对“无 lifecycle/runtime integration
    实质影响 + 极简验证”的定义；未发现 high-risk 触发边界（无 schema 迁移、无契约变化、
    无并发/后台任务/交叉模块架构改动，`FileStore::discard` 为既有清理原语），按
    standard 默认 bounded bugfix 流程执行（pre-edit 基线 -> 实现 -> 验证 -> 变更记录 ->
    completion-report）。
- Proposal and tier confirmation: 2026-08-25 用户回复「确认」，以 standard 终值批准，
  随后按 lower-tier 流程执行（pre-edit 基线 -> 实现 -> 验证 -> 变更记录 ->
  completion-report）。

## Background and Goal

- 现象（用户报告，中低危）：`publish_app` 更新 app 时用 upsert 直接覆盖
  `version_apps.file_id`，`delete_app` 删除 app 时直接移除引用行；两条路径都不回收
  原 `files` 行与磁盘物理文件（`server/src/versions/service.rs:227` 起的 upsert 与
  `delete_app`）。旧引用只能等服务重启后的 startup GC 清理，长时间运行会持续增长磁盘
  占用。
- 目标：更新/删除 app 成功且旧文件不再被任何版本 app 引用时，立即回收旧文件的
  `files` 元数据行与物理字节；仍被引用（含重复发布同一 file_id 的自引用）时保留，不
  误删。

## Scope

### In scope

- `SqliteVersionService` 增加 `Arc<dyn FileStore>` 依赖，并由
  `VersionModule::init` / `AppState::assemble` 完成装配（复用已有 db 连接池）；
- `publish_app`：更新分支（app 行已存在）在事务内捕获旧 `file_id`；事务提交后若旧
  file_id 与新 file_id 不同，调用 `files.discard(&old_file_id)` 即时回收；
- `delete_app`：删除行前读取被删行的 `file_id`（select+delete 放入同一事务，避免并发
  publish 换文件与删除交错），成功后调用 `files.discard(&old_file_id)`；
- 清理失败不回滚已提交的版本/引用变更：记录 warn 日志，由 startup GC 兜底后续回收；
- 新增回归测试：更新 app 后旧文件 `files` 行与 data_dir 物理文件立即消失；删除 app
  后文件同样立即消失；重复发布同一 file_id 不误删引用文件。

### Out of scope

- 不修改 `server/migrations/0005_files.sql` / `0006_versions.sql` 或新增迁移（
  `version_apps.file_id UNIQUE` 语义保持不变）；
- 不修改存储层 `discard` / `gc_orphans` 实现与语义；
- 不改变 HTTP 契约与响应码（成功路径仍返回既有 200/201/204，错误路径不变）；
- 不新增后台定时 GC 或启动时清理机制；
- 不处理 032 已记录的项目删除边界（files 为全局表，项目删除仍沿用 startup GC）；
- 不动 admin-web / cli / API 契约文档。

### Boundary with neighboring modules

- filehub-server versions -> storage 的既有调用面：版本服务新增持用 `FileStore`，
  存储层与上传 handler 的 discard 语义不变；任一侧失败都有 startup GC 兜底。

## Requirement Review

- 需求合理：与用户描述一致（更新/删除 app 两条路径都不回收旧物理文件），复用
  `FileStore::discard` 的“仍被引用则 Conflict、未引用则删 files 行+物理文件”语义即可
  收敛到“引用变更后立即回收”，与启动 GC 的目标集合一致；
- 材料风险/权衡：
  - 即时清理放在事务提交之后，避免回滚导致的新文件/旧文件状态不一致：DB 引用变更
    先原子生效，清理只是其后的资源回收；清理失败回退到既有 startup GC 兜底；
  - 相同 file_id 重复发布（自引用更新）时跳过 discard，避免把“仍在被引用”误判为
    可删；
  - `discard` 的物理删除本身是 best-effort（`remove_physical` 忽略删除失败）；若
    Windows 等环境删除物理文件失败，files 行已删、字节无法再被 startup GC 命中——
    这是存储层既有设计（031 已沿用 discard），如实记录为相邻缺陷、不扩
    大本次范围；
- 选择方向：服务层注入 `FileStore` 并在 `publish_app`/`delete_app` 提交后调用既有的
  `discard`，而不是在 HTTP handler 层补救——版本服务是所有调用方（HTTP/单测/未来
  CLI）的共同入口，且 handler 层拿不到被替换的旧 file_id。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-version-app-immediate-file-cleanup | 更新/删除 app 成功后，对不再被引用（且非自引用重复发布）的旧 file_id 立即执行 `files.discard`，回收 files 行与物理文件 | 仅 versions 服务（含 mod/装配）与 `server/tests/unit/versions.rs` 回归测试；存储层 discard/gc 语义不变 | 提交后清理、失败仅 warn 并由 startup GC 兜底，保证 API 语义不回滚；同时物理删除失败属既有 best-effort 边界 | 新增单测 red->green：更新/删除后旧文件 files 行与 data_dir 文件立即消失；自引用更新不误删；`cargo test -p filehub-server` 全量通过 | 不改 schema/迁移/存储层/HTTP 契约；不新增后台清理任务 |

## Success Criteria

- Concrete user-visible or system-visible result: 更新 app 使旧文件不可被引用、删除 app
  后，对应 `files` 元数据行与 data_dir 物理字节立即被回收，不再依赖服务重启后的
  startup GC；长时间运行磁盘占用不再因这两条路径持续增长。
- Required evidence:
  - 一条版本服务单测回归（修复前 red：更新/删除后旧文件 files 行与物理文件仍在；
    修复后 green：均已消失）；
  - 自引用边界用例：重复发布同一 file_id 后文件仍被引用、未被误删；
  - `cargo test -p filehub-server` 相关用例与全量测试全绿；
  - `docs/changes/033-version-app-file-cleanup.md` 与任务包 `completion-report.md`
    记录变更与独立缺陷发现结论。
- Explicit non-goals: 不引入迁移/后台清理任务；不改变 API 契约与响应码；不改变
  `discard`/startup GC 语义。

## Risks

- 低：即时回收在事务提交后执行，`discard` 失败不会影响已提交的引用变更（降级为启动
  GC 兜底并记录 warn）；`discard` 为既有原语，包含“仍被引用则拒绝删除”的冲突保护，
  不会误删仍被其它行引用的文件；
- 相邻边界：物理文件删除是存储层 best-effort（Windows 占用句柄等场景可能失败，且
  files 行已删后字节不再可被 GC 命中）——031 已沿用该语义，属既有设计遗留，不扩
  大本次范围，如需可后续单独立项；
- 无 schema/迁移、无公开契约、无依赖变化；回归测试覆盖更新、删除与自引用三条路径。
