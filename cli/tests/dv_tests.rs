//! DV：CLI 主工作流、失败工作流、持久化重开与安全打包反例。

mod common;

use common::{MockServer, Mode, TestEnv};
use filehub_cli::apiclient::contract::VersionDto;
use filehub_cli::archive;
use filehub_cli::cli::args::{LoginArgs, LogoutArgs, PullArgs, PushArgs, VersionsArgs};
use filehub_cli::cli::{CliError, login_handler, pull_handler, push_handler, versions_handler};
use filehub_cli::credential_store::CredentialStore;

async fn login_with_env_password(env: &TestEnv, server: &str) {
    unsafe {
        std::env::set_var("FILEHUB_PASSWORD", "alice-pass");
    }
    let args = LoginArgs {
        server: Some(server.to_string()),
        username: Some("alice".to_string()),
        password_stdin: false,
        token_stdin: false,
    };
    let code = login_handler::run_login(Some(&env.config), args)
        .await
        .expect("login");
    assert_eq!(code, 0);
    unsafe {
        std::env::remove_var("FILEHUB_PASSWORD");
    }
}

#[tokio::test]
async fn dv_full_push_pull_versions_workflow() {
    let _guard = common::lock_env();
    let payload = common::make_payload("dv");
    let server = MockServer::start(Mode::Normal, payload.clone()).await;
    let env = TestEnv::new();
    login_with_env_password(&env, &server.base).await;

    // push 一个文件（单文件打包为 .tar.gz）。
    let source = env.dir.path().join("release-notes.txt");
    std::fs::write(&source, "dv release notes").unwrap();
    let push = PushArgs {
        path: source,
        target: format!("{}/demo/v1-dv/web", server.base),
    };
    assert_eq!(
        push_handler::run(Some(&env.config), push)
            .await
            .expect("push"),
        0
    );

    // pull v1-dv 到精确文件路径并校验内容。
    let pulled = env.dir.path().join("downloads").join("dv.tar.gz");
    let pull = PullArgs {
        target: format!("{}/demo/v1-dv/default", server.base),
        path: pulled.clone(),
    };
    assert_eq!(
        pull_handler::run(Some(&env.config), pull)
            .await
            .expect("pull"),
        0
    );
    assert!(pulled.is_file(), "拉取到精确路径");
    assert_eq!(std::fs::read(&pulled).unwrap(), payload);

    // 版本查询（latest 语义由服务端承载；此处验证 v1/v2 列表输出文件）。
    let out_file = env.dir.path().join("versions.txt");
    let versions_args = VersionsArgs {
        target: format!("{}/demo", server.base),
        output: Some(out_file.clone()),
        format: "json".to_string(),
    };
    assert_eq!(
        versions_handler::run(Some(&env.config), versions_args)
            .await
            .expect("versions"),
        0
    );
    let records: Vec<VersionDto> =
        serde_json::from_slice(&std::fs::read(&out_file).unwrap()).expect("version json");
    assert_eq!(records.len(), 2);
    assert!(records.iter().any(|record| record.version == "v1"));
    assert!(records.iter().any(|record| record.version == "v2"));

    // 凭据持久化：重新打开 store 仍可读取 session；logout 后清空。
    let store = CredentialStore::open(&env.config).unwrap();
    let credential = store.current_credential(&server.base).expect("credential");
    assert!(format!("{credential:?}").contains("s1"));
    drop(store);
    let logout = LogoutArgs {
        server: Some(server.base.clone()),
    };
    assert_eq!(
        login_handler::run_logout(Some(&env.config), logout)
            .await
            .expect("logout"),
        0
    );
    let store = CredentialStore::open(&env.config).unwrap();
    assert!(store.current_credential(&server.base).is_none());

    // 生命周期：logout 后重新 login，凭据重新可用。
    unsafe {
        std::env::set_var("FILEHUB_PASSWORD", "alice-pass");
    }
    let code = login_handler::run_login(
        Some(&env.config),
        LoginArgs {
            server: Some(server.base.clone()),
            username: Some("alice".to_string()),
            password_stdin: false,
            token_stdin: false,
        },
    )
    .await
    .expect("re-login");
    assert_eq!(code, 0);
    unsafe {
        std::env::remove_var("FILEHUB_PASSWORD");
    }
    let store = CredentialStore::open(&env.config).unwrap();
    assert!(store.current_credential(&server.base).is_some());
}

#[tokio::test]
async fn dv_no_credential_and_409_failure_workflows() {
    let _guard = common::lock_env();
    let payload = common::make_payload("fail");
    let server = MockServer::start(Mode::Normal, payload).await;
    let env = TestEnv::new();

    // 未登录发布：exit 2（认证失败，要求先 login）。
    let source = env.dir.path().join("file.bin");
    std::fs::write(&source, b"x").unwrap();
    let result = push_handler::run(
        Some(&env.config),
        PushArgs {
            path: source.clone(),
            target: format!("{}/demo/v1/app", server.base),
        },
    )
    .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().exit_code(), 2);

    // 登录后发布已存在版本：409 -> exit 4。
    login_with_env_password(&env, &server.base).await;
    let result = push_handler::run(
        Some(&env.config),
        PushArgs {
            path: source,
            target: format!("{}/demo/exists/app", server.base),
        },
    )
    .await;
    let error: CliError = result.expect_err("409 expected");
    assert_eq!(error.exit_code(), 4);
}

#[tokio::test]
async fn dv_unsafe_symlink_archive_is_rejected() {
    let env = TestEnv::new();
    let tree = env.dir.path().join("pkg");
    std::fs::create_dir_all(tree.join("data")).unwrap();
    std::fs::write(tree.join("data/ok.txt"), "ok").unwrap();
    let outside = env.dir.path().join("outside.txt");
    std::fs::write(&outside, "secret").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, tree.join("data/escape.txt")).unwrap();

    #[cfg(unix)]
    {
        let packed = archive::pack_tar_gz(&tree);
        let error = packed.expect_err("outside symlink must be rejected");
        assert!(matches!(error, archive::ArchiveError::Unsupported(_)));
        std::fs::remove_file(tree.join("data/escape.txt")).unwrap();
    }

    // 同树内符号链接允许。
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(tree.join("data/ok.txt"), tree.join("data/inside.txt")).unwrap();
        let packed = archive::pack_tar_gz(&tree).expect("inside symlink allowed");
        let content = std::fs::read(&packed.path).expect("archive bytes");
        assert!(content.len() > 0);
        let _ = std::fs::remove_file(&packed.path);
    }
}

#[tokio::test]
async fn dv_corrupt_config_is_not_overwritten() {
    let _guard = common::lock_env();
    let payload = common::make_payload("corrupt");
    let server = MockServer::start(Mode::Normal, payload).await;
    let env = TestEnv::new();
    std::fs::write(&env.config, "not: [valid toml").unwrap();

    unsafe {
        std::env::set_var("FILEHUB_PASSWORD", "alice-pass");
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
    let error = result.expect_err("corrupt config must fail");
    assert_eq!(error.exit_code(), 8);
    assert_eq!(
        std::fs::read_to_string(&env.config).unwrap(),
        "not: [valid toml",
        "损坏配置不得被覆盖"
    );
}
