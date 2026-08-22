---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-19
approved_content_sha256: 880323e68be884282abe0593a2a7cf24818a8a8c4c8ff2ae7df3c6307375a691
---
## Approval Record

- approver: user
- approval_date: 2026-08-19
- user_statement: 确认，自动完成001任务吧


# files 子模块设计（P-04 fh-server-files）

## Design Scope

- 归属：`filehub-server` crate 内 `server/src/storage/` 子 mod。
- 覆盖：`.tar.gz` 物理文件管理、`data_dir` 布局、归档格式与大小上限校验、原子写入、SHA-256 完整性、下载流、路径防穿越、失败回滚与孤儿清理。
- 不覆盖：版本/项目语义、权限判定、上传协议组装（http 模块负责）。

## Module Relationship UML

```mermaid
classDiagram
  direction LR
  class store { 原子入库/读取 }
  class integrity { SHA-256 与路径安全 }
  class http { 上传/下载 handlers }
  store --> integrity
  http --> store
```

## File-Level Interfaces

```rust
pub struct FileId(pub String);
pub struct FileRecord { pub file_id: FileId, pub sha256: String, pub size: u64 }

pub trait FileStore {
    async fn ingest(&self, source: UploadStream, expected_sha256: Option<&str>) -> Result<FileRecord, FileStoreError>;
        // 校验：流式读取时校验 gzip magic + tar 结构（仅接受 .tar.gz），超限 files.max_archive_bytes 拒绝；
        // 写入失败/校验失败/连接中断时删除临时文件，不产生可见文件
    async fn open_read(&self, file_id: &FileId) -> Result<DownloadStream, FileStoreError>;
    async fn discard(&self, file_id: &FileId) -> Result<(), FileStoreError>;
        // 发布落库失败（含 409）后的立即回滚：删除未被任何版本引用的文件
    async fn gc_orphans(&self, keep: &HashSet<FileId>) -> Result<Vec<FileId>, FileStoreError>;
        // 启动/恢复回收：keep 来自 versions.referenced_file_ids()，清理崩溃残留
}
```

- Consumer: `versions`（发布时 ingest/引用）、`http`（下载流路由与发布失败回滚分支）；change_id `fh-server-files`
- Compatibility: new
- Migration path when required: 不适用（greenfield）

## State and Ownership

- Owner: `files` 索引表 + `data_dir` 物理字节；归档上限来自装配配置 `[files] max_archive_bytes`
- Access path for other modules: `FileStore` trait；版本模块只见 `FileId`
- Invariants: 发布时先写临时文件 -> 校验归档格式 + SHA-256 -> 落位；可读文件永不半成品；路径防穿越；非 `.tar.gz` 与超限上传在入库前拒绝（422）

## Change Mapping

| change_id | target_module | proposal_id | Design Coverage | Scope Paths |
|-----------|---------------|-------------|-----------------|-------------|
| fh-server-files | filehub | P-04 | 本文件 + design.md FileStore | `server/src/storage/`, `server/migrations/0005_files.sql`, `tests/` |

## Design Notes

- 只提供文件级能力，不感知版本/项目。
- 格式校验：接受流式 gzip/tar 结构判定（不信任文件名后缀），其它归档格式一律 `InvalidInput`（422）；`expected_sha256` 缺失时以落盘计算为准，存在时校验一致后方可落位。
- 回滚链路：`http` 发布流程在 `versions.publish` 返回失败（含 409）时调用 `discard(file_id)`；进程中断残留由启动时的 `gc_orphans(keep = versions.referenced_file_ids())` 回收。"引用计数"语义 = 被 `versions/version_files` 引用的文件集合，不新增计数表。
