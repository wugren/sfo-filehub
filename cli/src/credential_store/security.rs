//! 凭据文件最小权限与原子写入。

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::CredentialStoreError;

/// 类 Unix 下把目标文件权限收敛为 0600（忽略 umask 放宽）。
#[cfg(unix)]
fn set_min_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_min_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// 原子写：同目录临时文件 + fsync + rename；写入后收敛权限。
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<(), CredentialStoreError> {
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    fs::create_dir_all(parent)?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "config.toml".to_string());
    let tmp = parent.join(format!(".{file_name}.{}.{nanos}.tmp", std::process::id()));
    let result = write_and_sync(&tmp, content).and_then(|_| {
        set_min_permissions(&tmp)?;
        fs::rename(&tmp, path)
    });
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result?;
    set_min_permissions(path)?;
    Ok(())
}

fn write_and_sync(path: &Path, content: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(content)?;
    file.sync_all()?;
    Ok(())
}
