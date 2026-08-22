use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use super::ProjectId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
    #[serde(rename = "metadata:read")]
    MetadataRead,
    #[serde(rename = "artifacts:read")]
    ArtifactsRead,
    #[serde(rename = "artifacts:write")]
    ArtifactsWrite,
    #[serde(rename = "administration")]
    Administration,
    #[serde(rename = "projects:create")]
    ProjectsCreate,
    #[serde(rename = "projects:delete")]
    ProjectsDelete,
}

impl Display for Scope {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Scope::MetadataRead => write!(f, "metadata:read"),
            Scope::ArtifactsRead => write!(f, "artifacts:read"),
            Scope::ArtifactsWrite => write!(f, "artifacts:write"),
            Scope::Administration => write!(f, "administration"),
            Scope::ProjectsCreate => write!(f, "projects:create"),
            Scope::ProjectsDelete => write!(f, "projects:delete"),
        }
    }
}

impl FromStr for Scope {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "metadata:read" => Ok(Scope::MetadataRead),
            "artifacts:read" => Ok(Scope::ArtifactsRead),
            "artifacts:write" => Ok(Scope::ArtifactsWrite),
            "administration" => Ok(Scope::Administration),
            "projects:create" => Ok(Scope::ProjectsCreate),
            "projects:delete" => Ok(Scope::ProjectsDelete),
            other => Err(format!("invalid scope: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectScope {
    All,
    Specified(Vec<ProjectId>),
}

impl Display for ProjectScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectScope::All => write!(f, "all"),
            ProjectScope::Specified(ids) => {
                let joined = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
                write!(f, "{joined}")
            }
        }
    }
}

impl FromStr for ProjectScope {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.eq_ignore_ascii_case("all") {
            return Ok(ProjectScope::All);
        }
        let mut ids = Vec::new();
        for part in trimmed.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            ids.push(ProjectId(part.parse().map_err(|_| format!("invalid project id: {part}"))?));
        }
        if ids.is_empty() {
            return Err("empty project scope".to_string());
        }
        Ok(ProjectScope::Specified(ids))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeSet(pub HashSet<Scope>);

impl ScopeSet {
    pub fn contains_scope(&self, scope: Scope) -> bool {
        self.0.contains(&scope)
    }

    pub fn to_storage_string(&self) -> String {
        let mut values: Vec<String> = self.0.iter().map(|s| s.to_string()).collect();
        values.sort();
        values.join(",")
    }

    pub fn from_storage_string(s: &str) -> Result<Self, String> {
        let mut set = HashSet::new();
        for part in s.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            set.insert(part.parse()?);
        }
        Ok(ScopeSet(set))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Private,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Private
    }
}

impl Display for Visibility {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Visibility::Public => write!(f, "public"),
            Visibility::Private => write!(f, "private"),
        }
    }
}

impl FromStr for Visibility {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "public" => Ok(Visibility::Public),
            "private" => Ok(Visibility::Private),
            other => Err(format!("invalid visibility: {other}")),
        }
    }
}
