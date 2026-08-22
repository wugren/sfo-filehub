//! SqliteVersionService：版本显式创建/不可逆锁定与版本内 app 的发布/更新/删除（I-006 修订）。

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqlitePool, SqliteRow};
use sqlx::Row;
use std::collections::HashSet;
use std::sync::Arc;

use crate::model::{AppRecord, FileId, FileRecord, Principal, ProjectId, Resource, VersionPublish, VersionRecord};
use crate::permissions::checker::ACTION_ADMIN;
use crate::permissions::checker::ACTION_ARTIFACTS_WRITE;
use crate::permissions::checker::ACTION_METADATA_READ;
use crate::permissions::PermissionChecker;

use super::model::{VersionError, VersionResult};

pub struct SqliteVersionService {
    db: SqlitePool,
    checker: Arc<dyn PermissionChecker>,
}

impl SqliteVersionService {
    pub fn new(db: SqlitePool, checker: Arc<dyn PermissionChecker>) -> Self {
        Self { db, checker }
    }

    fn parse_time(raw: &str) -> DateTime<Utc> {
        chrono::DateTime::parse_from_rfc3339(raw)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now())
    }

    fn validate_app(app: &str) -> Result<(), VersionError> {
        let app = app.trim();
        if app.is_empty() {
            return Err(VersionError::invalid_input("app name required"));
        }
        if !app
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        {
            return Err(VersionError::invalid_input(
                "app name must contain only [A-Za-z0-9._-]",
            ));
        }
        Ok(())
    }

    fn app_from_row(row: &SqliteRow) -> Result<AppRecord, VersionError> {
        Ok(AppRecord {
            app: row.try_get("app")?,
            file_id: FileId::new(row.try_get::<String, _>("file_id")?),
            sha256: row.try_get("sha256")?,
            size: row.try_get::<i64, _>("size")? as u64,
            updated_at: Self::parse_time(&row.try_get::<String, _>("updated_at")?),
        })
    }

    /// 聚合单个版本（含 apps 列表）。
    async fn aggregate_version(&self, project: &ProjectId, version: &str) -> VersionResult<VersionRecord> {
        let rows = sqlx::query(
            "SELECT v.version, v.published_at, v.locked_at,
                    va.app, va.file_id, va.sha256, va.size, va.updated_at
             FROM versions v
             LEFT JOIN version_apps va ON va.version_id = v.id
             WHERE v.project_id = ? AND v.version = ?
             ORDER BY va.app ASC",
        )
        .bind(project.0)
        .bind(version)
        .fetch_all(&self.db)
        .await?;
        let mut record: Option<VersionRecord> = None;
        for row in rows {
            let published_raw: String = row.try_get("published_at")?;
            let locked_raw: Option<String> = row.try_get("locked_at")?;
            let app: Option<String> = row.try_get("app")?;
            if record.is_none() {
                let mut rec = VersionRecord {
                    project_id: *project,
                    version: version.to_string(),
                    published_at: Self::parse_time(&published_raw),
                    locked_at: locked_raw.as_deref().map(Self::parse_time),
                    apps: Vec::new(),
                };
                if app.is_some() {
                    rec.apps.push(Self::app_from_row(&row)?);
                }
                record = Some(rec);
            } else if app.is_some() {
                record.as_mut().expect("record").apps.push(Self::app_from_row(&row)?);
            }
        }
        record.ok_or_else(|| VersionError::not_found(format!("version {version} not found")))
    }

    /// 聚合项目全部版本（含每个版本的 apps）。
    async fn aggregate_all(&self, project: &ProjectId) -> VersionResult<Vec<VersionRecord>> {
        let rows = sqlx::query(
            "SELECT v.id, v.version, v.published_at, v.locked_at,
                    va.app, va.file_id, va.sha256, va.size, va.updated_at
             FROM versions v
             LEFT JOIN version_apps va ON va.version_id = v.id
             WHERE v.project_id = ?
             ORDER BY v.published_at DESC, v.id DESC, va.app ASC",
        )
        .bind(project.0)
        .fetch_all(&self.db)
        .await?;
        let mut out: Vec<VersionRecord> = Vec::new();
        for row in rows {
            let version: String = row.try_get("version")?;
            if out.last().map(|r: &VersionRecord| r.version == version).unwrap_or(false) {
                let app: Option<String> = row.try_get("app")?;
                if app.is_some() {
                    out.last_mut().expect("last").apps.push(Self::app_from_row(&row)?);
                }
                continue;
            }
            let published_raw: String = row.try_get("published_at")?;
            let locked_raw: Option<String> = row.try_get("locked_at")?;
            let app: Option<String> = row.try_get("app")?;
            let mut record = VersionRecord {
                project_id: *project,
                version: version.clone(),
                published_at: Self::parse_time(&published_raw),
                locked_at: locked_raw.as_deref().map(Self::parse_time),
                apps: Vec::new(),
            };
            if app.is_some() {
                record.apps.push(Self::app_from_row(&row)?);
            }
            out.push(record);
        }
        Ok(out)
    }

    /// 返回版本行的存在性/锁定元数据；不存在返回 None。
    async fn version_meta(
        &self,
        project: &ProjectId,
        version: &str,
    ) -> VersionResult<Option<(i64, Option<String>)>> {
        let row = sqlx::query("SELECT id, locked_at FROM versions WHERE project_id = ? AND version = ?")
            .bind(project.0)
            .bind(version)
            .fetch_optional(&self.db)
            .await?;
        let Some(row) = row else { return Ok(None); };
        let id: i64 = row.try_get("id")?;
        let locked_at: Option<String> = row.try_get("locked_at")?;
        Ok(Some((id, locked_at)))
    }

    async fn require_unlocked(&self, project: &ProjectId, version: &str) -> VersionResult<i64> {
        let Some((version_id, locked_at)) = self.version_meta(project, version).await? else {
            return Err(VersionError::not_found(format!("version {version} not found")));
        };
        if locked_at.is_some() {
            return Err(VersionError::conflict(format!("version {version} is locked")));
        }
        Ok(version_id)
    }
}

