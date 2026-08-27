//! 真实 filehub 二进制进程级命令行集成测试：
//! argv 解析、stdin/env 凭据、退出码、凭据文件与 MockServer 契约。

mod common;

use common::{Mode, MockServer, TestEnv};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_filehub");

struct RunOutcome {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run_bin(config: Option<&Path>, args: &[&str], stdin_data: Option<&str>) -> RunOutcome {
    let mut cmd = Command::new(BIN);
    if let Some(cfg) = config {
        cmd.arg("--config").arg(cfg);
    }
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

fn assert_secret_free(out: &RunOutcome, secret: &str) {
    assert!(
        !out.stdout.contains(secret) && !out.stderr.contains(secret),
        "secret must not leak into command output"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_help_version_and_usage_exit_codes() {
    common::log_case("cmd_help_version_and_usage_exit_codes");
    // K1/K2：--help/-h/--version 为成功（冻结文档退出码 0）；无参/未知命令为用法错误 1。
    for flag in ["--help", "-h"] {
        let out = run_bin(None, &[flag], None);
        assert_eq!(out.code, 0, "{flag} exits 0");
        assert!(out.stdout.contains("Usage:"), "{flag} prints usage");
        assert!(out.stdout.contains("退出码"), "{flag} prints exit-code table");
    }
    let version = run_bin(None, &["--version"], None);
    assert_eq!(version.code, 0, "--version exits 0");
    assert!(version.stdout.contains("filehub 0.1.0"), "version output");

    let no_args = run_bin(None, &[], None);
    assert_eq!(no_args.code, 1, "no subcommand is usage error");
    assert!(no_args.stderr.contains("Usage:"), "usage printed");
    let unknown = run_bin(None, &["frobnicate"], None);
    assert_eq!(unknown.code, 1, "unknown command is usage error");
    let bad_flag = run_bin(None, &["login", "--definitely-not-an-option"], None);
    assert_eq!(bad_flag.code, 1, "unknown option is usage error");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_login_mode_conflicts_and_nonterminal_usage() {
    common::log_case("cmd_login_mode_conflicts_and_nonterminal_usage");
    // K3/K4/K5：模式互斥、非终端必须显式模式、空用户名/密码的稳定退出码。
    let _guard = common::lock_env();
    let server = start_mock(Mode::Normal, common::make_payload("usage")).await;

    let conflict = run_bin(
        None,
        &[
            "login",
            &server.identity(),
            "-u",
            "alice",
            "--password-stdin",
            "--token-stdin",
        ],
        Some("x\n"),
    );
    assert_eq!(conflict.code, 1, "mutual exclusion is usage error");

    let nonterminal = run_bin(None, &["login", &server.identity()], Some(""));
    assert_eq!(nonterminal.code, 1, "non-terminal stdin requires explicit mode");

    let empty_user = run_bin(
        None,
        &["login", &server.identity(), "-u", "", "--password-stdin"],
        Some("alice-pass\n"),
    );
    assert_eq!(empty_user.code, 1, "empty username rejected");

    let empty_password = run_bin(
        None,
        &["login", &server.identity(), "-u", "alice", "--password-stdin"],
        Some(""),
    );
    assert_eq!(empty_password.code, 2, "empty password is auth failure");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_password_stdin_login_persists_credential_and_logout_clears() {
    common::log_case("cmd_password_stdin_login_persists_credential_and_logout_clears");
    // K7/K8：stdin 密码登录持久化凭据（0600），logout 清除；明文不回显。
    let _guard = common::lock_env();
    let server = start_mock(Mode::Normal, common::make_payload("login")).await;
    let env = TestEnv::new();

    let out = run_bin(
        Some(&env.config),
        &[
            "login",
            &server.identity(),
            "-u",
            "alice",
            "--password-stdin",
        ],
        Some("alice-pass\n"),
    );
    assert_eq!(out.code, 0, "password stdin login");
    assert!(out.stdout.contains("Login Succeeded"), "success message");
    assert_secret_free(&out, "alice-pass");
    assert_secret_free(&out, "s1");

    let raw = std::fs::read_to_string(&env.config).expect("config read");
    assert!(raw.contains("session = \"s1\""), "session persisted");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&env.config)
            .expect("config metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "credential file is 0600");
    }

    let logout = run_bin(
        Some(&env.config),
        &["logout", &server.identity()],
        None,
    );
    assert_eq!(logout.code, 0, "logout");
    let after = std::fs::read_to_string(&env.config).expect("config after logout");
    assert!(
        !after.contains("session = \"s1\""),
        "logout clears session credential"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_token_stdin_and_env_token_login() {
    common::log_case("cmd_token_stdin_and_env_token_login");
    // K6/K7：--token-stdin 与 FILEHUB_TOKEN 通道；stdout/stderr 不含 token。
    let _guard = common::lock_env();
    let server = start_mock(Mode::Normal, common::make_payload("token")).await;
    let env = TestEnv::new();

    let out = run_bin(
        Some(&env.config),
        &["login", &server.identity(), "--token-stdin"],
        Some("tok-valid\n"),
    );
    assert_eq!(out.code, 0, "token stdin login");
    assert!(out.stdout.contains("Login Succeeded"), "token success");
    assert_secret_free(&out, "tok-valid");
    let raw = std::fs::read_to_string(&env.config).expect("config read");
    assert!(raw.contains("token = \"tok-valid\""), "token persisted");

    // FILEHUB_TOKEN 通道（无 FILEHUB_SERVER 时仍用显式 server 参数不冲突）。
    let env_token_env = TestEnv::new();
    unsafe {
        std::env::set_var("FILEHUB_TOKEN", "tok-valid");
    }
    let out2 = run_bin(
        Some(&env_token_env.config),
        &["login", &server.identity()],
        None,
    );
    unsafe {
        std::env::remove_var("FILEHUB_TOKEN");
    }
    assert_eq!(out2.code, 0, "FILEHUB_TOKEN login");
    let raw2 = std::fs::read_to_string(&env_token_env.config).expect("config read 2");
    assert!(raw2.contains("token = \"tok-valid\""), "env token persisted");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_corrupt_config_is_not_overwritten() {
    common::log_case("cmd_corrupt_config_is_not_overwritten");
    // K9：损坏配置退出 8 且原文件不变。
    let _guard = common::lock_env();
    let server = start_mock(Mode::Normal, common::make_payload("corrupt")).await;
    let env = TestEnv::new();
    let broken = "not: [valid toml";
    std::fs::write(&env.config, broken).expect("write broken config");

    let out = run_bin(
        Some(&env.config),
        &[
            "login",
            &server.identity(),
            "-u",
            "alice",
            "--password-stdin",
        ],
        Some("alice-pass\n"),
    );
    assert_eq!(out.code, 8, "corrupt config is local failure");
    assert_eq!(
        std::fs::read_to_string(&env.config).expect("config read"),
        broken,
        "corrupt config must not be overwritten"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_target_syntax_errors_are_invalid_input() {
    common::log_case("cmd_target_syntax_errors_are_invalid_input");
    // K10：缺段/空段/多余段 → 输入无效 5；IPv6 与协议头解析在 args 单测覆盖。
    let out = run_bin(None, &["versions", "127.0.0.1:1"], None);
    assert_eq!(out.code, 5, "missing project segment");
    assert!(out.stderr.contains("目标必须为"), "target error message");
    let extra = run_bin(None, &["versions", "h/demo/extra"], None);
    assert_eq!(extra.code, 5, "extra segment");
    let empty = run_bin(None, &["push", "h//v/app", "/tmp/x"], None);
    assert_eq!(empty.code, 5, "empty segment");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_network_failure_maps_to_transport_exit_6() {
    common::log_case("cmd_network_failure_maps_to_transport_exit_6");
    // K16：连接失败 → 6；需要先伪造已登录凭据以便请求真正发出。
    let env = TestEnv::new();
    std::fs::write(
        &env.config,
        "schema_version = 1\n\n[server.\"127.0.0.1:1\"]\nusername = \"alice\"\nsession = \"s1\"\nrefresh_session = \"r1\"\n",
    )
    .expect("write config");
    let out = run_bin(
        Some(&env.config),
        &["versions", "127.0.0.1:1/demo"],
        None,
    );
    assert_eq!(out.code, 6, "connection failure maps to transport error");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_session_refresh_once_and_token_401_no_refresh() {
    common::log_case("cmd_session_refresh_once_and_token_401_no_refresh");
    // K13：session 401 续期一次成功；token 401 不续期直接失败。
    let _guard = common::lock_env();
    let server = start_mock(Mode::ExpiresOnce, common::make_payload("refresh")).await;
    let env = TestEnv::new();
    let login = run_bin(
        Some(&env.config),
        &[
            "login",
            &server.identity(),
            "-u",
            "alice",
            "--password-stdin",
        ],
        Some("alice-pass\n"),
    );
    assert_eq!(login.code, 0, "session login");

    let out_file = env.dir.path().join("refresh.tar.gz");
    let pull = run_bin(
        Some(&env.config),
        &[
            "pull",
            &format!("{}/refresh-once/v1/default", server.identity()),
            out_file.to_str().expect("path"),
        ],
        None,
    );
    assert_eq!(pull.code, 0, "expired session refreshes once and succeeds");
    assert!(out_file.is_file(), "pulled archive exists");

    // token 401 不续期：写入无效 token 后拉取直接认证失败。
    let token_cfg = TestEnv::new();
    std::fs::write(
        &token_cfg.config,
        format!(
            "schema_version = 1\n\n[server.\"{}\"]\nusername = \"alice\"\ntoken = \"bad-token\"\n",
            server.identity()
        ),
    )
    .expect("write token config");
    let stale_out = token_cfg.dir.path().join("stale.tar.gz");
    let token_pull = run_bin(
        Some(&token_cfg.config),
        &[
            "pull",
            &format!("{}/demo/v1/default", server.identity()),
            stale_out.to_str().expect("path"),
        ],
        None,
    );
    assert_eq!(token_pull.code, 2, "invalid token 401 no refresh");
    assert!(!stale_out.exists(), "no file on auth failure");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_readonly_token_reads_but_cannot_write() {
    common::log_case("cmd_readonly_token_reads_but_cannot_write");
    // K14：只读 token 读操作 0、写操作 3。
    let _guard = common::lock_env();
    let server = start_mock(Mode::Normal, common::make_payload("readonly")).await;
    let env = TestEnv::new();
    unsafe {
        std::env::set_var("FILEHUB_TOKEN", "tok-view");
    }
    let login = run_bin(
        Some(&env.config),
        &["login", &server.identity()],
        None,
    );
    unsafe {
        std::env::remove_var("FILEHUB_TOKEN");
    }
    assert_eq!(login.code, 0, "token login");

    let out_file = env.dir.path().join("ro.tar.gz");
    let pull = run_bin(
        Some(&env.config),
        &[
            "pull",
            &format!("{}/demo/v1/default", server.identity()),
            out_file.to_str().expect("path"),
        ],
        None,
    );
    assert_eq!(pull.code, 0, "readonly token can pull");

    let source = env.dir.path().join("artifact.txt");
    std::fs::write(&source, "artifact").expect("write source");
    for target in [
        format!("{}/demo/new-version/app", server.identity()),
        format!("{}/demo/w1/app", server.identity()),
    ] {
        let push = run_bin(
            Some(&env.config),
            &["push", &target, source.to_str().expect("path")],
            None,
        );
        assert_eq!(push.code, 3, "readonly token write forbidden: {target}");
    }
    let lock = run_bin(
        Some(&env.config),
        &["lock-version", &format!("{}/demo/v1", server.identity())],
        None,
    );
    assert_eq!(lock.code, 3, "readonly token cannot lock");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_pull_corrupt_body_exit_7_and_directory_target_exit_5() {
    common::log_case("cmd_pull_corrupt_body_exit_7_and_directory_target_exit_5");
    // K19：SHA 校验失败 7、不落盘；目录输出目标 5。
    let _guard = common::lock_env();
    let server = start_mock(Mode::Normal, common::make_payload("integrity")).await;
    let env = TestEnv::new();
    let login = run_bin(
        Some(&env.config),
        &[
            "login",
            &server.identity(),
            "-u",
            "alice",
            "--password-stdin",
        ],
        Some("alice-pass\n"),
    );
    assert_eq!(login.code, 0, "login");

    let corrupt_out = env.dir.path().join("corrupt.tar.gz");
    let corrupt = run_bin(
        Some(&env.config),
        &[
            "pull",
            &format!("{}/demo/corrupt/default", server.identity()),
            corrupt_out.to_str().expect("path"),
        ],
        None,
    );
    assert_eq!(corrupt.code, 7, "sha mismatch is integrity failure");
    assert!(!corrupt_out.exists(), "integrity failure must not leave file");
    let leftovers: Vec<String> = std::fs::read_dir(&env.dir.path())
        .expect("read dir")
        .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(".tmp-") || name.ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "no temp files after failed pull");

    let dir_target = env.dir.path().join("dir-output");
    std::fs::create_dir_all(&dir_target).expect("create dir");
    let dir_pull = run_bin(
        Some(&env.config),
        &[
            "pull",
            &format!("{}/demo/v1/default", server.identity()),
            dir_target.to_str().expect("path"),
        ],
        None,
    );
    assert_eq!(dir_pull.code, 5, "directory output target rejected");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_push_unsafe_symlink_archive_exit_7() {
    common::log_case("cmd_push_unsafe_symlink_archive_exit_7");
    // K17：越界符号链接目录 push 属归档不安全 → 7。
    let _guard = common::lock_env();
    let server = start_mock(Mode::Normal, common::make_payload("unsafe")).await;
    let env = TestEnv::new();
    let login = run_bin(
        Some(&env.config),
        &[
            "login",
            &server.identity(),
            "-u",
            "alice",
            "--password-stdin",
        ],
        Some("alice-pass\n"),
    );
    assert_eq!(login.code, 0, "login");

    let tree = env.dir.path().join("pkg");
    std::fs::create_dir_all(tree.join("data")).expect("create tree");
    std::fs::write(tree.join("data/ok.txt"), "ok").expect("write file");
    let outside = env.dir.path().join("outside.txt");
    std::fs::write(&outside, "secret").expect("write outside");
    std::os::unix::fs::symlink(&outside, tree.join("data/escape.txt")).expect("symlink");

    let push = run_bin(
        Some(&env.config),
        &[
            "push",
            &format!("{}/demo/v1/app", server.identity()),
            tree.to_str().expect("path"),
        ],
        None,
    );
    assert_eq!(push.code, 7, "unsafe archive rejected before upload");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_push_conflict_and_not_found_exit_codes() {
    common::log_case("cmd_push_conflict_and_not_found_exit_codes");
    // K4/K15/K22：push 成功 0、409 冲突 4、404 输入无效 5。
    let _guard = common::lock_env();
    let server = start_mock(Mode::Normal, common::make_payload("push")).await;
    let env = TestEnv::new();
    let login = run_bin(
        Some(&env.config),
        &[
            "login",
            &server.identity(),
            "-u",
            "alice",
            "--password-stdin",
        ],
        Some("alice-pass\n"),
    );
    assert_eq!(login.code, 0, "login");
    let source = env.dir.path().join("artifact.txt");
    std::fs::write(&source, "artifact").expect("write source");

    let ok = run_bin(
        Some(&env.config),
        &[
            "push",
            &format!("{}/demo/new-version/server", server.identity()),
            source.to_str().expect("path"),
        ],
        None,
    );
    assert_eq!(ok.code, 0, "push success");

    let conflict = run_bin(
        Some(&env.config),
        &[
            "push",
            &format!("{}/demo/exists/app", server.identity()),
            source.to_str().expect("path"),
        ],
        None,
    );
    assert_eq!(conflict.code, 4, "409 maps to conflict");

    let missing = run_bin(
        Some(&env.config),
        &[
            "push",
            &format!("{}/demo/missing/app", server.identity()),
            source.to_str().expect("path"),
        ],
        None,
    );
    assert_eq!(missing.code, 5, "404 maps to invalid input");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_versions_output_json_and_text_formats() {
    common::log_case("cmd_versions_output_json_and_text_formats");
    // K20：text 固定表头；json 为 VersionDto 数组；stdout 直接输出。
    let _guard = common::lock_env();
    let server = start_mock(Mode::Normal, common::make_payload("versions")).await;
    let env = TestEnv::new();
    let login = run_bin(
        Some(&env.config),
        &[
            "login",
            &server.identity(),
            "-u",
            "alice",
            "--password-stdin",
        ],
        Some("alice-pass\n"),
    );
    assert_eq!(login.code, 0, "login");

    let json_out = env.dir.path().join("versions.json");
    let json = run_bin(
        Some(&env.config),
        &[
            "versions",
            &format!("{}/demo", server.identity()),
            "-o",
            json_out.to_str().expect("path"),
            "--format",
            "json",
        ],
        None,
    );
    assert_eq!(json.code, 0, "versions json");
    let records: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&json_out).expect("json file"))
            .expect("json array");
    assert!(records.is_array() && records.as_array().expect("array").len() == 2);

    let text = run_bin(
        Some(&env.config),
        &[
            "versions",
            &format!("{}/demo", server.identity()),
            "--format",
            "text",
        ],
        None,
    );
    assert_eq!(text.code, 0, "versions text to stdout");
    assert!(text.stdout.starts_with("VERSION\tPUBLISHED_AT"), "text header");
    assert!(text.stdout.contains("v1") && text.stdout.contains("v2"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_versions_without_credentials_exit_2_and_help_exit_0_are_locked_together() {
    common::log_case("cmd_versions_without_credentials_exit_2_and_help_exit_0_are_locked_together");
    // K12：未登录执行命令 → 认证失败 2（与 K1 的 help=0 一起形成退出码矩阵锚点）。
    let server = start_mock(Mode::Normal, common::make_payload("auth")).await;
    let env = TestEnv::new();
    let out = run_bin(
        Some(&env.config),
        &["versions", &format!("{}/demo", server.identity())],
        None,
    );
    assert_eq!(out.code, 2, "no credential requires login");
}

/// 启动 MockServer 并用测试运行时保持后台 accept 任务存活。
async fn start_mock(mode: Mode, payload: Vec<u8>) -> MockServer {
    MockServer::start(mode, payload).await
}
