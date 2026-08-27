//! tokens 子模块：JWT 形态 token 生命周期与权限数据（P-03 fh-server-tokens）。

pub mod http;
pub mod model;
pub mod service;

use async_trait::async_trait;
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

use crate::model::{TokenId, TokenIssued, TokenSummary, UserId};

use model::{TokenCreateRequest, TokenPrincipal, TokenResult, TokenUpdateRequest};

#[async_trait]
pub trait TokenService: 'static + Send + Sync {
    async fn create(&self, req: TokenCreateRequest) -> TokenResult<TokenIssued>;
    async fn list(&self, owner: &UserId) -> TokenResult<Vec<TokenSummary>>;
    /// 属性修改（name/project_scope/scopes）只落库、不重签，返回最新摘要。
    async fn update(
        &self,
        token_id: &TokenId,
        owner: &UserId,
        patch: TokenUpdateRequest,
    ) -> TokenResult<TokenSummary>;
    async fn rotate(&self, token_id: &TokenId, owner: &UserId) -> TokenResult<TokenIssued>;
    async fn revoke(&self, token_id: &TokenId, owner: &UserId) -> TokenResult<()>;
    async fn resolve(&self, bearer: &str) -> TokenResult<TokenPrincipal>;
}

pub struct TokensModule {
    service: Arc<dyn TokenService>,
}

impl TokensModule {
    pub async fn init(db: &SqlitePool) -> Result<Self, String> {
        sqlx::raw_sql(include_str!("../../migrations/0004_tokens.sql"))
            .execute(db)
            .await
            .map_err(|e| format!("apply 0004_tokens.sql failed: {e}"))?;
        Ok(Self {
            service: Arc::new(service::SqliteTokenService::new(db.clone())),
        })
    }

    pub fn service(&self) -> Arc<dyn TokenService> {
        self.service.clone()
    }
}

pub use model::{TokenError, TokenErrorKind};
