//! CLI ↔ 真实 filehub-server 端到端集成测试：
//! 进程内装配真实服务端 + 子进程调用 filehub 二进制全命令流。

use filehub_server::account::store::connect_pool;
use filehub_server::http::{AppState, register_api};
use filehub_server::model::{HttpConfigSeed, ServerConfig, UserConfig, UsersConfig};
use sfo_http::actix_server::ActixHttpServer;
use sfo_http::http_server::HttpServerConfig;
use std::io::Write;
use std::net::TcpListener;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_filehub");

struct RunOutcome {
    code: i32,
    stdout: String,
    stderr: String,
}

fn log_case(name: &str) {
    println!("[integration] start {name}");
}

fn run_bin(config: &std::path::Path, args: &[&str], stdin_data: Option<&str>) -> RunOutcome {
    let mut cmd = Command::new(BIN);
    cmd.arg("--config").arg(config);
    cmd.args(args);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn filehub binary");
    match stdin_data {
        Some(data) => child
            .stdin
            .take()
            .expect("stdin")
            .write_all(data.as_bytes())
            .expect("write stdin"),
        None => drop(child.stdin.take()),
    }
    let output = child.wait_with_output().expect("wait filehub binary");
    RunOutcome {
        code: output.status.code().expect("exit code"),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

async fn start_real_server(dir: &std::path::Path) -> String {
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("bind")
        .local_addr()
        .expect("addr")
        .port();
    let config = ServerConfig {
        server: HttpConfigSeed {
            server_addr: "127.0.0.1".to_string(),
            port,
            allow_origins: vec![],
            allow_methods: vec![],
            allow_headers: vec![],
            expose_headers: vec![],
            max_age: 3600,
            support_credentials: false,
            login_rate_limit_per_minute: 30,
            login_rate_limit_window_secs: 60,
        },
        users: UsersConfig {
            users: vec![
                UserConfig {
                    username: "alice".to_string(),
                    password: Some("alice-pass".to_string()),
                    password_hash: None,
                },
                UserConfig {
                    username: "bob".to_string(),
                    password: Some("bob-pass".to_string()),
                    password_hash: None,
                },
            ],
            session_private_key: concat!(
                "-----BEGIN PRIVATE KEY-----\n",
                "MC4CAQAwBQYDK2VwBCIEIJGVLyTXHTLSMPclke6+1xCFTfX+TmVRcs6UNiMW35Ok\n",
                "-----END PRIVATE KEY-----\n"
            )
            .to_string(),
        },
        files: filehub_server::model::FilesConfig {
            data_dir: dir.join("files"),
            max_archive_bytes: 1024 * 1024,
        },
        db_path: dir.join("e2e.db").to_string_lossy().into_owned(),
    };
    std::fs::create_dir_all(&config.files.data_dir).expect("create files dir");
    let db = connect_pool(&config.db_path, 2).await.expect("pool");
    let state = AppState::assemble(&config, &db).await.expect("assemble");
    let base = format!("http://127.0.0.1:{port}");
    let server_config =
        HttpServerConfig::new(config.server.server_addr.clone(), config.server.port)
            .allow_any_origin()
            .allow_any_methods()
            .allow_any_header();
    let mut server = ActixHttpServer::new(server_config);
    register_api(&mut server, state).await;
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let _ = rt.block_on(server.run());
    });
    let client = reqwest::Client::new();
    for _ in 0..40 {
        if client
            .get(format!("{base}/api/v1/projects"))
            .send()
            .await
            .is_ok()
        {
            return base;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("real server did not become ready");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_full_workflow_against_real_server() {
    log_case("cli_full_workflow_against_real_server");
    let dir = tempfile::tempdir().expect("temp dir");
    let base = start_real_server(dir.path()).await;
    let identity = base
        .strip_prefix("http://")
        .expect("http base")
        .to_string();
    let config = dir.path().join("config.toml");
    let client = reqwest::Client::new();

    // 预置项目 demo（CLI 无建项目命令；项目名解析走 v1 列表/单查语义）。
    let login: serde_json::Value = client
        .post(format!("{base}/account/login"))
        .json(&serde_json::json!({
            "user_name": "alice",
            "password": "alice-pass",
            "timestamp": 1700000000u64,
        }))
        .send()
        .await
        .expect("login")
        .json()
        .await
        .expect("login body");
    let session = login["result"]["session"]
        .as_str()
        .expect("session")
        .to_string();
    let project = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", format!("Bearer {session}"))
        .json(&serde_json::json!({"name": "demo", "visibility": "private"}))
        .send()
        .await
        .expect("create project");
    assert_eq!(project.status(), reqwest::StatusCode::CREATED, "project created");

    // H1：login -> new-version -> push -> versions -> pull（SHA 闭环）。
    let login_cli = run_bin(
        &config,
        &[
            "login",
            &identity,
            "-u",
            "alice",
            "--password-stdin",
        ],
        Some("alice-pass\n"),
    );
    assert_eq!(login_cli.code, 0, "cli login");

    let new_version = run_bin(
        &config,
        &["new-version", &format!("{identity}/demo/v1")],
        None,
    );
    assert_eq!(new_version.code, 0, "new-version");

    let source = dir.path().join("release-notes.txt");
    std::fs::write(&source, "filehub e2e release").expect("write source");
    let push = run_bin(
        &config,
        &[
            "push",
            &format!("{identity}/demo/v1/app"),
            source.to_str().expect("path"),
        ],
        None,
    );
    assert_eq!(push.code, 0, "push app");
    assert!(push.stdout.contains("sha256 "), "push prints sha256");

    let versions_out = dir.path().join("versions.json");
    let versions = run_bin(
        &config,
        &[
            "versions",
            &format!("{identity}/demo"),
            "-o",
            versions_out.to_str().expect("path"),
            "--format",
            "json",
        ],
        None,
    );
    assert_eq!(versions.code, 0, "versions json");
    let records: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&versions_out).expect("versions file"))
            .expect("versions array");
    let v1 = records
        .as_array()
        .expect("array")
        .iter()
        .find(|item| item["version"] == "v1")
        .expect("v1 record");
    let server_sha = v1["apps"][0]["sha256"].as_str().expect("server sha").to_string();
    assert_eq!(server_sha.len(), 64, "server sha256 hex");

    let pulled = dir.path().join("pulled.tar.gz");
    let pull = run_bin(
        &config,
        &[
            "pull",
            &format!("{identity}/demo/v1/app"),
            pulled.to_str().expect("path"),
        ],
        None,
    );
    assert_eq!(pull.code, 0, "pull app");
    let downloaded = std::fs::read(&pulled).expect("pulled archive");
    let actual_sha = {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(&downloaded))
    };
    assert_eq!(actual_sha, server_sha, "downloaded sha matches server record");

    // lock 后写操作 409：退出码 4。
    let lock = run_bin(
        &config,
        &["lock-version", &format!("{identity}/demo/v1")],
        None,
    );
    assert_eq!(lock.code, 0, "lock-version");
    let locked_push = run_bin(
        &config,
        &[
            "push",
            &format!("{identity}/demo/v1/web"),
            source.to_str().expect("path"),
        ],
        None,
    );
    assert_eq!(locked_push.code, 4, "locked version push is conflict");

    // v2：发布、删除 app，删除后 pull 404 -> 5。
    let new_v2 = run_bin(
        &config,
        &["new-version", &format!("{identity}/demo/v2")],
        None,
    );
    assert_eq!(new_v2.code, 0, "new-version v2");
    let push_v2 = run_bin(
        &config,
        &[
            "push",
            &format!("{identity}/demo/v2/app2"),
            source.to_str().expect("path"),
        ],
        None,
    );
    assert_eq!(push_v2.code, 0, "push v2 app2");
    let delete_app = run_bin(
        &config,
        &["delete-app", &format!("{identity}/demo/v2/app2")],
        None,
    );
    assert_eq!(delete_app.code, 0, "delete-app");
    let after_delete = dir.path().join("gone.tar.gz");
    let pull_gone = run_bin(
        &config,
        &[
            "pull",
            &format!("{identity}/demo/v2/app2"),
            after_delete.to_str().expect("path"),
        ],
        None,
    );
    assert_eq!(pull_gone.code, 5, "deleted app pull is not found");

    // logout 后无凭据 → 认证失败 2。
    let logout = run_bin(&config, &["logout", &identity], None);
    assert_eq!(logout.code, 0, "logout");
    let after_logout = run_bin(
        &config,
        &["versions", &format!("{identity}/demo")],
        None,
    );
    assert_eq!(after_logout.code, 2, "no credential after logout");
}
