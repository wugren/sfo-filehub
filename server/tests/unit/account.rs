#[path = "../common/mod.rs"]
mod common;

use common::{assemble, temp_dir, test_config};
use filehub_server::account::store::FilehubAccount;
use filehub_server::model::UserConfig;
use jsonwebtoken::{Algorithm, EncodingKey, Header, decode_header, encode};
use sfo_account::{Account, AccountManager, AccountStore};

const OTHER_SESSION_PRIVATE_KEY: &str = "-----BEGIN PRIVATE KEY-----\n\
MC4CAQAwBQYDK2VwBCIEIMd9gpD7z+EHtSndzdbML+7ECkcSmEIi9ghcjFL1f34i\n\
-----END PRIVATE KEY-----\n";

#[tokio::test]
async fn seeds_users_and_logins() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");

    let alice = state
        .account
        .store()
        .get_account_by_name("alice")
        .await
        .expect("store ok")
        .expect("alice exists");
    assert_eq!(alice.account_name(), "alice");
    assert!(
        alice.password_hash.starts_with("$2"),
        "bcrypt hash expected, got {}",
        alice.password_hash
    );
    assert!(!alice.password_hash.is_empty());

    let (session, refresh) = state
        .account
        .manager()
        .login("alice", "alice-pass", 1700000000, None)
        .await
        .expect("login ok");
    assert_eq!(
        decode_header(&session).expect("session header").alg,
        Algorithm::EdDSA
    );
    assert_eq!(
        decode_header(&refresh).expect("refresh header").alg,
        Algorithm::EdDSA
    );
    let decoded = state
        .account
        .decode_session(&session)
        .await
        .expect("decode");
    assert_eq!(decoded.id, alice.id);
    assert!(state.account.decode_session("garbage").await.is_err());

    let (new_session, new_refresh) = state
        .account
        .manager()
        .refresh_session(&refresh)
        .await
        .expect("refresh ok");
    assert_eq!(
        decode_header(&new_session).expect("new session header").alg,
        Algorithm::EdDSA
    );
    assert_eq!(
        decode_header(&new_refresh).expect("new refresh header").alg,
        Algorithm::EdDSA
    );
    assert!(
        state
            .account
            .manager()
            .login("alice", "wrong-pass", 1700000000, None)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn session_rejects_tampering_hmac_and_a_different_ed25519_key() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("eddsa.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let (session, _refresh) = state
        .account
        .manager()
        .login("alice", "alice-pass", 1700000000, None)
        .await
        .expect("login ok");

    let mut tampered = session.clone();
    let last = tampered.pop().expect("signature byte");
    tampered.push(if last == 'a' { 'b' } else { 'a' });
    assert!(
        state.account.decode_session(&tampered).await.is_err(),
        "a modified EdDSA signature must fail"
    );

    let hmac = encode(
        &Header::new(Algorithm::HS256),
        &serde_json::json!({"exp": 4102444800_u64, "data": {}}),
        &EncodingKey::from_secret(b"0123456789abcdef0123456789abcdef"),
    )
    .expect("build HMAC token");
    assert!(
        state.account.decode_session(&hmac).await.is_err(),
        "an HMAC JWT must not enter the EdDSA session path"
    );
    assert!(
        state
            .account
            .manager()
            .refresh_session(&hmac)
            .await
            .is_err(),
        "an HMAC JWT must not enter the EdDSA refresh path"
    );

    let other_dir = temp_dir().await;
    let mut other_config = test_config(
        &other_dir,
        &other_dir.join("other-eddsa.db").to_string_lossy(),
    );
    other_config.users.session_private_key = OTHER_SESSION_PRIVATE_KEY.to_string();
    let (other_state, _other_db) = assemble(&other_config).await.expect("other assemble");
    assert!(
        other_state.account.decode_session(&session).await.is_err(),
        "a different Ed25519 key must not verify the session"
    );
}

#[tokio::test]
async fn decode_session_rejects_refresh_session() {
    // 高危回归：refresh token 只允许走 refresh_session 续期，
    // decode_session（认证桥与 /account/get_account_info* 共用）必须拒绝。
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("refresh.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");

    let (session, refresh) = state
        .account
        .manager()
        .login("alice", "alice-pass", 1700000000, None)
        .await
        .expect("login ok");
    let alice = state
        .account
        .store()
        .get_account_by_name("alice")
        .await
        .expect("store ok")
        .expect("alice exists");

    let decoded = state
        .account
        .decode_session(&session)
        .await
        .expect("normal session must decode");
    assert_eq!(decoded.id, alice.id);

    let err = state
        .account
        .decode_session(&refresh)
        .await
        .expect_err("refresh session must not decode as an access session");
    assert_eq!(
        err.code(),
        sfo_account::AccountErrorCode::SessionInvalid
    );

    // 续期端点不受影响：refresh 仍可换发新 session/refresh。
    let (_new_session, _new_refresh) = state
        .account
        .manager()
        .refresh_session(&refresh)
        .await
        .expect("refresh must still rotate credentials");
}

#[tokio::test]
async fn second_init_is_idempotent() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    let before = state
        .account
        .store()
        .get_account_by_name("alice")
        .await
        .expect("store")
        .expect("alice exists")
        .password_hash;
    let state2 = filehub_server::http::AppState::assemble(&config, &db)
        .await
        .expect("re-assemble");
    let after = state2
        .account
        .store()
        .get_account_by_name("alice")
        .await
        .expect("store2")
        .expect("alice still exists")
        .password_hash;
    assert_eq!(
        before, after,
        "unchanged password must not rewrite the bcrypt hash"
    );
}

