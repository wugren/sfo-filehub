//! projects 错误类型（I-007）。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectErrorKind {
    NotFound,
    Forbidden,
    Conflict,
    InvalidInput,
    Db,
}

#[derive(Debug, Clone)]
pub struct ProjectError {
    pub kind: ProjectErrorKind,
    pub message: String,
}

impl ProjectError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: ProjectErrorKind::NotFound,
            message: message.into(),
        }
    }
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            kind: ProjectErrorKind::Forbidden,
            message: message.into(),
        }
    }
    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            kind: ProjectErrorKind::Conflict,
            message: message.into(),
        }
    }
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self {
            kind: ProjectErrorKind::InvalidInput,
            message: message.into(),
        }
    }
    pub fn db(message: impl Into<String>) -> Self {
        Self {
            kind: ProjectErrorKind::Db,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "project error {:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for ProjectError {}

impl From<sqlx::Error> for ProjectError {
    fn from(value: sqlx::Error) -> Self {
        ProjectError::db(value.to_string())
    }
}

pub type ProjectResult<T> = Result<T, ProjectError>;
