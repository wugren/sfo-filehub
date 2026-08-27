//! account 子模块：配置驱动初始化 + sfo-account 装配（P-01 fh-server-account）。

pub mod authn;
pub mod http;
pub mod rate_limit;
pub mod store;

use sfo_account::{
    AccountManager, AccountStore, DefaultAccountManager, LoginRateLimiter, SessionConfig,
};
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

use crate::model::{CurrentUser, UserConfig, UsersConfig};

use self::store::{FilehubAccount, FilehubPasswordVerifier, SqliteAccountStore, bcrypt_hash};

pub struct AccountModule {
    manager: Arc<DefaultAccountManager<FilehubAccount, SqliteAccountStore>>,
    store: SqliteAccountStore,
    db: SqlitePool,
    login_rate_limiter: Option<Arc<dyn LoginRateLimiter>>,
}

impl AccountModule {
    /// 配置驱动初始化：users 表 + 初始账号（幂等）；无账号级角色。
    pub async fn init(
        config: &UsersConfig,
        login_rate_limiter: Option<Arc<dyn LoginRateLimiter>>,
        db: &SqlitePool,
    ) -> Result<Self, String> {
        sqlx::raw_sql(include_str!("../../migrations/0001_core.sql"))
            .execute(db)
            .await
            .map_err(|e| format!("apply 0001_core.sql failed: {e}"))?;
        sqlx::raw_sql(include_str!("../../migrations/0002_accounts.sql"))
            .execute(db)
            .await
            .map_err(|e| format!("apply 0002_accounts.sql failed: {e}"))?;
        ensure_account_active_column(db).await?;
        let store = SqliteAccountStore::new(db.clone());
        let manager = DefaultAccountManager::new_eddsa_with_login_verifier_and_session_config(
            store.clone(),
            config.session_private_key.as_bytes(),
            Arc::new(FilehubPasswordVerifier::default()),
            SessionConfig::default(),
        )
        .map_err(|e| format!("init sfo-account manager failed: {}", e.msg()))?;
        let configured_names: Vec<String> =
            config.users.iter().map(|u| u.username.clone()).collect();
        for user in &config.users {
            seed_user(&manager, &store, user).await?;
        }
        store
            .deactivate_not_in(&configured_names)
            .await
            .map_err(|e| format!("sync account active state failed: {}", e.msg()))?;
        Ok(Self {
            manager,
            store,
            db: db.clone(),
            login_rate_limiter,
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
    pub async fn decode_session(
        &self,
        bearer_session: &str,
    ) -> sfo_account::AccountResult<FilehubAccount> {
        self.manager.decode_session(bearer_session).await
    }

    pub fn current_user(&self, account: &FilehubAccount) -> CurrentUser {
        CurrentUser {
            id: account.id,
            username: account.name.clone(),
        }
    }
}

/// 幂等迁移守卫：SQLite 不支持 `ADD COLUMN IF NOT EXISTS`，仅当 `active`
/// 列缺失时执行 0008，新库与已有库走同一路径，重复启动无副作用。
async fn ensure_account_active_column(db: &SqlitePool) -> Result<(), String> {
    let active_exists = sqlx::query_scalar::<_, String>(
        "SELECT name FROM pragma_table_info('users') WHERE name = 'active'",
    )
    .fetch_optional(db)
    .await
    .map_err(|e| format!("check users.active column failed: {e}"))?
    .is_some();
    if !active_exists {
        sqlx::raw_sql(include_str!("../../migrations/0008_accounts_active.sql"))
            .execute(db)
            .await
            .map_err(|e| format!("apply 0008_accounts_active.sql failed: {e}"))?;
    }
    Ok(())
}

async fn seed_user(
    manager: &Arc<DefaultAccountManager<FilehubAccount, SqliteAccountStore>>,
    store: &SqliteAccountStore,
    user: &UserConfig,
) -> Result<(), String> {
    if let Some(existing) = store
        .get_managed_account_by_name(&user.username)
        .await
        .map_err(|e| e.msg().to_string())?
    {
        return upsert_seed_user(store, existing, user).await;
    }
    // 新增账号：校验配置凭据并创建。
    let hash = resolve_password_hash(user)?;
    let account = FilehubAccount::new_uncommitted(user.username.clone(), hash);
    manager
        .create_account(&account)
        .await
        .map_err(|e| e.msg().to_string())?;
    Ok(())
}

/// 同名已存在账号：按新配置校验凭据并同步 hash/active。
/// - `password` 配置：先对现有 hash 做 bcrypt 校验，密码未变（且账号活跃）
///   时不做写库，避免每次启动重写 hash；不匹配才重新生成并写入。
/// - `password_hash` 配置：始终做完整 bcrypt 解析与 cost 校验（修复"已存在
///   账号不校验新配置 hash"的缺陷），与库中 hash 不一致才写入。
/// - 两种路径都把账号恢复为 active=1（停用后重新入配置即恢复）。
async fn upsert_seed_user(
    store: &SqliteAccountStore,
    existing: FilehubAccount,
    user: &UserConfig,
) -> Result<(), String> {
    let (hash_matches, resolved_hash) = match &user.password {
        Some(password) => {
            if password.len() > 72 {
                return Err(format!(
                    "user {} password longer than 72 bytes; bcrypt supports at most 72 bytes",
                    user.username
                ));
            }
            let matches = existing.active
                && bcrypt::verify(password, &existing.password_hash).unwrap_or(false);
            (matches, None)
        }
        None => {
            let hash = resolve_password_hash(user)?;
            (
                existing.active && existing.password_hash == hash,
                Some(hash),
            )
        }
    };
    if hash_matches {
        return Ok(());
    }
    let updated_hash = match resolved_hash {
        Some(hash) => hash,
        None => resolve_password_hash(user)?,
    };
    let updated = FilehubAccount {
        id: existing.id,
        name: user.username.clone(),
        password_hash: updated_hash,
        active: true,
    };
    store
        .update_account(&updated)
        .await
        .map_err(|e| e.msg().to_string())?;
    Ok(())
}

/// 生成/校验配置凭据对应的目标 hash（创建与更新共用）。
fn resolve_password_hash(user: &UserConfig) -> Result<String, String> {
    if let Some(password) = &user.password {
        if password.len() > 72 {
            return Err(format!(
                "user {} password longer than 72 bytes; bcrypt supports at most 72 bytes",
                user.username
            ));
        }
        bcrypt_hash(password)
    } else if let Some(hash) = &user.password_hash {
        let parts = hash.parse::<bcrypt::HashParts>().map_err(|e| {
            format!(
                "user {} password_hash must be a valid bcrypt encoded string: {e}",
                user.username
            )
        })?;
        if !(4..=31).contains(&parts.get_cost()) {
            return Err(format!(
                "user {} password_hash has bcrypt cost {} outside supported range 4..=31",
                user.username,
                parts.get_cost()
            ));
        }
        Ok(hash.clone())
    } else {
        Err(format!(
            "user {} must set password or password_hash",
            user.username
        ))
    }
}
