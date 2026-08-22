//! storage 子模块：.tar.gz 物理文件存储与完整性（P-04 fh-server-files）。

pub mod http;
pub mod integrity;
pub mod store;

use async_trait::async_trait;
use sqlx::sqlite::SqlitePool;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use crate::model::{FileId, FileRecord};

use crate::permissions::model::PermissionError;

pub type UploadStream = Vec<u8>;
pub type DownloadStream = tokio::fs::File;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStoreErrorKind {
    NotFound,
    InvalidInput,
    Conflict,
    Io,
    Db,
}

#[derive(Debug, Clone)]
pub struct FileStoreError {
    pub kind: FileStoreErrorKind,
    pub message: String,
}

impl std::fmt::Display for FileStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "file store error {:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for FileStoreError {}

pub type FileStoreResult<T> = Result<T, FileStoreError>;

impl From<FileStoreError> for ApiError {
    fn from(value: FileStoreError) -> Self {
        match value.kind {
            FileStoreErrorKind::NotFound => ApiError::not_found(value.message),
            FileStoreErrorKind::InvalidInput => ApiError::invalid_input(value.message),
            FileStoreErrorKind::Conflict => ApiError::conflict(value.message),
            FileStoreErrorKind::Io | FileStoreErrorKind::Db => ApiError::server(value.message),
        }
    }
}

impl From<FileStoreError> for PermissionError {
    fn from(value: FileStoreError) -> Self {
        PermissionError::db(value.message)
    }
}

#[async_trait]
pub trait FileStore: 'static + Send + Sync {
    async fn ingest(&self, source: UploadStream, expected_sha256: Option<&str>) -> FileStoreResult<FileRecord>;
    async fn open_read(&self, file_id: &FileId) -> FileStoreResult<DownloadStream>;
    async fn discard(&self, file_id: &FileId) -> FileStoreResult<()>;
    async fn gc_orphans(&self, keep: &HashSet<FileId>) -> FileStoreResult<Vec<FileId>>;
}

pub struct FileModule {
    store: Arc<dyn FileStore>,
}

impl FileModule {
    pub async fn init(db: &SqlitePool, data_dir: PathBuf, max_archive_bytes: u64) -> Result<Self, String> {
        sqlx::raw_sql(include_str!("../../migrations/0005_files.sql"))
            .execute(db)
            .await
            .map_err(|e| format!("apply 0005_files.sql failed: {e}"))?;
        Ok(Self {
            store: Arc::new(store::SqliteFileStore::new(db.clone(), data_dir, max_archive_bytes)),
        })
    }

    pub fn store(&self) -> Arc<dyn FileStore> {
        self.store.clone()
    }
}

use crate::contract::ApiError;

pub(crate) fn crate_error_to_http<Resp: sfo_http::http_server::Response>(err: &ApiError) -> sfo_http::errors::HttpResult<Resp> {
    crate::contract::api_error_response(err)
}