#[test]
fn session_payload_does_not_expose_credentials() {
    use serde_json::json;
    let account = FilehubAccount {
        id: filehub_server::model::UserId(1),
        name: "alice".to_string(),
        password_hash: "deadbeef".to_string(),
        active: true,
    };
    let text = serde_json::to_string(&account).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(value.get("salt").is_none());
    assert!(value.get("password_hash").is_none());
    assert!(
        value.get("active").is_none(),
        "active must not enter claims/response"
    );
    assert_eq!(json!(text).as_str().is_some(), true);
}

#[tokio::test]
async fn config_password_change_applies_on_reinit() {
    let dir = temp_dir().await;
    let db_path = dir.join("pwd-change.db").to_string_lossy().to_string();
    let mut config = test_config(&dir, &db_path);
    config.users.users.retain(|u| u.username == "alice");
    let (state, db) = assemble(&config).await.expect("assemble");
    let old_hash = state
        .account
        .store()
        .get_account_by_name("alice")
        .await
        .expect("store")
        .expect("alice exists")
        .password_hash;

    config.users.users[0].password = Some("alice-pass-new".to_string());
    let state2 = filehub_server::http::AppState::assemble(&config, &db)
        .await
        .expect("re-assemble with new password");
    assert!(
        state2
            .account
            .manager()
            .login("alice", "alice-pass", 1700000000, None)
            .await
            .is_err(),
        "old password must fail after config change"
    );
    let (_session, _refresh) = state2
        .account
        .manager()
        .login("alice", "alice-pass-new", 1700000000, None)
        .await
        .expect("new password must succeed");
    let new_hash = state2
        .account
        .store()
        .get_account_by_name("alice")
        .await
        .expect("store2")
        .expect("alice exists")
        .password_hash;
    assert_ne!(old_hash, new_hash, "stored bcrypt hash must be updated");
}

