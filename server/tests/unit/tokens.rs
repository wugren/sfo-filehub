#[path = "../common/mod.rs"] mod common;

use common::{assemble, temp_dir, test_config, user_id};
use filehub_server::model::{Scope, UserId};
use filehub_server::tokens::model::{TokenCreateRequest, TokenUpdateRequest};

fn owner(alice_id: UserId) -> TokenCreateRequest {
    TokenCreateRequest {
        owner: alice_id,
        name: "deploy".to_string(),
        project_scope: None,
        scopes: vec![Scope::ArtifactsRead, Scope::MetadataRead],
        expires_at: None,
    }
}

#[tokio::test]
async fn token_lifecycle_create_list_update_rotate_revoke() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;

    let issued = state.tokens.create(owner(alice)).await.expect("create");
    let first_jwt = issued.jwt.clone();
    if let Err(e) = state.tokens.resolve(&first_jwt).await {
        panic!("resolve immediately after create failed: {e:?}");
    }
    assert_eq!(state.tokens.list(&alice).await.expect("list").len(), 1);

    // 仅改名字不重签
    let none = state
        .tokens
        .update(&issued.token_id, &alice, TokenUpdateRequest {
            name: Some("renamed".to_string()),
            project_scope: None,
            scopes: None,
            expires_at: None,
        })
        .await
        .expect("rename");
    assert!(none.is_none());
    // 名字变更后旧 JWT 仍可用
    if let Err(e) = state.tokens.resolve(&first_jwt).await {
        panic!("resolve after rename failed: {e:?}");
    }

    // 权限变更会重签，旧 JWT 失效
    let resigned = state
        .tokens
        .update(&issued.token_id, &alice, TokenUpdateRequest {
            name: None,
            project_scope: None,
            scopes: Some(vec![Scope::ArtifactsRead]),
            expires_at: None,
        })
        .await
        .expect("resign")
        .expect("issued");
    assert!(state.tokens.resolve(&first_jwt).await.is_err());
    let principal = state.tokens.resolve(&resigned.jwt).await.expect("new jwt ok");
    assert_eq!(principal.token_id, issued.token_id);
    assert_eq!(principal.scopes.0.len(), 1);

    // rotate 后旧 JWT 立即失效
    let rotated = state.tokens.rotate(&issued.token_id, &alice).await.expect("rotate");
    assert!(state.tokens.resolve(&resigned.jwt).await.is_err());
    assert!(state.tokens.resolve(&rotated.jwt).await.is_ok());

    // revoke
    state.tokens.revoke(&issued.token_id, &alice).await.expect("revoke");
    assert!(state.tokens.resolve(&rotated.jwt).await.is_err());
    assert!(state.tokens.list(&alice).await.expect("list").is_empty());
}

#[tokio::test]
async fn token_expiry_validation_and_owner_isolation() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;
    let bob = user_id(&state, "bob").await;

    // 超过 1 年拒绝
    let too_far = chrono::Utc::now() + chrono::Duration::days(366);
    let mut req = owner(alice);
    req.expires_at = Some(too_far);
    assert!(state.tokens.create(req).await.is_err());

    // 不能修改/撤销他人 token
    let issued = state.tokens.create(owner(alice)).await.expect("create");
    assert!(state.tokens.revoke(&issued.token_id, &bob).await.is_err());
}
