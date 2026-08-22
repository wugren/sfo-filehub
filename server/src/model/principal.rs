use serde::{Deserialize, Serialize};

use super::{AccountRole, ProjectId, ScopeSet, TokenId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Principal {
    Anonymous,
    User { user_id: UserId, account_role: AccountRole },
    Token { token_id: TokenId, scopes: ScopeSet, user_id: UserId },
}

impl Principal {
    pub fn user_id(&self) -> Option<UserId> {
        match self {
            Principal::Anonymous => None,
            Principal::User { user_id, .. } | Principal::Token { user_id, .. } => Some(*user_id),
        }
    }

    pub fn display_kind(&self) -> &'static str {
        match self {
            Principal::Anonymous => "anonymous",
            Principal::User { .. } => "user-session",
            Principal::Token { .. } => "token-session",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeatureName {
    ProjectsCreate,
    ProjectsDelete,
}

impl FeatureName {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeatureName::ProjectsCreate => "projects:create",
            FeatureName::ProjectsDelete => "projects:delete",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resource {
    Project(ProjectId),
    Feature(FeatureName),
}

impl Resource {
    pub fn project(project_id: ProjectId) -> Self {
        Resource::Project(project_id)
    }
}
