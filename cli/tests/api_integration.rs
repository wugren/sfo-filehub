//! 集成：v1 契约正反例、401 续期、错误映射与凭据安全边界。

mod common;

use common::{MockServer, Mode, TestEnv};
use filehub_cli::cli::args::{
    DeleteAppArgs, LockVersionArgs, LoginArgs, NewVersionArgs, PullArgs, PushArgs, VersionsArgs,
};
use filehub_cli::cli::{
    CliError, delete_app_handler, lock_version_handler, login_handler, new_version_handler,
    pull_handler, push_handler, versions_handler,
};
use filehub_cli::credential_store::CredentialStore;

async fn login_password(env: &TestEnv, server: &str) {
    unsafe {
        std::env::set_var("FILEHUB_PASSWORD", "alice-pass");
    }
    let code = login_handler::run_login(
        Some(&env.config),
        LoginArgs {
            server: Some(server.to_string()),
            username: Some("alice".to_string()),
            password_stdin: false,
            token_stdin: false,
        },
    )
    .await
    .expect("password login");
    assert_eq!(code, 0);
    unsafe {
        std::env::remove_var("FILEHUB_PASSWORD");
    }
}

#[tokio::test]
async fn login_password_persists_session_and_logout_clears() {
    common::log_case("login_password_persists_session_and_logout_clears");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("login")).await;
    let env = TestEnv::new();
    login_password(&env, &server.base).await;

    let store = CredentialStore::open(&env.config).unwrap();
    let credential = store
        .current_credential(&server.base)
        .expect("session stored");
    let text = format!("{credential:?}");
    assert!(
        text.contains("payload-secret") || text.contains("s1"),
        "session 已持久化"
    );

    let code = login_handler::run_logout(
        Some(&env.config),
        filehub_cli::cli::args::LogoutArgs {
            server: Some(server.base.clone()),
        },
    )
    .await
    .expect("logout");
    assert_eq!(code, 0);
    let store = CredentialStore::open(&env.config).unwrap();
    assert!(store.current_credential(&server.base).is_none());
}

#[tokio::test]
async fn login_without_scheme_stores_identity_credential() {
    common::log_case("login_without_scheme_stores_identity_credential");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("login-noscheme")).await;
    let env = TestEnv::new();
    login_password(&env, &server.identity()).await;

    let store = CredentialStore::open(&env.config).unwrap();
    assert!(store.current_credential(&server.identity()).is_some());
    assert!(store.current_credential(&server.base).is_some());

    // 落盘 key 必须是 host[:port]，不再包含协议头。
    let raw = std::fs::read_to_string(&env.config).unwrap();
    assert!(raw.contains(&format!("[server.\"{}\"]", server.identity())));
    assert!(!raw.contains("[server.\"https://"));
    assert!(!raw.contains("[server.\"http://"));
}

#[tokio::test]
async fn legacy_http_credential_matches_no_scheme_push() {
    common::log_case("legacy_http_credential_matches_no_scheme_push");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("legacy-push")).await;
    let env = TestEnv::new();

    // 模拟升级前由 `login http://host:port` 写出的旧配置。
    std::fs::write(
        &env.config,
        format!(
            "schema_version = 1\n\n[server.\"http://{}\"]\nusername = \"alice\"\nsession = \"s1\"\nrefresh_session = \"r1\"\n",
            server.identity()
        ),
    )
    .unwrap();

    let source = env.dir.path().join("legacy.bin");
    std::fs::write(&source, b"legacy").unwrap();
    let code = push_handler::run(
        Some(&env.config),
        PushArgs {
            path: source,
            target: format!("{}/demo/legacy-ok/app", server.identity()),
        },
    )
    .await
    .expect("legacy credential matches no-scheme server");
    assert_eq!(code, 0);
}

#[tokio::test]
async fn no_scheme_login_and_push_workflow() {
    common::log_case("no_scheme_login_and_push_workflow");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("noscheme-full")).await;
    let env = TestEnv::new();
    login_password(&env, &server.identity()).await;

    let source = env.dir.path().join("payload.bin");
    std::fs::write(&source, b"payload").unwrap();
    let code = push_handler::run(
        Some(&env.config),
        PushArgs {
            path: source,
            target: format!("{}/demo/noscheme-v1/app", server.identity()),
        },
    )
    .await
    .expect("no-scheme login and push against local HTTP mock");
    assert_eq!(code, 0);
}

