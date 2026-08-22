use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub password_hash: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsersConfig {
    pub users: Vec<UserConfig>,
    pub session_key: String,
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
}

fn default_max_age() -> usize {
    3600
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
