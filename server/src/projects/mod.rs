//! projects 子模块：项目 CRUD 与 public/private 可见性（P-06 fh-server-projects）。

pub mod http;
pub mod model;
pub mod service;

use async_trait::async_trait;
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

use crate::model::{Principal, ProjectId, ProjectRecord, Visibility};
use crate::permissions::{PermissionChecker, model::ProjectAccess};


#[async_trait]
pub trait ProjectService: 'static + Send + Sync {
    async fn create(&self, actor: &Principal, name: &str, visibility: Visibility) -> ProjectResult<ProjectRecord>;
    async fn list(&self, actor: &Principal) -> ProjectResult<Vec<ProjectRecord>>;
    async fn set_visibility(&self, project: &ProjectId, actor: &Principal, visibility: Visibility) -> ProjectResult<()>;
    async fn delete(&self, project: &ProjectId, actor: &Principal) -> ProjectResult<()>;
}

pub struct ProjectModule {
    service: Arc<dyn ProjectService>,
    project_access: Arc<service::SqliteProjectAccess>,
    db: SqlitePool,
}

impl ProjectModule {
    pub async fn init(db: &SqlitePool, checker: Arc<dyn PermissionChecker>) -> Result<Self, String> {
        sqlx::raw_sql(include_str!("../../migrations/0007_projects.sql"))
            .execute(db)
            .await
            .map_err(|e| format!("apply 0007_projects.sql failed: {e}"))?;
        let project_access = Arc::new(service::SqliteProjectAccess::new(db.clone()));
        let service: Arc<dyn ProjectService> = Arc::new(service::SqliteProjectService::new(db.clone(), project_access.clone(), checker));
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
