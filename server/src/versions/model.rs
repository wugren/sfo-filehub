//! versions 错误类型与记录（I-006）。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionErrorKind {
    NotFound,
    Forbidden,
    Conflict,
    InvalidInput,
    Db,
}

#[derive(Debug, Clone)]
pub struct VersionError {
    pub kind: VersionErrorKind,
    pub message: String,
}

impl VersionError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: VersionErrorKind::NotFound,
            message: message.into(),
        }
    }
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            kind: VersionErrorKind::Forbidden,
            message: message.into(),
        }
    }
    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            kind: VersionErrorKind::Conflict,
            message: message.into(),
        }
    }
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self {
            kind: VersionErrorKind::InvalidInput,
            message: message.into(),
        }
    }
    pub fn db(message: impl Into<String>) -> Self {
        Self {
            kind: VersionErrorKind::Db,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for VersionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "version error {:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for VersionError {}

impl From<sqlx::Error> for VersionError {
    fn from(value: sqlx::Error) -> Self {
        VersionError::db(value.to_string())
    }
}

pub type VersionResult<T> = Result<T, VersionError>;
