#[path = "../common/mod.rs"]
mod common;

use common::{assemble, temp_dir, test_config, user_id};
use filehub_server::model::{ProjectId, ProjectScope, Scope, ScopeSet, UserId};
use filehub_server::tokens::model::{TokenCreateRequest, TokenErrorKind, TokenUpdateRequest};

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

    // 仅改名字：返回摘要且不重签
    let renamed = state
        .tokens
        .update(
            &issued.token_id,
            &alice,
            TokenUpdateRequest {
                name: Some("renamed".to_string()),
                project_scope: None,
                scopes: None,
            },
        )
        .await
        .expect("rename");
    assert_eq!(renamed.name, "renamed");
    // 名字变更后旧 JWT 仍可用
    if let Err(e) = state.tokens.resolve(&first_jwt).await {
        panic!("resolve after rename failed: {e:?}");
    }

    // 权限变更不重签：返回摘要，旧 JWT 保持有效，权限按数据库立即生效
    let updated = state
        .tokens
        .update(
            &issued.token_id,
            &alice,
            TokenUpdateRequest {
                name: None,
                project_scope: None,
                scopes: Some(vec![Scope::ArtifactsRead]),
            },
        )
        .await
        .expect("update");
    assert_eq!(updated.scopes.0.len(), 1);
    let principal = state
        .tokens
        .resolve(&first_jwt)
        .await
        .expect("old jwt still ok");
    assert_eq!(principal.token_id, issued.token_id);
    assert_eq!(principal.scopes.0.len(), 1);

    // rotate 后旧 JWT 立即失效
    let rotated = state
        .tokens
        .rotate(&issued.token_id, &alice)
        .await
        .expect("rotate");
    assert!(state.tokens.resolve(&first_jwt).await.is_err());
    assert!(state.tokens.resolve(&rotated.jwt).await.is_ok());

    // revoke
    state
        .tokens
        .revoke(&issued.token_id, &alice)
        .await
        .expect("revoke");
    assert!(state.tokens.resolve(&rotated.jwt).await.is_err());
    assert!(state.tokens.list(&alice).await.expect("list").is_empty());
}

#[tokio::test]
async fn token_rotate_rejects_revoked_token() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;

    let issued = state.tokens.create(owner(alice)).await.expect("create");
    state
        .tokens
        .revoke(&issued.token_id, &alice)
        .await
        .expect("revoke");

    let rotated = state.tokens.rotate(&issued.token_id, &alice).await;
    let err = rotated.expect_err("revoked token must not rotate");
    assert_eq!(
        err.kind,
        TokenErrorKind::NotFound,
        "revoked rotate must map to 404"
    );
    assert!(
        state.tokens.resolve(&issued.jwt).await.is_err(),
        "revoked token JWT stays rejected"
    );
}

#[tokio::test]
async fn token_concurrent_rotates_have_single_usable_winner() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;

    for _ in 0..5 {
        let issued = state.tokens.create(owner(alice)).await.expect("create");
        let (a, b) = tokio::join!(
            state.tokens.rotate(&issued.token_id, &alice),
            state.tokens.rotate(&issued.token_id, &alice),
        );
        let (winner, loser) = match (a, b) {
            (Ok(winner), Err(err)) => (winner, err),
            (Err(err), Ok(winner)) => (winner, err),
            (Ok(_), Ok(_)) => panic!("concurrent rotates must not both succeed"),
            (Err(a_err), Err(b_err)) => panic!("one concurrent rotate must win: {a_err:?} / {b_err:?}"),
        };
        assert_eq!(
            loser.kind,
            TokenErrorKind::Conflict,
            "lost concurrent rotate must map to 409"
        );
        assert!(
            state.tokens.resolve(&winner.jwt).await.is_ok(),
            "winner jwt must be immediately usable"
        );
        // 后续合法顺序轮换读到新快照，CAS 命中仍可正常成功（guard 不误伤）。
        let next = state
            .tokens
            .rotate(&issued.token_id, &alice)
            .await
            .expect("sequential rotate after concurrent race");
        assert!(state.tokens.resolve(&next.jwt).await.is_ok());
        assert!(state.tokens.resolve(&winner.jwt).await.is_err());
    }
}