#[tokio::test]
async fn config_hash_change_applies_and_invalid_hash_rejected_on_existing_account() {
    let dir = temp_dir().await;
    let db_path = dir.join("pwd-hash.db").to_string_lossy().to_string();
    let mut config = test_config(&dir, &db_path);
    config.users.users.retain(|u| u.username == "alice");
    let (state, _db) = assemble(&config).await.expect("assemble");
    assert!(
        state
            .account
            .manager()
            .login("alice", "alice-pass", 1700000000, None)
            .await
            .is_ok()
    );

    // 合法的新 hash：旧密码失效、新密码生效，且库中 hash 与配置一致。
    let new_hash = bcrypt::hash("hash-pass", bcrypt::DEFAULT_COST).expect("bcrypt hash");
    config.users.users[0].password = None;
    config.users.users[0].password_hash = Some(new_hash.clone());
    let (state2, db) = assemble(&config).await.expect("re-assemble with new hash");
    assert!(
        state2
            .account
            .manager()
            .login("alice", "alice-pass", 1700000000, None)
            .await
            .is_err()
    );
    let (_session, _refresh) = state2
        .account
        .manager()
        .login("alice", "hash-pass", 1700000000, None)
        .await
        .expect("hash-configured password must succeed");
    let stored = state2
        .account
        .store()
        .get_account_by_name("alice")
        .await
        .expect("store")
        .expect("alice exists")
        .password_hash;
    assert_eq!(stored, new_hash);

    // 非法 hash：即使账号已存在，启动也必须报错（修复"已存在账号跳过校验"）。
    config.users.users[0].password_hash = Some("deadbeef".to_string());
    let error = match filehub_server::http::AppState::assemble(&config, &db).await {
        Ok(_) => panic!("invalid hash must be rejected for an existing account"),
        Err(e) => e,
    };
    assert!(error.contains("bcrypt"), "unexpected error: {error}");
}

#[tokio::test]
async fn removed_from_config_is_deactivated_and_readd_restores() {
    let dir = temp_dir().await;
    let db_path = dir.join("deactivate.db").to_string_lossy().to_string();
    let mut config = test_config(&dir, &db_path);
    let (state, db) = assemble(&config).await.expect("assemble");
    assert!(
        state
            .account
            .manager()
            .login("bob", "bob-pass", 1700000000, None)
            .await
            .is_ok()
    );
    let bob_id = state
        .account
        .store()
        .get_account_by_name("bob")
        .await
        .expect("store")
        .expect("bob exists")
        .id;

    // 从配置移除 bob：登录被拒、常规查询视为不存在，但行保留且停用。
    config.users.users.retain(|u| u.username == "alice");
    let state2 = filehub_server::http::AppState::assemble(&config, &db)
        .await
        .expect("re-assemble without bob");
    assert!(
        state2
            .account
            .manager()
            .login("bob", "bob-pass", 1700000000, None)
            .await
            .is_err(),
        "removed user must not log in"
    );
    assert!(
        state2
            .account
            .store()
            .get_account_by_name("bob")
            .await
            .expect("store2")
            .is_none(),
        "removed user must be invisible to regular lookups"
    );
    let managed = state2
        .account
        .store()
        .get_managed_account_by_name("bob")
        .await
        .expect("managed store")
        .expect("row must be kept");
    assert_eq!(managed.id, bob_id);
    assert!(!managed.active, "row must be deactivated, not deleted");

    // 重新加入配置：账号恢复登录，active 回 1。
    config.users.users.push(UserConfig {
        username: "bob".to_string(),
        password: Some("bob-pass".to_string()),
        password_hash: None,
    });
    let state3 = filehub_server::http::AppState::assemble(&config, &db)
        .await
        .expect("re-assemble with bob restored");
    assert!(
        state3
            .account
            .manager()
            .login("bob", "bob-pass", 1700000000, None)
            .await
            .is_ok(),
        "readding a removed user must restore login"
    );
    assert!(
        state3
            .account
            .store()
            .get_account_by_name("bob")
            .await
            .expect("store3")
            .is_some()
    );
}

#[tokio::test]
async fn rejects_non_bcrypt_config_hash() {
    let dir = temp_dir().await;
    let mut config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    config.users.users = vec![UserConfig {
        username: "admin".to_string(),
        password: None,
        password_hash: Some("deadbeef-sha256-style".to_string()),
    }];
    let error = match assemble(&config).await {
        Ok(_) => panic!("init should reject non-bcrypt hash"),
        Err(e) => e,
    };
    assert!(error.contains("bcrypt"), "unexpected error: {error}");
}

#[tokio::test]
async fn rejects_malformed_bcrypt_prefix_config_hash() {
    let dir = temp_dir().await;
    let mut config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    config.users.users = vec![UserConfig {
        username: "admin".to_string(),
        password: None,
        password_hash: Some("$2-invalid".to_string()),
    }];
    let error = match assemble(&config).await {
        Ok(_) => panic!("init should reject malformed bcrypt hash"),
        Err(e) => e,
    };
    assert!(error.contains("bcrypt"), "unexpected error: {error}");
}