#[tokio::test]
async fn login_bad_password_fails_and_writes_nothing() {
    common::log_case("login_bad_password_fails_and_writes_nothing");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("bad")).await;
    let env = TestEnv::new();
    unsafe {
        std::env::set_var("FILEHUB_PASSWORD", "wrong-pass");
    }
    let result = login_handler::run_login(
        Some(&env.config),
        LoginArgs {
            server: Some(server.base.clone()),
            username: Some("alice".to_string()),
            password_stdin: false,
            token_stdin: false,
        },
    )
    .await;
    unsafe {
        std::env::remove_var("FILEHUB_PASSWORD");
    }
    let error: CliError = result.expect_err("bad password");
    assert_eq!(error.exit_code(), 2);
    assert!(!env.config.exists() || env.config.metadata().unwrap().len() == 0);
}

#[tokio::test]
async fn login_mode_conflict_is_usage_error() {
    common::log_case("login_mode_conflict_is_usage_error");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("conflict")).await;
    let env = TestEnv::new();
    let result = login_handler::run_login(
        Some(&env.config),
        LoginArgs {
            server: Some(server.base.clone()),
            username: Some("alice".to_string()),
            password_stdin: true,
            token_stdin: true,
        },
    )
    .await;
    let error: CliError = result.expect_err("mutual exclusion");
    assert_eq!(error.exit_code(), 1);
}

#[tokio::test]
async fn token_login_validates_before_persisting() {
    common::log_case("token_login_validates_before_persisting");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("token")).await;
    let env = TestEnv::new();
    unsafe {
        std::env::set_var("FILEHUB_TOKEN", "tok-valid");
    }
    let code = login_handler::run_login(
        Some(&env.config),
        LoginArgs {
            server: Some(server.base.clone()),
            username: None,
            password_stdin: false,
            token_stdin: false,
        },
    )
    .await
    .expect("token login");
    unsafe {
        std::env::remove_var("FILEHUB_TOKEN");
    }
    assert_eq!(code, 0);
    let store = CredentialStore::open(&env.config).unwrap();
    let credential = store
        .current_credential(&server.base)
        .expect("token stored");
    assert!(format!("{credential:?}").contains("tok-valid"));
}

#[tokio::test]
async fn versions_outputs_json_and_text_files() {
    common::log_case("versions_outputs_json_and_text_files");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("versions")).await;
    let env = TestEnv::new();
    login_password(&env, &server.base).await;

    let json_out = env.dir.path().join("versions.json");
    let code = versions_handler::run(
        Some(&env.config),
        VersionsArgs {
            target: format!("{}/demo", server.base),
            output: Some(json_out.clone()),
            format: "json".to_string(),
        },
    )
    .await
    .expect("json versions");
    assert_eq!(code, 0);
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&json_out).unwrap()).unwrap();
    assert!(value.is_array());
    assert_eq!(value.as_array().unwrap().len(), 2);

    let text_out = env.dir.path().join("versions.txt");
    let code = versions_handler::run(
        Some(&env.config),
        VersionsArgs {
            target: format!("{}/demo", server.base),
            output: Some(text_out.clone()),
            format: "text".to_string(),
        },
    )
    .await
    .expect("text versions");
    assert_eq!(code, 0);
    let text = std::fs::read_to_string(&text_out).unwrap();
    assert!(text.starts_with("VERSION\tPUBLISHED_AT\tLOCKED\tAPPS"));
    assert!(text.contains("v1") && text.contains("v2"));
}

#[tokio::test]
async fn resolve_project_scans_paginated_projects_beyond_first_page() {
    common::log_case("resolve_project_scans_paginated_projects_beyond_first_page");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("paged")).await;
    let env = TestEnv::new();
    unsafe {
        std::env::set_var("FILEHUB_TOKEN", "tok-paged");
    }
    let code = login_handler::run_login(
        Some(&env.config),
        LoginArgs {
            server: Some(server.base.clone()),
            username: None,
            password_stdin: false,
            token_stdin: false,
        },
    )
    .await
    .expect("token login validates against first page");
    assert_eq!(code, 0);
    unsafe {
        std::env::remove_var("FILEHUB_TOKEN");
    }

    // 520 个项目、CLI 500/页：`pg-520` 在第 2 页，必须能被按名解析。
    let code = versions_handler::run(
        Some(&env.config),
        VersionsArgs {
            target: format!("{}/pg-520", server.base),
            output: None,
            format: "text".to_string(),
        },
    )
    .await
    .expect("resolve project on second page");
    assert_eq!(code, 0);

    // 不存在项目仍按既有语义报 InvalidInput。
    let error = versions_handler::run(
        Some(&env.config),
        VersionsArgs {
            target: format!("{}/missing-pg", server.base),
            output: None,
            format: "text".to_string(),
        },
    )
    .await
    .expect_err("missing project name rejects");
    assert_eq!(error.exit_code(), 5);
}

