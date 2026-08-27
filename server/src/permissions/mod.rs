//! permissions 子模块：权限数据存储与校验（P-02 fh-server-permissions）。

pub mod checker;
pub mod http;
pub mod model;

use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnection, SqlitePool};
use std::sync::Arc;

use crate::model::{Collaborator, Principal, ProjectId, ProjectRole, Resource, UserId};
use model::{PermissionResult, ProjectAccess};

#[async_trait]
pub trait PermissionChecker: 'static + Send + Sync {
    async fn can_access(
        &self,
        principal: &Principal,
        resource: &Resource,
        action: &str,
    ) -> PermissionResult<bool>;
    /// 在调用方提供的事务连接上执行与 `can_access` 相同的项目存在性/权限判定，
    /// 供「确认 + 写入」必须处于同一事务边界的方法（如创建版本、授予协作者）使用。
    async fn can_access_tx(
        &self,
        conn: &mut SqliteConnection,
        principal: &Principal,
        resource: &Resource,
        action: &str,
    ) -> PermissionResult<bool>;
    async fn list_collaborators(
        &self,
        project_id: &ProjectId,
        actor: &Principal,
    ) -> PermissionResult<Vec<Collaborator>>;
    async fn grant_collaborator(
        &self,
        project_id: &ProjectId,
        actor: &Principal,
        user_id: &UserId,
        role: ProjectRole,
    ) -> PermissionResult<()>;
    async fn update_collaborator(
        &self,
        project_id: &ProjectId,
        actor: &Principal,
        user_id: &UserId,
        role: ProjectRole,
    ) -> PermissionResult<()>;
    async fn remove_collaborator(
        &self,
        project_id: &ProjectId,
        actor: &Principal,
        user_id: &UserId,
    ) -> PermissionResult<()>;
}

pub struct PermissionsModule {
    checker: Arc<dyn PermissionChecker>,
    db: SqlitePool,
}

impl PermissionsModule {
    /// 统一权限核心初始化。project_access 由 projects 子模块注入。
    pub async fn init(
        db: &SqlitePool,
        project_access: Arc<dyn ProjectAccess>,
    ) -> Result<Self, String> {
        sqlx::raw_sql(include_str!("../../migrations/0003_roles_grants.sql"))
            .execute(db)
            .await
            .map_err(|e| format!("apply 0003_roles_grants.sql failed: {e}"))?;
        let checker: Arc<dyn PermissionChecker> = Arc::new(checker::SqlitePermissionChecker::new(
            db.clone(),
            project_access,
        ));
        Ok(Self {
            checker,
            db: db.clone(),
        })
    }

    pub fn checker(&self) -> Arc<dyn PermissionChecker> {
        self.checker.clone()
    }

    pub fn db(&self) -> &SqlitePool {
        &self.db
    }
}

use model::PermissionError;
