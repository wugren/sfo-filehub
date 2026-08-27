#[path = "../common/mod.rs"]
mod common;

use common::{assemble, make_targz, sha256_hex, temp_dir, test_config, user_id};
use filehub_server::model::{
    Principal, ProjectId, ProjectRole, ProjectScope, Scope, ScopeSet, TokenId, Visibility,
};
use filehub_server::projects::model::ProjectErrorKind;
use filehub_server::storage::UploadStream;

#[tokio::test]
async fn project_crud_and_visibility_flow() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;
    let owner_p = Principal::User { user_id: alice };
    let member_p = Principal::User { user_id: bob };

    // 每个账号都能创建自己的项目并成为 owner，不存在账号级 owner/member 区分。
    let bob_project = state
        .projects
        .create(&member_p, "bob-own", Visibility::Private)
        .await
        .expect("bob can create");
    assert_eq!(bob_project.owner, bob);
    let project = state
        .projects
        .create(&owner_p, "demo", Visibility::Private)
        .await
        .expect("create");
    assert_eq!(project.owner, alice);
    assert_eq!(project.visibility, Visibility::Private);

    // 项目名全局唯一：其他账号也不能重复创建同名项目。
    assert!(
        state
            .projects
            .create(&owner_p, "demo", Visibility::Private)
            .await
            .is_err()
    );
    assert!(
        state
            .projects
            .create(&member_p, "demo", Visibility::Private)
            .await
            .is_err()
    );

    // owner 可切 public
    state
        .projects
        .set_visibility(&project.project_id, &owner_p, Visibility::Public)
        .await
        .expect("set public");

    // list 按可见性过滤：无授权关系时 Anonymous 与已登录 member 都可见 public（本提案修复点）
    let anon_list = state
        .projects
        .list(&Principal::Anonymous, 100, 0)
        .await
        .expect("anon list");
    assert_eq!(anon_list.items.len(), 1);
    let member_list = state
        .projects
        .list(&member_p, 100, 0)
        .await
        .expect("member list");
    assert_eq!(member_list.items.len(), 2);

    // read 协作者不能切换可见性
    state
        .permissions
        .grant_collaborator(&project.project_id, &owner_p, &bob, ProjectRole::Read)
        .await
        .expect("grant");
    assert!(
        state
            .projects
            .set_visibility(&project.project_id, &member_p, Visibility::Public)
            .await
            .is_err()
    );

    // admin 协作者不能删除（项目级 projects:delete 仅 owner）
    state
        .permissions
        .update_collaborator(&project.project_id, &owner_p, &bob, ProjectRole::Admin)
        .await
        .expect("admin");
    assert!(
        state
            .projects
            .delete(&project.project_id, &member_p)
            .await
            .is_err()
    );
    state
        .projects
        .delete(&project.project_id, &owner_p)
        .await
        .expect("delete");
    assert!(
        state
            .projects
            .list(&Principal::Anonymous, 100, 0)
            .await
            .expect("anon list after")
            .items
            .is_empty()
    );
}

#[tokio::test]
async fn create_rejects_cli_unaddressable_names() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let owner = Principal::User { user_id: alice };

    // 最小拒绝集：CLI `<server>/<project>` 按 `/` 分段并对字段先 trim，
    // 含 `/` 或带首尾空白的项目名无法被项目型 CLI 命令精确寻址。
    for bad in ["a/b", "/x", "x/", "  ", " demo", "demo ", " demo "] {
        let err = state
            .projects
            .create(&owner, bad, Visibility::Private)
            .await
            .expect_err("cli-unaddressable name must be rejected");
        assert_eq!(
            err.kind,
            ProjectErrorKind::InvalidInput,
            "unexpected kind for {bad:?}: {:?}",
            err.kind
        );
    }
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(&db)
        .await
        .expect("count projects");
    assert_eq!(rows, 0, "rejected names must not be inserted");

    // 放行集：大小写、Unicode、内部空格等名称经 CLI 分段/trim 后仍可精确寻址。
    for good in ["demo", "demo-1", "demo_1", "demo 1", "Demo", "项目"] {
        let created = state
            .projects
            .create(&owner, good, Visibility::Private)
            .await
            .expect("addressable name must be created");
        assert_eq!(created.name, good);
    }
}

