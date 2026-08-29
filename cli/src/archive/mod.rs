//! 归档安全：安全 `.tar.gz` 打包、SHA-256、文件名净化与校验后落盘。

use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

mod safe_tar;

pub use safe_tar::{is_safe_entry, pack_directory, pack_single_file};

/// 打包结果：临时归档路径 + SHA-256 + 字节大小（调用方负责清理临时文件）。
#[derive(Debug, Clone)]
pub struct PackedArchive {
    pub path: PathBuf,
    pub sha256: String,
    pub size: u64,
}

/// 归档/完整性错误。
#[derive(Debug)]
pub enum ArchiveError {
    Unsupported(String),
    LocalFs(String),
    Integrity(String),
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveError::Unsupported(message) => write!(f, "{message}"),
            ArchiveError::LocalFs(message) => write!(f, "{message}"),
            ArchiveError::Integrity(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ArchiveError {}

impl From<io::Error> for ArchiveError {
    fn from(value: io::Error) -> Self {
        ArchiveError::LocalFs(value.to_string())
    }
}

/// 把单个文件或目录打包为安全 `.tar.gz`，返回临时归档 + SHA-256 + 大小。
pub fn pack_tar_gz(source: &Path) -> Result<PackedArchive, ArchiveError> {
    let metadata = fs::metadata(source).map_err(|e| {
        ArchiveError::LocalFs(format!(
            "failed to read path to publish {}: {e}",
            source.display()
        ))
    })?;
    if !metadata.is_dir() && !metadata.is_file() {
        return Err(ArchiveError::Unsupported(format!(
            "{} is not a regular file or directory and cannot be published",
            source.display()
        )));
    }
    let tmp = temp_archive_path()?;
    if metadata.is_dir() {
        pack_directory(source, &tmp)?;
    } else {
        pack_single_file(source, &tmp)?;
    }
    let (sha256, size) = hash_file(&tmp)?;
    Ok(PackedArchive {
        path: tmp,
        sha256,
        size,
    })
}

/// 生成净化后的下载文件名 `<project>-<version>.tar.gz`。
pub fn sanitize_artifact_name(project: &str, version: &str) -> Result<String, ArchiveError> {
    let mut project_segment = sanitize_segment(project, "project");
    let mut version_segment = sanitize_segment(version, "version");
    project_segment = avoid_reserved(project_segment.as_str());
    version_segment = avoid_reserved(version_segment.as_str());
    let mut name = format!("{project_segment}-{version_segment}.tar.gz");
    if name.len() > 255 {
        loop {
            if project_segment.len() > 64 {
                project_segment.truncate(64);
            } else if version_segment.len() > 64 {
                version_segment.truncate(64);
            } else if project_segment.len() > 16 {
                project_segment.truncate(16);
            } else if version_segment.len() > 16 {
                version_segment.truncate(16);
            } else {
                break;
            }
            name = format!("{project_segment}-{version_segment}.tar.gz");
            if name.len() <= 255 {
                break;
            }
        }
    }
    if name.len() > 255 || name.is_empty() {
        return Err(ArchiveError::Unsupported(
            "failed to generate download filename because the sanitized project/version is still too long"
                .to_string(),
        ));
    }
    Ok(name)
}

/// 校验文件 SHA-256（十六进制，大小写不敏感）。
pub fn verify_sha256(path: &Path, expected: &str) -> Result<(), ArchiveError> {
    let (actual, _size) = hash_file(path)?;
    if !actual.eq_ignore_ascii_case(expected.trim()) {
        return Err(ArchiveError::Integrity(format!(
            "content verification failed: expected sha256={}, actual sha256={}",
            expected.trim(),
            actual
        )));
    }
    Ok(())
}

/// 校验 `.tar.gz` 魔数（gzip 头 1f 8b）。
pub fn verify_gzip_magic(path: &Path) -> Result<(), ArchiveError> {
    let mut file = fs::File::open(path)?;
    use std::io::Read;
    let mut head = [0u8; 2];
    let read = file
        .read(&mut head)
        .map_err(|e| ArchiveError::LocalFs(format!("failed to read downloaded content: {e}")))?;
    if read != 2 || head != [0x1f, 0x8b] {
        return Err(ArchiveError::Integrity(
            "downloaded content is not a valid `.tar.gz` archive (gzip magic bytes do not match)"
                .to_string(),
        ));
    }
    Ok(())
}

/// 校验通过后把临时文件原子落盘为最终名（脚本幂等可覆盖旧文件）。
pub fn finalize_download(
    tmp: &Path,
    final_path: &Path,
    expected_sha256: &str,
) -> Result<(), ArchiveError> {
    verify_sha256(tmp, expected_sha256)?;
    verify_gzip_magic(tmp)?;
    if final_path.exists() {
        fs::remove_file(final_path)?;
    }
    fs::rename(tmp, final_path)?;
    Ok(())
}

/// 生成打包用临时归档路径。
pub fn temp_archive_path() -> Result<PathBuf, ArchiveError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    Ok(std::env::temp_dir().join(format!(
        "filehub-pack-{}-{nanos}.tar.gz",
        std::process::id()
    )))
}

fn hash_file(path: &Path) -> Result<(String, u64), ArchiveError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut size: u64 = 0;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buf).map_err(|e| {
            ArchiveError::LocalFs(format!("failed to read {}: {e}", path.display()))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
        size += read as u64;
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    Ok((hex, size))
}

fn sanitize_segment(value: &str, fallback: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    while out.starts_with('.') {
        out.remove(0);
    }
    if out.is_empty() {
        out.push_str(fallback);
    }
    out.truncate(128);
    out
}

fn avoid_reserved(value: &str) -> String {
    let stem = value.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if reserved.contains(&stem.as_str()) {
        format!("_{value}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitized_name_keeps_safe_chars_and_caps_length() {
        let name = sanitize_artifact_name("my/项目/..name", "v1.0  beta").unwrap();
        assert!(name.starts_with("my___"));
        assert!(name.ends_with(".tar.gz"));
        assert!(!name.contains('/') && !name.contains('\\'));
        assert!(name.len() <= 255);
        let long = sanitize_artifact_name(&"p".repeat(300), &"v".repeat(300)).unwrap();
        assert!(long.len() <= 255);
    }

    #[test]
    fn reserved_names_are_escaped() {
        assert!(
            sanitize_artifact_name("CON", "v1")
                .unwrap()
                .starts_with("_")
        );
        assert!(
            sanitize_artifact_name("NUL", "v1")
                .unwrap()
                .starts_with("_")
        );
        assert!(
            sanitize_artifact_name("demo", "v1")
                .unwrap()
                .starts_with("demo")
        );
    }
}
