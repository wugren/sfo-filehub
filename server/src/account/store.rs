//! sfo-account 的 SQLite 存储实现与账号值对象（I-002）。

use async_trait::async_trait;
use chrono::Utc;
use sfo_account::{account_err, Account, AccountErrorCode, AccountResult, AccountStore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::str::FromStr;

use crate::model::{UserId, UsersConfig};

pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex_encode(&hasher.finalize())
}

pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    getrandom::getrandom(&mut buf).expect("os random failed");
    hex_encode(&buf)
}

pub fn password_hash_hex(password: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(b":");
    hasher.update(salt.as_bytes());
    hex_encode(&hasher.finalize())
}

/// filehub 用户记录。salt/password_hash 只用于登录校验，不进 session JWT。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilehubAccount {
    pub id: UserId,
    pub name: String,
    #[serde(skip)]
    pub salt: String,
    #[serde(skip)]
    pub password_hash: String,
}

impl FilehubAccount {
    pub fn new_uncommitted(name: impl Into<String>, salt: String, password_hash: String) -> Self {
        Self {
            id: UserId(0),
            name: name.into(),
            salt,
            password_hash,
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
        if self.salt.is_empty() || self.password_hash.is_empty() {
            return false;
        }
        password_hash_hex(password, &self.salt) == self.password_hash
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
            salt: row.try_get::<String, _>("salt")?,
            password_hash: row.try_get::<String, _>("password_hash")?,
        })
    }
}

#[async_trait]
impl AccountStore<FilehubAccount> for SqliteAccountStore {
    async fn get_account(&self, account_id: &UserId) -> AccountResult<Option<FilehubAccount>> {
        let row = sqlx::query("SELECT id, name, salt, password_hash FROM users WHERE id = ?")
            .bind(account_id.0)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| account_err!(AccountErrorCode::AccountStoreError, "get account {} failed: {}", account_id.0, e))?;
        Ok(match row {
            Some(row) => Some(Self::row_to_account(&row).await.map_err(|e| account_err!(AccountErrorCode::AccountStoreError, "row decode failed: {}", e))?),
            None => None,
        })
    }

    async fn get_account_by_name(&self, account_name: &str) -> AccountResult<Option<FilehubAccount>> {
        let row = sqlx::query("SELECT id, name, salt, password_hash FROM users WHERE name = ?")
            .bind(account_name)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| account_err!(AccountErrorCode::AccountStoreError, "get account {} failed: {}", account_name, e))?;
        Ok(match row {
            Some(row) => Some(Self::row_to_account(&row).await.map_err(|e| account_err!(AccountErrorCode::AccountStoreError, "row decode failed: {}", e))?),
            None => None,
        })
    }

    async fn remove_account(&self, account_id: &UserId) -> AccountResult<()> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(account_id.0)
            .execute(&self.db)
            .await
            .map_err(|e| account_err!(AccountErrorCode::AccountStoreError, "remove account {} failed: {}", account_id.0, e))?;
        Ok(())
    }

    async fn add_account(&self, account: &FilehubAccount) -> AccountResult<UserId> {
        let result = sqlx::query("INSERT INTO users (name, password_hash, salt, created_at) VALUES (?, ?, ?, ?)")
            .bind(&account.name)
            .bind(&account.password_hash)
            .bind(&account.salt)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.db)
            .await
            .map_err(|e| account_err!(AccountErrorCode::AccountStoreError, "add account {} failed: {}", account.account_name(), e))?;
        Ok(UserId(result.last_insert_rowid()))
    }

    async fn update_account(&self, account: &FilehubAccount) -> AccountResult<()> {
        sqlx::query("UPDATE users SET name = ?, password_hash = ?, salt = ? WHERE id = ?")
            .bind(&account.name)
            .bind(&account.password_hash)
            .bind(&account.salt)
            .bind(account.id.0)
            .execute(&self.db)
            .await
            .map_err(|e| account_err!(AccountErrorCode::AccountStoreError, "update account {} failed: {}", account.account_name(), e))?;
        Ok(())
    }
}

/// 打开 SQLite 连接池。`path == ":memory:"` 时使用单连接（内存库）。
pub async fn connect_pool(path: &str, max_connections: u32) -> Result<SqlitePool, sqlx::Error> {
    let options = if path == ":memory:" {
        SqliteConnectOptions::from_str("sqlite::memory:")?.create_if_missing(true)
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