#[tokio::test]
async fn token_delete_requires_project_scope_and_project_admin() {
    fn scoped_token(
        token_id: i64,
        user: filehub_server::model::UserId,
        scopes: &[Scope],
        project_scope: ProjectScope,
    ) -> Principal {
        Principal::Token {
            token_id: TokenId(token_id),
            scopes: ScopeSet(scopes.iter().copied().collect()),
            user_id: user,
            project_scope,
        }
    }

    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;
    let owner_p = Principal::User { user_id: alice };
    let bob_p = Principal::User { user_id: bob };

    let inside = state
        .projects
        .create(&owner_p, "delete-inside", Visibility::Private)
        .await
        .expect("create inside");
    let outside = state
        .projects
        .create(&owner_p, "delete-outside", Visibility::Private)
        .await
        .expect("create outside");

    // 非 owner 用户不能删除；admin 协作者同样不可删除（项目级 owner-only）。
    assert!(
        state
            .projects
            .delete(&inside.project_id, &bob_p)
            .await
            .is_err()
    );
    state
        .permissions
        .grant_collaborator(&inside.project_id, &owner_p, &bob, ProjectRole::Admin)
        .await
        .expect("grant admin");
    assert!(
        state
            .projects
            .delete(&inside.project_id, &bob_p)
            .await
            .is_err()
    );
    // 项目 owner 可删除自己项目。
    state
        .projects
        .delete(&inside.project_id, &owner_p)
        .await
        .expect("owner delete");

    // 指定项目 + administration + projects:delete：范围内可删、范围外拒绝。
    let scoped = state
        .projects
        .create(&owner_p, "delete-scoped", Visibility::Private)
        .await
        .expect("create scoped");
    let scoped_tok = scoped_token(
        20,
        alice,
        &[Scope::Administration, Scope::ProjectsDelete],
        ProjectScope::Specified(vec![outside.project_id]),
    );
    assert!(
        state
            .projects
            .delete(&outside.project_id, &scoped_tok)
            .await
            .is_ok()
    );
    assert!(
        state
            .projects
            .delete(&scoped.project_id, &scoped_tok)
            .await
            .is_err()
    );

    // 缺少 administration scope 或 projects:delete scope 的 token 均不能删除。
    let keep = state
        .projects
        .create(&owner_p, "delete-no-admin", Visibility::Private)
        .await
        .expect("create no-admin");
    let no_admin = scoped_token(21, alice, &[Scope::ProjectsDelete], ProjectScope::All);
    assert!(
        state
            .projects
            .delete(&keep.project_id, &no_admin)
            .await
            .is_err()
    );
    let admin_only = scoped_token(26, alice, &[Scope::Administration], ProjectScope::All);
    assert!(
        state
            .projects
            .delete(&keep.project_id, &admin_only)
            .await
            .is_err()
    );

    // All 范围 + administration + projects:delete 且所属用户为项目 owner：可删除。
    let all = scoped_token(
        22,
        alice,
        &[Scope::Administration, Scope::ProjectsDelete],
        ProjectScope::All,
    );
    assert!(state.projects.delete(&keep.project_id, &all).await.is_ok());

    // token 所属用户必须是目标项目 owner：非 owner 用户的 token 即使全 scope 也不能删除。
    let guard = state
        .projects
        .create(&owner_p, "delete-guard", Visibility::Private)
        .await
        .expect("create guard");
    let bob_full = scoped_token(
        25,
        bob,
        &[Scope::Administration, Scope::ProjectsDelete],
        ProjectScope::All,
    );
    assert!(
        state
            .projects
            .delete(&guard.project_id, &bob_full)
            .await
            .is_err()
    );

    // 项目不存在（token 路径）返回错误而非放行。
    let missing = scoped_token(
        23,
        alice,
        &[Scope::Administration, Scope::ProjectsDelete],
        ProjectScope::All,
    );
    assert!(
        state
            .projects
            .delete(&filehub_server::model::ProjectId(9999), &missing)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn delete_project_removes_versions_apps_and_grants() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;
    let owner_p = Principal::User { user_id: alice };

    let project = state
        .projects
        .create(&owner_p, "cascade", Visibility::Private)
        .await
        .expect("create");

    // 创建并发布版本/app，再授予协作者，制造三类关联数据。
    state
        .versions
        .create_version(&project.project_id, "1.0.0", &owner_p)
        .await
        .expect("create version");
    let file = state
        .files
        .ingest(
            filehub_server::storage::UploadStream::from_bytes(common::make_targz("a.txt", b"a")),
            None,
        )
        .await
        .expect("ingest");
    state
        .versions
        .publish_app(&project.project_id, "1.0.0", "server", file, &owner_p)
        .await
        .expect("publish app");
    state
        .permissions
        .grant_collaborator(&project.project_id, &owner_p, &bob, ProjectRole::Read)
        .await
        .expect("grant");

    state
        .projects
        .delete(&project.project_id, &owner_p)
        .await
        .expect("delete");

    // 删除后 project/versions/version_apps/project_grants 均不得残留该项目相关行。
    let project_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id = ?")
        .bind(project.project_id.0)
        .fetch_one(&db)
        .await
        .expect("count projects");
    let version_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM versions WHERE project_id = ?")
            .bind(project.project_id.0)
            .fetch_one(&db)
            .await
            .expect("count versions");
    let app_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM version_apps WHERE version_id IN (SELECT id FROM versions WHERE project_id = ?)",
    )
    .bind(project.project_id.0)
    .fetch_one(&db)
    .await
    .expect("count apps");
    let grant_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_grants WHERE project_id = ?")
            .bind(project.project_id.0)
            .fetch_one(&db)
            .await
            .expect("count grants");

    assert_eq!(project_rows, 0);
    assert_eq!(version_rows, 0);
    assert_eq!(app_rows, 0);
    assert_eq!(grant_rows, 0);
}

