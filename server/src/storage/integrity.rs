//! .tar.gz 与 SHA-256 完整性校验（I-005）。

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::io::Read;
use tar::Archive;

pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// 只接受 gzip magic + 可解析的 tar 结构（不信任文件名后缀）。
pub fn validate_targz(data: &[u8]) -> Result<(), String> {
    if data.len() < 2 || data[0] != 0x1f || data[1] != 0x8b {
        return Err("upload is not a gzip stream".to_string());
    }
    let decoder = GzDecoder::new(data);
    let mut archive = Archive::new(decoder);
    let mut entries = archive
        .entries()
        .map_err(|e| format!("upload is not a valid tar archive: {e}"))?;
    let mut sink = Vec::new();
    for entry in entries {
        let mut entry = entry.map_err(|e| format!("corrupt tar entry: {e}"))?;
        std::io::copy(&mut entry, &mut sink)
            .map_err(|e| format!("corrupt tar entry stream: {e}"))?;
    }
    Ok(())
}
