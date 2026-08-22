//! SqlitePermissionChecker：统一权限判定与协作者管理（I-003）。

use async_trait::async_trait;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::sync::Arc;

use crate::model::{AccountRole, Collaborator, Principal, ProjectId, ProjectRecord, ProjectRole, Resource, Scope, ScopeSet, UserId, Visibility};

use super::model::{row_to_project, PermissionError, PermissionResult, ProjectAccess};

pub const ACTION_METADATA_READ: &str = "metadata:read";
pub const ACTION_ARTIFACTS_READ: &str = "artifacts:read";
pub const ACTION_ARTIFACTS_WRITE: &str = "artifacts:write";
pub const ACTION_ADMIN: &str = "administration";

pub struct SqlitePermissionChecker {
    db: SqlitePool,
    project_access: Arc<dyn ProjectAccess>,
}

impl SqlitePermissionChecker {
    pub fn new(db: SqlitePool, project_access: Arc<dyn ProjectAccess>) -> Self {
        Self { db, project_access }
    }

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

    async fn grant_role_of(&self, project_id: &ProjectId, user_id: &UserId) -> PermissionResult<Option<ProjectRole>> {
        let row = sqlx::query("SELECT role FROM project_grants WHERE project_id = ? AND user_id = ?")
            .bind(project_id.0)
            .bind(user_id.0)
            .fetch_optional(&self.db)
            .await?;
        Ok(match row {
            Some(row) => {
                let raw: String = row.try_get("role")?;
                Some(raw.parse().map_err(|e: String| PermissionError::db(format!("invalid grant row: {e}")))?)
            }
            None => None,
        })
    }

    async fn project_owner(&self, project_id: &ProjectId) -> PermissionResult<Option<UserId>> {
        Ok(self
            .project_access
            .project(project_id)
            .await?
            .map(|p| p.owner))
    }

    async fn project_permission(&self, user_id: UserId, project: &ProjectRecord, action: &str) -> PermissionResult<bool> {
        if project.owner == user_id {
            return Ok(matches!(
                action,
                ACTION_METADATA_READ | ACTION_ARTIFACTS_READ | ACTION_ARTIFACTS_WRITE | ACTION_ADMIN
            ));
        }
        let role = match self.grant_role_of(&project.project_id, &user_id).await? {
            Some(role) => role,
            None => return Ok(false),
        };
        Ok(project_action_allowed(role, action))
    }

    async fn user_account_role(&self, user_id: UserId) -> PermissionResult<AccountRole> {
        self.role_for_user(user_id).await
    }
}

pub fn project_action_allowed(role: ProjectRole, action: &str) -> bool {
    match action {
        ACTION_METADATA_READ | ACTION_ARTIFACTS_READ => true,
        ACTION_ARTIFACTS_WRITE => matches!(role, ProjectRole::Write | ProjectRole::Admin),
        ACTION_ADMIN => matches!(role, ProjectRole::Admin),
        _ => false,
    }
}

fn token_has_scope(scopes: &ScopeSet, action: &str) -> bool {
    let scope = match action {
        ACTION_METADATA_READ => Scope::MetadataRead,
        ACTION_ARTIFACTS_READ => Scope::ArtifactsRead,
        ACTION_ARTIFACTS_WRITE => Scope::ArtifactsWrite,
        ACTION_ADMIN => Scope::Administration,
        "projects:create" => Scope::ProjectsCreate,
        "projects:delete" => Scope::ProjectsDelete,
        _ => return false,
    };
    scopes.contains_scope(scope)
}