#[tokio::test]
async fn rejects_truncated_bcrypt_config_hash() {
    let dir = temp_dir().await;
    let mut config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let encoded = bcrypt::hash("cfg-pass", bcrypt::DEFAULT_COST).expect("bcrypt hash");
    let truncated = encoded[..40].to_string();
    assert!(truncated.starts_with("$2"));
    config.users.users = vec![UserConfig {
        username: "admin".to_string(),
        password: None,
        password_hash: Some(truncated),
    }];
    let error = match assemble(&config).await {
        Ok(_) => panic!("init should reject truncated bcrypt hash"),
        Err(e) => e,
    };
    assert!(error.contains("bcrypt"), "unexpected error: {error}");
}

#[tokio::test]
async fn rejects_out_of_range_bcrypt_cost() {
    let dir = temp_dir().await;
    let mut config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let encoded = bcrypt::hash("cfg-pass", bcrypt::DEFAULT_COST).expect("bcrypt hash");
    let syntactically_valid = format!("$2b$99${}", &encoded[7..]);
    config.users.users = vec![UserConfig {
        username: "admin".to_string(),
        password: None,
        password_hash: Some(syntactically_valid),
    }];
    let error = match assemble(&config).await {
        Ok(_) => panic!("init should reject out-of-range bcrypt cost"),
        Err(e) => e,
    };
    assert!(error.contains("cost"), "unexpected error: {error}");
}

#[tokio::test]
async fn rejects_overlong_password() {
    let dir = temp_dir().await;
    let mut config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    config.users.users = vec![UserConfig {
        username: "admin".to_string(),
        password: Some("a".repeat(73)),
        password_hash: None,
    }];
    let error = match assemble(&config).await {
        Ok(_) => panic!("init should reject overlong password"),
        Err(e) => e,
    };
    assert!(error.contains("72 bytes"), "unexpected error: {error}");
}

#[tokio::test]
async fn accepts_bcrypt_config_hash_and_logs_in() {
    let dir = temp_dir().await;
    let mut config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let encoded = bcrypt::hash("cfg-pass", bcrypt::DEFAULT_COST).expect("bcrypt hash");
    config.users.users = vec![UserConfig {
        username: "admin".to_string(),
        password: None,
        password_hash: Some(encoded),
    }];
    let (state, _db) = assemble(&config).await.expect("assemble");
    state
        .account
        .manager()
        .login("admin", "cfg-pass", 1700000000, None)
        .await
        .expect("login with bcrypt config hash");
}

#[tokio::test]
async fn login_failure_is_uniform_for_unknown_and_wrong_password() {
    // 054 起跟随 crates.io sfo-account 0.2.1：未知账号与密码错误区分
    // 错误码/消息（用户确认接受，不做等成本伪校验）。
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("uniform.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");

    let unknown = state
        .account
        .manager()
        .login("ghost", "whatever", 1700000000, None)
        .await
        .expect_err("unknown account must fail");
    let wrong = state
        .account
        .manager()
        .login("alice", "wrong-pass", 1700000000, None)
        .await
        .expect_err("wrong password must fail");

    assert_eq!(unknown.code(), sfo_account::AccountErrorCode::InvalidAccount);
    assert_eq!(unknown.msg(), "account ghost not found");
    assert_eq!(
        wrong.code(),
        sfo_account::AccountErrorCode::InvalidPassword
    );
    assert_eq!(wrong.msg(), "Invalid username or password");
}

#[test]
fn login_dummy_hash_matches_production_bcrypt_cost() {
    let parts: bcrypt::HashParts = filehub_server::account::store::LOGIN_DUMMY_BCRYPT_HASH
        .parse()
        .expect("dummy hash must be a valid bcrypt encoded string");
    assert_eq!(parts.get_cost(), bcrypt::DEFAULT_COST);
    assert!(
        !bcrypt::verify(
            "whatever",
            filehub_server::account::store::LOGIN_DUMMY_BCRYPT_HASH
        )
        .expect("dummy hash verify must not error"),
        "dummy hash must never authenticate any password"
    );
}