#[tokio::test]
async fn project_fk_cascade_prevents_orphan_versions_and_grants() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;
    let owner_p = Principal::User { user_id: alice };

    let project = state
        .projects
        .create(&owner_p, "fk-cascade", Visibility::Private)
        .await
        .expect("create");
    state
        .versions
        .create_version(&project.project_id, "1.0.0", &owner_p)
        .await
        .expect("create version");
    state
        .permissions
        .grant_collaborator(&project.project_id, &owner_p, &bob, ProjectRole::Read)
        .await
        .expect("grant");

    // 直删项目主行：新库外键 ON DELETE CASCADE 应同时清除 versions/project_grants。
    let removed = sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(project.project_id.0)
        .execute(&db)
        .await
        .expect("delete project row");
    assert_eq!(removed.rows_affected(), 1);
    let version_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM versions WHERE project_id = ?")
            .bind(project.project_id.0)
            .fetch_one(&db)
            .await
            .expect("count versions");
    let grant_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_grants WHERE project_id = ?")
            .bind(project.project_id.0)
            .fetch_one(&db)
            .await
            .expect("count grants");
    assert_eq!(version_rows, 0);
    assert_eq!(grant_rows, 0);

    // 指向不存在项目的直写必须被外键拒绝（数据库级兜底）。
    let orphan_version =
        sqlx::query("INSERT INTO versions (project_id, version, published_at) VALUES (?, ?, ?)")
            .bind(999_999_i64)
            .bind("orphan")
            .bind("2026-01-01T00:00:00Z")
            .execute(&db)
            .await;
    assert!(
        orphan_version.is_err(),
        "inserting a version for a deleted project must be rejected"
    );
    let orphan_grant =
        sqlx::query("INSERT INTO project_grants (project_id, user_id, role) VALUES (?, ?, ?)")
            .bind(999_999_i64)
            .bind(bob.0)
            .bind("read")
            .execute(&db)
            .await;
    assert!(
        orphan_grant.is_err(),
        "inserting a grant for a deleted project must be rejected"
    );
}