#[tokio::test]
async fn push_success_conflict_and_forbidden() {
    common::log_case("push_success_conflict_and_forbidden");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("publish")).await;
    let env = TestEnv::new();
    login_password(&env, &server.base).await;
    let source = env.dir.path().join("artifact.txt");
    std::fs::write(&source, "artifact").unwrap();

    let code = push_handler::run(
        Some(&env.config),
        PushArgs {
            path: source.clone(),
            target: format!("{}/demo/new-version/server", server.base),
        },
    )
    .await
    .expect("push");
    assert_eq!(code, 0);

    let conflict = push_handler::run(
        Some(&env.config),
        PushArgs {
            path: source.clone(),
            target: format!("{}/demo/exists/app", server.base),
        },
    )
    .await
    .expect_err("409 conflict");
    assert_eq!(conflict.exit_code(), 4);

    // token 只读：push -> 403。
    unsafe {
        std::env::set_var("FILEHUB_TOKEN", "tok-view");
    }
    let _ = login_handler::run_logout(
        Some(&env.config),
        filehub_cli::cli::args::LogoutArgs {
            server: Some(server.base.clone()),
        },
    )
    .await;
    let _ = login_handler::run_login(
        Some(&env.config),
        LoginArgs {
            server: Some(server.base.clone()),
            username: None,
            password_stdin: false,
            token_stdin: false,
        },
    )
    .await
    .expect("token login");
    unsafe {
        std::env::remove_var("FILEHUB_TOKEN");
    }
    let forbidden = push_handler::run(
        Some(&env.config),
        PushArgs {
            path: source,
            target: format!("{}/demo/forbidden/app", server.base),
        },
    )
    .await
    .expect_err("403 forbidden");
    assert_eq!(forbidden.exit_code(), 3);
}

#[tokio::test]
async fn pull_verifies_sha_and_rejects_corrupt_body() {
    common::log_case("pull_verifies_sha_and_rejects_corrupt_body");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("download")).await;
    let env = TestEnv::new();
    login_password(&env, &server.base).await;
    let out_file = env.dir.path().join("dl").join("artifact.tar.gz");

    let code = pull_handler::run(
        Some(&env.config),
        PullArgs {
            target: format!("{}/demo/v1/default", server.base),
            path: out_file.clone(),
        },
    )
    .await
    .expect("pull");
    assert_eq!(code, 0);
    assert!(out_file.is_file(), "精确文件路径落盘");
    assert_eq!(
        filehub_cli::archive::verify_sha256(&out_file, &server_payload_sha()).is_err(),
        false
    );

    let corrupt_out = env.dir.path().join("corrupt.tar.gz");
    let corrupt = pull_handler::run(
        Some(&env.config),
        PullArgs {
            target: format!("{}/demo/corrupt/default", server.base),
            path: corrupt_out.clone(),
        },
    )
    .await
    .expect_err("corrupt stream");
    assert_eq!(corrupt.exit_code(), 7);
    assert!(!corrupt_out.exists(), "校验失败不得落盘");
}

#[tokio::test]
async fn pull_rejects_directory_target() {
    common::log_case("pull_rejects_directory_target");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("pull-dir")).await;
    let env = TestEnv::new();
    login_password(&env, &server.base).await;
    let dir = env.dir.path().join("dir-output");
    std::fs::create_dir_all(&dir).unwrap();

    let result = pull_handler::run(
        Some(&env.config),
        PullArgs {
            target: format!("{}/demo/v1/default", server.base),
            path: dir.clone(),
        },
    )
    .await;
    let error: CliError = result.expect_err("directory target rejected");
    assert_eq!(error.exit_code(), 5);
}

fn server_payload_sha() -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(common::make_payload("download"));
    format!("{digest:x}")
}

#[tokio::test]
async fn pull_refreshes_expired_session_once() {
    common::log_case("pull_refreshes_expired_session_once");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("refresh")).await;
    let env = TestEnv::new();
    login_password(&env, &server.base).await;
    let out_file = env.dir.path().join("dl").join("refresh.tar.gz");

    let code = pull_handler::run(
        Some(&env.config),
        PullArgs {
            target: format!("{}/demo/refresh-once/default", server.base),
            path: out_file.clone(),
        },
    )
    .await
    .expect("refresh retry");
    assert_eq!(code, 0);
    assert!(out_file.is_file());

    // 下载端点对 s1 401 一次；续期后 store 应当已更新为 s2。
    let store = CredentialStore::open(&env.config).unwrap();
    let credential = store.current_credential(&server.base).unwrap();
    assert!(
        format!("{credential:?}").contains("s2"),
        "session 已续期为 s2"
    );
}

