//! 认证桥实现：sfo-account session 解码 + PermissionsModule 角色 + TokenService 解析。

use async_trait::async_trait;
use std::sync::Arc;

use crate::account::AccountModule;
use crate::contract::{SessionAuth, TokenAuth};
use crate::model::{Principal, UserId};
use crate::tokens::TokenService;

pub struct SessionAuthWrapper {
    pub account: Arc<AccountModule>,
}

#[async_trait]
impl SessionAuth for SessionAuthWrapper {
    async fn decode_user(&self, bearer: &str) -> Option<UserId> {
        self.account
            .decode_session(bearer)
            .await
            .ok()
            .map(|account| account.id)
    }
}

pub struct TokenAuthWrapper {
    pub tokens: Arc<dyn TokenService>,
}

#[async_trait]
impl TokenAuth for TokenAuthWrapper {
    async fn resolve_token(&self, bearer: &str) -> Option<Principal> {
        self.tokens
            .resolve(bearer)
            .await
            .ok()
            .map(|tp| Principal::Token {
                token_id: tp.token_id,
                scopes: tp.scopes,
                user_id: tp.user_id,
                project_scope: tp.project_scope,
            })
    }
}