#[tokio::test]
async fn concurrent_project_delete_and_child_creates_leave_no_orphans() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;
    let owner_p = Principal::User { user_id: alice };
    let project = state
        .projects
        .create(&owner_p, "race", Visibility::Private)
        .await
        .expect("create");
    let project_id = project.project_id;
    let state = std::sync::Arc::new(state);

    // 创建版本、授予协作者与删除项目并发执行；无论交错顺序如何，
    // 终态都不能残留指向已删项目的孤儿行。
    let deleter = {
        let state = state.clone();
        let owner_p = owner_p.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            for _ in 0..6 {
                let _ = state.projects.delete(&project_id, &owner_p).await;
            }
        })
    };
    let creator = {
        let state = state.clone();
        let owner_p = owner_p.clone();
        tokio::spawn(async move {
            for i in 0..240 {
                let _ = state
                    .versions
                    .create_version(&project_id, &format!("v{i}"), &owner_p)
                    .await;
            }
        })
    };
    let granter = {
        let state = state.clone();
        let owner_p = owner_p.clone();
        tokio::spawn(async move {
            for _ in 0..120 {
                let _ = state
                    .permissions
                    .grant_collaborator(&project_id, &owner_p, &bob, ProjectRole::Read)
                    .await;
            }
        })
    };
    let _ = tokio::join!(deleter, creator, granter);

    let orphan_versions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM versions v LEFT JOIN projects p ON p.id = v.project_id WHERE p.id IS NULL",
    )
    .fetch_one(&db)
    .await
    .expect("count orphan versions");
    let orphan_grants: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM project_grants g LEFT JOIN projects p ON p.id = g.project_id WHERE p.id IS NULL",
    )
    .fetch_one(&db)
    .await
    .expect("count orphan grants");
    assert_eq!(orphan_versions, 0);
    assert_eq!(orphan_grants, 0);
}

