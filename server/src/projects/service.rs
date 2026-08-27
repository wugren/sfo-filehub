//! SqliteProjectAccess / SqliteProjectService（I-007）。

use async_trait::async_trait;
use chrono::Utc;
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

use crate::model::{
    FeatureName, FileId, Principal, ProjectId, ProjectRecord, ProjectScope, Resource, Scope,
    UserId, Visibility,
};
use crate::permissions::model::{PermissionResult, ProjectAccess, row_to_project};
use crate::permissions::{
    PermissionChecker, checker::ACTION_ADMIN, checker::ACTION_PROJECTS_DELETE,
};
use crate::storage::FileStore;

use super::{ProjectPage, ProjectService};
use super::model::{ProjectError, ProjectResult};

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
    files: Arc<dyn FileStore>,
}

impl SqliteProjectService {
    pub fn new(
        db: SqlitePool,
        project_access: Arc<SqliteProjectAccess>,
        checker: Arc<dyn PermissionChecker>,
        files: Arc<dyn FileStore>,
    ) -> Self {
        Self {
            db,
            project_access,
            checker,
            files,
        }
    }

    /// 项目名最小拒绝集（049）：只拒绝 CLI `<server>/<project>` 目标串永远
    /// 无法精确寻址的名称——字段分隔符 `/`，或首尾空白（CLI 对每个目标字段
    /// 先 trim 再精确匹配）。校验通过后仍按原始 name 落库，不做静默改写。
    fn validate_project_name(name: &str) -> ProjectResult<()> {
        if name.trim().is_empty() {
            return Err(ProjectError::invalid_input("project name required"));
        }
        if name.contains('/') {
            return Err(ProjectError::invalid_input(
                "project name must not contain '/'",
            ));
        }
        if name != name.trim() {
            return Err(ProjectError::invalid_input(
                "project name must not have leading or trailing whitespace",
            ));
        }
        Ok(())
    }

    /// 把 `decide_project_access` 的可见性判定下沉为单一 SQL 过滤条件。
    /// 返回的占位符参数全部为 i64（user_id / owner_id / 指定项目 id）。
    fn auth_filter(principal: &Principal) -> (String, Vec<i64>) {
        match principal {
            Principal::Anonymous => ("p.visibility = 'public'".to_string(), Vec::new()),
            Principal::User { user_id } => {
                let mut params = Vec::with_capacity(2);
                params.push(user_id.0);
                params.push(user_id.0);
                (
                    "(p.visibility = 'public' OR p.owner_id = ? OR EXISTS \
                     (SELECT 1 FROM project_grants g \
                      WHERE g.project_id = p.id AND g.user_id = ?))"
                        .to_string(),
                    params,
                )
            }
            Principal::Token {
                scopes,
                user_id,
                project_scope,
                ..
            } => {
                if !scopes.contains_scope(Scope::MetadataRead) {
                    return ("0 = 1".to_string(), Vec::new());
                }
                let mut params = Vec::with_capacity(2);
                params.push(user_id.0);
                params.push(user_id.0);
                let mut sql = "(p.visibility = 'public' OR p.owner_id = ? OR EXISTS \
                               (SELECT 1 FROM project_grants g \
                                WHERE g.project_id = p.id AND g.user_id = ?))"
                    .to_string();
                match project_scope {
                    ProjectScope::All => {}
                    // 与 checker 的 token_in_project_scope 语义一致：空集合不放行
                    // 任何项目；同时避免生成 SQLite 非法的 `IN ()`。
                    ProjectScope::Specified(ids) if ids.is_empty() => {
                        sql.push_str(" AND 1 = 0");
                    }
                    ProjectScope::Specified(ids) => {
                        let markers = vec!["?"; ids.len()].join(",");
                        sql.push_str(&format!(" AND p.id IN ({markers})"));
                        params.extend(ids.iter().map(|id| id.0));
                    }
                }
                (sql, params)
            }
        }
    }
}

