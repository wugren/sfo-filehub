use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountRole {
    Owner,
    Member,
}

impl Default for AccountRole {
    fn default() -> Self {
        AccountRole::Member
    }
}

impl Display for AccountRole {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountRole::Owner => write!(f, "owner"),
            AccountRole::Member => write!(f, "member"),
        }
    }
}

impl FromStr for AccountRole {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "owner" => Ok(AccountRole::Owner),
            "member" => Ok(AccountRole::Member),
            other => Err(format!("invalid account role: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectRole {
    Read,
    Write,
    Admin,
}

impl Display for ProjectRole {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectRole::Read => write!(f, "read"),
            ProjectRole::Write => write!(f, "write"),
            ProjectRole::Admin => write!(f, "admin"),
        }
    }
}

impl FromStr for ProjectRole {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "read" => Ok(ProjectRole::Read),
            "write" => Ok(ProjectRole::Write),
            "admin" => Ok(ProjectRole::Admin),
            other => Err(format!("invalid project role: {other}")),
        }
    }
}
