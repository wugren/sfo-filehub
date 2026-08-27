//! SqliteFileStore：物理字节的原子入库、读取、回滚与孤儿清理（I-005）。

use async_trait::async_trait;
use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::Row;
use sqlx::sqlite::SqlitePool;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use crate::model::{FileId, FileRecord};

use super::{DownloadStream, FileStoreError, FileStoreResult, UploadStream};

pub struct SqliteFileStore {
    db: SqlitePool,
    data_dir: PathBuf,
    max_archive_bytes: u64,
}

impl SqliteFileStore {
    pub fn new(db: SqlitePool, data_dir: PathBuf, max_archive_bytes: u64) -> Self {
        Self {
            db,
            data_dir,
            max_archive_bytes,
        }
    }

    fn safe_path(&self, rel_path: &str) -> Result<PathBuf, FileStoreError> {
        let path = Path::new(rel_path);
        if path.is_absolute()
            || path.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::RootDir
                )
            })
        {
            return Err(FileStoreError {
                kind: super::FileStoreErrorKind::InvalidInput,
                message: "unsafe stored file path".to_string(),
            });
        }
        Ok(self.data_dir.join(path))
    }

    async fn remove_physical(&self, rel_path: &str) {
        if let Ok(path) = self.safe_path(rel_path) {
            let _ = tokio::fs::remove_file(&path).await;
        }
    }
}