#[tokio::test]
async fn project_list_pagination_and_get() {
    fn scoped_token(
        token_id: i64,
        user: filehub_server::model::UserId,
        scopes: &[Scope],
        project_scope: ProjectScope,
    ) -> Principal {
        Principal::Token {
            token_id: TokenId(token_id),
            scopes: ScopeSet(scopes.iter().copied().collect()),
            user_id: user,
            project_scope,
        }
    }

    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;
    let alice_p = Principal::User { user_id: alice };
    let bob_p = Principal::User { user_id: bob };

    let a_private = state
        .projects
        .create(&alice_p, "a-private", Visibility::Private)
        .await
        .expect("create a-private");
    let b_private = state
        .projects
        .create(&bob_p, "b-private", Visibility::Private)
        .await
        .expect("create b-private");
    let a_public = state
        .projects
        .create(&alice_p, "a-public", Visibility::Public)
        .await
        .expect("create a-public");

    // 分页：alice 可见 a-private(id 最小) 与 a-public(id 最大)。
    let page1 = state
        .projects
        .list(&alice_p, 1, 0)
        .await
        .expect("page1");
    assert_eq!(page1.total, 2);
    assert_eq!(page1.items.len(), 1);
    assert_eq!(page1.items[0].project_id, a_private.project_id);
    let page2 = state
        .projects
        .list(&alice_p, 1, 1)
        .await
        .expect("page2");
    assert_eq!(page2.items[0].project_id, a_public.project_id);
    let overflow = state
        .projects
        .list(&alice_p, 1, 2)
        .await
        .expect("overflow");
    assert!(overflow.items.is_empty());
    assert_eq!(overflow.total, 2);

    // 匿名只可见 public，且 total 正确。
    let anon_page = state
        .projects
        .list(&Principal::Anonymous, 100, 0)
        .await
        .expect("anon page");
    assert_eq!(anon_page.total, 1);
    assert_eq!(anon_page.items[0].project_id, a_public.project_id);

    // 单项目直查：owner 命中；非 owner/匿名对 private 不可见；public 匿名可见。
    let owner_get = state
        .projects
        .get(&a_private.project_id, &alice_p)
        .await
        .expect("owner get");
    assert_eq!(owner_get.map(|p| p.project_id), Some(a_private.project_id));
    assert!(
        state
            .projects
            .get(&a_private.project_id, &bob_p)
            .await
            .expect("bob get private")
            .is_none()
    );
    assert!(
        state
            .projects
            .get(&a_private.project_id, &Principal::Anonymous)
            .await
            .expect("anon get private")
            .is_none()
    );
    assert!(
        state
            .projects
            .get(&a_public.project_id, &Principal::Anonymous)
            .await
            .expect("anon get public")
            .is_some()
    );
    assert!(
        state
            .projects
            .get(&ProjectId(999_999), &alice_p)
            .await
            .expect("missing get")
            .is_none()
    );

    // token：metadata:read + 指定项目范围只返回范围内项目；缺 scope 全空。
    let scoped = scoped_token(
        10,
        alice,
        &[Scope::MetadataRead],
        ProjectScope::Specified(vec![a_private.project_id]),
    );
    let token_page = state
        .projects
        .list(&scoped, 100, 0)
        .await
        .expect("scoped token page");
    assert_eq!(token_page.total, 1);
    assert_eq!(token_page.items[0].project_id, a_private.project_id);
    let no_scope = scoped_token(11, alice, &[Scope::Administration], ProjectScope::All);
    assert!(
        state
            .projects
            .list(&no_scope, 100, 0)
            .await
            .expect("no-scope page")
            .items
            .is_empty()
    );
    assert!(
        state
            .projects
            .get(&a_private.project_id, &no_scope)
            .await
            .expect("no-scope get")
            .is_none()
    );
    let empty_scope = scoped_token(12, alice, &[Scope::MetadataRead], ProjectScope::Specified(vec![]));
    assert!(
        state
            .projects
            .list(&empty_scope, 100, 0)
            .await
            .expect("empty scope page")
            .items
            .is_empty()
    );
    assert!(
        state
            .projects
            .get(&a_private.project_id, &empty_scope)
            .await
            .expect("empty scope get")
            .is_none()
    );

    // grant 协作者后，bob 的可见集合扩大，直查命中被授权项目。
    let before_grant = state
        .projects
        .list(&bob_p, 100, 0)
        .await
        .expect("bob before grant");
    assert_eq!(before_grant.total, 2);
    state
        .permissions
        .grant_collaborator(&a_private.project_id, &alice_p, &bob, ProjectRole::Read)
        .await
        .expect("grant");
    let after_grant = state
        .projects
        .list(&bob_p, 100, 0)
        .await
        .expect("bob after grant");
    assert_eq!(after_grant.total, 3);
    assert!(
        state
            .projects
            .get(&a_private.project_id, &bob_p)
            .await
            .expect("bob get granted")
            .is_some()
    );
}

#[tokio::test]
async fn delete_project_reclaims_files_immediately() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("files.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let owner = Principal::User { user_id: alice };
    let project = state
        .projects
        .create(&owner, "files-cleanup", Visibility::Private)
        .await
        .expect("create");
    state
        .versions
        .create_version(&project.project_id, "1.0.0", &owner)
        .await
        .expect("create version");
    let archive = make_targz("ui.txt", b"ui");
    let file = state
        .files
        .ingest(
            UploadStream::from_bytes(archive.clone()),
            Some(&sha256_hex(&archive)),
        )
        .await
        .expect("ingest");
    state
        .versions
        .publish_app(&project.project_id, "1.0.0", "ui", file.clone(), &owner)
        .await
        .expect("publish");

    state
        .projects
        .delete(&project.project_id, &owner)
        .await
        .expect("delete project");

    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE id = ?")
        .bind(&file.file_id.0)
        .fetch_one(&db)
        .await
        .expect("count rows");
    assert_eq!(rows, 0, "project delete removes file rows immediately");
    assert!(
        !config
            .files
            .data_dir
            .join(format!("{}.tar.gz", file.file_id.0))
            .exists(),
        "project delete removes physical archives immediately"
    );
}
