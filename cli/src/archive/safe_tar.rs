//! tar 条目安全过滤与确定性打包。

use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use flate2::Compression;
use flate2::write::GzEncoder;

use super::ArchiveError;

#[derive(Debug, Clone)]
enum Entry {
    Dir(PathBuf),
    File(PathBuf),
    Symlink { path: PathBuf, target: String },
}

/// 校验 tar 条目路径：仅允许相对、无 `..`/绝对路径/盘符/反斜杠段的条目。
pub fn is_safe_entry(path: &str, link_target: Option<&str>) -> bool {
    if path.is_empty() {
        return false;
    }
    let normalized = path.replace('\\', "/").trim_end_matches('/').to_string();
    if normalized.starts_with('/') || normalized.contains(":/") {
        return false;
    }
    if normalized
        .split('/')
        .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return false;
    }
    if let Some(target) = link_target {
        let target_norm = target.replace('\\', "/");
        if target_norm.starts_with('/') || target_norm.is_empty() {
            return false;
        }
    }
    true
}

/// 目录打包：归档根为目录名（`<name>/...`），排除越界符号链接。
pub fn pack_directory(source: &Path, destination: &Path) -> Result<(), ArchiveError> {
    let root_name = source
        .file_name()
        .ok_or_else(|| {
            ArchiveError::Unsupported("目录根部没有可用的名字段，无法安全打包".to_string())
        })?
        .to_string_lossy()
        .into_owned();
    let root_real = fs::canonicalize(source)
        .map_err(|e| ArchiveError::LocalFs(format!("解析源目录失败：{e}")))?;
    let mut entries = Vec::new();
    walk_directory(source, Path::new(&root_name), &root_real, &mut entries)?;
    entries.sort_by(|a, b| display(a).cmp(&display(b)));

    let file = fs::File::create(destination)
        .map_err(|e| ArchiveError::LocalFs(format!("创建临时归档失败：{e}")))?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(encoder);
    for entry in entries {
        append_entry(&mut builder, &entry, source)?;
    }
    let encoder = builder
        .into_inner()
        .map_err(|e| ArchiveError::LocalFs(format!("完成 tar 流失败：{e}")))?;
    encoder
        .finish()
        .map_err(|e| ArchiveError::LocalFs(format!("完成 gzip 流失败：{e}")))?;
    Ok(())
}

/// 单文件打包：归档只含一个文件条目（以文件名为条目名）。
pub fn pack_single_file(source: &Path, destination: &Path) -> Result<(), ArchiveError> {
    let name = source.file_name().ok_or_else(|| {
        ArchiveError::Unsupported("文件没有可用的名字段，无法安全打包".to_string())
    })?;
    let rel = PathBuf::from(name.to_string_lossy().into_owned());
    if !is_safe_entry(&rel.to_string_lossy(), None) {
        return Err(ArchiveError::Unsupported(format!(
            "文件名不安全：{}",
            rel.display()
        )));
    }
    let file = fs::File::create(destination)
        .map_err(|e| ArchiveError::LocalFs(format!("创建临时归档失败：{e}")))?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(encoder);
    append_entry(&mut builder, &Entry::File(rel), source)?;
    let encoder = builder
        .into_inner()
        .map_err(|e| ArchiveError::LocalFs(format!("完成 tar 流失败：{e}")))?;
    encoder
        .finish()
        .map_err(|e| ArchiveError::LocalFs(format!("完成 gzip 流失败：{e}")))?;
    Ok(())
}

fn walk_directory(
    root: &Path,
    prefix: &Path,
    root_real: &Path,
    out: &mut Vec<Entry>,
) -> Result<(), ArchiveError> {
    let mut children: Vec<_> = fs::read_dir(root)
        .map_err(|e| ArchiveError::LocalFs(format!("读取目录 {} 失败：{e}", root.display())))?
        .collect::<Result<Vec<_>, io::Error>>()?;
    children.sort_by_key(|entry| entry.file_name());
    for child in children {
        let child_path = child.path();
        let rel = prefix.join(child.file_name());
        let rel_text = rel.to_string_lossy().into_owned();
        let file_type = child
            .file_type()
            .map_err(|e| ArchiveError::LocalFs(format!("读取条目类型失败：{e}")))?;
        if file_type.is_dir() {
            if !is_safe_entry(&format!("{rel_text}/"), None) {
                return Err(ArchiveError::Unsupported(format!(
                    "目录条目不安全：{rel_text}"
                )));
            }
            out.push(Entry::Dir(rel.clone()));
            walk_directory(&child_path, &rel, root_real, out)?;
        } else if file_type.is_symlink() {
            let target = fs::read_link(&child_path)
                .map_err(|e| ArchiveError::LocalFs(format!("读取符号链接失败：{e}")))?;
            let real_target = fs::canonicalize(&child_path).map_err(|e| {
                ArchiveError::Unsupported(format!(
                    "越界/失效符号链接 {}：{e}",
                    child_path.display()
                ))
            })?;
            if !real_target.starts_with(root_real) {
                return Err(ArchiveError::Unsupported(format!(
                    "拒绝打包指向源目录树外的符号链接：{}",
                    child_path.display()
                )));
            }
            let stored_target = stored_link_target(prefix, &target, root_real);
            if !is_safe_entry(&rel_text, Some(&stored_target)) {
                return Err(ArchiveError::Unsupported(format!(
                    "符号链接条目不安全：{rel_text}"
                )));
            }
            out.push(Entry::Symlink {
                path: rel,
                target: stored_target,
            });
        } else if file_type.is_file() {
            if !is_safe_entry(&rel_text, None) {
                return Err(ArchiveError::Unsupported(format!(
                    "文件条目不安全：{rel_text}"
                )));
            }
            out.push(Entry::File(rel));
        } else {
            return Err(ArchiveError::Unsupported(format!(
                "不支持打包特殊文件条目：{}",
                child_path.display()
            )));
        }
    }
    Ok(())
}

