use ed25519_dalek::SigningKey;
use ed25519_dalek::pkcs8::DecodePrivateKey;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub password_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsersConfig {
    pub users: Vec<UserConfig>,
    pub session_private_key: String,
}

impl UsersConfig {
    /// 账号 session/refresh JWT 只接受 Ed25519 PKCS#8 PEM 私钥。
    pub fn validate(&self) -> Result<(), String> {
        SigningKey::from_pkcs8_pem(&self.session_private_key)
            .map(|_| ())
            .map_err(|_| {
                "users.session_private_key must be an Ed25519 PKCS#8 PEM private key".to_string()
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesConfig {
    pub data_dir: PathBuf,
    pub max_archive_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfigSeed {
    pub server_addr: String,
    pub port: u16,
    #[serde(default)]
    pub allow_origins: Vec<String>,
    #[serde(default)]
    pub allow_methods: Vec<String>,
    #[serde(default)]
    pub allow_headers: Vec<String>,
    #[serde(default)]
    pub expose_headers: Vec<String>,
    #[serde(default = "default_max_age")]
    pub max_age: usize,
    #[serde(default)]
    pub support_credentials: bool,
    /// 登录限流：每个来源 key 每分钟最多放行的登录尝试（0=关闭）。
    #[serde(default = "default_login_rate_limit_per_minute")]
    pub login_rate_limit_per_minute: u32,
    /// 登录限流统计窗口（秒）。
    #[serde(default = "default_login_rate_limit_window_secs")]
    pub login_rate_limit_window_secs: u64,
}

fn default_max_age() -> usize {
    3600
}

fn default_login_rate_limit_per_minute() -> u32 {
    30
}

fn default_login_rate_limit_window_secs() -> u64 {
    60
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server: HttpConfigSeed,
    pub users: UsersConfig,
    pub files: FilesConfig,
    #[serde(default = "default_db_path")]
    pub db_path: String,
}

fn default_db_path() -> String {
    "filehub.db".to_string()
}