#[tokio::test]
async fn token_rotate_cas_rejects_stale_public_key_snapshot() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;

    let issued = state.tokens.create(owner(alice)).await.expect("create");
    let before: String = sqlx::query_scalar("SELECT public_key_pem FROM tokens WHERE id = ?")
        .bind(issued.token_id.0)
        .fetch_one(&db)
        .await
        .expect("read pre-rotate public key");
    state
        .tokens
        .rotate(&issued.token_id, &alice)
        .await
        .expect("rotate");

    // 模拟并发 rotate 落败方用旋转前的旧公钥快照提交：CAS 必须拒绝（0 行），
    // 否则会覆盖获胜者公钥、使获胜者已返回的 JWT 立即失效。
    let stale_attempt = sqlx::query(
        "UPDATE tokens SET public_key_pem = ?, updated_at = ? WHERE id = ? AND owner_id = ? AND revoked_at IS NULL AND public_key_pem = ?",
    )
    .bind("stale-cas-marker")
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(issued.token_id.0)
    .bind(alice.0)
    .bind(&before)
    .execute(&db)
    .await
    .expect("stale cas attempt");
    assert_eq!(
        stale_attempt.rows_affected(),
        0,
        "stale public-key snapshot must be rejected"
    );

    // 当前公钥快照仍可命中（证明 CAS 只拒绝过期快照，不误伤正常写入）。
    let current: String = sqlx::query_scalar("SELECT public_key_pem FROM tokens WHERE id = ?")
        .bind(issued.token_id.0)
        .fetch_one(&db)
        .await
        .expect("read post-rotate public key");
    let fresh_attempt = sqlx::query(
        "UPDATE tokens SET public_key_pem = ?, updated_at = ? WHERE id = ? AND owner_id = ? AND revoked_at IS NULL AND public_key_pem = ?",
    )
    .bind(&current)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(issued.token_id.0)
    .bind(alice.0)
    .bind(&current)
    .execute(&db)
    .await
    .expect("fresh cas attempt");
    assert_eq!(
        fresh_attempt.rows_affected(),
        1,
        "fresh public-key snapshot must be accepted"
    );
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

#[tokio::test]
async fn token_permissions_are_read_from_db_not_jwt() {
    use base64::Engine;

    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;

    let mut req = owner(alice);
    req.project_scope = Some(ProjectScope::Specified(vec![ProjectId(7)]));
    req.scopes = vec![Scope::ArtifactsRead, Scope::MetadataRead];
    let issued = state.tokens.create(req).await.expect("create");

    // JWT data 载荷不含权限属性（scopes/project_scope 不在 claims 中）。
    let parts: Vec<&str> = issued.jwt.split('.').collect();
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .expect("payload base64");
    let value: serde_json::Value = serde_json::from_slice(&payload_bytes).expect("payload json");
    let data = value.get("data").expect("data claim");
    assert!(data.get("scopes").is_none());
    assert!(data.get("project_scope").is_none());

    // resolve 从数据库读取 scopes 与 project_scope。
    let principal = state.tokens.resolve(&issued.jwt).await.expect("resolve");
    assert_eq!(
        principal.project_scope,
        ProjectScope::Specified(vec![ProjectId(7)])
    );
    assert_eq!(
        principal.scopes,
        ScopeSet(
            [Scope::ArtifactsRead, Scope::MetadataRead]
                .into_iter()
                .collect()
        )
    );

    // scope/project_scope 变更不重签：旧 JWT 仍有效，resolve 以数据库为准。
    let updated = state
        .tokens
        .update(
            &issued.token_id,
            &alice,
            TokenUpdateRequest {
                name: None,
                project_scope: Some(ProjectScope::Specified(vec![ProjectId(8)])),
                scopes: Some(vec![Scope::MetadataRead]),
            },
        )
        .await
        .expect("update");
    assert_eq!(
        updated.project_scope,
        ProjectScope::Specified(vec![ProjectId(8)])
    );
    let after = state
        .tokens
        .resolve(&issued.jwt)
        .await
        .expect("resolve old jwt");
    assert_eq!(
        after.project_scope,
        ProjectScope::Specified(vec![ProjectId(8)])
    );
    assert_eq!(after.scopes.0.len(), 1);
    assert!(after.scopes.contains_scope(Scope::MetadataRead));
}