#[async_trait]
impl super::VersionService for SqliteVersionService {
    async fn create_version(&self, project: &ProjectId, version: &str, actor: &Principal) -> VersionResult<VersionRecord> {
        let version = version.trim();
        if version.is_empty() {
            return Err(VersionError::invalid_input("version is required"));
        }
        if !self.checker.can_access(actor, &Resource::Project(*project), ACTION_ARTIFACTS_WRITE).await? {
            return Err(VersionError::forbidden("artifacts:write required"));
        }
        let published_at = Utc::now().to_rfc3339();
        let insert = sqlx::query("INSERT INTO versions (project_id, version, published_at) VALUES (?, ?, ?)")
            .bind(project.0)
            .bind(version)
            .bind(&published_at)
            .execute(&self.db)
            .await;
        match insert {
            Ok(_) => {}
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                return Err(VersionError::conflict(format!(
                    "version {version} already exists in project {}",
                    project.0
                )));
            }
            Err(e) => return Err(VersionError::from(e)),
        }
        self.aggregate_version(project, version).await
    }

    async fn publish_app(
        &self,
        project: &ProjectId,
        version: &str,
        app: &str,
        file: FileRecord,
        actor: &Principal,
    ) -> VersionResult<VersionPublish> {
        let version = version.trim();
        Self::validate_app(app)?;
        if version.is_empty() {
            return Err(VersionError::invalid_input("version is required"));
        }
        if !self.checker.can_access(actor, &Resource::Project(*project), ACTION_ARTIFACTS_WRITE).await? {
            return Err(VersionError::forbidden("artifacts:write required"));
        }
        let now = Utc::now().to_rfc3339();
        let mut tx = self.db.begin().await?;
        let row = sqlx::query("SELECT id, locked_at FROM versions WHERE project_id = ? AND version = ?")
            .bind(project.0)
            .bind(version)
            .fetch_optional(&mut *tx)
            .await?;
        let Some(row) = row else {
            return Err(VersionError::not_found(format!("version {version} not found")));
        };
        let version_id: i64 = row.try_get("id")?;
        if row.try_get::<Option<String>, _>("locked_at")?.is_some() {
            return Err(VersionError::conflict(format!("version {version} is locked")));
        }
        let existing = sqlx::query("SELECT 1 AS one FROM version_apps WHERE version_id = ? AND app = ?")
            .bind(version_id)
            .bind(app.trim())
            .fetch_optional(&mut *tx)
            .await?;
        let created = existing.is_none();
        sqlx::query(
            "INSERT INTO version_apps (version_id, app, file_id, sha256, size, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(version_id, app) DO UPDATE SET
                 file_id = excluded.file_id,
                 sha256 = excluded.sha256,
                 size = excluded.size,
                 updated_at = excluded.updated_at",
        )
        .bind(version_id)
        .bind(app.trim())
        .bind(&file.file_id.0)
        .bind(&file.sha256)
        .bind(file.size as i64)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        let record = self.aggregate_version(project, version).await?;
        Ok(VersionPublish { created, record })
    }

    async fn delete_app(&self, project: &ProjectId, version: &str, app: &str, actor: &Principal) -> VersionResult<()> {
        let version = version.trim();
        Self::validate_app(app)?;
        if !self.checker.can_access(actor, &Resource::Project(*project), ACTION_ARTIFACTS_WRITE).await? {
            return Err(VersionError::forbidden("artifacts:write required"));
        }
        let version_id = self.require_unlocked(project, version).await?;
        let result = sqlx::query("DELETE FROM version_apps WHERE version_id = ? AND app = ?")
            .bind(version_id)
            .bind(app.trim())
            .execute(&self.db)
            .await?;
        if result.rows_affected() == 0 {
            return Err(VersionError::not_found(format!("app {app} not found in version {version}")));
        }
        Ok(())
    }

    async fn lock(&self, project: &ProjectId, version: &str, actor: &Principal) -> VersionResult<VersionRecord> {
        let version = version.trim();
        if !self.checker.can_access(actor, &Resource::Project(*project), ACTION_ADMIN).await? {
            return Err(VersionError::forbidden("administration required"));
        }
        let result = sqlx::query(
            "UPDATE versions SET locked_at = COALESCE(locked_at, ?) WHERE project_id = ? AND version = ?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(project.0)
        .bind(version)
        .execute(&self.db)
        .await?;
        if result.rows_affected() == 0 {
            return Err(VersionError::not_found(format!("version {version} not found")));
        }
        self.aggregate_version(project, version).await
    }

    async fn list(&self, project: &ProjectId, actor: &Principal) -> VersionResult<Vec<VersionRecord>> {
        if !self.checker.can_access(actor, &Resource::Project(*project), ACTION_METADATA_READ).await? {
            return Err(VersionError::forbidden("metadata:read required"));
        }
        self.aggregate_all(project).await
    }

    async fn get(&self, project: &ProjectId, version: Option<&str>, actor: &Principal) -> VersionResult<VersionRecord> {
        if !self.checker.can_access(actor, &Resource::Project(*project), ACTION_METADATA_READ).await? {
            return Err(VersionError::forbidden("metadata:read required"));
        }
        match version {
            Some(version) => self.aggregate_version(project, version.trim()).await,
            None => {
                let all = self.aggregate_all(project).await?;
                all.into_iter()
                    .next()
                    .ok_or_else(|| VersionError::not_found("no versions in project"))
            }
        }
    }

    async fn referenced_file_ids(&self) -> VersionResult<HashSet<FileId>> {
        let rows = sqlx::query(
            "SELECT va.file_id FROM version_apps va
             JOIN versions v ON va.version_id = v.id
             JOIN projects p ON v.project_id = p.id",
        )
        .fetch_all(&self.db)
        .await?;
        let mut set = HashSet::new();
        for row in rows {
            set.insert(FileId::new(row.try_get::<String, _>("file_id")?));
        }
        Ok(set)
    }
}

impl From<crate::permissions::model::PermissionError> for VersionError {
    fn from(value: crate::permissions::model::PermissionError) -> Self {
        VersionError::db(value.message)
    }
}
