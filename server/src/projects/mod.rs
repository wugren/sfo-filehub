//! projects 子模块：项目 CRUD 与 public/private 可见性（P-06 fh-server-projects）。

pub mod http;
pub mod model;
pub mod service;

use async_trait::async_trait;
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

use crate::model::{Principal, ProjectId, ProjectRecord, Visibility};
use crate::permissions::{PermissionChecker, model::ProjectAccess};
use crate::storage::FileStore;

#[async_trait]
pub trait ProjectService: 'static + Send + Sync {
    async fn create(
        &self,
        actor: &Principal,
        name: &str,
        visibility: Visibility,
    ) -> ProjectResult<ProjectRecord>;
    /// 分页列出当前身份可见的项目：一次授权过滤 SQL + 一次 COUNT。
    async fn list(
        &self,
        actor: &Principal,
        limit: u32,
        offset: u32,
    ) -> ProjectResult<ProjectPage>;
    /// 按 id 直查单个项目；不可见/不存在返回 None（HTTP 层负责 401/404 语义）。
    async fn get(
        &self,
        project_id: &ProjectId,
        actor: &Principal,
    ) -> ProjectResult<Option<ProjectRecord>>;
    async fn set_visibility(
        &self,
        project: &ProjectId,
        actor: &Principal,
        visibility: Visibility,
    ) -> ProjectResult<()>;
    async fn delete(&self, project: &ProjectId, actor: &Principal) -> ProjectResult<()>;
}

#[derive(Debug, Clone)]
pub struct ProjectPage {
    pub items: Vec<ProjectRecord>,
    pub total: i64,
}

pub struct ProjectModule {
    service: Arc<dyn ProjectService>,
    project_access: Arc<service::SqliteProjectAccess>,
    db: SqlitePool,
}

impl ProjectModule {
    pub async fn init(
        db: &SqlitePool,
        checker: Arc<dyn PermissionChecker>,
        files: Arc<dyn FileStore>,
    ) -> Result<Self, String> {
        sqlx::raw_sql(include_str!("../../migrations/0007_projects.sql"))
            .execute(db)
            .await
            .map_err(|e| format!("apply 0007_projects.sql failed: {e}"))?;
        let project_access = Arc::new(service::SqliteProjectAccess::new(db.clone()));
        let service: Arc<dyn ProjectService> = Arc::new(service::SqliteProjectService::new(
            db.clone(),
            project_access.clone(),
            checker,
            files,
        ));
        Ok(Self {
            service,
            project_access,
            db: db.clone(),
        })
    }

    pub fn service(&self) -> Arc<dyn ProjectService> {
        self.service.clone()
    }

    /// 供 PermissionsModule::init 注入只读项目端口。
    pub fn project_access(&self) -> Arc<dyn ProjectAccess> {
        self.project_access.clone()
    }

    pub fn db(&self) -> &SqlitePool {
        &self.db
    }
}

pub use model::{ProjectError, ProjectErrorKind, ProjectResult};