#[async_trait]
impl super::PermissionChecker for SqlitePermissionChecker {
    async fn can_access(&self, principal: &Principal, resource: &Resource, action: &str) -> PermissionResult<bool> {
        match resource {
            Resource::Feature(feature) => {
                if action != feature.as_str() {
                    return Ok(false);
                }
                match principal {
                    Principal::Anonymous => Ok(false),
                    Principal::User { account_role, .. } => Ok(*account_role == AccountRole::Owner),
                    Principal::Token { scopes, user_id, .. } => {
                        if !token_has_scope(scopes, action) {
                            return Ok(false);
                        }
                        Ok(self.user_account_role(*user_id).await? == AccountRole::Owner)
                    }
                }
            }
            Resource::Project(project_id) => {
                let project = match self.project_access.project(project_id).await? {
                    Some(project) => project,
                    None => return Ok(false),
                };
                match principal {
                    Principal::Anonymous => Ok(project.visibility == Visibility::Public
                        && matches!(action, ACTION_METADATA_READ | ACTION_ARTIFACTS_READ)),
                    Principal::User { user_id, .. } => self.project_permission(*user_id, &project, action).await,
                    Principal::Token { scopes, user_id, .. } => {
                        if !token_has_scope(scopes, action) {
                            return Ok(false);
                        }
                        self.project_permission(*user_id, &project, action).await
                    }
                }
            }
        }
    }

    async fn list_collaborators(&self, project_id: &ProjectId, actor: &Principal) -> PermissionResult<Vec<Collaborator>> {
        if !self.can_access(actor, &Resource::Project(*project_id), ACTION_ADMIN).await? {
            return Err(PermissionError::forbidden("administration required"));
        }
        let rows = sqlx::query("SELECT user_id, role FROM project_grants WHERE project_id = ? ORDER BY user_id")
            .bind(project_id.0)
            .fetch_all(&self.db)
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let role: String = row.try_get("role")?;
            out.push(Collaborator {
                user_id: UserId(row.try_get("user_id")?),
                role: role.parse().map_err(|e: String| PermissionError::db(format!("invalid grant: {e}")))?,
            });
        }
        Ok(out)
    }

    async fn grant_collaborator(&self, project_id: &ProjectId, actor: &Principal, user_id: &UserId, role: ProjectRole) -> PermissionResult<()> {
        if !self.can_access(actor, &Resource::Project(*project_id), ACTION_ADMIN).await? {
            return Err(PermissionError::forbidden("administration required"));
        }
        if self.project_owner(project_id).await? == Some(*user_id) {
            return Err(PermissionError::forbidden("project owner is implicit admin and cannot be granted"));
        }
        sqlx::query("INSERT INTO project_grants (project_id, user_id, role) VALUES (?, ?, ?) ON CONFLICT(project_id, user_id) DO UPDATE SET role = excluded.role")
            .bind(project_id.0)
            .bind(user_id.0)
            .bind(role.to_string())
            .execute(&self.db)
            .await?;
        Ok(())
    }

    async fn update_collaborator(&self, project_id: &ProjectId, actor: &Principal, user_id: &UserId, role: ProjectRole) -> PermissionResult<()> {
        if !self.can_access(actor, &Resource::Project(*project_id), ACTION_ADMIN).await? {
            return Err(PermissionError::forbidden("administration required"));
        }
        if self.project_owner(project_id).await? == Some(*user_id) {
            return Err(PermissionError::forbidden("project owner cannot be changed"));
        }
        sqlx::query("UPDATE project_grants SET role = ? WHERE project_id = ? AND user_id = ?")
            .bind(role.to_string())
            .bind(project_id.0)
            .bind(user_id.0)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    async fn remove_collaborator(&self, project_id: &ProjectId, actor: &Principal, user_id: &UserId) -> PermissionResult<()> {
        if !self.can_access(actor, &Resource::Project(*project_id), ACTION_ADMIN).await? {
            return Err(PermissionError::forbidden("administration required"));
        }
        if self.project_owner(project_id).await? == Some(*user_id) {
            return Err(PermissionError::forbidden("project owner cannot be removed"));
        }
        sqlx::query("DELETE FROM project_grants WHERE project_id = ? AND user_id = ?")
            .bind(project_id.0)
            .bind(user_id.0)
            .execute(&self.db)
            .await?;
        Ok(())
    }
}
