#[path = "../common/mod.rs"] mod common;

use common::{assemble, temp_dir, test_config, user_id};
use filehub_server::model::{AccountRole, Principal, ProjectRole, Visibility};

#[tokio::test]
async fn project_crud_and_visibility_flow() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;
    let owner_p = Principal::User { user_id: alice, account_role: AccountRole::Owner };
    let member_p = Principal::User { user_id: bob, account_role: AccountRole::Member };

    // member 不能创建
    assert!(state.projects.create(&member_p, "denied", Visibility::Private).await.is_err());

    let project = state.projects.create(&owner_p, "demo", Visibility::Private).await.expect("create");
    assert_eq!(project.owner, alice);
    assert_eq!(project.visibility, Visibility::Private);

    // 重复名字冲突
    assert!(state.projects.create(&owner_p, "demo", Visibility::Private).await.is_err());

    // owner 可切 public；read 协作者不可切
    state.permissions.grant_collaborator(&project.project_id, &owner_p, &bob, ProjectRole::Read).await.expect("grant");
    assert!(state.projects.set_visibility(&project.project_id, &member_p, Visibility::Public).await.is_err());
    state.projects.set_visibility(&project.project_id, &owner_p, Visibility::Public).await.expect("set public");

    // list 按可见性过滤：匿名可见 public
    let anon_list = state.projects.list(&Principal::Anonymous).await.expect("anon list");
    assert_eq!(anon_list.len(), 1);

    // admin 协作者不能删除（账号级 projects:delete 仍为 owner）
    state.permissions.update_collaborator(&project.project_id, &owner_p, &bob, ProjectRole::Admin).await.expect("admin");
    assert!(state.projects.delete(&project.project_id, &member_p).await.is_err());
    state.projects.delete(&project.project_id, &owner_p).await.expect("delete");
    assert!(state.projects.list(&Principal::Anonymous).await.expect("anon list after").is_empty());
}
