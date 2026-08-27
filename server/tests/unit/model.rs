use filehub_server::model::{ProjectRole, Scope, ScopeSet, Visibility};
use std::collections::HashSet;
use std::str::FromStr;

#[test]
fn role_parse_round_trips() {
    for role in [ProjectRole::Read, ProjectRole::Write, ProjectRole::Admin] {
        assert_eq!(ProjectRole::from_str(&role.to_string()).unwrap(), role);
    }
    assert!(ProjectRole::from_str("owner").is_err());
}

#[test]
fn scope_parse_round_trips() {
    for scope in [
        Scope::MetadataRead,
        Scope::ArtifactsRead,
        Scope::ArtifactsWrite,
        Scope::Administration,
        Scope::ProjectsCreate,
        Scope::ProjectsDelete,
    ] {
        assert_eq!(Scope::from_str(&scope.to_string()).unwrap(), scope);
    }
    assert!(Scope::from_str("metadata:write").is_err());

    let mut set = HashSet::new();
    set.insert(Scope::ArtifactsRead);
    set.insert(Scope::ProjectsDelete);
    let stored = ScopeSet(set.clone()).to_storage_string();
    assert_eq!(ScopeSet::from_storage_string(&stored).unwrap().0, set);
}

#[test]
fn visibility_parse_round_trips() {
    assert_eq!(Visibility::from_str("public").unwrap(), Visibility::Public);
    assert_eq!(
        Visibility::from_str("private").unwrap(),
        Visibility::Private
    );
    assert!(Visibility::from_str("other").is_err());
}
