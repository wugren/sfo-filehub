//! permissions 子模块：权限数据存储与校验（P-02 fh-server-permissions）。

pub mod checker;
pub mod http;
pub mod model;

use async_trait::async_trait;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::sync::Arc;

use crate::model::{AccountRole, Collaborator, Principal, ProjectId, ProjectRole, Resource, UserId};
use model::{PermissionResult, ProjectAccess};

#[async_trait]
pub trait PermissionChecker: 'static + Send + Sync {
    async fn can_access(&self, principal: &Principal, resource: &Resource, action: &str) -> PermissionResult<bool>;
    async fn list_collaborators(&self, project_id: &ProjectId, actor: &Principal) -> PermissionResult<Vec<Collaborator>>;
    async fn grant_collaborator(&self, project_id: &ProjectId, actor: &Principal, user_id: &UserId, role: ProjectRole) -> PermissionResult<()>;
    async fn update_collaborator(&self, project_id: &ProjectId, actor: &Principal, user_id: &UserId, role: ProjectRole) -> PermissionResult<()>;
    async fn remove_collaborator(&self, project_id: &ProjectId, actor: &Principal, user_id: &UserId) -> PermissionResult<()>;
}

pub struct PermissionsModule {
    checker: Arc<dyn PermissionChecker>,
    db: SqlitePool,
}

impl PermissionsModule {
    /// 配置驱动角色初始化 + 统一权限核心。project_access 由 projects 子模块注入。
    pub async fn init(
        config: &crate::model::UsersConfig,
        db: &SqlitePool,
        project_access: Arc<dyn ProjectAccess>,
    ) -> Result<Self, String> {
        sqlx::raw_sql(include_str!("../../migrations/0003_roles_grants.sql"))
            .execute(db)
            .await
            .map_err(|e| format!("apply 0003_roles_grants.sql failed: {e}"))?;
        for user in &config.users {
            let row = sqlx::query("SELECT id FROM users WHERE name = ?")
                .bind(&user.username)
                .fetch_optional(db)
                .await
                .map_err(|e| format!("lookup user {} failed: {}", user.username, e))?;
            let Some(row) = row else {
                return Err(format!("user {} missing from account seed", user.username));
            };
            let user_id: i64 = row.try_get("id").map_err(|e| format!("decode user id failed: {e}"))?;
            let role = match user.role.as_deref().unwrap_or("member") {
                "owner" => AccountRole::Owner,
                "member" => AccountRole::Member,
                other => return Err(format!("invalid role for {}: {other}", user.username)),
            };
            sqlx::query("INSERT INTO account_roles (user_id, role) VALUES (?, ?) ON CONFLICT(user_id) DO UPDATE SET role = excluded.role")
                .bind(user_id)
                .bind(role.to_string())
                .execute(db)
                .await
                .map_err(|e| format!("upsert role for {} failed: {}", user.username, e))?;
        }
        let checker: Arc<dyn PermissionChecker> = Arc::new(checker::SqlitePermissionChecker::new(db.clone(), project_access));
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

    /// http 认证包装用：补齐 Principal::User 的账号角色（缺省 member，fail-closed）。
    pub async fn role_for_user(&self, user_id: UserId) -> PermissionResult<AccountRole> {
        let row = sqlx::query("SELECT role FROM account_roles WHERE user_id = ?")
            .bind(user_id.0)
            .fetch_optional(&self.db)
            .await?;
        Ok(match row {
            Some(row) => {
                let raw: String = row.try_get("role")?;
                raw.parse().map_err(|e: String| PermissionError::db(format!("invalid role row: {e}")))?
            }
            None => AccountRole::Member,
        })
    }
}

use model::PermissionError;
