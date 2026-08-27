//! sfo-account 的 SQLite 存储实现与账号值对象（I-002）。

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sfo_account::{
    Account, AccountErrorCode, AccountResult, AccountStore, LoginPasswordVerifier, account_err,
};
use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

use crate::model::{UserId, UsersConfig};

pub fn bcrypt_hash(password: &str) -> Result<String, String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| format!("bcrypt hash failed: {e}"))
}

/// 不存在账号登录时使用的固定 bcrypt hash（cost=12，与 `bcrypt_hash` 的
/// `DEFAULT_COST` 一致）。只用于等成本伪校验，不关联任何真实用户，
/// 即使公开也不泄露账号信息。
pub const LOGIN_DUMMY_BCRYPT_HASH: &str =
    "$2b$12$llhysrWVbSpOShBc/i6sRu2v0UEXe0Db.oDVzzRqt9bXaKf/Upi96";

/// 生产落地登录密码校验器：真实账号与账号缺失分支都执行 cost=12 的 bcrypt
/// 校验，且全部移入 `spawn_blocking`，避免占用 sfo-http 的 async worker。
#[derive(Clone, Default)]
pub struct FilehubPasswordVerifier;

#[async_trait::async_trait]
impl LoginPasswordVerifier<FilehubAccount> for FilehubPasswordVerifier {
    async fn verify(&self, account: &FilehubAccount, password: &str, salt: &[u8]) -> bool {
        let account = account.clone();
        let password = password.to_owned();
        let salt = salt.to_vec();
        tokio::task::spawn_blocking(move || account.verify_password(&password, &salt))
            .await
            .unwrap_or(false)
    }

    async fn verify_dummy(&self, password: &str, _salt: &[u8]) -> bool {
        let password = password.to_owned();
        tokio::task::spawn_blocking(move || {
            bcrypt::verify(&password, LOGIN_DUMMY_BCRYPT_HASH).unwrap_or(false)
        })
        .await
        .unwrap_or(false)
    }
}

/// filehub 用户记录。password_hash 只用于登录校验，不进 session JWT。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilehubAccount {
    pub id: UserId,
    pub name: String,
    #[serde(skip)]
    pub password_hash: String,
    /// 停用标记（配置同步维护）。JWT claims 不携带该字段；解码旧 token 时
    /// 缺失即按活跃处理，避免升级前已签发会话被误判失效。
    #[serde(skip_serializing, default = "default_account_active")]
    pub active: bool,
}

fn default_account_active() -> bool {
    true
}

impl FilehubAccount {
    pub fn new_uncommitted(name: impl Into<String>, password_hash: String) -> Self {
        Self {
            id: UserId(0),
            name: name.into(),
            password_hash,
            active: true,
        }
    }
}

impl Account for FilehubAccount {
    type Id = UserId;

    fn account_id(&self) -> &Self::Id {
        &self.id
    }

    fn account_name(&self) -> &str {
        &self.name
    }

    fn verify_password(&self, password: &str, _outer_salt: &[u8]) -> bool {
        if !self.active || self.password_hash.is_empty() {
            return false;
        }
        bcrypt::verify(password, &self.password_hash).unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub struct SqliteAccountStore {
    db: SqlitePool,
}

impl SqliteAccountStore {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.db
    }

    async fn row_to_account(row: &sqlx::sqlite::SqliteRow) -> Result<FilehubAccount, sqlx::Error> {
        Ok(FilehubAccount {
            id: UserId(row.try_get::<i64, _>("id")?),
            name: row.try_get::<String, _>("name")?,
            password_hash: row.try_get::<String, _>("password_hash")?,
            active: row.try_get::<bool, _>("active")?,
        })
    }

    /// seed 同步专用：与 `get_account_by_name` 不同，不按 active 过滤，用于
    /// 更新密码 hash、恢复被停用账号。常规登录/存在性查询不得使用本方法。
    pub async fn get_managed_account_by_name(
        &self,
        account_name: &str,
    ) -> AccountResult<Option<FilehubAccount>> {
        let row = sqlx::query("SELECT id, name, password_hash, active FROM users WHERE name = ?")
            .bind(account_name)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| {
                account_err!(
                    AccountErrorCode::AccountStoreError,
                    "get managed account {} failed: {}",
                    account_name,
                    e
                )
            })?;
        Ok(match row {
            Some(row) => Some(Self::row_to_account(&row).await.map_err(|e| {
                account_err!(
                    AccountErrorCode::AccountStoreError,
                    "row decode failed: {}",
                    e
                )
            })?),
            None => None,
        })
    }

