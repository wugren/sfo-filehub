use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{FileId, ProjectId, ProjectRole, ProjectScope, ScopeSet, TokenId, UserId, Visibility};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: UserId,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub project_id: ProjectId,
    pub name: String,
    pub visibility: Visibility,
    pub owner: UserId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub file_id: FileId,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRecord {
    pub app: String,
    pub file_id: FileId,
    pub sha256: String,
    pub size: u64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRecord {
    pub project_id: ProjectId,
    pub version: String,
    pub published_at: DateTime<Utc>,
    pub locked_at: Option<DateTime<Utc>>,
    pub apps: Vec<AppRecord>,
}

/// app 发布/更新结果：created 区分“新建”(201) 与“更新”(200)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionPublish {
    pub created: bool,
    pub record: VersionRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collaborator {
    pub user_id: UserId,
    pub role: ProjectRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSummary {
    pub token_id: TokenId,
    pub name: String,
    pub project_scope: ProjectScope,
    pub scopes: ScopeSet,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenIssued {
    pub token_id: TokenId,
    pub jwt: String,
    pub name: String,
    pub expires_at: Option<DateTime<Utc>>,
}