#[async_trait]
impl super::FileStore for SqliteFileStore {
    async fn ingest(
        &self,
        source: UploadStream,
        expected_sha256: Option<&str>,
    ) -> FileStoreResult<FileRecord> {
        tokio::fs::create_dir_all(&self.data_dir)
            .await
            .map_err(|e| FileStoreError {
                kind: super::FileStoreErrorKind::Io,
                message: format!("create data dir failed: {e}"),
            })?;
        let file_id = Uuid::new_v4().to_string();
        let rel_path = format!("{file_id}.tar.gz");
        let temp_path = self.data_dir.join(format!(".tmp-{file_id}"));
        let final_path = self.data_dir.join(&rel_path);

        let mut reader = source.into_reader();
        let mut file = match tokio::fs::OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&temp_path)
            .await
        {
            Ok(file) => file,
            Err(e) => {
                return Err(FileStoreError {
                    kind: super::FileStoreErrorKind::Io,
                    message: format!("create temp archive failed: {e}"),
                });
            }
        };
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 64 * 1024];
        let mut size: u64 = 0;
        let spool = async {
            // Windows：FlushFileBuffers 需要可写句柄，且 rename 前必须释放句柄，
            // 否则过滤驱动/杀软可能以 ERROR_ACCESS_DENIED（os error 5）拒绝操作。
            loop {
                let n = reader.read(&mut buf).await.map_err(|e| FileStoreError {
                    kind: super::FileStoreErrorKind::Io,
                    message: format!("read upload stream failed: {e}"),
                })?;
                if n == 0 {
                    break;
                }
                size += n as u64;
                if size > self.max_archive_bytes {
                    return Err(FileStoreError {
                        kind: super::FileStoreErrorKind::InvalidInput,
                        message: format!(
                            "archive exceeds max_archive_bytes ({})",
                            self.max_archive_bytes
                        ),
                    });
                }
                hasher.update(&buf[..n]);
                file.write_all(&buf[..n])
                    .await
                    .map_err(|e| FileStoreError {
                        kind: super::FileStoreErrorKind::Io,
                        message: format!("write temp archive failed: {e}"),
                    })?;
            }
            file.sync_all().await.map_err(|e| FileStoreError {
                kind: super::FileStoreErrorKind::Io,
                message: format!("sync temp archive failed: {e}"),
            })?;
            drop(file);
            Ok::<(), FileStoreError>(())
        }
        .await;
        if let Err(error) = spool {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(error);
        }

        let actual_sha256 = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        if let Some(expected) = expected_sha256 {
            if !actual_sha256.eq_ignore_ascii_case(expected) {
                let _ = tokio::fs::remove_file(&temp_path).await;
                return Err(FileStoreError {
                    kind: super::FileStoreErrorKind::InvalidInput,
                    message: "sha256 mismatch".to_string(),
                });
            }
        }

        if let Err(e) = tokio::fs::rename(&temp_path, &final_path).await {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(FileStoreError {
                kind: super::FileStoreErrorKind::Io,
                message: format!("rename temp archive failed: {e}"),
            });
        }
        let store = self;
        match sqlx::query(
            "INSERT INTO files (id, rel_path, sha256, size, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&file_id)
        .bind(&rel_path)
        .bind(&actual_sha256)
        .bind(size as i64)
        .bind(Utc::now().to_rfc3339())
        .execute(&store.db)
        .await
        {
            Ok(_) => Ok(FileRecord {
                file_id: FileId::new(file_id),
                sha256: actual_sha256,
                size,
            }),
            Err(e) => {
                let message = format!("insert file row failed: {e}");
                store.remove_physical(&rel_path).await;
                Err(FileStoreError {
                    kind: super::FileStoreErrorKind::Db,
                    message,
                })
            }
        }
    }

    async fn open_read(&self, file_id: &FileId) -> FileStoreResult<DownloadStream> {
        let row = sqlx::query("SELECT rel_path FROM files WHERE id = ?")
            .bind(&file_id.0)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| FileStoreError {
                kind: super::FileStoreErrorKind::Db,
                message: e.to_string(),
            })?;
        let Some(row) = row else {
            return Err(FileStoreError {
                kind: super::FileStoreErrorKind::NotFound,
                message: "file not found".to_string(),
            });
        };
        let rel_path: String = row.try_get("rel_path").map_err(|e| FileStoreError {
            kind: super::FileStoreErrorKind::Db,
            message: e.to_string(),
        })?;
        let path = self.safe_path(&rel_path)?;
        tokio::fs::File::open(path)
            .await
            .map_err(|e| FileStoreError {
                kind: super::FileStoreErrorKind::Io,
                message: format!("open archive failed: {e}"),
            })
    }

    async fn discard(&self, file_id: &FileId) -> FileStoreResult<()> {
        let row = sqlx::query("SELECT COUNT(*) AS c FROM version_apps WHERE file_id = ?")
            .bind(&file_id.0)
            .fetch_one(&self.db)
            .await
            .map_err(|e| FileStoreError {
                kind: super::FileStoreErrorKind::Db,
                message: e.to_string(),
            })?;
        let count: i64 = row.try_get("c").map_err(|e| FileStoreError {
            kind: super::FileStoreErrorKind::Db,
            message: e.to_string(),
        })?;
        if count > 0 {
            return Err(FileStoreError {
                kind: super::FileStoreErrorKind::Conflict,
                message: "file is referenced by a version".to_string(),
            });
        }
        let row = sqlx::query("SELECT rel_path FROM files WHERE id = ?")
            .bind(&file_id.0)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| FileStoreError {
                kind: super::FileStoreErrorKind::Db,
                message: e.to_string(),
            })?;
        let Some(row) = row else {
            return Ok(());
        };
        let rel_path: String = row.try_get("rel_path").map_err(|e| FileStoreError {
            kind: super::FileStoreErrorKind::Db,
            message: e.to_string(),
        })?;
        sqlx::query("DELETE FROM files WHERE id = ?")
            .bind(&file_id.0)
            .execute(&self.db)
            .await
            .map_err(|e| FileStoreError {
                kind: super::FileStoreErrorKind::Db,
                message: e.to_string(),
            })?;
        self.remove_physical(&rel_path).await;
        Ok(())
    }

    async fn gc_orphans(&self, keep: &HashSet<FileId>) -> FileStoreResult<Vec<FileId>> {
        let rows = sqlx::query("SELECT id, rel_path FROM files")
            .fetch_all(&self.db)
            .await
            .map_err(|e| FileStoreError {
                kind: super::FileStoreErrorKind::Db,
                message: e.to_string(),
            })?;
        let mut removed = Vec::new();
        for row in rows {
            let id: String = row.try_get("id").map_err(|e| FileStoreError {
                kind: super::FileStoreErrorKind::Db,
                message: e.to_string(),
            })?;
            let rel_path: String = row.try_get("rel_path").map_err(|e| FileStoreError {
                kind: super::FileStoreErrorKind::Db,
                message: e.to_string(),
            })?;
            let file_id = FileId::new(id);
            if keep.contains(&file_id) {
                continue;
            }
            sqlx::query("DELETE FROM files WHERE id = ?")
                .bind(&file_id.0)
                .execute(&self.db)
                .await
                .map_err(|e| FileStoreError {
                    kind: super::FileStoreErrorKind::Db,
                    message: e.to_string(),
                })?;
            self.remove_physical(&rel_path).await;
            removed.push(file_id);
        }
        Ok(removed)
    }
}
