---
task_manifest: task.yaml
status: draft
---

## Design Scope

- model 共享模块为 `FilesConfig` 增加可选解压上限配置，装配时解析默认值。

## File-Level Interfaces

```rust
// server/src/model/config.rs（修改）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesConfig {
    pub data_dir: PathBuf,
    pub max_archive_bytes: u64,
    #[serde(default)]
    pub max_decompressed_bytes: Option<u64>,
}
```

- Consumer: `server/src/http/mod.rs` 装配层与 `server/config.example.json`
  （fh-upload-size-and-decompression-bounds）。
- Compatibility: backward-compatible（可选字段，缺省行为不变）。

## State and Ownership

- Owner: model（配置 DTO 唯一归属；解析默认后由装配层传给 storage）。

## Design Notes

- 缺省值：`max_archive_bytes * 20`（文档与 proposal 冻结的经验默认），
  `Option` 缺省读取后立即解析为具体 u64 传入 `FileModule::init`，运行期
  不再保留可选语义。
