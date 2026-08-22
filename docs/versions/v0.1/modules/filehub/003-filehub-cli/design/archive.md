---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-20
approved_content_sha256: dfad6e2b9ef74455db67e4878322648c94842eeec972d7a240032f2b71920f2e
---

## Approval Record

- approver: user
- approval_date: 2026-08-20
- user_statement: 自动完成003任务吧


# archive 子模块设计（归档安全）

## Responsibility

- 发布侧：把单个文件或目录打包为安全 `.tar.gz`（排除绝对路径、`..`、越界符号链接），计算 SHA-256 与大小；下载侧：生成净化文件名、校验下载内容完整性，并用临时文件 + 原子 rename 落盘。
- 不感知项目/版本语义；不自动解压下载内容（提案非目标）。

## Interfaces

```rust
// cli/src/archive/mod.rs（fh-cli-publish / fh-cli-download）
pub struct PackedArchive { pub path: PathBuf, pub sha256: String, pub size: u64 }
pub fn pack_tar_gz(source: &Path) -> Result<PackedArchive, ArchiveError>;
pub fn sanitize_artifact_name(project: &str, version: &str) -> Result<String, ArchiveError>;
pub fn verify_sha256(path: &Path, expected: &str) -> Result<(), ArchiveError>;
```

```rust
// cli/src/archive/safe_tar.rs：tar 条目安全过滤
fn is_safe_entry(path: &str, link_target: Option<&str>) -> bool;
fn append_entry(sink: &mut TarBuilder, source: &Path);
```

## Packaging Rules

- 目录打包时归档根为目录名（`<name>/...`），文件打包时归档含单个文件条目；不允许出现绝对路径、以 `/` 开头、`..` 段、Windows 盘符前缀。
- 符号链接：链接目标经规范化后仍在源目录树内才允许保留；指向源目录树外（含绝对目标）的条目直接报错拒绝打包（服务端仍二次校验）。
- 文件/目录枚举按相对路径压入 tar，遇到不可读/权限错误立即失败并返回本地文件系统错误，不留半成品归档。
- gzip 压缩使用确定性级别；打包完成后返回临时 `.tar.gz` 路径、SHA-256 与字节大小；调用方负责清理临时文件。

## Download Naming and Verification

- 文件名：`<project>-<version>.tar.gz`，其中 project/version 均先净化：仅保留 `[A-Za-z0-9._-]`，替换风险字符，控制长度（单段 ≤ 64，总长 ≤ 255），避免 Windows 保留名（CON/PRN/AUX/NUL/COM1-9/LPT1-9）。
- 落盘：先写入目标目录内隐藏临时文件，流式写完后校验 SHA-256 与服务端 `VersionDto.sha256` 一致；一致才 `rename` 为最终名；校验失败删除临时文件并返回内容/完整性退出码 7，不覆盖旧文件。
- 目标目录不可写/预检失败：在下载开始前返回退出码 8，不产生半成品。

## State and Ownership

- 无持久状态；临时文件由 `archive` 创建并负责在成功/失败路径清理；最终下载文件归属调用方指定目录。

## Design Notes

- 打包是“本地先做安全裁剪”，不替代服务端校验；服务端拒绝后客户端原样呈现契约错误。
- 重复下载同版本采用覆盖语义（先校验后 rename），保证脚本幂等；校验失败时旧文件保留。
- 下载响应若是压缩流之外的意外内容（HTTP 200 但 gzip/tar 魔数不符合 `.tar.gz`），按内容完整性错误处理。
