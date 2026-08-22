//! SqliteProjectAccess / SqliteProjectService（I-007）。

use async_trait::async_trait;
use chrono::Utc;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::sync::Arc;

use crate::model::{FeatureName, Principal, ProjectId, ProjectRecord, Resource, UserId, Visibility};
use crate::permissions::{checker::ACTION_ADMIN, checker::ACTION_METADATA_READ, PermissionChecker};
use crate::permissions::model::{row_to_project, PermissionResult, ProjectAccess};

use super::model::{ProjectError, ProjectResult};
use super::ProjectService;

pub struct SqliteProjectAccess {
    db: SqlitePool,
}

impl SqliteProjectAccess {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProjectAccess for SqliteProjectAccess {
    async fn project(&self, project_id: &ProjectId) -> PermissionResult<Option<ProjectRecord>> {
        let row = sqlx::query("SELECT id, name, visibility, owner_id FROM projects WHERE id = ?")
            .bind(project_id.0)
            .fetch_optional(&self.db)
            .await?;
        Ok(match row {
            Some(row) => Some(row_to_project(&row)?),
            None => None,
        })
    }

    async fn list_projects(&self) -> PermissionResult<Vec<ProjectRecord>> {
        let rows = sqlx::query("SELECT id, name, visibility, owner_id FROM projects ORDER BY id")
            .fetch_all(&self.db)
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(row_to_project(&row)?);
        }
        Ok(out)
    }
}

pub struct SqliteProjectService {
    db: SqlitePool,
    project_access: Arc<SqliteProjectAccess>,
    checker: Arc<dyn PermissionChecker>,
}

impl SqliteProjectService {
    pub fn new(db: SqlitePool, project_access: Arc<SqliteProjectAccess>, checker: Arc<dyn PermissionChecker>) -> Self {
        Self { db, project_access, checker }
    }
}

#[async_trait]
impl ProjectService for SqliteProjectService {
    async fn create(&self, actor: &Principal, name: &str, visibility: Visibility) -> ProjectResult<ProjectRecord> {
        let Some(owner) = actor.user_id() else {
            return Err(ProjectError::forbidden("login required to create a project"));
        };
        if name.trim().is_empty() {
            return Err(ProjectError::invalid_input("project name required"));
        }
        if !self.checker.can_access(actor, &Resource::Feature(FeatureName::ProjectsCreate), "projects:create").await? {
            return Err(ProjectError::forbidden("projects:create permission required"));
        }
        let result = sqlx::query("INSERT INTO projects (name, visibility, owner_id, created_at) VALUES (?, ?, ?, ?)")
            .bind(name)
            .bind(visibility.to_string())
            .bind(owner.0)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.db)
            .await;
        let project_id = match result {
            Ok(result) => ProjectId(result.last_insert_rowid()),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                return Err(ProjectError::conflict("project name already exists"));
            }
            Err(e) => return Err(ProjectError::from(e)),
        };
        Ok(ProjectRecord {
            project_id,
            name: name.to_string(),
            visibility,
            owner,
        })
    }

    async fn list(&self, actor: &Principal) -> ProjectResult<Vec<ProjectRecord>> {
        let projects = self.project_access.list_projects().await?;
        let mut out = Vec::new();
        for project in projects {
            if self.checker.can_access(actor, &Resource::Project(project.project_id), ACTION_METADATA_READ).await? {
                out.push(project);
            }
        }
        Ok(out)
    }

    async fn set_visibility(&self, project: &ProjectId, actor: &Principal, visibility: Visibility) -> ProjectResult<()> {
        if !self.checker.can_access(actor, &Resource::Project(*project), ACTION_ADMIN).await? {
            let exists = self.project_access.project(project).await?.is_some();
            if !exists {
                return Err(ProjectError::not_found("project not found"));
            }
            return Err(ProjectError::forbidden("administration permission required"));
        }
        let result = sqlx::query("UPDATE projects SET visibility = ? WHERE id = ?")
            .bind(visibility.to_string())
            .bind(project.0)
            .execute(&self.db)
            .await?;
        if result.rows_affected() == 0 {
            return Err(ProjectError::not_found("project not found"));
        }
        Ok(())
    }

    async fn delete(&self, project: &ProjectId, actor: &Principal) -> ProjectResult<()> {
        if !self.checker.can_access(actor, &Resource::Feature(FeatureName::ProjectsDelete), "projects:delete").await? {
            return Err(ProjectError::forbidden("projects:delete permission required"));
        }
        let result = sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(project.0)
            .execute(&self.db)
            .await?;
        if result.rows_affected() == 0 {
            return Err(ProjectError::not_found("project not found"));
        }
        Ok(())
    }
}

impl From<crate::permissions::model::PermissionError> for ProjectError {
    fn from(value: crate::permissions::model::PermissionError) -> Self {
        ProjectError::db(value.message)
    }
}
