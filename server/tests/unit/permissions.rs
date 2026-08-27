#[path = "../common/mod.rs"]
mod common;

use common::{assemble, temp_dir, test_config, user_id};
use filehub_server::model::{
    FeatureName, Principal, ProjectRole, ProjectScope, Resource, Scope, ScopeSet, TokenId,
    UserId, Visibility,
};
use filehub_server::permissions::model::PermissionErrorKind;
use std::collections::HashSet;

#[tokio::test]
async fn any_authenticated_user_can_create_projects() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;

    // 所有已登录账号都能 projects:create，不存在账号级 owner/member 区分。
    let alice_principal = Principal::User { user_id: alice };
    let bob_principal = Principal::User { user_id: bob };
    assert!(
        state
            .permissions
            .can_access(
                &alice_principal,
                &Resource::Feature(FeatureName::ProjectsCreate),
                "projects:create"
            )
            .await
            .unwrap()
    );
    assert!(
        state
            .permissions
            .can_access(
                &bob_principal,
                &Resource::Feature(FeatureName::ProjectsCreate),
                "projects:create"
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .permissions
            .can_access(
                &Principal::Anonymous,
                &Resource::Feature(FeatureName::ProjectsCreate),
                "projects:create"
            )
            .await
            .unwrap()
    );

    // token 仍需显式 projects:create scope；无该 scope 的 token 不能创建。
    let mut create_scopes = HashSet::new();
    create_scopes.insert(Scope::ProjectsCreate);
    let create_token = Principal::Token {
        token_id: TokenId(30),
        scopes: ScopeSet(create_scopes),
        user_id: bob,
        project_scope: ProjectScope::All,
    };
    assert!(
        state
            .permissions
            .can_access(
                &create_token,
                &Resource::Feature(FeatureName::ProjectsCreate),
                "projects:create"
            )
            .await
            .unwrap()
    );
    let mut read_scopes = HashSet::new();
    read_scopes.insert(Scope::ArtifactsRead);
    let read_token = Principal::Token {
        token_id: TokenId(31),
        scopes: ScopeSet(read_scopes),
        user_id: bob,
        project_scope: ProjectScope::All,
    };
    assert!(
        !state
            .permissions
            .can_access(
                &read_token,
                &Resource::Feature(FeatureName::ProjectsCreate),
                "projects:create"
            )
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn can_access_feature_and_project_matrix() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;

    let owner_principal = Principal::User { user_id: alice };
    let member_principal = Principal::User { user_id: bob };

    // 项目资源：owner 隐式 admin
    let project = state
        .projects
        .create(&owner_principal, "matrix", Visibility::Private)
        .await
        .expect("create");
    assert!(
        state
            .permissions
            .can_access(
                &owner_principal,
                &Resource::Project(project.project_id),
                "administration"
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .permissions
            .can_access(
                &member_principal,
                &Resource::Project(project.project_id),
                "artifacts:read"
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .permissions
            .can_access(
                &member_principal,
                &Resource::Project(project.project_id),
                "metadata:read"
            )
            .await
            .unwrap()
    );

    // public 匿名只读
    state
        .projects
        .set_visibility(&project.project_id, &owner_principal, Visibility::Public)
        .await
        .expect("public");
    assert!(
        state
            .permissions
            .can_access(
                &Principal::Anonymous,
                &Resource::Project(project.project_id),
                "metadata:read"
            )
            .await
            .unwrap()
    );
    assert!(
        state
            .permissions
            .can_access(
                &Principal::Anonymous,
                &Resource::Project(project.project_id),
                "artifacts:read"
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .permissions
            .can_access(
                &Principal::Anonymous,
                &Resource::Project(project.project_id),
                "artifacts:write"
            )
            .await
            .unwrap()
    );

    // 无授权关系的已登录用户也可读 public（projects.md 契约：User/Token 可见 public + 有权 private）
    assert!(
        state
            .permissions
            .can_access(
                &member_principal,
                &Resource::Project(project.project_id),
                "metadata:read"
            )
            .await
            .unwrap()
    );
    assert!(
        state
            .permissions
            .can_access(
                &member_principal,
                &Resource::Project(project.project_id),
                "artifacts:read"
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .permissions
            .can_access(
                &member_principal,
                &Resource::Project(project.project_id),
                "artifacts:write"
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .permissions
            .can_access(
                &member_principal,
                &Resource::Project(project.project_id),
                "administration"
            )
            .await
            .unwrap()
    );

    // 协作者 write 可发布、不可管理
    state
        .permissions
        .grant_collaborator(
            &project.project_id,
            &owner_principal,
            &bob,
            ProjectRole::Write,
        )
        .await
        .expect("grant");
    assert!(
        state
            .permissions
            .can_access(
                &member_principal,
                &Resource::Project(project.project_id),
                "artifacts:write"
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .permissions
            .can_access(
                &member_principal,
                &Resource::Project(project.project_id),
                "administration"
            )
            .await
            .unwrap()
    );

    // token 二次限制：只有 artifacts:read 无法 write
    let mut set = HashSet::new();
    set.insert(Scope::ArtifactsRead);
    let token_principal = Principal::Token {
        token_id: filehub_server::model::TokenId(9),
        scopes: ScopeSet(set),
        user_id: bob,
        project_scope: ProjectScope::All,
    };
    // public 不绕过 token scope：缺 metadata:read 时 public 的 metadata:read 仍拒绝
    assert!(
        !state
            .permissions
            .can_access(
                &token_principal,
                &Resource::Project(project.project_id),
                "metadata:read"
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .permissions
            .can_access(
                &token_principal,
                &Resource::Project(project.project_id),
                "artifacts:write"
            )
            .await
            .unwrap()
    );

    // 无授权关系的 All 范围 token（带读 scope）可读 public
    let mut read_scopes = HashSet::new();
    read_scopes.insert(Scope::MetadataRead);
    read_scopes.insert(Scope::ArtifactsRead);
    let read_token = Principal::Token {
        token_id: filehub_server::model::TokenId(12),
        scopes: ScopeSet(read_scopes),
        user_id: bob,
        project_scope: ProjectScope::All,
    };
    assert!(
        state
            .permissions
            .can_access(
                &read_token,
                &Resource::Project(project.project_id),
                "metadata:read"
            )
            .await
            .unwrap()
    );
    assert!(
        state
            .permissions
            .can_access(
                &read_token,
                &Resource::Project(project.project_id),
                "artifacts:read"
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .permissions
            .can_access(
                &read_token,
                &Resource::Project(project.project_id),
                "artifacts:write"
            )
            .await
            .unwrap()
    );

    // public 不新增 token 能力：只带 metadata:read 时不能读 artifacts
    let mut metadata_scopes = HashSet::new();
    metadata_scopes.insert(Scope::MetadataRead);
    let metadata_token = Principal::Token {
        token_id: filehub_server::model::TokenId(14),
        scopes: ScopeSet(metadata_scopes),
        user_id: bob,
        project_scope: ProjectScope::All,
    };
    assert!(
        state
            .permissions
            .can_access(
                &metadata_token,
                &Resource::Project(project.project_id),
                "metadata:read"
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .permissions
            .can_access(
                &metadata_token,
                &Resource::Project(project.project_id),
                "artifacts:read"
            )
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn token_project_scope_restricts_access_outside_scope() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;
    let owner_principal = Principal::User { user_id: alice };

    let scoped = state
        .projects
        .create(&owner_principal, "scoped-a", Visibility::Private)
        .await
        .expect("create scoped-a");
    let outside = state
        .projects
        .create(&owner_principal, "scoped-b", Visibility::Private)
        .await
        .expect("create scoped-b");
    state
        .permissions
        .grant_collaborator(
            &scoped.project_id,
            &owner_principal,
            &bob,
            ProjectRole::Write,
        )
        .await
        .expect("grant scoped");
    state
        .permissions
        .grant_collaborator(
            &outside.project_id,
            &owner_principal,
            &bob,
            ProjectRole::Write,
        )
        .await
        .expect("grant outside");
    let pub_inside = state
        .projects
        .create(&owner_principal, "public-inside", Visibility::Private)
        .await
        .expect("create public-inside");
    let pub_outside = state
        .projects
        .create(&owner_principal, "public-outside", Visibility::Private)
        .await
        .expect("create public-outside");
    state
        .projects
        .set_visibility(&pub_inside.project_id, &owner_principal, Visibility::Public)
        .await
        .expect("public pub-inside");
    state
        .projects
        .set_visibility(
            &pub_outside.project_id,
            &owner_principal,
            Visibility::Public,
        )
        .await
        .expect("public pub-outside");

    let mut scopes = HashSet::new();
    scopes.insert(Scope::ArtifactsRead);
    let scoped_token = Principal::Token {
        token_id: filehub_server::model::TokenId(10),
        scopes: ScopeSet(scopes),
        user_id: bob,
        project_scope: ProjectScope::Specified(vec![scoped.project_id]),
    };
    assert!(
        state
            .permissions
            .can_access(
                &scoped_token,
                &Resource::Project(scoped.project_id),
                "artifacts:read"
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .permissions
            .can_access(
                &scoped_token,
                &Resource::Project(outside.project_id),
                "artifacts:read"
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .permissions
            .can_access(
                &scoped_token,
                &Resource::Project(outside.project_id),
                "metadata:read"
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .permissions
            .can_access(
                &scoped_token,
                &Resource::Project(outside.project_id),
                "administration"
            )
            .await
            .unwrap()
    );
    // Specified 范围外 public 不因可见性放宽（025 fail-closed 边界）
    assert!(
        !state
            .permissions
            .can_access(
                &scoped_token,
                &Resource::Project(pub_outside.project_id),
                "metadata:read"
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .permissions
            .can_access(
                &scoped_token,
                &Resource::Project(pub_outside.project_id),
                "artifacts:read"
            )
            .await
            .unwrap()
    );

    // 范围内的 public 项目：Specified token 无授权关系也可读
    let mut inside_scopes = HashSet::new();
    inside_scopes.insert(Scope::MetadataRead);
    let inside_pub_token = Principal::Token {
        token_id: filehub_server::model::TokenId(13),
        scopes: ScopeSet(inside_scopes),
        user_id: bob,
        project_scope: ProjectScope::Specified(vec![pub_inside.project_id]),
    };
    assert!(
        state
            .permissions
            .can_access(
                &inside_pub_token,
                &Resource::Project(pub_inside.project_id),
                "metadata:read"
            )
            .await
            .unwrap()
    );

    // All 范围 token 行为不变：只要用户有项目权限即可访问。
    let mut all_scopes = HashSet::new();
    all_scopes.insert(Scope::MetadataRead);
    let all_token = Principal::Token {
        token_id: filehub_server::model::TokenId(11),
        scopes: ScopeSet(all_scopes),
        user_id: bob,
        project_scope: ProjectScope::All,
    };
    assert!(
        state
            .permissions
            .can_access(
                &all_token,
                &Resource::Project(outside.project_id),
                "metadata:read"
            )
            .await
            .unwrap()
    );
    // 无授权关系的 All 范围 token 也可读 public（pub_outside 无 bob 授权）
    assert!(
        state
            .permissions
            .can_access(
                &all_token,
                &Resource::Project(pub_outside.project_id),
                "metadata:read"
            )
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn collaborator_management_requires_admin() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;
    let owner_p = Principal::User { user_id: alice };
    let member_p = Principal::User { user_id: bob };

    let project = state
        .projects
        .create(&owner_p, "collab", Visibility::Private)
        .await
        .expect("create");
    assert!(
        state
            .permissions
            .list_collaborators(&project.project_id, &member_p)
            .await
            .is_err()
    );
    state
        .permissions
        .grant_collaborator(&project.project_id, &owner_p, &bob, ProjectRole::Read)
        .await
        .expect("grant");
    let list = state
        .permissions
        .list_collaborators(&project.project_id, &owner_p)
        .await
        .expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].user_id, bob);
    // owner 不可被 grant/remove
    assert!(
        state
            .permissions
            .grant_collaborator(&project.project_id, &owner_p, &alice, ProjectRole::Admin)
            .await
            .is_err()
    );
    assert!(
        state
            .permissions
            .remove_collaborator(&project.project_id, &owner_p, &alice)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn grant_collaborator_rejects_nonexistent_user_and_fk() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;
    let owner_p = Principal::User { user_id: alice };

    let project = state
        .projects
        .create(&owner_p, "grant-exists", Visibility::Private)
        .await
        .expect("create");

    // 负数与未创建的正整数 user_id 均不得写入授权（避免未来账号接管历史授权）。
    for missing in [UserId(-1), UserId(9_999_999)] {
        let err = state
            .permissions
            .grant_collaborator(&project.project_id, &owner_p, &missing, ProjectRole::Read)
            .await
            .expect_err("nonexistent user must be rejected");
        assert_eq!(err.kind, PermissionErrorKind::NotFound);
    }

    // 数据库层外键兜底：绕过 checker 直接插入不存在 user_id 必须失败。
    let fk_err = sqlx::query(
        "INSERT INTO project_grants (project_id, user_id, role) VALUES (?, ?, ?)",
    )
    .bind(project.project_id.0)
    .bind(UserId(9_999_999).0)
    .bind("read")
    .execute(&db)
    .await
    .expect_err("foreign key must reject nonexistent user_id");
    assert!(
        fk_err.to_string().to_lowercase().contains("foreign key"),
        "unexpected FK error: {fk_err}"
    );

    // 已存在用户仍可正常授权，不受影响。
    state
        .permissions
        .grant_collaborator(&project.project_id, &owner_p, &bob, ProjectRole::Read)
        .await
        .expect("existing user grant");
}
