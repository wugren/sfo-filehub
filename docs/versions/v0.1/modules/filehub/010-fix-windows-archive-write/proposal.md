---
task_manifest: task.yaml
status: approved
---

# Windows 归档写入失败（os error 5）修复

Risk profile: not-created（lower-tier 不创建 risk-profile）

## Workflow Tier Judgment
- Proposed tier: trivial
- Final tier: trivial
- Tier rationale / triggered boundaries:
  - 请求明确、影响集中在 filehub 单一模块 storage 的物理写入路径；不涉及公共 API/协议/CLI 契约、持久化 schema/迁移、安全边界、并发/生命周期、依赖/构建图、产物、发布/部署或跨项目界面，不构成 high-risk。
  - 属于单文件、有界服务端 bugfix，带可用的定向验证信号（server 既有测试 + Windows 端重建验证），符合 trivial 判定。
- Proposal and tier confirmation:
  - 用户于 2026-08-21 回复“确认，修改吧”，采纳 trivial 层级；`workflow_tier` 已写为 `trivial`，本提案置为 `status: approved`。

## Background and Goal
- 现象：在 Windows 上调用 `PUT /api/v1/projects/1/versions/1.0.0/apps/gateway` 上传归档返回 `{"error":"server_error","message":"write archive failed: 拒绝访问。 (os error 5)"}`。
- 根因（已定位）：
  - 错误来自 [server/src/storage/store.rs] 的 `SqliteFileStore::ingest()`；"os error 5 / 拒绝访问" 是 Windows ERROR_ACCESS_DENIED 的本地化文案，运行的是 `target/debug/filehub-server.exe`。
  - 现有写入序列把三步合并成一句错误：写临时文件 `.tmp-<uuid>` → 以只读句柄 `sync_all()` → `rename()`；Windows 上只读句柄 `FlushFileBuffers` 或 `rename` 时句柄未释放/目标被占用都可能返回 ACCESS_DENIED。
- 目标：对 Windows 做最小兼容加固：同步时使用 read+write 句柄、rename 前释放句柄，并把写/同步/改名分别报错，便于后续一步定位；Linux 行为不变。

## Scope
### In scope
- 修改 `server/src/storage/store.rs` 的 `ingest()` 物理写入序列：
  - 临时文件以 read+write 打开后再 `sync_all()`；
  - `rename()` 前显式释放句柄（drop）；
  - 写临时文件、sync、rename 各步骤使用独立错误信息，外层仍统一为 `write archive failed: <步骤> ...`。
- 定向验证：`cargo test -p filehub-server`（含既有 ingest/上传集成测试）通过。
### Out of scope
- 不修改 API 契约、路由、数据库迁移、配置（`data_dir`/`max_archive_bytes`）、CLI 或 admin-web。
- 不改变临时文件/正式文件命名与清理策略，不做目录权限自动修复或安全软件配置变更。
- 不引入 Windows 专属编译条件或新依赖。
### Boundary with neighboring modules
- 仅 touch `server/src/storage/store.rs`，归属 001-filehub-core-platform 的 files 物理存储职责。

## Requirement Review
- 需求合理：写入失败本身是 Windows 运行环境/IO 层面的拒绝；代码侧最小可改进点是消除两个已知 Windows 兼容性弱点（只读句柄 sync、rename 时句柄未释放）并让错误可定位。
- 权衡：read+write 打开临时文件不影响原子性；显式 drop 后 rename 仍是同目录原子改名；分步错误只改变错误文案，不改 API 结构（仍是 500 server_error）。
- 注意：如果环境根因是目录 ACL 或安全软件拦截，代码加固不能消除该根因；本任务同时交付 Windows 侧核验步骤，由用户端完成最终确认。

## Proposal Items
| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-win-archive-write | `ingest()` 使用 read+write 句柄 sync、rename 前释放句柄，并按同步/改名分步报错 | 仅改 `server/src/storage/store.rs` | 兼容 Windows；保留原子写入与既有清理 | `cargo test -p filehub-server` 通过；Windows 重建后上传不再报 os error 5（用户端确认） | 不改 API/配置/CLI/迁移，不加平台条件编译 |

## Success Criteria
- Concrete user-visible or system-visible result:
  - Windows 重建 `filehub-server.exe` 后，同一上传接口不再返回 `write archive failed: 拒绝访问`；若目录级权限问题仍存在，错误会明确指出失败步骤，用户可据此处理 ACL/安全软件。
- Required evidence:
  - `cargo test -p filehub-server` 通过（Linux 侧回归）。
  - 变更后错误消息包含分步信息；Windows 端重建与上传验证由用户执行并反馈。
- Explicit non-goals:
  - 不在本任务内修改 Windows 目录 ACL、防病毒配置或 `data_dir` 位置。

## Risks
- 若用户环境的拒绝来自目录 ACL/受控文件夹访问，本代码修复不能仅凭自身消除；已在交付说明中给出核验步骤，错误消息分步化后可明确区分根因。
- read+write 打开与 rename 前释放句柄不改变 Linux 语义；既有测试覆盖确认无回归。
