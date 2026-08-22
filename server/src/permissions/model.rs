//! 权限数据模型与错误类型（I-003）。

use std::error::Error;
use std::fmt::{Display, Formatter};

use async_trait::async_trait;
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use crate::model::{AccountRole, ProjectId, ProjectRecord, ProjectRole, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionErrorKind {
    Db,
    NotFound,
    Forbidden,
}

#[derive(Debug, Clone)]
pub struct PermissionError {
    pub kind: PermissionErrorKind,
    pub message: String,
}

impl PermissionError {
    pub fn db(message: impl Into<String>) -> Self {
        Self { kind: PermissionErrorKind::Db, message: message.into() }
    }
    pub fn not_found(message: impl Into<String>) -> Self {
        Self { kind: PermissionErrorKind::NotFound, message: message.into() }
    }
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self { kind: PermissionErrorKind::Forbidden, message: message.into() }
    }
}

impl Display for PermissionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "permission error {:?}: {}", self.kind, self.message)
    }
}

impl Error for PermissionError {}

impl From<sqlx::Error> for PermissionError {
    fn from(value: sqlx::Error) -> Self {
        PermissionError::db(value.to_string())
    }
}

pub type PermissionResult<T> = Result<T, PermissionError>;

/// 项目只读端口：projects 表归属 projects 子模块，权限核心只经该端口读取。
#[async_trait]
pub trait ProjectAccess: 'static + Send + Sync {
    async fn project(&self, project_id: &ProjectId) -> PermissionResult<Option<ProjectRecord>>;
    async fn list_projects(&self) -> PermissionResult<Vec<ProjectRecord>>;
}

pub(crate) fn row_to_project(row: &SqliteRow) -> Result<ProjectRecord, sqlx::Error> {
    let visibility = row.try_get::<String, _>("visibility")?;
    Ok(ProjectRecord {
        project_id: ProjectId(row.try_get::<i64, _>("id")?),
        name: row.try_get::<String, _>("name")?,
        visibility: visibility.parse().map_err(|e: String| sqlx::Error::Decode(Box::new(std::io::Error::other(e))))?,
        owner: UserId(row.try_get::<i64, _>("owner_id")?),
    })
}

pub(crate) fn parse_role<T>(value: &str) -> Result<T, String>
where
    T: std::str::FromStr<Err = String>,
{
    value.parse()
}

pub(crate) fn parse_project_role(value: &str) -> Result<ProjectRole, String> {
    parse_role(value)
}

pub(crate) fn parse_account_role(value: &str) -> Result<AccountRole, String> {
    parse_role(value)
}
