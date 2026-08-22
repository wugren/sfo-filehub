//! tokens 数据模型、请求/响应结构与错误类型（I-004）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::{ProjectScope, ScopeSet, TokenId, UserId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCreateRequest {
    /// HTTP 层会覆盖为当前登录用户；缺省 0 仅用于反序列化占位。
    #[serde(default = "default_owner")]
    pub owner: UserId,
    pub name: String,
    #[serde(default)]
    pub project_scope: Option<ProjectScope>,
    #[serde(default)]
    pub scopes: Vec<crate::model::Scope>,
    /// 仅写入本次签发 JWT 的 exp；token 记录不保存过期字段。
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUpdateRequest {
    pub name: Option<String>,
    pub project_scope: Option<ProjectScope>,
    pub scopes: Option<Vec<crate::model::Scope>>,
    /// None=不修改；Some(None)=改成不过期；Some(Some(t))=新 JWT exp。
    pub expires_at: Option<Option<DateTime<Utc>>>,
}

/// token JWT 载荷：token_id 由 jti 冗余携带，解析时二次校验。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPayload {
    pub token_id: TokenId,
    pub user_id: UserId,
    pub scopes: ScopeSet,
}

/// tokens::resolve 的结果（认证中间件由此构造 Principal::Token）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrincipal {
    pub token_id: TokenId,
    pub user_id: UserId,
    pub scopes: ScopeSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenErrorKind {
    NotFound,
    InvalidInput,
    Db,
}

#[derive(Debug, Clone)]
pub struct TokenError {
    pub kind: TokenErrorKind,
    pub message: String,
}

impl TokenError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self { kind: TokenErrorKind::NotFound, message: message.into() }
    }
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self { kind: TokenErrorKind::InvalidInput, message: message.into() }
    }
    pub fn db(message: impl Into<String>) -> Self {
        Self { kind: TokenErrorKind::Db, message: message.into() }
    }
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "token error {:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for TokenError {}

impl From<sqlx::Error> for TokenError {
    fn from(value: sqlx::Error) -> Self {
        TokenError::db(value.to_string())
    }
}

pub type TokenResult<T> = Result<T, TokenError>;

fn default_owner() -> UserId {
    UserId(0)
}
