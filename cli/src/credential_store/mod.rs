//! 本地凭据与配置存储（003 设计：credential_store 独占本地持久状态）。

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

mod model;
mod security;

pub use model::{ConfigDocument, ServerCredential};

/// 服务器凭据（token 优先于登录 session 复用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Credential {
    PasswordSession {
        session: String,
        refresh_session: String,
    },
    Token {
        token: String,
    },
}

/// 凭据存储错误（本地文件系统/格式类，不携带凭据明文）。
#[derive(Debug)]
pub enum CredentialStoreError {
    Io(String),
    Corrupt(String),
    NoServer(String),
    NotLoggedIn(String),
}

impl fmt::Display for CredentialStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CredentialStoreError::Io(message) => write!(f, "{message}"),
            CredentialStoreError::Corrupt(message) => write!(f, "{message}"),
            CredentialStoreError::NoServer(message) => write!(f, "{message}"),
            CredentialStoreError::NotLoggedIn(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for CredentialStoreError {}

impl From<io::Error> for CredentialStoreError {
    fn from(value: io::Error) -> Self {
        CredentialStoreError::Io(value.to_string())
    }
}

/// 本地凭据/配置存储，持有内存模型并通过 `flush` 原子落盘。
#[derive(Debug, Clone)]
pub struct CredentialStore {
    path: PathBuf,
    default_server: Option<String>,
    servers: HashMap<String, ServerCredential>,
}

impl CredentialStore {
    /// 打开（不存在则空配置）；解析失败即报错，不自动覆盖/删除既有文件。
    pub fn open(path: &Path) -> Result<Self, CredentialStoreError> {
        if !path.exists() {
            return Ok(CredentialStore {
                path: path.to_path_buf(),
                default_server: None,
                servers: HashMap::new(),
            });
        }
        let raw = std::fs::read_to_string(path)?;
        let doc: ConfigDocument = toml::from_str(&raw).map_err(|e| {
            CredentialStoreError::Corrupt(format!(
                "failed to parse configuration/credential file ({}): {e}; sign in again or remove the file manually and retry",
                path.display()
            ))
        })?;
        Ok(CredentialStore {
            path: path.to_path_buf(),
            default_server: doc.default_server,
            servers: doc.server,
        })
    }

    /// 解析目标服务器：显式参数 > FILEHUB_SERVER > 配置 default_server > 唯一已存 server。
    pub fn resolve_server(
        &self,
        explicit: Option<&str>,
        env_server: Option<&str>,
    ) -> Result<String, CredentialStoreError> {
        if let Some(value) = explicit {
            return Ok(normalize_server(value)?);
        }
        if let Some(value) = env_server {
            return Ok(normalize_server(value)?);
        }
        if let Some(value) = &self.default_server {
            return Ok(normalize_server(value)?);
        }
        if self.servers.len() == 1 {
            if let Some(key) = self.servers.keys().next() {
                return normalize_server(key);
            }
        }
        Err(CredentialStoreError::NoServer(
            "no server was specified and no default or sole stored server is available; pass SERVER explicitly or run filehub login first"
                .to_string(),
        ))
    }

    /// 取当前可用凭据：token 优先，否则要求 session 存在。
    pub fn current_credential(&self, server: &str) -> Option<Credential> {
        let identity = normalize_server(server).ok()?;
        let credential = self.servers.get(&identity).or_else(|| {
            self.servers
                .iter()
                .find(|(key, _)| normalize_server(key).ok().as_deref() == Some(identity.as_str()))
                .map(|(_, entry)| entry)
        })?;
        if let Some(token) = &credential.token {
            return Some(Credential::Token {
                token: token.clone(),
            });
        }
        match (&credential.session, &credential.refresh_session) {
            (Some(session), Some(refresh_session)) => Some(Credential::PasswordSession {
                session: session.clone(),
                refresh_session: refresh_session.clone(),
            }),
            _ => None,
        }
    }

    /// 保存密码登录结果；覆盖该 server 的 token（两种凭据互斥）。
    pub fn save_session(
        &mut self,
        server: &str,
        user: &str,
        session: &str,
        refresh: &str,
    ) -> Result<(), CredentialStoreError> {
        let identity = normalize_server(server)?;
        let key = self
            .credential_key(&identity)
            .unwrap_or_else(|| identity.clone());
        let entry = self.servers.entry(key).or_default();
        entry.username = Some(user.to_string());
        entry.session = Some(session.to_string());
        entry.refresh_session = Some(refresh.to_string());
        entry.token = None;
        self.default_server.get_or_insert_with(|| identity.clone());
        Ok(())
    }

    /// 保存 token 登录结果；清除该 server 的 session 字段。
    pub fn save_token(&mut self, server: &str, token: &str) -> Result<(), CredentialStoreError> {
        let identity = normalize_server(server)?;
        let key = self
            .credential_key(&identity)
            .unwrap_or_else(|| identity.clone());
        let entry = self.servers.entry(key).or_default();
        entry.token = Some(token.to_string());
        entry.username = None;
        entry.session = None;
        entry.refresh_session = None;
        self.default_server.get_or_insert_with(|| identity.clone());
        Ok(())
    }

    /// refresh 续期落盘（不改变凭据类型）。
    pub fn update_session(
        &mut self,
        server: &str,
        session: &str,
        refresh: &str,
    ) -> Result<(), CredentialStoreError> {
        let identity = normalize_server(server)?;
        let key = self.credential_key(&identity).ok_or_else(|| {
            CredentialStoreError::NotLoggedIn(
                "session refresh failed because this server has no local credentials; sign in again"
                    .to_string(),
            )
        })?;
        let entry = self.servers.get_mut(&key).expect("key from credential_key");
        entry.session = Some(session.to_string());
        entry.refresh_session = Some(refresh.to_string());
        Ok(())
    }

    /// 清除指定（或解析所得默认）服务器的全部本地凭据。
    pub fn logout(
        &mut self,
        server: Option<&str>,
        env_server: Option<&str>,
    ) -> Result<String, CredentialStoreError> {
        let target = self.resolve_server(server, env_server)?;
        let matched: Vec<String> = self
            .servers
            .keys()
            .filter(|key| normalize_server(key).ok().as_deref() == Some(target.as_str()))
            .cloned()
            .collect();
        if matched.is_empty() {
            return Err(CredentialStoreError::NotLoggedIn(format!(
                "not signed in to {target}; there are no credentials to remove"
            )));
        }
        for key in matched {
            self.servers.remove(&key);
        }
        if self
            .default_server
            .as_ref()
            .and_then(|value| normalize_server(value).ok())
            .as_deref()
            == Some(target.as_str())
        {
            self.default_server = None;
        }
        Ok(target)
    }

    /// 原子写 + 权限收敛（类 Unix 0600）。
    pub fn flush(&self) -> Result<(), CredentialStoreError> {
        let doc = ConfigDocument {
            schema_version: 1,
            default_server: self.default_server.clone(),
            server: self.servers.clone(),
        };
        let content = toml::to_string(&doc).map_err(|e| {
            CredentialStoreError::Io(format!("failed to serialize configuration: {e}"))
        })?;
        security::atomic_write(&self.path, content.as_bytes())
    }

    /// 按身份 key 定位实际存储 key：优先精确 key，其次兼容旧带协议 key。
    fn credential_key(&self, identity: &str) -> Option<String> {
        if self.servers.contains_key(identity) {
            return Some(identity.to_string());
        }
        self.servers
            .keys()
            .find(|key| normalize_server(key).ok().as_deref() == Some(identity))
            .cloned()
    }

    /// 配置文件路径（`--config`/FILEHUB_CONFIG 覆盖平台默认路径）。
    pub fn config_path(config_override: Option<&Path>) -> PathBuf {
        if let Some(path) = config_override {
            return path.to_path_buf();
        }
        if let Ok(value) = std::env::var("FILEHUB_CONFIG") {
            if !value.trim().is_empty() {
                return PathBuf::from(value);
            }
        }
        default_config_path()
    }
}

/// 平台默认配置路径：类 Unix `~/.config/filehub/config.toml`、
/// macOS `~/Library/Application Support/filehub/config.toml`、
/// Windows `%APPDATA%\filehub\config.toml`。
pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .map(|dir| dir.join("filehub").join("config.toml"))
        .unwrap_or_else(|| PathBuf::from("config.toml"))
}

