//! SqlitePermissionChecker：统一权限判定与协作者管理（I-003）。

use async_trait::async_trait;
use sqlx::{Connection, Row};
use sqlx::sqlite::{SqliteConnection, SqlitePool};
use std::sync::Arc;

use crate::model::{
    Collaborator, FeatureName, Principal, ProjectId, ProjectRecord, ProjectRole, ProjectScope,
    Resource, Scope, ScopeSet, UserId, Visibility,
};

use super::model::{PermissionError, PermissionResult, ProjectAccess, row_to_project};

pub const ACTION_METADATA_READ: &str = "metadata:read";
pub const ACTION_ARTIFACTS_READ: &str = "artifacts:read";
pub const ACTION_ARTIFACTS_WRITE: &str = "artifacts:write";
pub const ACTION_ADMIN: &str = "administration";
pub const ACTION_PROJECTS_DELETE: &str = "projects:delete";

pub struct SqlitePermissionChecker {
    db: SqlitePool,
    project_access: Arc<dyn ProjectAccess>,
}

impl SqlitePermissionChecker {
    pub fn new(db: SqlitePool, project_access: Arc<dyn ProjectAccess>) -> Self {
        Self { db, project_access }
    }

    async fn grant_role_of(
        &self,
        project_id: &ProjectId,
        user_id: &UserId,
    ) -> PermissionResult<Option<ProjectRole>> {
        Self::grant_role_of_query(&self.db, project_id, user_id).await
    }

    /// 事务连接版：与 `grant_role_of` 相同查询，但在同一事务连接上读取授权。
    async fn grant_role_of_tx(
        conn: &mut SqliteConnection,
        project_id: &ProjectId,
        user_id: &UserId,
    ) -> PermissionResult<Option<ProjectRole>> {
        Self::grant_role_of_query(&mut *conn, project_id, user_id).await
    }