/// 把符号链接目标转为归档内部的相对路径（绝对/根内目标都收敛为相对）。
fn stored_link_target(prefix: &Path, target: &Path, root_real: &Path) -> String {
    let canonical = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());
    let rel = match canonical.strip_prefix(root_real) {
        Ok(relative) => relative,
        Err(_) => {
            // 相对目标：尽量转为以链接目录为基准的相对形式（保留原样即可）。
            return target.to_string_lossy().into_owned();
        }
    };
    let from: Vec<String> = prefix
        .components()
        .filter_map(|component| {
            use std::path::Component;
            match component {
                Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
                _ => None,
            }
        })
        .collect();
    let target_parts = {
        let mut parts = Vec::new();
        if let Some(name) = prefix.iter().next() {
            parts.push(name.to_string_lossy().into_owned());
        }
        for component in rel.components() {
            use std::path::Component;
            if let Component::Normal(name) = component {
                parts.push(name.to_string_lossy().into_owned());
            }
        }
        parts
    };
    // 从链接所在目录（from）到目标（target_parts）的相对路径。
    let mut common = 0;
    while common < from.len() && common < target_parts.len() && from[common] == target_parts[common]
    {
        common += 1;
    }
    let mut relative = PathBuf::new();
    for _ in common..from.len() {
        relative.push("..");
    }
    for segment in &target_parts[common..] {
        relative.push(segment);
    }
    relative.to_string_lossy().into_owned()
}

fn append_entry<W: Write>(
    builder: &mut tar::Builder<W>,
    entry: &Entry,
    source_root: &Path,
) -> Result<(), ArchiveError> {
    match entry {
        Entry::Dir(rel) => {
            let path_text = format!("{}/", rel.to_string_lossy());
            if !is_safe_entry(&path_text, None) {
                return Err(ArchiveError::Unsupported(format!(
                    "目录条目不安全：{path_text}"
                )));
            }
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::dir());
            header.set_mode(0o755);
            header.set_mtime(0);
            header.set_size(0);
            header.set_cksum();
            builder
                .append_data(&mut header, path_text.as_str(), io::empty())
                .map_err(|e| ArchiveError::LocalFs(format!("写入目录条目失败：{e}")))?;
        }
        Entry::File(rel) => {
            let path_text = rel.to_string_lossy().into_owned();
            if !is_safe_entry(&path_text, None) {
                return Err(ArchiveError::Unsupported(format!(
                    "文件条目不安全：{path_text}"
                )));
            }
            let file_path = if source_root.is_dir() {
                // 归档内 rel 带根目录名前缀，打开时去掉该前缀。
                let relative = source_root
                    .file_name()
                    .and_then(|name| rel.strip_prefix(name).ok())
                    .unwrap_or(rel);
                source_root.join(relative)
            } else {
                source_root.to_path_buf()
            };
            let mut file = fs::File::open(&file_path).map_err(|e| {
                ArchiveError::LocalFs(format!("打开 {} 失败：{e}", file_path.display()))
            })?;
            let size = file
                .metadata()
                .map_err(|e| ArchiveError::LocalFs(format!("读取元数据失败：{e}")))?
                .len();
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::file());
            header.set_mode(0o644);
            header.set_mtime(0);
            header.set_size(size);
            header.set_cksum();
            builder
                .append_data(&mut header, path_text.as_str(), &mut file)
                .map_err(|e| ArchiveError::LocalFs(format!("写入文件条目失败：{e}")))?;
        }
        Entry::Symlink { path, target } => {
            let path_text = path.to_string_lossy().into_owned();
            if !is_safe_entry(&path_text, Some(target)) {
                return Err(ArchiveError::Unsupported(format!(
                    "符号链接条目不安全：{path_text}"
                )));
            }
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::symlink());
            header.set_mode(0o777);
            header.set_mtime(0);
            header.set_size(0);
            header
                .set_link_name(target.as_str())
                .map_err(|e| ArchiveError::LocalFs(format!("设置链接目标失败：{e}")))?;
            header.set_cksum();
            builder
                .append_data(&mut header, path_text.as_str(), io::empty())
                .map_err(|e| ArchiveError::LocalFs(format!("写入符号链接条目失败：{e}")))?;
        }
    }
    Ok(())
}

fn display(entry: &Entry) -> String {
    match entry {
        Entry::Dir(path) => format!("{}/", path.to_string_lossy()),
        Entry::File(path) => path.to_string_lossy().into_owned(),
        Entry::Symlink { path, .. } => path.to_string_lossy().into_owned(),
    }
}

/// 补充：路径组件级安全校验（防 `..`/绝对路径逃逸）。
#[allow(dead_code)]
fn path_has_escape(rel: &Path) -> bool {
    rel.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) || matches!(component, Component::CurDir)
    })
}