    /// 配置同步收尾：把不在配置名单中的账号停用（active=0，保留行与引用）。
    pub async fn deactivate_not_in(&self, configured_names: &[String]) -> AccountResult<()> {
        if configured_names.is_empty() {
            sqlx::query("UPDATE users SET active = 0 WHERE active = 1")
                .execute(&self.db)
                .await
                .map_err(|e| {
                    account_err!(
                        AccountErrorCode::AccountStoreError,
                        "deactivate unconfigured accounts failed: {}",
                        e
                    )
                })?;
            return Ok(());
        }
        let mut builder = sqlx::QueryBuilder::new(
            "UPDATE users SET active = 0 WHERE active = 1 AND name NOT IN (",
        );
        let mut separated = builder.separated(", ");
        for name in configured_names {
            separated.push_bind(name);
        }
        separated.push_unseparated(")");
        builder.build().execute(&self.db).await.map_err(|e| {
            account_err!(
                AccountErrorCode::AccountStoreError,
                "deactivate unconfigured accounts failed: {}",
                e
            )
        })?;
        Ok(())
    }
}

#[async_trait]
impl AccountStore<FilehubAccount> for SqliteAccountStore {
    async fn get_account(&self, account_id: &UserId) -> AccountResult<Option<FilehubAccount>> {
        let row = sqlx::query("SELECT id, name, password_hash, active FROM users WHERE id = ?")
            .bind(account_id.0)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| {
                account_err!(
                    AccountErrorCode::AccountStoreError,
                    "get account {} failed: {}",
                    account_id.0,
                    e
                )
            })?;
        Ok(match row {
            Some(row) => Some(Self::row_to_account(&row).await.map_err(|e| {
                account_err!(
                    AccountErrorCode::AccountStoreError,
                    "row decode failed: {}",
                    e
                )
            })?),
            None => None,
        })
    }

    async fn get_account_by_name(
        &self,
        account_name: &str,
    ) -> AccountResult<Option<FilehubAccount>> {
        let row = sqlx::query(
            "SELECT id, name, password_hash, active FROM users WHERE name = ? AND active = 1",
        )
        .bind(account_name)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            account_err!(
                AccountErrorCode::AccountStoreError,
                "get account {} failed: {}",
                account_name,
                e
            )
        })?;
        Ok(match row {
            Some(row) => Some(Self::row_to_account(&row).await.map_err(|e| {
                account_err!(
                    AccountErrorCode::AccountStoreError,
                    "row decode failed: {}",
                    e
                )
            })?),
            None => None,
        })
    }

    async fn remove_account(&self, account_id: &UserId) -> AccountResult<()> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(account_id.0)
            .execute(&self.db)
            .await
            .map_err(|e| {
                account_err!(
                    AccountErrorCode::AccountStoreError,
                    "remove account {} failed: {}",
                    account_id.0,
                    e
                )
            })?;
        Ok(())
    }

    async fn add_account(&self, account: &FilehubAccount) -> AccountResult<UserId> {
        let result =
            sqlx::query("INSERT INTO users (name, password_hash, created_at) VALUES (?, ?, ?)")
                .bind(&account.name)
                .bind(&account.password_hash)
                .bind(Utc::now().to_rfc3339())
                .execute(&self.db)
                .await
                .map_err(|e| {
                    account_err!(
                        AccountErrorCode::AccountStoreError,
                        "add account {} failed: {}",
                        account.account_name(),
                        e
                    )
                })?;
        Ok(UserId(result.last_insert_rowid()))
    }

    async fn update_account(&self, account: &FilehubAccount) -> AccountResult<()> {
        sqlx::query("UPDATE users SET name = ?, password_hash = ?, active = ? WHERE id = ?")
            .bind(&account.name)
            .bind(&account.password_hash)
            .bind(account.active)
            .bind(account.id.0)
            .execute(&self.db)
            .await
            .map_err(|e| {
                account_err!(
                    AccountErrorCode::AccountStoreError,
                    "update account {} failed: {}",
                    account.account_name(),
                    e
                )
            })?;
        Ok(())
    }
}

/// 打开 SQLite 连接池。`path == ":memory:"` 时使用单连接（内存库）。
pub async fn connect_pool(path: &str, max_connections: u32) -> Result<SqlitePool, sqlx::Error> {
    let options = if path == ":memory:" {
        // 内存库同样强制外键约束：新库(含内存库)的 project_id 级联语义一致。
        SqliteConnectOptions::from_str("sqlite::memory:")?
            .create_if_missing(true)
            .foreign_keys(true)
    } else {
        SqliteConnectOptions::from_str(&format!("sqlite://{path}"))?
            .create_if_missing(true)
            .foreign_keys(true)
    };
    SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await
}
