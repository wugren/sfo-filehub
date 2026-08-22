#[path = "../common/mod.rs"] mod common;

use common::{assemble, temp_dir, test_config};
use filehub_server::account::store::FilehubAccount;
use sfo_account::{Account, AccountManager, AccountStore};

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
    assert!(!alice.salt.is_empty());
    assert!(!alice.password_hash.is_empty());

    let (session, refresh) = state
        .account
        .manager()
        .login("alice", "alice-pass", 1700000000, None)
        .await
        .expect("login ok");
    let decoded = state.account.decode_session(&session).await.expect("decode");
    assert_eq!(decoded.id, alice.id);
    assert!(state.account.decode_session("garbage").await.is_err());

    let (_new_session, _new_refresh) = state
        .account
        .manager()
        .refresh_session(&refresh)
        .await
        .expect("refresh ok");
    assert!(state
        .account
        .manager()
        .login("alice", "wrong-pass", 1700000000, None)
        .await
        .is_err());
}

#[tokio::test]
async fn second_init_is_idempotent() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, db) = assemble(&config).await.expect("assemble");
    let state2 = filehub_server::http::AppState::assemble(&config, &db).await.expect("re-assemble");
    assert!(state.account.store().get_account_by_name("alice").await.expect("store").is_some());
    assert!(state2.account.store().get_account_by_name("alice").await.expect("store2").is_some());
}

#[test]
fn session_payload_does_not_expose_credentials() {
    use serde_json::json;
    let account = FilehubAccount {
        id: filehub_server::model::UserId(1),
        name: "alice".to_string(),
        salt: "sekret".to_string(),
        password_hash: "deadbeef".to_string(),
    };
    let text = serde_json::to_string(&account).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(value.get("salt").is_none());
    assert!(value.get("password_hash").is_none());
    assert_eq!(json!(text).as_str().is_some(), true);
}
