#[path = "../common/mod.rs"] mod common;

use common::{assemble, temp_dir, test_config, user_id};
use filehub_server::model::{AccountRole, FeatureName, Principal, ProjectRole, Resource, Scope, ScopeSet, Visibility};
use std::collections::HashSet;

#[tokio::test]
async fn role_initialization_owner_and_member() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;

    assert_eq!(state.permissions_module.role_for_user(alice).await.unwrap(), AccountRole::Owner);
    assert_eq!(state.permissions_module.role_for_user(bob).await.unwrap(), AccountRole::Member);
}

#[tokio::test]
async fn can_access_feature_and_project_matrix() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;

    let owner_principal = Principal::User { user_id: alice, account_role: AccountRole::Owner };
    let member_principal = Principal::User { user_id: bob, account_role: AccountRole::Member };

    assert!(state.permissions.can_access(&owner_principal, &Resource::Feature(FeatureName::ProjectsCreate), "projects:create").await.unwrap());
    assert!(!state.permissions.can_access(&member_principal, &Resource::Feature(FeatureName::ProjectsCreate), "projects:create").await.unwrap());
    assert!(!state.permissions.can_access(&Principal::Anonymous, &Resource::Feature(FeatureName::ProjectsDelete), "projects:delete").await.unwrap());

    // 项目资源：owner 隐式 admin
    let project = state.projects.create(&owner_principal, "matrix", Visibility::Private).await.expect("create");
    assert!(state.permissions.can_access(&owner_principal, &Resource::Project(project.project_id), "administration").await.unwrap());
    assert!(!state.permissions.can_access(&member_principal, &Resource::Project(project.project_id), "artifacts:read").await.unwrap());

    // public 匿名只读
    state.projects.set_visibility(&project.project_id, &owner_principal, Visibility::Public).await.expect("public");
    assert!(state.permissions.can_access(&Principal::Anonymous, &Resource::Project(project.project_id), "metadata:read").await.unwrap());
    assert!(!state.permissions.can_access(&Principal::Anonymous, &Resource::Project(project.project_id), "artifacts:write").await.unwrap());

    // 协作者 write 可发布、不可管理
    state.permissions.grant_collaborator(&project.project_id, &owner_principal, &bob, ProjectRole::Write).await.expect("grant");
    assert!(state.permissions.can_access(&member_principal, &Resource::Project(project.project_id), "artifacts:write").await.unwrap());
    assert!(!state.permissions.can_access(&member_principal, &Resource::Project(project.project_id), "administration").await.unwrap());

    // token 二次限制：只有 artifacts:read 无法 write
    let mut set = HashSet::new();
    set.insert(Scope::ArtifactsRead);
    let token_principal = Principal::Token { token_id: filehub_server::model::TokenId(9), scopes: ScopeSet(set), user_id: bob };
    assert!(!state.permissions.can_access(&token_principal, &Resource::Project(project.project_id), "artifacts:write").await.unwrap());
}

#[tokio::test]
async fn collaborator_management_requires_admin() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;
    let owner_p = Principal::User { user_id: alice, account_role: AccountRole::Owner };
    let member_p = Principal::User { user_id: bob, account_role: AccountRole::Member };

    let project = state.projects.create(&owner_p, "collab", Visibility::Private).await.expect("create");
    assert!(state.permissions.list_collaborators(&project.project_id, &member_p).await.is_err());
    state.permissions.grant_collaborator(&project.project_id, &owner_p, &bob, ProjectRole::Read).await.expect("grant");
    let list = state.permissions.list_collaborators(&project.project_id, &owner_p).await.expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].user_id, bob);
    // owner 不可被 grant/remove
    assert!(state.permissions.grant_collaborator(&project.project_id, &owner_p, &alice, ProjectRole::Admin).await.is_err());
    assert!(state.permissions.remove_collaborator(&project.project_id, &owner_p, &alice).await.is_err());
}
