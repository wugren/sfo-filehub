//! 认证桥实现：sfo-account session 解码 + PermissionsModule 角色 + TokenService 解析。

use async_trait::async_trait;
use std::sync::Arc;

use crate::account::AccountModule;
use crate::contract::{SessionAuth, TokenAuth};
use crate::model::{AccountRole, Principal, UserId};
use crate::permissions::PermissionsModule;
use crate::tokens::TokenService;

pub struct SessionAuthWrapper {
    pub account: Arc<AccountModule>,
    pub permissions: Arc<PermissionsModule>,
}

#[async_trait]
impl SessionAuth for SessionAuthWrapper {
    async fn decode_user(&self, bearer: &str) -> Option<UserId> {
        self.account.decode_session(bearer).await.ok().map(|account| account.id)
    }

    async fn role_of(&self, user_id: UserId) -> AccountRole {
        self.permissions.role_for_user(user_id).await.unwrap_or(AccountRole::Member)
    }
}

pub struct TokenAuthWrapper {
    pub tokens: Arc<dyn TokenService>,
}

#[async_trait]
impl TokenAuth for TokenAuthWrapper {
    async fn resolve_token(&self, bearer: &str) -> Option<Principal> {
        self.tokens.resolve(bearer).await.ok().map(|tp| Principal::Token {
            token_id: tp.token_id,
            scopes: tp.scopes,
            user_id: tp.user_id,
        })
    }
}