    async fn grant_role_of_query<'e, E>(
        executor: E,
        project_id: &ProjectId,
        user_id: &UserId,
    ) -> PermissionResult<Option<ProjectRole>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let row =
            sqlx::query("SELECT role FROM project_grants WHERE project_id = ? AND user_id = ?")
                .bind(project_id.0)
                .bind(user_id.0)
                .fetch_optional(executor)
                .await?;
        Ok(match row {
            Some(row) => {
                let raw: String = row.try_get("role")?;
                Some(
                    raw.parse().map_err(|e: String| {
                        PermissionError::db(format!("invalid grant row: {e}"))
                    })?,
                )
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

    fn feature_allowed(principal: &Principal, feature: &FeatureName, action: &str) -> bool {
        if action != feature.as_str() {
            return false;
        }
        match principal {
            Principal::Anonymous => false,
            Principal::User { .. } => true,
            Principal::Token { scopes, .. } => token_has_scope(scopes, action),
        }
    }

    /// 纯判定：owner 隐式 admin；其余依赖调用方已取出的授权行。
    fn project_permission(
        project: &ProjectRecord,
        user_id: UserId,
        role: Option<ProjectRole>,
        action: &str,
    ) -> bool {
        if project.owner == user_id {
            return matches!(
                action,
                ACTION_METADATA_READ
                    | ACTION_ARTIFACTS_READ
                    | ACTION_ARTIFACTS_WRITE
                    | ACTION_ADMIN
                    | ACTION_PROJECTS_DELETE
            );
        }
        match role {
            Some(role) => project_action_allowed(role, action),
            None => false,
        }
    }

    /// 池查询版与事务连接版共用的项目资源判定主体。
    fn decide_project_access(
        principal: &Principal,
        project: &ProjectRecord,
        role: Option<ProjectRole>,
        action: &str,
    ) -> bool {
        match principal {
            Principal::Anonymous => {
                project.visibility == Visibility::Public
                    && matches!(action, ACTION_METADATA_READ | ACTION_ARTIFACTS_READ)
            }
            Principal::User { user_id, .. } => {
                if project.visibility == Visibility::Public
                    && matches!(action, ACTION_METADATA_READ | ACTION_ARTIFACTS_READ)
                {
                    return true;
                }
                Self::project_permission(project, *user_id, role, action)
            }
            Principal::Token {
                scopes,
                user_id,
                project_scope,
                ..
            } => {
                if !token_in_project_scope(project_scope, &project.project_id) {
                    return false;
                }
                if !token_has_scope(scopes, action) {
                    return false;
                }
                if project.visibility == Visibility::Public
                    && matches!(action, ACTION_METADATA_READ | ACTION_ARTIFACTS_READ)
                {
                    return true;
                }
                Self::project_permission(project, *user_id, role, action)
            }
        }
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

/// token 项目范围包含性判断：`All` 放行，`Specified` 仅放行集合内项目。
fn token_in_project_scope(project_scope: &ProjectScope, project_id: &ProjectId) -> bool {
    match project_scope {
        ProjectScope::All => true,
        ProjectScope::Specified(ids) => ids.contains(project_id),
    }
}

#[async_trait]
impl super::PermissionChecker for SqlitePermissionChecker {
    async fn can_access(
        &self,
        principal: &Principal,
        resource: &Resource,
        action: &str,
    ) -> PermissionResult<bool> {
        match resource {
            Resource::Feature(feature) => Ok(Self::feature_allowed(principal, feature, action)),
            Resource::Project(project_id) => {
                let project = match self.project_access.project(project_id).await? {
                    Some(project) => project,
                    None => return Ok(false),
                };
                let role = match principal {
                    Principal::User { user_id, .. } | Principal::Token { user_id, .. } => {
                        self.grant_role_of(&project.project_id, user_id).await?
                    }
                    Principal::Anonymous => None,
                };
                Ok(Self::decide_project_access(principal, &project, role, action))
            }
        }
    }

    async fn can_access_tx(
        &self,
        conn: &mut SqliteConnection,
        principal: &Principal,
        resource: &Resource,
        action: &str,
    ) -> PermissionResult<bool> {
        match resource {
            Resource::Feature(feature) => Ok(Self::feature_allowed(principal, feature, action)),
            Resource::Project(project_id) => {
                // 项目存在性确认在事务连接上完成，与后续写入同一边界。
                let row = sqlx::query(
                    "SELECT id, name, visibility, owner_id FROM projects WHERE id = ?",
                )
                .bind(project_id.0)
                .fetch_optional(&mut *conn)
                .await?;
                let Some(row) = row else {
                    return Ok(false);
                };
                let project = row_to_project(&row)?;
                let role = match principal {
                    Principal::User { user_id, .. } | Principal::Token { user_id, .. } => {
                        Self::grant_role_of_tx(conn, &project.project_id, user_id).await?
                    }
                    Principal::Anonymous => None,
                };
                Ok(Self::decide_project_access(principal, &project, role, action))
            }
        }
    }

    async fn list_collaborators(
        &self,
        project_id: &ProjectId,
        actor: &Principal,
    ) -> PermissionResult<Vec<Collaborator>> {
        if !self
            .can_access(actor, &Resource::Project(*project_id), ACTION_ADMIN)
            .await?
        {
            return Err(PermissionError::forbidden("administration required"));
        }
        let rows = sqlx::query(
            "SELECT user_id, role FROM project_grants WHERE project_id = ? ORDER BY user_id",
        )
        .bind(project_id.0)
        .fetch_all(&self.db)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let role: String = row.try_get("role")?;
            out.push(Collaborator {
                user_id: UserId(row.try_get("user_id")?),
                role: role
                    .parse()
                    .map_err(|e: String| PermissionError::db(format!("invalid grant: {e}")))?,
            });
        }
        Ok(out)
    }

    async fn grant_collaborator(
        &self,
        project_id: &ProjectId,
        actor: &Principal,
        user_id: &UserId,
        role: ProjectRole,
    ) -> PermissionResult<()> {
        // 存在性/权限确认与授权写入处于同一 BEGIN IMMEDIATE 事务：
        // 并发删除项目时不可能出现「确认后写入已删项目的孤儿授权」。
        let mut conn = self.db.acquire().await?;
        let mut tx = conn.begin_with("BEGIN IMMEDIATE").await?;
        if !self
            .can_access_tx(&mut *tx, actor, &Resource::Project(*project_id), ACTION_ADMIN)
            .await?
        {
            return Err(PermissionError::forbidden("administration required"));
        }
        let owner: Option<i64> = sqlx::query_scalar("SELECT owner_id FROM projects WHERE id = ?")
            .bind(project_id.0)
            .fetch_optional(&mut *tx)
            .await?;
        if owner == Some(user_id.0) {
            return Err(PermissionError::forbidden(
                "project owner is implicit admin and cannot be granted",
            ));
        }
        // 授权必须以真实存在的账号为前提：不存在（含负数、尚未创建的正整数）ID
        // 直接返回 not_found，避免未来新增账号接管历史授权。
        let exists = sqlx::query_scalar::<_, i64>("SELECT 1 FROM users WHERE id = ?")
            .bind(user_id.0)
            .fetch_optional(&mut *tx)
            .await?;
        if exists.is_none() {
            return Err(PermissionError::not_found(format!(
                "user {user_id} does not exist"
            )));
        }
        match sqlx::query("INSERT INTO project_grants (project_id, user_id, role) VALUES (?, ?, ?) ON CONFLICT(project_id, user_id) DO UPDATE SET role = excluded.role")
            .bind(project_id.0)
            .bind(user_id.0)
            .bind(role.to_string())
            .execute(&mut *tx)
            .await
        {
            Ok(_) => {}
            Err(sqlx::Error::Database(db_err)) if db_err.is_foreign_key_violation() => {
                return Err(PermissionError::not_found("project not found"));
            }
            Err(e) => return Err(PermissionError::from(e)),
        }
        tx.commit().await?;
        Ok(())
    }

    async fn update_collaborator(
        &self,
        project_id: &ProjectId,
        actor: &Principal,
        user_id: &UserId,
        role: ProjectRole,
    ) -> PermissionResult<()> {
        if !self
            .can_access(actor, &Resource::Project(*project_id), ACTION_ADMIN)
            .await?
        {
            return Err(PermissionError::forbidden("administration required"));
        }
        if self.project_owner(project_id).await? == Some(*user_id) {
            return Err(PermissionError::forbidden(
                "project owner cannot be changed",
            ));
        }
        sqlx::query("UPDATE project_grants SET role = ? WHERE project_id = ? AND user_id = ?")
            .bind(role.to_string())
            .bind(project_id.0)
            .bind(user_id.0)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    async fn remove_collaborator(
        &self,
        project_id: &ProjectId,
        actor: &Principal,
        user_id: &UserId,
    ) -> PermissionResult<()> {
        if !self
            .can_access(actor, &Resource::Project(*project_id), ACTION_ADMIN)
            .await?
        {
            return Err(PermissionError::forbidden("administration required"));
        }
        if self.project_owner(project_id).await? == Some(*user_id) {
            return Err(PermissionError::forbidden(
                "project owner cannot be removed",
            ));
        }
        sqlx::query("DELETE FROM project_grants WHERE project_id = ? AND user_id = ?")
            .bind(project_id.0)
            .bind(user_id.0)
            .execute(&self.db)
            .await?;
        Ok(())
    }
}
