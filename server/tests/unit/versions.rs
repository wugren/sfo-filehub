#[path = "../common/mod.rs"]
mod common;

use common::{assemble, make_targz, temp_dir, test_config, user_id};
use filehub_server::model::{Principal, ProjectRole, Scope, ScopeSet, Visibility};
use filehub_server::storage::UploadStream;
use filehub_server::versions::VersionErrorKind;
use std::collections::HashSet;

#[tokio::test]
async fn version_lifecycle_create_publish_update_lock_delete() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let owner = Principal::User { user_id: alice };

    let project = state
        .projects
        .create(&owner, "v", Visibility::Public)
        .await
        .expect("create");

    // 显式创建版本；重复创建 409。
    let created = state
        .versions
        .create_version(&project.project_id, "1.0.0", &owner)
        .await
        .expect("create version");
    assert_eq!(created.version, "1.0.0");
    assert!(created.apps.is_empty());
    assert!(created.locked_at.is_none());
    assert!(
        state
            .versions
            .create_version(&project.project_id, "1.0.0", &owner)
            .await
            .is_err()
    );

    // 同一版本发布两个 app；重复发布同 app 为更新（created=false）。
    let file_a = state
        .files
        .ingest(UploadStream::from_bytes(make_targz("a.txt", b"a")), None)
        .await
        .expect("ingest a");
    let first = state
        .versions
        .publish_app(
            &project.project_id,
            "1.0.0",
            "server",
            file_a.clone(),
            &owner,
        )
        .await
        .expect("publish server");
    assert!(first.created);
    assert_eq!(first.record.apps.len(), 1);
    let file_b = state
        .files
        .ingest(UploadStream::from_bytes(make_targz("b.txt", b"b")), None)
        .await
        .expect("ingest b");
    let second = state
        .versions
        .publish_app(&project.project_id, "1.0.0", "cli", file_b.clone(), &owner)
        .await
        .expect("publish cli");
    assert!(second.created);
    assert_eq!(second.record.apps.len(), 2);

    let file_a2 = state
        .files
        .ingest(UploadStream::from_bytes(make_targz("a2.txt", b"a2")), None)
        .await
        .expect("ingest a2");
    let updated = state
        .versions
        .publish_app(
            &project.project_id,
            "1.0.0",
            "server",
            file_a2.clone(),
            &owner,
        )
        .await
        .expect("update server");
    assert!(!updated.created);
    let server_app = updated
        .record
        .apps
        .iter()
        .find(|a| a.app == "server")
        .expect("server app");
    assert_eq!(server_app.file_id, file_a2.file_id);
    assert_eq!(server_app.sha256, file_a2.sha256);

    // 查询单版本返回全部 app 信息。
    let single = state
        .versions
        .get(&project.project_id, Some("1.0.0"), &owner)
        .await
        .expect("get version");
    assert_eq!(single.apps.len(), 2);
    assert!(
        single
            .apps
            .iter()
            .any(|a| a.app == "server" && a.sha256 == file_a2.sha256)
    );
    assert!(
        single
            .apps
            .iter()
            .any(|a| a.app == "cli" && a.sha256 == file_b.sha256)
    );

    // latest 仍是最近创建的版本。
    state
        .versions
        .create_version(&project.project_id, "2.0.0", &owner)
        .await
        .expect("create v2");
    let latest = state
        .versions
        .get(&project.project_id, None, &owner)
        .await
        .expect("latest");
    assert_eq!(latest.version, "2.0.0");

    // 锁定：owner 可锁；重复锁定幂等；锁定后发布/删除被拒。
    let locked = state
        .versions
        .lock(&project.project_id, "1.0.0", &owner)
        .await
        .expect("lock");
    assert!(locked.locked_at.is_some());
    let locked_again = state
        .versions
        .lock(&project.project_id, "1.0.0", &owner)
        .await
        .expect("lock idempotent");
    assert!(locked_again.locked_at.is_some());
    let file_c = state
        .files
        .ingest(UploadStream::from_bytes(make_targz("c.txt", b"c")), None)
        .await
        .expect("ingest c");
    let publish_locked = state
        .versions
        .publish_app(&project.project_id, "1.0.0", "web", file_c.clone(), &owner)
        .await;
    assert!(publish_locked.is_err(), "locked publish rejected");
    assert!(
        state
            .versions
            .delete_app(&project.project_id, "1.0.0", "cli", &owner)
            .await
            .is_err(),
        "locked delete rejected"
    );

    // 锁定后读取与下载引用仍然可用。
    let refs = state.versions.referenced_file_ids().await.expect("refs");
    assert!(refs.contains(&file_a2.file_id) && refs.contains(&file_b.file_id));

    // 匿名可读 public 项目。
    assert!(
        state
            .versions
            .get(&project.project_id, Some("1.0.0"), &Principal::Anonymous)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn publish_and_create_require_artifacts_write() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;
    let owner = Principal::User { user_id: alice };
    let member = Principal::User { user_id: bob };

    let project = state
        .projects
        .create(&owner, "deny", Visibility::Private)
        .await
        .expect("create");
    state
        .permissions
        .grant_collaborator(&project.project_id, &owner, &bob, ProjectRole::Read)
        .await
        .expect("grant");

    // 只读成员不能创建版本、不能发布 app、不能锁定。
    assert!(
        state
            .versions
            .create_version(&project.project_id, "1.0.0", &member)
            .await
            .is_err()
    );
    let file = state
        .files
        .ingest(UploadStream::from_bytes(make_targz("f", b"x")), None)
        .await
        .expect("ingest");
    assert!(
        state
            .versions
            .publish_app(
                &project.project_id,
                "1.0.0",
                "default",
                file.clone(),
                &member
            )
            .await
            .is_err()
    );
    assert!(
        state
            .versions
            .lock(&project.project_id, "1.0.0", &member)
            .await
            .is_err()
    );

    // 只读 token 同样被拒。
    let mut scopes = HashSet::new();
    scopes.insert(Scope::ArtifactsRead);
    let token = Principal::Token {
        token_id: filehub_server::model::TokenId(1),
        scopes: ScopeSet(scopes),
        user_id: bob,
        project_scope: filehub_server::model::ProjectScope::All,
    };
    assert!(
        state
            .versions
            .create_version(&project.project_id, "1.0.0", &token)
            .await
            .is_err()
    );
    assert!(
        state
            .versions
            .publish_app(&project.project_id, "1.0.0", "default", file, &token)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn delete_app_and_missing_targets() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let owner = Principal::User { user_id: alice };

    let project = state
        .projects
        .create(&owner, "del", Visibility::Private)
        .await
        .expect("create");
    // 发布到不存在版本 -> NotFound。
    let file = state
        .files
        .ingest(UploadStream::from_bytes(make_targz("f", b"x")), None)
        .await
        .expect("ingest");
    assert!(
        state
            .versions
            .publish_app(
                &project.project_id,
                "9.9.9",
                "default",
                file.clone(),
                &owner
            )
            .await
            .is_err()
    );
    // 删除不存在版本/app -> NotFound。
    assert!(
        state
            .versions
            .delete_app(&project.project_id, "9.9.9", "default", &owner)
            .await
            .is_err()
    );

    state
        .versions
        .create_version(&project.project_id, "1.0.0", &owner)
        .await
        .expect("create");
    let out = state
        .versions
        .publish_app(&project.project_id, "1.0.0", "server", file.clone(), &owner)
        .await
        .expect("publish");
    let file_id = out.record.apps[0].file_id.clone();

    // 删除后引用集不再包含该文件；再删同 app -> NotFound。
    state
        .versions
        .delete_app(&project.project_id, "1.0.0", "server", &owner)
        .await
        .expect("delete app");
    let refs = state.versions.referenced_file_ids().await.expect("refs");
    assert!(!refs.contains(&file_id));
    assert!(
        state
            .versions
            .delete_app(&project.project_id, "1.0.0", "server", &owner)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn version_and_app_input_validation_branches() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let owner = Principal::User { user_id: alice };
    let project = state
        .projects
        .create(&owner, "valid", Visibility::Private)
        .await
        .expect("create");

    // 空版本创建/发布被拒。
    assert!(
        state
            .versions
            .create_version(&project.project_id, "  ", &owner)
            .await
            .is_err()
    );
    let f0 = state
        .files
        .ingest(UploadStream::from_bytes(make_targz("f0", b"0")), None)
        .await
        .expect("ingest f0");
    assert!(
        state
            .versions
            .publish_app(&project.project_id, "  ", "default", f0.clone(), &owner)
            .await
            .is_err()
    );

    state
        .versions
        .create_version(&project.project_id, "1.0.0", &owner)
        .await
        .expect("create");
    // 空 app 名与非法字符（空白、感叹号、斜杠）被拒。
    let f1 = state
        .files
        .ingest(UploadStream::from_bytes(make_targz("f1", b"1")), None)
        .await
        .expect("ingest f1");
    assert!(
        state
            .versions
            .publish_app(&project.project_id, "1.0.0", "", f1.clone(), &owner)
            .await
            .is_err()
    );
    let f2 = state
        .files
        .ingest(UploadStream::from_bytes(make_targz("f2", b"2")), None)
        .await
        .expect("ingest f2");
    assert!(
        state
            .versions
            .publish_app(&project.project_id, "1.0.0", "bad app", f2.clone(), &owner)
            .await
            .is_err()
    );
    let f3 = state
        .files
        .ingest(UploadStream::from_bytes(make_targz("f3", b"3")), None)
        .await
        .expect("ingest f3");
    assert!(
        state
            .versions
            .publish_app(&project.project_id, "1.0.0", "web/evil", f3.clone(), &owner)
            .await
            .is_err()
    );
    let f4 = state
        .files
        .ingest(UploadStream::from_bytes(make_targz("f4", b"4")), None)
        .await
        .expect("ingest f4");
    assert!(
        state
            .versions
            .publish_app(
                &project.project_id,
                "1.0.0",
                "ok-app.v1",
                f4.clone(),
                &owner
            )
            .await
            .is_ok()
    );

    // 锁定不存在的版本 -> NotFound。
    assert!(
        state
            .versions
            .lock(&project.project_id, "9.9.9", &owner)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn update_and_delete_app_reclaim_old_files_immediately() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let owner = Principal::User { user_id: alice };
    let project = state
        .projects
        .create(&owner, "reclaim", Visibility::Private)
        .await
        .expect("create");
    state
        .versions
        .create_version(&project.project_id, "1.0.0", &owner)
        .await
        .expect("create version");

    // 发布旧文件后更新 app：旧文件应立即被回收，新文件保留。
    let old_file = state
        .files
        .ingest(
            UploadStream::from_bytes(make_targz("old.txt", b"old")),
            None,
        )
        .await
        .expect("ingest old");
    state
        .versions
        .publish_app(
            &project.project_id,
            "1.0.0",
            "server",
            old_file.clone(),
            &owner,
        )
        .await
        .expect("publish");
    let new_file = state
        .files
        .ingest(
            UploadStream::from_bytes(make_targz("new.txt", b"new")),
            None,
        )
        .await
        .expect("ingest new");
    let updated = state
        .versions
        .publish_app(
            &project.project_id,
            "1.0.0",
            "server",
            new_file.clone(),
            &owner,
        )
        .await
        .expect("update");
    assert!(!updated.created);

    let old_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE id = ?")
        .bind(&old_file.file_id.0)
        .fetch_one(&db)
        .await
        .expect("count old file rows");
    assert_eq!(old_rows, 0, "updated-away file row is gone immediately");
    assert!(
        !config
            .files
            .data_dir
            .join(format!("{}.tar.gz", old_file.file_id.0))
            .exists(),
        "updated-away physical file is gone immediately"
    );
    let new_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE id = ?")
        .bind(&new_file.file_id.0)
        .fetch_one(&db)
        .await
        .expect("count new file rows");
    assert_eq!(new_rows, 1, "new file row remains referenced");

    // 删除 app：被删引用的文件应立即被回收。
    state
        .versions
        .delete_app(&project.project_id, "1.0.0", "server", &owner)
        .await
        .expect("delete app");
    let deleted_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE id = ?")
        .bind(&new_file.file_id.0)
        .fetch_one(&db)
        .await
        .expect("count deleted file rows");
    assert_eq!(deleted_rows, 0, "deleted file row is gone immediately");
    assert!(
        !config
            .files
            .data_dir
            .join(format!("{}.tar.gz", new_file.file_id.0))
            .exists(),
        "deleted physical file is gone immediately"
    );

    // startup GC 不再有这两条路径的残留可回收。
    let removed = state.startup_gc().await.expect("startup gc");
    assert!(
        removed.is_empty(),
        "no orphan remains after immediate cleanup, removed={:?}",
        removed
    );
}

#[tokio::test]
async fn republish_same_file_keeps_reference() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let owner = Principal::User { user_id: alice };
    let project = state
        .projects
        .create(&owner, "self-ref", Visibility::Private)
        .await
        .expect("create");
    state
        .versions
        .create_version(&project.project_id, "1.0.0", &owner)
        .await
        .expect("create version");

    let file = state
        .files
        .ingest(
            UploadStream::from_bytes(make_targz("same.txt", b"same")),
            None,
        )
        .await
        .expect("ingest");
    let first = state
        .versions
        .publish_app(&project.project_id, "1.0.0", "server", file.clone(), &owner)
        .await
        .expect("publish");
    assert!(first.created);
    // 自引用更新：同一 file_id 重复发布不应触发回收/误删。
    let again = state
        .versions
        .publish_app(&project.project_id, "1.0.0", "server", file.clone(), &owner)
        .await
        .expect("republish");
    assert!(!again.created);

    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE id = ?")
        .bind(&file.file_id.0)
        .fetch_one(&db)
        .await
        .expect("count rows");
    assert_eq!(rows, 1, "self-referenced file row is kept");
    assert!(
        config
            .files
            .data_dir
            .join(format!("{}.tar.gz", file.file_id.0))
            .exists(),
        "self-referenced physical file is kept"
    );
    let refs = state.versions.referenced_file_ids().await.expect("refs");
    assert!(
        refs.contains(&file.file_id),
        "self-referenced file remains referenced"
    );
}

#[tokio::test]
async fn create_version_rejects_reserved_and_route_unsafe_names() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let owner = Principal::User { user_id: alice };
    let project = state
        .projects
        .create(&owner, "vnames", Visibility::Private)
        .await
        .expect("create");

    // 最小拒绝集：保留字 latest（含 trim 后命中）、URL 归一化后不可寻址的整段
    // '.'/'..'、路径/查询/片段分隔符、Content-Disposition quoted-string 元字符
    // 与控制字符（trim 后判定 latest/. /..；字符类检查原始输入）。
    for bad in [
        "latest",
        " latest ",
        "   ",
        ".",
        "..",
        " . ",
        " .. ",
        "1.0.0/rc1",
        "1.0.0?beta",
        "1.0.0#meta",
        "1.0\"rc1",
        "1.0\\rc1",
        "1.0.0\nrc1",
        "1.0.0\tbeta",
        "1.0.0\x7f",
        "1.0.0\u{85}",
    ] {
        let err = state
            .versions
            .create_version(&project.project_id, bad, &owner)
            .await
            .expect_err("unsafe version name must be rejected");
        assert_eq!(
            err.kind,
            VersionErrorKind::InvalidInput,
            "unexpected kind for {bad:?}: {:?}",
            err.kind
        );
    }
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM versions")
        .fetch_one(&db)
        .await
        .expect("count versions");
    assert_eq!(rows, 0, "rejected names must not be inserted");

    // 不限制格式：非字面保留字、空格、非三字段、含点段/前导点的非整段名称均放行。
    for good in [
        "1.0.0",
        "1.0",
        "Latest",
        "1.0.0 beta",
        "1.0.0-rc1",
        "1.0.0.1",
        "..rel",
    ] {
        let created = state
            .versions
            .create_version(&project.project_id, good, &owner)
            .await
            .expect("allowed version name must be created");
        assert_eq!(created.version, good);
    }
}
