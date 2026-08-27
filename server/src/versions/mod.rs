//! versions 子模块：版本元数据、不可覆盖、latest 与原子发布协调（P-05 fh-server-versions）。

pub mod http;
pub mod model;
pub mod service;
pub mod upload;

use async_trait::async_trait;
use sqlx::sqlite::SqlitePool;
use std::collections::HashSet;
use std::sync::Arc;

use crate::model::{FileId, FileRecord, Principal, ProjectId, VersionPublish, VersionRecord};
use crate::permissions::PermissionChecker;
use crate::storage::FileStore;

/// 未显式指定 app 时的缺省应用名（单应用版本兼容）。
pub const APP_DEFAULT: &str = "default";

#[async_trait]
pub trait VersionService: 'static + Send + Sync {
    /// 显式创建版本；`(project, version)` 已存在返回 Conflict。
    async fn create_version(
        &self,
        project: &ProjectId,
        version: &str,
        actor: &Principal,
    ) -> VersionResult<VersionRecord>;
    /// 发布/更新版本内 app：版本不存在 NotFound；已锁定 Conflict；app 缺省 "default"。
    async fn publish_app(
        &self,
        project: &ProjectId,
        version: &str,
        app: &str,
        file: FileRecord,
        actor: &Principal,
    ) -> VersionResult<VersionPublish>;
    /// 删除版本内 app：版本不存在 NotFound；已锁定 Conflict；app 不存在 NotFound。
    async fn delete_app(
        &self,
        project: &ProjectId,
        version: &str,
        app: &str,
        actor: &Principal,
    ) -> VersionResult<()>;
    /// 不可逆锁定版本：需 administration；已锁定幂等返回。
    async fn lock(
        &self,
        project: &ProjectId,
        version: &str,
        actor: &Principal,
    ) -> VersionResult<VersionRecord>;
    async fn list(
        &self,
        project: &ProjectId,
        actor: &Principal,
    ) -> VersionResult<Vec<VersionRecord>>;
    async fn get(
        &self,
        project: &ProjectId,
        version: Option<&str>,
        actor: &Principal,
    ) -> VersionResult<VersionRecord>;
    async fn referenced_file_ids(&self) -> VersionResult<HashSet<FileId>>;
}

pub struct VersionModule {
    service: Arc<dyn VersionService>,
}

impl VersionModule {
    pub async fn init(
        db: &SqlitePool,
        checker: Arc<dyn PermissionChecker>,
        files: Arc<dyn FileStore>,
    ) -> Result<Self, String> {
        sqlx::raw_sql(include_str!("../../migrations/0006_versions.sql"))
            .execute(db)
            .await
            .map_err(|e| format!("apply 0006_versions.sql failed: {e}"))?;
        Ok(Self {
            service: Arc::new(service::SqliteVersionService::new(
                db.clone(),
                checker,
                files,
            )),
        })
    }

    pub fn service(&self) -> Arc<dyn VersionService> {
        self.service.clone()
    }
}

pub use model::{VersionError, VersionErrorKind, VersionResult};
