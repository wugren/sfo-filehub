//! DV：filehub-server 单模块可运行验证（生命周期/主流程/失败/配置/持久化）。

#[path = "common/mod.rs"] mod common;

use common::{assemble, login_session, make_targz, sha256_hex, temp_dir, test_config, user_id};
use filehub_server::model::{AccountRole, Principal, Scope, Visibility};
use filehub_server::tokens::model::TokenCreateRequest;
use std::collections::HashSet;

async fn set_up() -> (filehub_server::http::AppState, std::path::PathBuf, sqlx::sqlite::SqlitePool) {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("dv.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    assert_eq!(state.startup_gc().await.expect("gc").len(), 0);
    (state, dir, db)
}

#[tokio::test]
async fn dv_full_workflow_with_tokens_and_gc() {
    let (state, dir, _db) = set_up().await;
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;

    let alice_session = login_session(&state, "alice", "alice-pass").await;
    let owner_principal = to_principal(&state, &alice_session)
        .await
        .expect("session principal");

    // 主流程：创建 private 项目（owner）
    let project = state
        .projects
        .create(&owner_principal, "dv-project", Visibility::Private)
        .await
        .expect("create project");

    // failure：bob 未授权读 private
    let member_principal = Principal::User { user_id: bob, account_role: AccountRole::Member };
    assert!(state.versions.list(&project.project_id, &member_principal).await.is_err());

    // 主流程：显式创建两个版本，再向版本发布 app
    let v1 = make_targz("v1.txt", b"one");
    let f1 = state.files.ingest(v1.clone(), Some(&sha256_hex(&v1))).await.expect("ingest v1");
    state.versions.create_version(&project.project_id, "1.0.0", &owner_principal).await.expect("create v1");
    let out1 = state.versions.publish_app(&project.project_id, "1.0.0", "default", f1.clone(), &owner_principal).await.expect("app v1");
    assert!(out1.created);
    let v2 = make_targz("v2.txt", b"two");
    let f2 = state.files.ingest(v2.clone(), None).await.expect("ingest v2");
    state.versions.create_version(&project.project_id, "2.0.0", &owner_principal).await.expect("create v2");
    state.versions.publish_app(&project.project_id, "2.0.0", "default", f2.clone(), &owner_principal).await.expect("app v2");

    // failure：重复创建版本 409
    assert!(state.versions.create_version(&project.project_id, "2.0.0", &owner_principal).await.is_err());

    // 主流程：切 public，匿名可见并下载
    state.projects.set_visibility(&project.project_id, &owner_principal, Visibility::Public).await.expect("public");
    let anon_latest = state.versions.get(&project.project_id, None, &Principal::Anonymous).await.expect("anon latest");
    assert_eq!(anon_latest.version, "2.0.0");
    assert!(state.versions.get(&project.project_id, Some("1.0.0"), &Principal::Anonymous).await.is_ok());

    // 主流程：token 生命周期 + 二次限制
    let token = state
        .tokens
        .create(TokenCreateRequest {
            owner: alice,
            name: "reader".to_string(),
            project_scope: None,
            scopes: vec![Scope::MetadataRead, Scope::ArtifactsRead],
            expires_at: None,
        })
        .await
        .expect("create token");
    let token_principal = to_principal(&state, &format!("Bearer {}", token.jwt))
        .await
        .expect("token principal");
    assert!(state.versions.get(&project.project_id, None, &token_principal).await.is_ok());
    assert!(state.versions.create_version(&project.project_id, "9.0.0", &token_principal).await.is_err());

    // rotate：旧 JWT 失效
    let rotated = state.tokens.rotate(&token.token_id, &alice).await.expect("rotate");
    assert!(to_principal(&state, &format!("Bearer {}", token.jwt)).await.is_err());
    let rotated_principal = to_principal(&state, &format!("Bearer {}", rotated.jwt)).await.expect("rotated ok");
    assert!(state.versions.list(&project.project_id, &rotated_principal).await.is_ok());

    // config 变体：发行上限（另一实例）
    let small_config = {
        let mut c = test_config(&dir.join("small"), &dir.join("small.db").to_string_lossy());
        c.files.max_archive_bytes = 32;
        c
    };
    std::fs::create_dir_all(small_config.files.data_dir.clone()).unwrap();
    let (small_state, _) = assemble(&small_config).await.expect("small assemble");
    assert!(small_state.files.ingest(make_targz("big", &vec![0u8; 64]), None).await.is_err());

    // 删除项目后 gc 清理文件
    state.projects.delete(&project.project_id, &owner_principal).await.expect("delete");
    let removed = state.startup_gc().await.expect("gc after delete");
    let file_ids: HashSet<String> = removed.iter().map(|f| f.0.clone()).collect();
    assert!(file_ids.contains(&f1.file_id.0) || file_ids.contains(&f2.file_id.0));
}

#[tokio::test]
async fn dv_persistence_across_reopen() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("persist.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let p = Principal::User { user_id: alice, account_role: AccountRole::Owner };
    let project = state.projects.create(&p, "persist", Visibility::Private).await.expect("create");

    drop(state);
    let (state2, _db2) = assemble(&config).await.expect("reopen");
    let alice2 = user_id(&state2, "alice").await;
    let p2 = Principal::User { user_id: alice2, account_role: AccountRole::Owner };
    let list = state2.projects.list(&p2).await.expect("owner list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].project_id, project.project_id);
    assert!(state2.versions.list(&project.project_id, &p2).await.is_ok());
}

async fn to_principal(state: &filehub_server::http::AppState, bearer: &str) -> Result<Principal, String> {
    let value = bearer.trim().strip_prefix("Bearer ").unwrap_or(bearer.trim());
    let auth = state.auth_provider();
    auth.current_principal(Some(value))
        .await
        .map_err(|e| e.to_string())
}