#[tokio::test]
async fn versions_refreshes_session_via_auth_client() {
    common::log_case("versions_refreshes_session_via_auth_client");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::ExpiresOnce, common::make_payload("auth-refresh")).await;
    let env = TestEnv::new();
    login_password(&env, &server.base).await;

    let out = env.dir.path().join("versions.json");
    let code = versions_handler::run(
        Some(&env.config),
        VersionsArgs {
            target: format!("{}/refresh-once", server.base),
            output: Some(out.clone()),
            format: "json".to_string(),
        },
    )
    .await
    .expect("refresh via run_auth");
    assert_eq!(code, 0);

    let store = CredentialStore::open(&env.config).unwrap();
    let credential = store.current_credential(&server.base).unwrap();
    assert!(
        format!("{credential:?}").contains("s2"),
        "run_auth 已续期并重试"
    );
}

#[tokio::test]
async fn pull_requires_explicit_version_and_rejects_bad_target() {
    common::log_case("pull_requires_explicit_version_and_rejects_bad_target");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("latest")).await;
    let env = TestEnv::new();
    login_password(&env, &server.base).await;

    // 缺 version/name 的目标解析失败 -> exit 5（按 / 分段后段数不足，带端口
    // server 也不会与合法形态混淆）。
    let bad = pull_handler::run(
        Some(&env.config),
        PullArgs {
            target: "hub.example.com/demo/missing-name".to_string(),
            path: env.dir.path().join("bad.tar.gz"),
        },
    )
    .await
    .expect_err("three-field target is invalid");
    assert_eq!(bad.exit_code(), 5);

    // 空 name 段同样被拒。
    let empty_name = pull_handler::run(
        Some(&env.config),
        PullArgs {
            target: format!("{}/demo/1.0.0/", server.base),
            path: env.dir.path().join("empty.tar.gz"),
        },
    )
    .await
    .expect_err("empty name is invalid");
    assert_eq!(empty_name.exit_code(), 5);

    // 显式版本（此处 version=latest 由服务端解析为 v2）仍可拉取。
    let out = env.dir.path().join("latest.tar.gz");
    let code = pull_handler::run(
        Some(&env.config),
        PullArgs {
            target: format!("{}/demo/latest/default", server.base),
            path: out.clone(),
        },
    )
    .await
    .expect("explicit version pull");
    assert_eq!(code, 0);
    assert!(out.is_file());
}

#[tokio::test]
async fn new_version_lock_and_delete_app_workflows() {
    common::log_case("new_version_lock_and_delete_app_workflows");
    let _guard = common::lock_env();
    let server = MockServer::start(Mode::Normal, common::make_payload("lifecycle")).await;
    let env = TestEnv::new();
    login_password(&env, &server.base).await;

    // 创建版本成功。
    let code = new_version_handler::run(
        Some(&env.config),
        NewVersionArgs {
            target: format!("{}/demo/created-v1", server.base),
        },
    )
    .await
    .expect("new version");
    assert_eq!(code, 0);

    // 重复创建 -> 409 -> exit 4。
    let conflict = new_version_handler::run(
        Some(&env.config),
        NewVersionArgs {
            target: format!("{}/demo/exists", server.base),
        },
    )
    .await
    .expect_err("409 duplicate version");
    assert_eq!(conflict.exit_code(), 4);

    // 锁定版本成功；锁定后的删除被拒（409 -> exit 4）。
    let code = lock_version_handler::run(
        Some(&env.config),
        LockVersionArgs {
            target: format!("{}/demo/locked-v1", server.base),
        },
    )
    .await
    .expect("lock version");
    assert_eq!(code, 0);
    let locked_delete = delete_app_handler::run(
        Some(&env.config),
        DeleteAppArgs {
            target: format!("{}/demo/locked-v1/web", server.base),
        },
    )
    .await
    .expect_err("locked delete rejected");
    assert_eq!(locked_delete.exit_code(), 4);

    // 未锁定版本删除成功；app 不存在 -> 404 -> exit 5。
    let code = delete_app_handler::run(
        Some(&env.config),
        DeleteAppArgs {
            target: format!("{}/demo/v1/web", server.base),
        },
    )
    .await
    .expect("delete app");
    assert_eq!(code, 0);
    let missing = delete_app_handler::run(
        Some(&env.config),
        DeleteAppArgs {
            target: format!("{}/demo/v1/missing", server.base),
        },
    )
    .await
    .expect_err("app not found");
    assert_eq!(missing.exit_code(), 5);
}