#[tokio::test]
async fn empty_project_scope_means_all() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;

    let mut req = owner(alice);
    req.project_scope = Some(ProjectScope::Specified(vec![]));
    let issued = state.tokens.create(req).await.expect("create");

    let principal = state.tokens.resolve(&issued.jwt).await.expect("resolve");
    assert_eq!(principal.project_scope, ProjectScope::All);
    let summary = state.tokens.list(&alice).await.expect("list");
    assert_eq!(summary[0].project_scope, ProjectScope::All);
    let stored: String = sqlx::query_scalar("SELECT project_scope FROM tokens WHERE id = ?")
        .bind(issued.token_id.0)
        .fetch_one(&db)
        .await
        .expect("stored scope");
    assert_eq!(stored, "all");

    // 更新为空集合同样归一化为 All，且不重签。
    let updated = state
        .tokens
        .update(
            &issued.token_id,
            &alice,
            TokenUpdateRequest {
                name: None,
                project_scope: Some(ProjectScope::Specified(vec![])),
                scopes: None,
            },
        )
        .await
        .expect("update");
    assert_eq!(updated.project_scope, ProjectScope::All);
    let after = state
        .tokens
        .resolve(&issued.jwt)
        .await
        .expect("resolve old jwt");
    assert_eq!(after.project_scope, ProjectScope::All);
}

#[tokio::test]
async fn token_attribute_update_preserves_exp_and_does_not_resign() {
    use base64::Engine;

    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    let alice = user_id(&state, "alice").await;

    let mut req = owner(alice);
    req.expires_at = Some(chrono::Utc::now() + chrono::Duration::days(30));
    let issued = state.tokens.create(req).await.expect("create");

    fn exp_of(jwt: &str) -> i64 {
        let parts: Vec<&str> = jwt.split('.').collect();
        let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[1])
            .expect("payload base64");
        let value: serde_json::Value =
            serde_json::from_slice(&payload_bytes).expect("payload json");
        value
            .get("exp")
            .and_then(|v| v.as_i64())
            .expect("exp claim")
    }

    let exp_before = exp_of(&issued.jwt);
    let public_key_before: String =
        sqlx::query_scalar("SELECT public_key_pem FROM tokens WHERE id = ?")
            .bind(issued.token_id.0)
            .fetch_one(&db)
            .await
            .expect("public key before");

    let summary = state
        .tokens
        .update(
            &issued.token_id,
            &alice,
            TokenUpdateRequest {
                name: None,
                project_scope: None,
                scopes: Some(vec![Scope::ArtifactsRead]),
            },
        )
        .await
        .expect("update");
    assert_eq!(summary.scopes.0.len(), 1);

    // 不重签：验签公钥不变，原来的有限期 JWT 仍可解析且 exp 保持原样。
    let public_key_after: String =
        sqlx::query_scalar("SELECT public_key_pem FROM tokens WHERE id = ?")
            .bind(issued.token_id.0)
            .fetch_one(&db)
            .await
            .expect("public key after");
    assert_eq!(
        public_key_before, public_key_after,
        "attribute update must not rotate the signing key"
    );
    assert_eq!(
        exp_of(&issued.jwt),
        exp_before,
        "attribute update must not touch the JWT exp claim"
    );
    state
        .tokens
        .resolve(&issued.jwt)
        .await
        .expect("old limited-lifetime jwt still resolves");
}
