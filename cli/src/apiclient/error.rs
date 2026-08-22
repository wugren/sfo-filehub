//! 客户端接口错误分类（与 CLI 退出码表一致）。

use std::fmt;

/// API 客户端错误。kind 与 CLI 退出码对应：
/// Auth=2、Forbidden=3、Conflict=4、InvalidInput=5、Transport=6、
/// Integrity=7、Local=8。
#[derive(Debug, Clone)]
pub enum ClientError {
    Auth(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    InvalidInput(String),
    Transport(String),
    Integrity(String),
    Local(String),
}

impl ClientError {
    pub fn auth(message: impl Into<String>) -> Self {
        ClientError::Auth(message.into())
    }

    pub fn transport(message: impl Into<String>) -> Self {
        ClientError::Transport(message.into())
    }
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::Auth(message) => write!(f, "{message}"),
            ClientError::Forbidden(message) => write!(f, "{message}"),
            ClientError::NotFound(message) => write!(f, "{message}"),
            ClientError::Conflict(message) => write!(f, "{message}"),
            ClientError::InvalidInput(message) => write!(f, "{message}"),
            ClientError::Transport(message) => write!(f, "{message}"),
            ClientError::Integrity(message) => write!(f, "{message}"),
            ClientError::Local(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<crate::credential_store::CredentialStoreError> for ClientError {
    fn from(value: crate::credential_store::CredentialStoreError) -> Self {
        ClientError::Local(value.to_string())
    }
}

impl From<crate::archive::ArchiveError> for ClientError {
    fn from(value: crate::archive::ArchiveError) -> Self {
        match value {
            crate::archive::ArchiveError::Unsupported(message)
            | crate::archive::ArchiveError::Integrity(message) => ClientError::Integrity(message),
            crate::archive::ArchiveError::LocalFs(message) => ClientError::Local(message),
        }
    }
}
