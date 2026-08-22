//! v1 契约 DTO（与 `docs/api/v1-contract.md` 数据形状对齐）。

use serde::{Deserialize, Serialize};

/// sfo-account 登录请求体。
#[derive(Debug, Clone, Serialize)]
pub struct LoginRequest<'a> {
    pub user_name: &'a str,
    pub password: &'a str,
    pub timestamp: i64,
}

/// 登录/refresh 成功结果。
#[derive(Debug, Clone, Deserialize)]
pub struct LoginResp {
    pub session: String,
    pub refresh_session: String,
}

/// sfo-http 统一包装 `{err, result}`。
#[derive(Debug, Clone, Deserialize)]
pub struct SfoEnvelope<T> {
    #[serde(default)]
    pub err: i64,
    pub result: Option<T>,
}

/// 项目记录（服务端 `ProjectRecord`）。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectDto {
    pub project_id: i64,
    pub name: String,
    pub visibility: String,
    pub owner: i64,
}

/// 版本记录（服务端 `VersionRecord`）。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppDto {
    pub app: String,
    pub file_id: String,
    pub sha256: String,
    pub size: u64,
    pub updated_at: String,
}

/// 版本记录（服务端 `VersionRecord`，含全部 app 信息与锁定状态）。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionDto {
    pub project_id: i64,
    pub version: String,
    pub published_at: String,
    pub locked_at: Option<String>,
    pub apps: Vec<AppDto>,
}

/// 统一错误体 `{"error": code, "message": text}`。
#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorBody {
    pub error: String,
    pub message: String,
}