#[async_trait]
impl ProjectService for SqliteProjectService {
    async fn create(
        &self,
        actor: &Principal,
        name: &str,
        visibility: Visibility,
    ) -> ProjectResult<ProjectRecord> {
        let Some(owner) = actor.user_id() else {
            return Err(ProjectError::forbidden(
                "login required to create a project",
            ));
        };
        Self::validate_project_name(name)?;
        if !self
            .checker
            .can_access(
                actor,
                &Resource::Feature(FeatureName::ProjectsCreate),
                "projects:create",
            )
            .await?
        {
            return Err(ProjectError::forbidden(
                "projects:create permission required",
            ));
        }
        let result = sqlx::query(
            "INSERT INTO projects (name, visibility, owner_id, created_at) VALUES (?, ?, ?, ?)",
        )
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

    async fn list(&self, actor: &Principal, limit: u32, offset: u32) -> ProjectResult<ProjectPage> {
        let (filter, params) = Self::auth_filter(actor);
        let count_sql = format!("SELECT COUNT(*) FROM projects p WHERE {filter}");
        let page_sql = format!(
            "SELECT id, name, visibility, owner_id FROM projects p \
             WHERE {filter} ORDER BY p.id LIMIT ? OFFSET ?"
        );

        let mut count = sqlx::query_scalar(sqlx::AssertSqlSafe(count_sql.as_str()));
        for value in &params {
            count = count.bind(value);
        }
        let total: i64 = count.fetch_one(&self.db).await?;

        let mut page = sqlx::query(sqlx::AssertSqlSafe(page_sql.as_str()));
        for value in &params {
            page = page.bind(value);
        }
        let page = page.bind(limit as i64).bind(offset as i64);
        let rows = page.fetch_all(&self.db).await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(row_to_project(&row)?);
        }
        Ok(ProjectPage { items, total })
    }

    async fn get(
        &self,
        project_id: &ProjectId,
        actor: &Principal,
    ) -> ProjectResult<Option<ProjectRecord>> {
        let (filter, mut params) = Self::auth_filter(actor);
        params.push(project_id.0);
        let sql = format!(
            "SELECT id, name, visibility, owner_id FROM projects p \
             WHERE {filter} AND p.id = ?"
        );
        let mut query = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()));
        for value in &params {
            query = query.bind(value);
        }
        let row = query.fetch_optional(&self.db).await?;
        Ok(match row {
            Some(row) => Some(row_to_project(&row)?),
            None => None,
        })
    }

    async fn set_visibility(
        &self,
        project: &ProjectId,
        actor: &Principal,
        visibility: Visibility,
    ) -> ProjectResult<()> {
        if !self
            .checker
            .can_access(actor, &Resource::Project(*project), ACTION_ADMIN)
            .await?
        {
            let exists = self.project_access.project(project).await?.is_some();
            if !exists {
                return Err(ProjectError::not_found("project not found"));
            }
            return Err(ProjectError::forbidden(
                "administration permission required",
            ));
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
        if matches!(actor, Principal::Token { .. }) {
            if !self
                .checker
                .can_access(actor, &Resource::Project(*project), ACTION_ADMIN)
                .await?
            {
                let exists = self.project_access.project(project).await?.is_some();
                if !exists {
                    return Err(ProjectError::not_found("project not found"));
                }
                return Err(ProjectError::forbidden(
                    "project-level administration permission required",
                ));
            }
        }
        // 项目级删除权限：仅项目 owner（隐式 admin）可删，admin 协作者不可删。
        if !self
            .checker
            .can_access(actor, &Resource::Project(*project), ACTION_PROJECTS_DELETE)
            .await?
        {
            return Err(ProjectError::forbidden(
                "project delete permission required",
            ));
        }
        let mut tx = self.db.begin().await?;
        // 事务内先收集本项目 versions 引用的 file_id（必须在删除
        // version_apps/projects 之前），提交后再逐个即时回收 files 行与
        // 物理归档；清理失败只记告警，残留由启动 GC 兜底（050）。
        let file_ids: Vec<FileId> = sqlx::query_scalar::<_, String>(
            "SELECT va.file_id FROM version_apps va \
             JOIN versions v ON v.id = va.version_id WHERE v.project_id = ?",
        )
        .bind(project.0)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(FileId::new)
        .collect();
        let result = sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(project.0)
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() == 0 {
            return Err(ProjectError::not_found("project not found"));
        }
        // 显式清理关联数据，不依赖外键/pragma：项目删除后不再残留
        // versions/version_apps/project_grants 僵尸记录。
        sqlx::query("DELETE FROM project_grants WHERE project_id = ?")
            .bind(project.0)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "DELETE FROM version_apps WHERE version_id IN (SELECT id FROM versions WHERE project_id = ?)",
        )
        .bind(project.0)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM versions WHERE project_id = ?")
            .bind(project.0)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        for file_id in file_ids {
            if let Err(e) = self.files.discard(&file_id).await {
                log::warn!(
                    "delete project {}: discard file {} failed: {e}",
                    project.0,
                    file_id.0
                );
            }
        }
        Ok(())
    }
}

impl From<crate::permissions::model::PermissionError> for ProjectError {
    fn from(value: crate::permissions::model::PermissionError) -> Self {
        ProjectError::db(value.message)
    }
}
