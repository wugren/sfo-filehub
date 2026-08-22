//! account 子模块：配置驱动初始化 + sfo-account 装配（P-01 fh-server-account）。

pub mod authn;
pub mod http;
pub mod store;

use sfo_account::{AccountManager, AccountStore, DefaultAccountManager};
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

use crate::model::{CurrentUser, UserConfig, UsersConfig};

use self::store::{password_hash_hex, random_hex, FilehubAccount, SqliteAccountStore};

pub struct AccountModule {
    manager: Arc<DefaultAccountManager<FilehubAccount, SqliteAccountStore>>,
    store: SqliteAccountStore,
    db: SqlitePool,
}

impl AccountModule {
    /// 配置驱动初始化：users 表 + 初始账号（幂等），[users].role 由 permissions 消费。
    pub async fn init(config: &UsersConfig, db: &SqlitePool) -> Result<Self, String> {
        sqlx::raw_sql(include_str!("../../migrations/0001_core.sql"))
            .execute(db)
            .await
            .map_err(|e| format!("apply 0001_core.sql failed: {e}"))?;
        sqlx::raw_sql(include_str!("../../migrations/0002_accounts.sql"))
            .execute(db)
            .await
            .map_err(|e| format!("apply 0002_accounts.sql failed: {e}"))?;
        let store = SqliteAccountStore::new(db.clone());
        let manager = DefaultAccountManager::new(store.clone(), config.session_key.as_bytes().to_vec());
        for user in &config.users {
            seed_user(&manager, &store, user).await?;
        }
        Ok(Self {
            manager,
            store,
            db: db.clone(),
        })
    }

    pub fn manager(&self) -> &Arc<DefaultAccountManager<FilehubAccount, SqliteAccountStore>> {
        &self.manager
    }

    pub fn store(&self) -> &SqliteAccountStore {
        &self.store
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.db
    }

    /// 认证中间件复用 sfo-account 解码（不保留独立 JwtSessionVerifier）。
    pub async fn decode_session(&self, bearer_session: &str) -> sfo_account::AccountResult<FilehubAccount> {
        self.manager.decode_session(bearer_session).await
    }

    pub fn current_user(&self, account: &FilehubAccount) -> CurrentUser {
        CurrentUser {
            id: account.id,
            username: account.name.clone(),
        }
    }
}

async fn seed_user(
    manager: &Arc<DefaultAccountManager<FilehubAccount, SqliteAccountStore>>,
    store: &SqliteAccountStore,
    user: &UserConfig,
) -> Result<(), String> {
    if store
        .get_account_by_name(&user.username)
        .await
        .map_err(|e| e.msg().to_string())?
        .is_some()
    {
        return Ok(());
    }
    let (salt, hash) = if let Some(password) = &user.password {
        let salt = random_hex(16);
        let hash = password_hash_hex(password, &salt);
        (salt, hash)
    } else if let Some(hash) = &user.password_hash {
        ("config".to_string(), hash.clone())
    } else {
        return Err(format!("user {} must set password or password_hash", user.username));
    };
    let account = FilehubAccount::new_uncommitted(user.username.clone(), salt, hash);
    manager
        .create_account(&account)
        .await
        .map_err(|e| e.msg().to_string())?;
    Ok(())
}