/// 归一化服务器身份：去掉协议头与路径，只保留 `host[:port]`（Docker ConvertToHostname 语义）。
pub(crate) fn normalize_server(value: &str) -> Result<String, CredentialStoreError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CredentialStoreError::NoServer(
            "server address cannot be empty".to_string(),
        ));
    }
    let without_scheme = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed);
    let identity = without_scheme
        .split('/')
        .next()
        .unwrap_or("")
        .trim()
        .trim_end_matches('/');
    if identity.is_empty() {
        return Err(CredentialStoreError::NoServer(
            "server address is missing host[:port]".to_string(),
        ));
    }
    Ok(identity.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_precedes_session() {
        let mut store = CredentialStore {
            path: PathBuf::from("/tmp/not-used.toml"),
            default_server: Some("https://fh.example.com".to_string()),
            servers: HashMap::new(),
        };
        let server = "https://fh.example.com";
        store
            .save_session(server, "alice", "session-a", "refresh-a")
            .unwrap();
        assert!(matches!(
            store.current_credential(server),
            Some(Credential::PasswordSession { .. })
        ));
        store.save_token(server, "token-b").unwrap();
        assert!(matches!(
            store.current_credential(server),
            Some(Credential::Token { .. })
        ));
        store
            .save_session(server, "alice", "session-c", "refresh-c")
            .unwrap();
        assert!(matches!(
            store.current_credential(server),
            Some(Credential::PasswordSession { .. })
        ));
    }

    #[test]
    fn resolve_server_precedence() {
        let mut servers = HashMap::new();
        servers.insert(
            "https://a.example.com".to_string(),
            ServerCredential::default(),
        );
        let store = CredentialStore {
            path: PathBuf::from("/tmp/x.toml"),
            default_server: Some("https://a.example.com".to_string()),
            servers,
        };
        assert_eq!(
            store
                .resolve_server(Some("b.example.com:8443"), None)
                .unwrap(),
            "b.example.com:8443"
        );
        assert_eq!(
            store
                .resolve_server(Some("https://c.example.com/"), None)
                .unwrap(),
            "c.example.com"
        );
        assert_eq!(
            store
                .resolve_server(None, Some("https://d.example.com"))
                .unwrap(),
            "d.example.com"
        );
        assert_eq!(store.resolve_server(None, None).unwrap(), "a.example.com");
    }

    #[test]
    fn legacy_scheme_keys_are_matched_by_identity() {
        let mut servers = HashMap::new();
        servers.insert(
            "http://127.0.0.1:8080".to_string(),
            ServerCredential {
                username: Some("alice".to_string()),
                session: Some("legacy-session".to_string()),
                refresh_session: Some("legacy-refresh".to_string()),
                token: None,
            },
        );
        let mut store = CredentialStore {
            path: PathBuf::from("/tmp/legacy.toml"),
            default_server: None,
            servers: servers.clone(),
        };

        // 无协议身份也能命中旧协议 key。
        let credential = store
            .current_credential("127.0.0.1:8080")
            .expect("legacy match");
        assert!(matches!(
            credential,
            Credential::PasswordSession { session, .. } if session == "legacy-session"
        ));

        // 重新登录写入并缓存续期到现有 key，不产生重复身份记录。
        store
            .save_session("127.0.0.1:8080", "alice", "s-new", "r-new")
            .unwrap();
        assert_eq!(store.servers.len(), 1);
        assert!(store.servers.contains_key("http://127.0.0.1:8080"));

        store
            .update_session("127.0.0.1:8080", "s-called", "r-called")
            .unwrap();
        assert_eq!(store.servers.len(), 1);

        let target = store.logout(Some("127.0.0.1:8080"), None).unwrap();
        assert_eq!(target, "127.0.0.1:8080");
        assert!(store.servers.is_empty());
    }
}
