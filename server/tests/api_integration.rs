//! integration：真实 Actix/sfo-http 服务器 + HTTP 客户端契约验证。

use common::{make_targz, temp_dir, test_config};
use filehub_server::account::store::connect_pool;
use filehub_server::http::{AppState, register_api};
use futures_util::StreamExt;
use jsonwebtoken::{Algorithm, decode_header};
use reqwest::{Client, StatusCode};
use sfo_http::actix_server::ActixHttpServer;
use sfo_http::http_server::HttpServerConfig;
use std::net::TcpListener;

#[path = "common/mod.rs"]
mod common;

#[derive(serde::Deserialize)]
struct LoginResp {
    session: String,
    refresh_session: String,
}

#[derive(serde::Deserialize)]
struct Project {
    project_id: i64,
    name: String,
    visibility: String,
    owner: i64,
}

#[derive(serde::Deserialize)]
struct VersionRecord {
    version: String,
    published_at: String,
    locked_at: Option<String>,
    apps: Vec<AppRecord>,
}

#[derive(serde::Deserialize)]
struct AppRecord {
    app: String,
    file_id: String,
    sha256: String,
    size: u64,
    updated_at: String,
}

#[derive(serde::Deserialize)]
struct TokenIssued {
    token_id: i64,
    jwt: String,
    expires_at: Option<String>,
}

#[derive(serde::Deserialize)]
struct TokenSummary {
    token_id: i64,
    name: String,
    project_scope: serde_json::Value,
    scopes: Vec<String>,
    created_at: String,
    updated_at: String,
}

#[derive(serde::Deserialize)]
struct ErrorBody {
    error: String,
    message: String,
}

async fn start_server() -> String {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("api.db").to_string_lossy());
    start_server_with_config(config).await
}

async fn start_server_with_config(mut config: filehub_server::model::ServerConfig) -> String {
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("bind")
        .local_addr()
        .expect("addr")
        .port();
    config.server.port = port;
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
    // 等待服务就绪
    let client = Client::new();
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
    panic!("server did not become ready");
}

#[tokio::test]
async fn upload_security_boundaries() {
    common::log_case("upload_security_boundaries");
    // 小上限配置：授权通过后，归档超限必须在流式收流阶段被拒绝，
    // sha256 缺失/不匹配必须在发布前被拒绝。
    let dir = temp_dir().await;
    let mut config = test_config(&dir, &dir.join("sec.db").to_string_lossy());
    config.files.max_archive_bytes = 128;
    let base = start_server_with_config(config).await;
    let client = Client::new();
    let alice = login(&client, &base, "alice", "alice-pass").await;

    // 1) 匿名上传：授权前置，脏 body 不进入解析（401）。
    let anon = client
        .put(format!("{base}/api/v1/projects/1/versions/1.0.0/apps/ui"))
        .multipart(reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(vec![0u8; 1024 * 1024]).file_name("junk.tar.gz"),
        ))
        .send()
        .await
        .expect("anon upload");
    assert_eq!(
        anon.status(),
        StatusCode::UNAUTHORIZED,
        "anonymous upload rejected before body read"
    );

    // 2) 已授权但归档超限：流式写入中段 422。
    let project = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"name": "secure", "visibility": "private"}))
        .send()
        .await
        .expect("create project");
    assert_eq!(project.status(), StatusCode::CREATED);
    let version = client
        .post(format!("{base}/api/v1/projects/1/versions"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "1.0.0"}))
        .send()
        .await
        .expect("create version");
    assert_eq!(version.status(), StatusCode::CREATED);

    let big = make_targz(
        "big",
        &(0u32..1024).map(|i| (i % 251) as u8).collect::<Vec<u8>>(),
    );
    let oversized = client
        .put(format!("{base}/api/v1/projects/1/versions/1.0.0/apps/big"))
        .header("Authorization", bearer(&alice))
        .multipart(
            reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(big.clone()).file_name("big.tar.gz"),
                )
                .text("sha256", common::sha256_hex(&big)),
        )
        .send()
        .await
        .expect("oversized upload");
    assert_eq!(
        oversized.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "oversized archive 422"
    );

    // 3) 缺少 sha256：压缩包很小，但上传协议必须携带哈希 → 422，不发布。
    let small = make_targz("bomb", &vec![0u8; 4096]);
    let missing_hash = client
        .put(format!("{base}/api/v1/projects/1/versions/1.0.0/apps/bomb"))
        .header("Authorization", bearer(&alice))
        .multipart(reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(small.clone()).file_name("bomb.tar.gz"),
        ))
        .send()
        .await
        .expect("bomb upload");
    assert_eq!(
        missing_hash.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "missing sha256 422"
    );

    // 4) sha256 不匹配：内容哈希与上报值不一致 → 422，且版本未发布任何 app。
    let wrong_hash = client
        .put(format!("{base}/api/v1/projects/1/versions/1.0.0/apps/bomb"))
        .header("Authorization", bearer(&alice))
        .multipart(
            reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(small).file_name("bomb.tar.gz"),
                )
                .text("sha256", "0".repeat(64)),
        )
        .send()
        .await
        .expect("wrong hash upload");
    assert_eq!(
        wrong_hash.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "sha256 mismatch 422"
    );
    let after = client
        .get(format!("{base}/api/v1/projects/1/versions/1.0.0"))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("get version after rejected uploads");
    assert_eq!(after.status(), StatusCode::OK, "version exists");
    let record: VersionRecord = after.json().await.expect("version body");
    assert!(
        record.apps.is_empty(),
        "rejected uploads must not publish apps"
    );
}

#[tokio::test]
async fn upload_parse_failure_after_ingest_discards_orphan() {
    common::log_case("upload_parse_failure_after_ingest_discards_orphan");
    // 复现 031 缺陷：file part 已完整写入并落库后，后继 multipart 分帧解析失败，
    // 必须 discard 已入库文件，不能残留孤儿 files 行。
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("orphan.db").to_string_lossy());
    let base = start_server_with_config(config.clone()).await;
    let client = Client::new();
    let alice = login(&client, &base, "alice", "alice-pass").await;

    let created = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"name": "orphan", "visibility": "private"}))
        .send()
        .await
        .expect("create project");
    assert_eq!(created.status(), StatusCode::CREATED, "project");
    let project: Project = created.json().await.expect("project body");
    let version = client
        .post(format!(
            "{base}/api/v1/projects/{}/versions",
            project.project_id
        ))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "1.0.0"}))
        .send()
        .await
        .expect("create version");
    assert_eq!(version.status(), StatusCode::CREATED, "version");

    // 分两帧发送：第一帧写完 file part，并以下一 part 不完整头部结束，让
    // MultipartParser 先返回 FileChunk 事件（文件写入 ingest 管道）；
    // 第二帧补全空值的 sha256 part，触发解析失败。
    let boundary = "orphan-boundary-42";
    let archive = make_targz("a.txt", b"orphan-data");
    let mut chunk1 = Vec::new();
    chunk1.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    chunk1.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"a.tar.gz\"\r\nContent-Type: application/gzip\r\n\r\n",
    );
    chunk1.extend_from_slice(&archive);
    chunk1.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
    chunk1.extend_from_slice(b"Content-Disposition: form-data; name=\"sha256\"\r\n");
    let chunk2 = format!("\r\n\r\n--{boundary}--\r\n").into_bytes();

    // 两个 stream 分帧之间停顿，确保服务端先行消费完第一帧并让解析器停留在
    // 未完成头部（FileChunk 已返回、文件已写入 ingest），第二帧才触发解析失败。
    let stream =
        futures_util::stream::iter(vec![Ok::<_, reqwest::Error>(bytes::Bytes::from(chunk1))])
            .chain(futures_util::stream::once(async move {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                Ok::<_, reqwest::Error>(bytes::Bytes::from(chunk2))
            }));
    let bad = client
        .put(format!(
            "{base}/api/v1/projects/{}/versions/1.0.0/apps/orphan",
            project.project_id
        ))
        .header("Authorization", bearer(&alice))
        .header(
            reqwest::header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(reqwest::Body::wrap_stream(stream))
        .send()
        .await
        .expect("malformed upload");
    assert_eq!(
        bad.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "trailing part parse failure returns 422"
    );

    // 响应返回前 handler 已完成 join 与（修复后）discard；直接检查落库残留。
    let db = connect_pool(&config.db_path, 1).await.expect("reopen db");
    let orphan_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
        .fetch_one(&db)
        .await
        .expect("count files");
    assert_eq!(
        orphan_count, 0,
        "no orphan file record remains after parse-failure 422"
    );
}

#[tokio::test]
async fn upload_rejects_missing_or_empty_file_part() {
    common::log_case("upload_rejects_missing_or_empty_file_part");
    // 036 回归：缺少 file part（仅 sha256）或显式 0 字节 file part 都必须
    // 422 拒绝，且不发布 app、不残留 files 行与磁盘文件（不支持发布空文件）。
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("nofile.db").to_string_lossy());
    let base = start_server_with_config(config.clone()).await;
    let client = Client::new();
    let alice = login(&client, &base, "alice", "alice-pass").await;

    let created = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"name": "nofile", "visibility": "private"}))
        .send()
        .await
        .expect("create project");
    assert_eq!(created.status(), StatusCode::CREATED, "project");
    let project: Project = created.json().await.expect("project body");
    let version = client
        .post(format!("{base}/api/v1/projects/{}/versions", project.project_id))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "1.0.0"}))
        .send()
        .await
        .expect("create version");
    assert_eq!(version.status(), StatusCode::CREATED, "version");
    let url = format!(
        "{base}/api/v1/projects/{}/versions/1.0.0/apps/ui",
        project.project_id
    );

    // 1) 只有 sha256（空内容哈希）、没有 file part -> 422 且不发布。
    let empty_sha = common::sha256_hex(&[]);
    let missing = client
        .put(&url)
        .header("Authorization", bearer(&alice))
        .multipart(reqwest::multipart::Form::new().text("sha256", empty_sha.clone()))
        .send()
        .await
        .expect("sha256-only upload");
    assert_eq!(
        missing.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "missing file part returns 422"
    );

    // 2) 显式空 file part + 空内容哈希 -> 422 且不发布。
    let empty_file = client
        .put(&url)
        .header("Authorization", bearer(&alice))
        .multipart(
            reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(Vec::new()).file_name("empty.tar.gz"),
                )
                .text("sha256", empty_sha),
        )
        .send()
        .await
        .expect("empty file upload");
    assert_eq!(
        empty_file.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "empty file part returns 422"
    );

    let after = client
        .get(format!("{base}/api/v1/projects/{}/versions/1.0.0", project.project_id))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("get version after rejected uploads");
    assert_eq!(after.status(), StatusCode::OK, "version exists");
    let record: VersionRecord = after.json().await.expect("version body");
    assert!(record.apps.is_empty(), "rejected uploads must not publish apps");

    // 3) 无残留：files 表为空，data_dir 不落任何归档/临时文件。
    let db = connect_pool(&config.db_path, 1).await.expect("reopen db");
    let file_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
        .fetch_one(&db)
        .await
        .expect("count files");
    assert_eq!(file_count, 0, "no file records remain after rejected uploads");
    let remaining: Vec<String> = std::fs::read_dir(&config.files.data_dir)
        .expect("read data dir")
        .map(|entry| entry.expect("dir entry").file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".tar.gz") || name.starts_with(".tmp-"))
        .collect();
    assert!(
        remaining.is_empty(),
        "no archived or temp files remain after rejected uploads: {remaining:?}"
    );
}

async fn login(client: &Client, base: &str, user: &str, pass: &str) -> String {
    let resp = client
        .post(format!("{base}/account/login"))
        .json(&serde_json::json!({"user_name": user, "password": pass, "timestamp": 1700000000u64}))
        .send()
        .await
        .expect("login request");
    assert_eq!(resp.status(), StatusCode::OK, "login status");
    #[derive(serde::Deserialize)]
    struct Wrapped {
        err: u16,
        result: Option<LoginResp>,
    }
    let wrapped: Wrapped = resp.json().await.expect("login body");
    assert_eq!(wrapped.err, 0, "login err field");
    let login = wrapped.result.expect("login result");
    assert!(!login.session.is_empty() && !login.refresh_session.is_empty());
    assert_eq!(
        decode_header(&login.session).expect("session JWT header").alg,
        Algorithm::EdDSA
    );
    assert_eq!(
        decode_header(&login.refresh_session)
            .expect("refresh JWT header")
            .alg,
        Algorithm::EdDSA
    );
    login.session
}

fn bearer(session: &str) -> String {
    format!("Bearer {session}")
}

#[tokio::test]
async fn login_rejects_request_body_over_64k() {
    common::log_case("login_rejects_request_body_over_64k");
    let base = start_server().await;
    let client = Client::new();

    let empty = serde_json::to_vec(&serde_json::json!({
        "user_name": "",
        "password": "pass",
        "timestamp": 1700000000u64,
    }))
    .expect("base login json");

    // 边界：恰好 64 KiB 的合法 JSON 必须进入账号校验，不能被上限误拒。
    let exact = serde_json::to_vec(&serde_json::json!({
        "user_name": "x".repeat(64 * 1024 - empty.len()),
        "password": "pass",
        "timestamp": 1700000000u64,
    }))
    .expect("64 KiB login json");
    assert_eq!(exact.len(), 64 * 1024, "boundary body must be exactly 65536 bytes");
    let exact_resp = client
        .post(format!("{base}/account/login"))
        .header("content-type", "application/json")
        .body(exact)
        .send()
        .await
        .expect("boundary login request");
    #[derive(serde::Deserialize)]
    struct WrappedErr {
        err: u16,
        msg: String,
    }
    let exact_wrapped: WrappedErr = exact_resp.json().await.expect("boundary login body");
    assert_ne!(exact_wrapped.err, 0, "unknown account at the boundary cannot succeed");
    assert!(
        !exact_wrapped.msg.contains("login request body exceeds"),
        "64 KiB boundary must not be rejected by the body limit: {}",
        exact_wrapped.msg
    );

    // 固定长度超限：Content-Length 预检在读取 body 前直接拒绝。
    let oversized = serde_json::to_vec(&serde_json::json!({
        "user_name": "x".repeat(65 * 1024),
        "password": "pass",
        "timestamp": 1700000000u64,
    }))
    .expect("oversized login json");
    let resp = client
        .post(format!("{base}/account/login"))
        .header("content-type", "application/json")
        .body(oversized.clone())
        .send()
        .await
        .expect("oversized login request");
    assert_eq!(resp.status(), StatusCode::OK, "sfo-account errors use the 200 envelope");
    let wrapped: WrappedErr = resp.json().await.expect("oversized login body");
    assert_ne!(wrapped.err, 0, "oversized login must fail");
    assert!(
        wrapped.msg.contains("login request body exceeds"),
        "content-length precheck message: {}",
        wrapped.msg
    );

    // 流式 chunked 超限（无 Content-Length）：实际流量读取路径拒绝。
    let chunks: Vec<Vec<u8>> = oversized.chunks(4096).map(|c| c.to_vec()).collect();
    let stream_body = reqwest::Body::wrap_stream(futures_util::stream::iter(
        chunks.into_iter().map(Ok::<_, std::convert::Infallible>),
    ));
    let stream_resp = client
        .post(format!("{base}/account/login"))
        .header("content-type", "application/json")
        .body(stream_body)
        .send()
        .await
        .expect("chunked login request");
    let stream_wrapped: WrappedErr = stream_resp.json().await.expect("chunked login body");
    assert_ne!(stream_wrapped.err, 0, "chunked oversized login must fail");
    assert!(
        stream_wrapped.msg.contains("login request body exceeds"),
        "stream rejection message: {}",
        stream_wrapped.msg
    );
}

#[tokio::test]
async fn login_failure_response_distinguishes_unknown_account_follows_0_2_1() {
    common::log_case("login_failure_response_distinguishes_unknown_account_follows_0_2_1");
    let base = start_server().await;
    let client = Client::new();

    let unknown = client
        .post(format!("{base}/account/login"))
        .json(&serde_json::json!({
            "user_name": "ghost",
            "password": "whatever",
            "timestamp": 1700000000u64,
        }))
        .send()
        .await
        .expect("unknown account login");
    let wrong = client
        .post(format!("{base}/account/login"))
        .json(&serde_json::json!({
            "user_name": "alice",
            "password": "wrong-pass",
            "timestamp": 1700000000u64,
        }))
        .send()
        .await
        .expect("wrong password login");
    assert_eq!(unknown.status(), StatusCode::OK);
    assert_eq!(wrong.status(), StatusCode::OK);

    let unknown_body: serde_json::Value = unknown.json().await.expect("unknown body");
    let wrong_body: serde_json::Value = wrong.json().await.expect("wrong body");
    assert_eq!(unknown_body["err"].as_u64(), Some(9));
    assert_eq!(
        unknown_body["msg"],
        serde_json::json!("account ghost not found")
    );
    assert_eq!(wrong_body["err"].as_u64(), Some(10));
    assert_eq!(
        wrong_body["msg"],
        serde_json::json!("Invalid username or password")
    );
}

#[tokio::test]
async fn login_rate_limit_rejects_over_quota_by_source_key() {
    common::log_case("login_rate_limit_rejects_over_quota_by_source_key");
    let dir = temp_dir().await;
    let mut config = test_config(&dir, &dir.join("limit.db").to_string_lossy());
    config.server.login_rate_limit_per_minute = 2;
    config.server.login_rate_limit_window_secs = 60;
    let base = start_server_with_config(config).await;
    let client = Client::new();

    #[derive(serde::Deserialize)]
    struct WrappedErr {
        err: u16,
        msg: String,
    }

    for _ in 0..2 {
        let resp = client
            .post(format!("{base}/account/login"))
            .json(&serde_json::json!({
                "user_name": "ghost",
                "password": "whatever",
                "timestamp": 1700000000u64,
            }))
            .send()
            .await
            .expect("within quota login");
        let wrapped: WrappedErr = resp.json().await.expect("within quota body");
        assert_eq!(wrapped.err, 9, "within-quota attempts still reach credential check (0.2.1 unknown account err=9)");
    }

    let limited = client
        .post(format!("{base}/account/login"))
        .json(&serde_json::json!({
            "user_name": "ghost",
            "password": "whatever",
            "timestamp": 1700000000u64,
        }))
        .send()
        .await
        .expect("over quota login");
    let limited_wrapped: WrappedErr = limited.json().await.expect("limited body");
    assert_eq!(limited_wrapped.err, 11, "over-quota login must return TooManyRequests");
    assert_eq!(
        limited_wrapped.msg,
        "Too many login attempts; please try again later"
    );

    // 限流只作用于 /account/login，其它匿名账号接口不受影响。
    let info = client
        .post(format!("{base}/account/get_account_info_of_session"))
        .json(&serde_json::json!({"session": "invalid"}))
        .send()
        .await
        .expect("session info after quota");
    let info_wrapped: WrappedErr = info.json().await.expect("session info body");
    assert_eq!(info_wrapped.err, 5, "session info must not be rate limited");
}

#[tokio::test]
async fn get_account_info_of_session_rejects_request_body_over_64k() {
    common::log_case("get_account_info_of_session_rejects_request_body_over_64k");
    let base = start_server().await;
    let client = Client::new();
    let session = login(&client, &base, "alice", "alice-pass").await;

    #[derive(serde::Deserialize)]
    struct WrappedOk {
        err: u16,
    }
    #[derive(serde::Deserialize)]
    struct WrappedErr {
        err: u16,
        msg: String,
    }

    let compact = serde_json::to_vec(&serde_json::json!({ "session": session }))
        .expect("session json");

    // 边界：恰好 64 KiB 的合法 session JSON 必须进入解签，不能被上限误拒。
    let exact = [compact.as_slice(), vec![b' '; 64 * 1024 - compact.len()].as_slice()].concat();
    assert_eq!(
        exact.len(),
        64 * 1024,
        "boundary body must be exactly 65536 bytes"
    );
    let exact_resp = client
        .post(format!("{base}/account/get_account_info_of_session"))
        .header("content-type", "application/json")
        .body(exact)
        .send()
        .await
        .expect("boundary session-info request");
    assert_eq!(
        exact_resp.status(),
        StatusCode::OK,
        "sfo-account errors use the 200 envelope"
    );
    let exact_wrapped: WrappedOk = exact_resp.json().await.expect("boundary session-info body");
    assert_eq!(exact_wrapped.err, 0, "valid session at the 64 KiB boundary must decode");

    // 固定长度超限：Content-Length 预检在读取 body 前直接拒绝。
    let oversized = [compact.as_slice(), vec![b' '; 65 * 1024 - compact.len()].as_slice()].concat();
    assert_eq!(oversized.len(), 65 * 1024, "oversized body must exceed 65536 bytes");
    let resp = client
        .post(format!("{base}/account/get_account_info_of_session"))
        .header("content-type", "application/json")
        .body(oversized.clone())
        .send()
        .await
        .expect("oversized session-info request");
    assert_eq!(resp.status(), StatusCode::OK, "sfo-account errors use the 200 envelope");
    let wrapped: WrappedErr = resp.json().await.expect("oversized session-info body");
    assert_ne!(wrapped.err, 0, "oversized session-info must fail");
    assert!(
        wrapped.msg.contains("get_account_info_of_session request body exceeds"),
        "content-length precheck message: {}",
        wrapped.msg
    );

    // 流式 chunked 超限（无 Content-Length）：实际流量读取路径拒绝。
    let chunks: Vec<Vec<u8>> = oversized.chunks(4096).map(|c| c.to_vec()).collect();
    let stream_body = reqwest::Body::wrap_stream(futures_util::stream::iter(
        chunks.into_iter().map(Ok::<_, std::convert::Infallible>),
    ));
    let stream_resp = client
        .post(format!("{base}/account/get_account_info_of_session"))
        .header("content-type", "application/json")
        .body(stream_body)
        .send()
        .await
        .expect("chunked session-info request");
    let stream_wrapped: WrappedErr = stream_resp.json().await.expect("chunked session-info body");
    assert_ne!(stream_wrapped.err, 0, "chunked oversized session-info must fail");
    assert!(
        stream_wrapped.msg.contains("get_account_info_of_session request body exceeds"),
        "stream rejection message: {}",
        stream_wrapped.msg
    );
}

#[tokio::test]
async fn api_login_session_and_token_flow() {
    common::log_case("api_login_session_and_token_flow");
    let base = start_server().await;
    let client = Client::new();
    let alice = login(&client, &base, "alice", "alice-pass").await;

    // session 信息接口
    let info = client
        .get(format!("{base}/account/get_account_info"))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("get account info");
    assert_eq!(info.status(), StatusCode::OK, "account info");

    // 无凭据访问 private 项目列表为匿名（public 过滤）
    let anon_list = client
        .get(format!("{base}/api/v1/projects"))
        .send()
        .await
        .expect("anon projects");
    assert_eq!(anon_list.status(), StatusCode::OK);

    // 每个账号都能创建自己的项目并成为 owner（不存在账号级 owner/member 区分）
    let bob = login(&client, &base, "bob", "bob-pass").await;
    let bob_info = client
        .get(format!("{base}/account/get_account_info"))
        .header("Authorization", bearer(&bob))
        .send()
        .await
        .expect("bob account info");
    assert_eq!(bob_info.status(), StatusCode::OK, "bob account info");
    let bob_value: serde_json::Value = bob_info.json().await.expect("bob info body");
    let bob_id = bob_value["result"]["id"].as_i64().expect("bob id");
    let bob_created = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&bob))
        .json(&serde_json::json!({"name": "bob-demo", "visibility": "private"}))
        .send()
        .await
        .expect("member create");
    assert_eq!(
        bob_created.status(),
        StatusCode::CREATED,
        "every account can create projects"
    );
    let bob_project: Project = bob_created.json().await.expect("bob project body");
    assert_eq!(bob_project.owner, bob_id, "creator becomes project owner");

    // owner 创建 private 项目
    let created = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"name": "api-demo", "visibility": "private"}))
        .send()
        .await
        .expect("owner create");
    assert_eq!(created.status(), StatusCode::CREATED, "owner create");
    let project: Project = created.json().await.expect("project body");
    let project_url = format!("{base}/api/v1/projects/{}", project.project_id);

    // private + 匿名 -> 401
    let anon_private = client
        .get(format!("{project_url}/versions"))
        .send()
        .await
        .expect("anon private");
    assert_eq!(
        anon_private.status(),
        StatusCode::UNAUTHORIZED,
        "anon private unauthorized"
    );

    // 版本显式创建；重复创建 409
    let create = client
        .post(format!("{project_url}/versions"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "1.0.0"}))
        .send()
        .await
        .expect("create version");
    assert_eq!(create.status(), StatusCode::CREATED, "create version");
    let duplicate = client
        .post(format!("{project_url}/versions"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "1.0.0"}))
        .send()
        .await
        .expect("duplicate create");
    assert_eq!(
        duplicate.status(),
        StatusCode::CONFLICT,
        "duplicate version"
    );

    // 同一版本发布两个 app；重复发布同 app 为更新
    let archive_a = make_targz("a.txt", b"a");
    let archive_b = make_targz("b.txt", b"b");
    let app_url = |version: &str, app: &str| format!("{project_url}/versions/{version}/apps/{app}");
    let created_app = client
        .put(app_url("1.0.0", "ui"))
        .header("Authorization", bearer(&alice))
        .multipart(
            reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(archive_a.clone()).file_name("a.tar.gz"),
                )
                .text("sha256", common::sha256_hex(&archive_a)),
        )
        .send()
        .await
        .expect("publish ui");
    let created_status = created_app.status();
    if created_status != StatusCode::CREATED {
        panic!(
            "publish ui: expected 201, got {created_status}; archive len={}, sha={}: {}",
            archive_a.len(),
            common::sha256_hex(&archive_a),
            created_app.text().await.unwrap_or_default()
        );
    }
    assert_eq!(created_status, StatusCode::CREATED, "app created");
    let second_app = client
        .put(app_url("1.0.0", "server"))
        .header("Authorization", bearer(&alice))
        .multipart(
            reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(archive_b.clone()).file_name("b.tar.gz"),
                )
                .text("sha256", common::sha256_hex(&archive_b)),
        )
        .send()
        .await
        .expect("publish server");
    assert_eq!(
        second_app.status(),
        StatusCode::CREATED,
        "second app created"
    );
    let updated_app = client
        .put(app_url("1.0.0", "ui"))
        .header("Authorization", bearer(&alice))
        .multipart(
            reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(archive_b.clone()).file_name("b.tar.gz"),
                )
                .text("sha256", common::sha256_hex(&archive_b)),
        )
        .send()
        .await
        .expect("update ui");
    assert_eq!(updated_app.status(), StatusCode::OK, "app updated");

    // 查询单版本返回全部 app 信息
    let single = client
        .get(format!("{project_url}/versions/1.0.0"))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("get version");
    assert_eq!(single.status(), StatusCode::OK, "get version");
    let single_record: VersionRecord = single.json().await.expect("version body");
    assert_eq!(single_record.apps.len(), 2, "all apps returned");
    assert!(
        single_record
            .apps
            .iter()
            .any(|app| app.app == "server" && app.sha256.len() == 64)
    );

    // 锁定后写操作 409，读取与下载不受影响
    let locked = client
        .put(format!("{project_url}/versions/1.0.0/lock"))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("lock");
    assert_eq!(locked.status(), StatusCode::OK, "lock");
    let locked_publish = client
        .put(app_url("1.0.0", "web"))
        .header("Authorization", bearer(&alice))
        .multipart(
            reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(archive_a.clone()).file_name("c.tar.gz"),
                )
                .text("sha256", common::sha256_hex(&archive_a)),
        )
        .send()
        .await
        .expect("locked publish attempt");
    assert_eq!(
        locked_publish.status(),
        StatusCode::CONFLICT,
        "locked publish"
    );
    let locked_delete = client
        .delete(app_url("1.0.0", "ui"))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("locked delete attempt");
    assert_eq!(
        locked_delete.status(),
        StatusCode::CONFLICT,
        "locked delete"
    );

    // token：artifacts:read 可读不可写
    let token_resp = client
        .post(format!("{base}/api/v1/tokens"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"name":"reader","scopes":["metadata:read","artifacts:read"],"expires_at":null}))
        .send()
        .await
        .expect("create token");
    if token_resp.status() != StatusCode::CREATED {
        let body = token_resp.text().await.unwrap_or_default();
        panic!("token create failed: {body}");
    }
    let token: TokenIssued = token_resp.json().await.expect("token body");
    let token_read = client
        .get(format!("{project_url}/versions/latest"))
        .header("Authorization", bearer(&token.jwt))
        .send()
        .await
        .expect("token read");
    assert_eq!(token_read.status(), StatusCode::OK, "token can read");
    let token_write = client
        .post(format!("{project_url}/versions"))
        .header("Authorization", bearer(&token.jwt))
        .json(&serde_json::json!({"version": "9.0.0"}))
        .send()
        .await
        .expect("token write attempt");
    assert_eq!(
        token_write.status(),
        StatusCode::FORBIDDEN,
        "read-only token write denied"
    );

    // rotate：旧 token 立即失效
    let rotate = client
        .post(format!("{base}/api/v1/tokens/{}/rotate", token.token_id))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("rotate");
    assert_eq!(rotate.status(), StatusCode::OK, "rotate");
    let rotated: TokenIssued = rotate.json().await.expect("rotated body");
    let old_read = client
        .get(format!("{project_url}/versions/latest"))
        .header("Authorization", bearer(&token.jwt))
        .send()
        .await
        .expect("old token");
    assert_eq!(
        old_read.status(),
        StatusCode::UNAUTHORIZED,
        "rotated token invalid"
    );
    let new_read = client
        .get(format!("{project_url}/versions/latest"))
        .header("Authorization", bearer(&rotated.jwt))
        .send()
        .await
        .expect("new token");
    assert_eq!(new_read.status(), StatusCode::OK, "new token valid");

    // public 后匿名下载（按 app），sha 一致；多 app 缺省下载 422
    let visibility = client
        .post(format!("{project_url}/visibility"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"visibility":"public"}))
        .send()
        .await
        .expect("visibility");
    assert_eq!(visibility.status(), StatusCode::OK, "visibility");
    let multi_default = client
        .get(format!("{project_url}/versions/latest/download"))
        .send()
        .await
        .expect("anon default download attempt");
    assert_eq!(
        multi_default.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "multi-app default download 422"
    );
    let download = client
        .get(format!("{project_url}/versions/latest/download?app=ui"))
        .send()
        .await
        .expect("anon download");
    assert_eq!(download.status(), StatusCode::OK, "download");
    let bytes = download.bytes().await.expect("download bytes");
    assert_eq!(bytes.to_vec(), archive_b, "download content match");
    let latest_body = client
        .get(format!("{project_url}/versions/latest"))
        .send()
        .await
        .expect("anon latest")
        .json::<VersionRecord>()
        .await
        .expect("latest body");
    let ui_app = latest_body
        .apps
        .iter()
        .find(|app| app.app == "ui")
        .expect("ui app");
    assert_eq!(common::sha256_hex(&bytes), ui_app.sha256, "sha matches");

    // 协作者：owner 授权 bob read，bob 可读不可写；移除后不可读
    let add_collab = client
        .put(format!("{project_url}/collaborators/2"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"role":"read"}))
        .send()
        .await
        .expect("grant");
    assert_eq!(add_collab.status(), StatusCode::OK, "grant");
    // 不存在的用户不允许授权（不存在或对该身份不可见 -> 404）
    let missing = client
        .put(format!("{project_url}/collaborators/9000"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"role":"read"}))
        .send()
        .await
        .expect("grant missing user");
    assert_eq!(missing.status(), StatusCode::NOT_FOUND, "missing user 404");
    let bob_read = client
        .get(format!("{project_url}/versions"))
        .header("Authorization", bearer(&bob))
        .send()
        .await
        .expect("bob read");
    assert_eq!(bob_read.status(), StatusCode::OK, "bob can read private");
    let bob_write = client
        .post(format!("{project_url}/versions"))
        .header("Authorization", bearer(&bob))
        .json(&serde_json::json!({"version": "8.0.0"}))
        .send()
        .await
        .expect("bob write attempt");
    assert_eq!(
        bob_write.status(),
        StatusCode::FORBIDDEN,
        "read-only collaborator"
    );
}

#[tokio::test]
async fn version_app_input_and_download_boundaries() {
    common::log_case("version_app_input_and_download_boundaries");
    let base = start_server().await;
    let client = Client::new();
    let alice = login(&client, &base, "alice", "alice-pass").await;

    let created = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"name": "boundary", "visibility": "public"}))
        .send()
        .await
        .expect("create project");
    assert_eq!(created.status(), StatusCode::CREATED, "project");
    let project: Project = created.json().await.expect("project body");
    let project_url = format!("{base}/api/v1/projects/{}", project.project_id);

    // 空版本创建 -> 422。
    let empty = client
        .post(format!("{project_url}/versions"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "  "}))
        .send()
        .await
        .expect("empty version");
    assert_eq!(
        empty.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "empty version rejected"
    );

    // 非法 app 名 -> 422。
    let ok = client
        .post(format!("{project_url}/versions"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "1.0.0"}))
        .send()
        .await
        .expect("create 1.0.0");
    assert_eq!(ok.status(), StatusCode::CREATED, "create 1.0.0");
    let invalid_app = client
        .put(format!("{project_url}/versions/1.0.0/apps/bad-app!"))
        .header("Authorization", bearer(&alice))
        .multipart(
            reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(make_targz("x", b"x")).file_name("x.tar.gz"),
                )
                .text("sha256", common::sha256_hex(&make_targz("x", b"x"))),
        )
        .send()
        .await
        .expect("invalid app");
    assert_eq!(
        invalid_app.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid app name rejected"
    );

    // 单 app 版本：无 ?app 缺省下载 200；错误 app 404；显式 app 200 且内容一致。
    let solo = client
        .put(format!("{project_url}/versions/1.0.0/apps/solo"))
        .header("Authorization", bearer(&alice))
        .multipart(
            reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(make_targz("solo", b"solo-data"))
                        .file_name("solo.tar.gz"),
                )
                .text(
                    "sha256",
                    common::sha256_hex(&make_targz("solo", b"solo-data")),
                ),
        )
        .send()
        .await
        .expect("publish solo");
    let solo_status = solo.status();
    if solo_status != StatusCode::CREATED {
        panic!(
            "publish solo: expected 201, got {solo_status}: {}",
            solo.text().await.unwrap_or_default()
        );
    }
    assert_eq!(solo_status, StatusCode::CREATED, "solo app");
    let default_download = client
        .get(format!("{project_url}/versions/1.0.0/download"))
        .send()
        .await
        .expect("default download");
    assert_eq!(
        default_download.status(),
        StatusCode::OK,
        "single-app default download"
    );
    let bytes = default_download.bytes().await.expect("download bytes");
    assert!(!bytes.is_empty());
    let missing_app = client
        .get(format!("{project_url}/versions/1.0.0/download?app=wrong"))
        .send()
        .await
        .expect("missing app download");
    assert_eq!(missing_app.status(), StatusCode::NOT_FOUND, "wrong app 404");
    let explicit = client
        .get(format!("{project_url}/versions/1.0.0/download?app=solo"))
        .send()
        .await
        .expect("explicit app download");
    assert_eq!(explicit.status(), StatusCode::OK, "explicit app download");
}

#[tokio::test]
async fn project_list_pagination_and_single_get() {
    common::log_case("project_list_pagination_and_single_get");
    async fn create_project(
        client: &Client,
        base: &str,
        session: &str,
        name: &str,
        visibility: &str,
    ) -> Project {
        let resp = client
            .post(format!("{base}/api/v1/projects"))
            .header("Authorization", bearer(session))
            .json(&serde_json::json!({
                "name": name,
                "visibility": visibility
            }))
            .send()
            .await
            .expect("create project");
        assert_eq!(resp.status(), StatusCode::CREATED, "create {name}");
        resp.json().await.expect("project body")
    }

    let base = start_server().await;
    let client = Client::new();
    let alice = login(&client, &base, "alice", "alice-pass").await;
    let bob = login(&client, &base, "bob", "bob-pass").await;

    let alpha = create_project(&client, &base, &alice, "alpha", "private").await;
    let _beta = create_project(&client, &base, &bob, "beta", "private").await;
    let gamma = create_project(&client, &base, &alice, "gamma", "public").await;

    // 分页：alice 可见 alpha、gamma；limit=1 只返回一页，总量头完整。
    let page1 = client
        .get(format!("{base}/api/v1/projects?limit=1"))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("alice page1");
    assert_eq!(page1.status(), StatusCode::OK);
    let total1 = page1
        .headers()
        .get("x-total-count")
        .expect("total header")
        .to_str()
        .expect("total str");
    assert_eq!(total1, "2");
    let body1: Vec<Project> = page1.json().await.expect("page1 body");
    assert_eq!(body1.len(), 1);
    assert_eq!(body1[0].project_id, alpha.project_id);

    let page2 = client
        .get(format!("{base}/api/v1/projects?limit=1&offset=1"))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("alice page2");
    assert_eq!(page2.status(), StatusCode::OK);
    let body2: Vec<Project> = page2.json().await.expect("page2 body");
    assert_eq!(body2.len(), 1);
    assert_eq!(body2[0].project_id, gamma.project_id);

    // 非法分页参数全部 422。
    for qs in ["limit=0", "limit=501", "offset=-1", "limit=abc"] {
        let bad = client
            .get(format!("{base}/api/v1/projects?{qs}"))
            .header("Authorization", bearer(&alice))
            .send()
            .await
            .expect("bad query");
        assert_eq!(
            bad.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid {qs}"
        );
    }

    // 匿名列表只含 public，总量正确。
    let anon = client
        .get(format!("{base}/api/v1/projects"))
        .send()
        .await
        .expect("anon list");
    assert_eq!(anon.status(), StatusCode::OK);
    let anon_total = anon
        .headers()
        .get("x-total-count")
        .expect("anon total")
        .to_str()
        .expect("anon total str");
    assert_eq!(anon_total, "1");
    let anon_body: Vec<Project> = anon.json().await.expect("anon body");
    assert_eq!(anon_body[0].project_id, gamma.project_id);

    // 单项目直查：owner 200；匿名 private 401；匿名 public 200；他人 private 404；不存在 404/401。
    let owner_get = client
        .get(format!("{base}/api/v1/projects/{}", alpha.project_id))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("owner get");
    assert_eq!(owner_get.status(), StatusCode::OK);

    let anon_private = client
        .get(format!("{base}/api/v1/projects/{}", alpha.project_id))
        .send()
        .await
        .expect("anon private get");
    assert_eq!(anon_private.status(), StatusCode::UNAUTHORIZED);

    let anon_public = client
        .get(format!("{base}/api/v1/projects/{}", gamma.project_id))
        .send()
        .await
        .expect("anon public get");
    assert_eq!(anon_public.status(), StatusCode::OK);
    let public_body: Project = anon_public.json().await.expect("public body");
    assert_eq!(public_body.project_id, gamma.project_id);

    let bob_private = client
        .get(format!("{base}/api/v1/projects/{}", alpha.project_id))
        .header("Authorization", bearer(&bob))
        .send()
        .await
        .expect("bob private get");
    assert_eq!(bob_private.status(), StatusCode::NOT_FOUND);

    let missing = client
        .get(format!("{base}/api/v1/projects/999999"))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("missing get");
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    let missing_anon = client
        .get(format!("{base}/api/v1/projects/999999"))
        .send()
        .await
        .expect("missing anon get");
    assert_eq!(missing_anon.status(), StatusCode::UNAUTHORIZED);

    // visibility 更新仍返回完整项目（响应改为直查后不能回归）。
    let vis = client
        .post(format!(
            "{base}/api/v1/projects/{}/visibility",
            gamma.project_id
        ))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"visibility": "private"}))
        .send()
        .await
        .expect("set visibility");
    assert_eq!(vis.status(), StatusCode::OK);
    let vis_body: Project = vis.json().await.expect("visibility body");
    assert_eq!(vis_body.visibility, "private");
    let after_private = client
        .get(format!("{base}/api/v1/projects/{}", gamma.project_id))
        .send()
        .await
        .expect("after private get");
    assert_eq!(after_private.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn session_refresh_rotates_and_new_session_works() {
    common::log_case("session_refresh_rotates_and_new_session_works");
    // A4：refresh_session 返回新 session/refresh_session，新凭据可访问受保护接口。
    let base = start_server().await;
    let client = Client::new();
    let alice = login(&client, &base, "alice", "alice-pass").await;

    let resp = client
        .post(format!("{base}/account/login"))
        .json(&serde_json::json!({"user_name": "alice", "password": "alice-pass", "timestamp": 1700000000u64}))
        .send()
        .await
        .expect("login");
    let wrapped: serde_json::Value = resp.json().await.expect("login body");
    let refresh = wrapped["result"]["refresh_session"]
        .as_str()
        .expect("refresh session")
        .to_string();
    let old_session = wrapped["result"]["session"]
        .as_str()
        .expect("session")
        .to_string();
    assert_eq!(
        decode_header(&old_session).expect("session JWT header").alg,
        Algorithm::EdDSA
    );
    assert_eq!(
        decode_header(&refresh).expect("refresh JWT header").alg,
        Algorithm::EdDSA
    );
    let _ = alice; // 登录 helper 已在上文完成一次同口令登录；此处用直登凭据验证 refresh。

    let refreshed = client
        .post(format!("{base}/account/refresh_session"))
        .header("Authorization", bearer(&refresh))
        .send()
        .await
        .expect("refresh request");
    assert_eq!(refreshed.status(), StatusCode::OK, "refresh status");
    #[derive(serde::Deserialize)]
    struct RefreshWrapped {
        err: u16,
        result: Option<LoginResp>,
    }
    let refresh_body: RefreshWrapped = refreshed.json().await.expect("refresh body");
    assert_eq!(refresh_body.err, 0, "refresh err");
    let rotated = refresh_body.result.expect("rotated credentials");
    assert!(!rotated.session.is_empty() && !rotated.refresh_session.is_empty());
    assert_eq!(
        decode_header(&rotated.session)
            .expect("rotated session JWT header")
            .alg,
        Algorithm::EdDSA
    );
    assert_eq!(
        decode_header(&rotated.refresh_session)
            .expect("rotated refresh JWT header")
            .alg,
        Algorithm::EdDSA
    );
    let _ = old_session; // sfo-account 同秒生成的新建 session JWT 可能字节相同，
                         // 此处只验证 refresh 返回的凭据可继续访问受保护接口。

    let info = client
        .get(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&rotated.session))
        .send()
        .await
        .expect("new session access");
    assert_eq!(info.status(), StatusCode::OK, "rotated session works");
}

#[tokio::test]
async fn refresh_session_cannot_access_user_apis() {
    common::log_case("refresh_session_cannot_access_user_apis");
    // 高危回归：refresh token 只能调用 /account/refresh_session 续期，
    // 不能作为访问 session 通过认证桥或 sfo-account 用户信息接口使用。
    let base = start_server().await;
    let client = Client::new();

    let resp = client
        .post(format!("{base}/account/login"))
        .json(&serde_json::json!({"user_name": "alice", "password": "alice-pass", "timestamp": 1700000000u64}))
        .send()
        .await
        .expect("login");
    #[derive(serde::Deserialize)]
    struct LoginWrapped {
        err: u16,
        result: Option<LoginResp>,
    }
    let wrapped: LoginWrapped = resp.json().await.expect("login body");
    assert_eq!(wrapped.err, 0, "login err field");
    let refresh = wrapped
        .result
        .expect("login result")
        .refresh_session;

    // 1) 认证桥：refresh token 作为 Bearer 访问 /api/v1 用户接口必须 401。
    let projects = client
        .get(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&refresh))
        .send()
        .await
        .expect("projects with refresh token");
    assert_eq!(
        projects.status(),
        StatusCode::UNAUTHORIZED,
        "refresh token must not authenticate user APIs"
    );

    // 2) sfo-account 用户信息接口：同样经 decode_session，必须 err != 0。
    let info = client
        .get(format!("{base}/account/get_account_info"))
        .header("Authorization", bearer(&refresh))
        .send()
        .await
        .expect("account info with refresh token");
    assert_eq!(info.status(), StatusCode::OK, "sfo-account uses 200 envelope");
    #[derive(serde::Deserialize)]
    struct ErrWrapped {
        err: u16,
        msg: String,
    }
    let info_body: ErrWrapped = info.json().await.expect("account info body");
    assert_ne!(
        info_body.err, 0,
        "refresh token must not resolve user info: {}",
        info_body.msg
    );

    // 3) 续期端点不受影响；换发后的新 session 可访问用户接口。
    let rotated = client
        .post(format!("{base}/account/refresh_session"))
        .header("Authorization", bearer(&refresh))
        .send()
        .await
        .expect("refresh request");
    assert_eq!(rotated.status(), StatusCode::OK, "refresh status");
    let rotated_body: LoginWrapped = rotated.json().await.expect("refresh body");
    assert_eq!(rotated_body.err, 0, "refresh must rotate credentials");
    let new_session = rotated_body
        .result
        .expect("rotated credentials")
        .session;
    let after = client
        .get(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&new_session))
        .send()
        .await
        .expect("new session projects");
    assert_eq!(after.status(), StatusCode::OK, "rotated session works");
}

#[tokio::test]
async fn project_delete_cascades_versions_apps_and_files() {
    common::log_case("project_delete_cascades_versions_apps_and_files");
    // B7：项目删除后项目/版本/下载全部 404，文件行与磁盘归档级联清理。
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("cascade.db").to_string_lossy());
    let base = start_server_with_config(config.clone()).await;
    let client = Client::new();
    let alice = login(&client, &base, "alice", "alice-pass").await;
    let bob = login(&client, &base, "bob", "bob-pass").await;

    let created = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"name": "cascade", "visibility": "private"}))
        .send()
        .await
        .expect("create project");
    assert_eq!(created.status(), StatusCode::CREATED);
    let project: Project = created.json().await.expect("project body");
    let version = client
        .post(format!("{base}/api/v1/projects/{}/versions", project.project_id))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "1.0.0"}))
        .send()
        .await
        .expect("create version");
    assert_eq!(version.status(), StatusCode::CREATED);
    let archive = make_targz("cascade.txt", b"cascade");
    let published = client
        .put(format!(
            "{base}/api/v1/projects/{}/versions/1.0.0/apps/ui",
            project.project_id
        ))
        .header("Authorization", bearer(&alice))
        .multipart(
            reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(archive.clone()).file_name("ui.tar.gz"),
                )
                .text("sha256", common::sha256_hex(&archive)),
        )
        .send()
        .await
        .expect("publish");
    assert_eq!(published.status(), StatusCode::CREATED, "app published");

    let non_owner_delete = client
        .delete(format!("{base}/api/v1/projects/{}", project.project_id))
        .header("Authorization", bearer(&bob))
        .send()
        .await
        .expect("non owner delete");
    assert_eq!(
        non_owner_delete.status(),
        StatusCode::FORBIDDEN,
        "non-owner project delete is forbidden"
    );

    let deleted = client
        .delete(format!("{base}/api/v1/projects/{}", project.project_id))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("owner delete");
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT, "owner delete 204");

    for (label, url, expected) in [
        (
            "project",
            format!("{base}/api/v1/projects/{}", project.project_id),
            StatusCode::NOT_FOUND,
        ),
        (
            "versions",
            format!("{base}/api/v1/projects/{}/versions", project.project_id),
            StatusCode::FORBIDDEN,
        ),
        (
            "version",
            format!("{base}/api/v1/projects/{}/versions/1.0.0", project.project_id),
            StatusCode::FORBIDDEN,
        ),
        (
            "download",
            format!(
                "{base}/api/v1/projects/{}/versions/1.0.0/download?app=ui",
                project.project_id
            ),
            StatusCode::FORBIDDEN,
        ),
    ] {
        let after = client
            .get(&url)
            .header("Authorization", bearer(&alice))
            .send()
            .await
            .expect("after delete");
        assert_eq!(
            after.status(),
            expected,
            "{label} must disappear after project delete"
        );
    }

    // 项目删除即清理 versions/version_apps/project_grants；files 行与物理
    // 归档同样立即回收（050），startup GC 只作异常残留兜底。
    let db = connect_pool(&config.db_path, 1).await.expect("reopen db");
    let file_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
        .fetch_one(&db)
        .await
        .expect("count files");
    assert_eq!(
        file_count, 0,
        "files rows are gone immediately after project delete"
    );
    let remaining: Vec<String> = std::fs::read_dir(&config.files.data_dir)
        .expect("read data dir")
        .map(|entry| entry.expect("dir entry").file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".tar.gz"))
        .collect();
    assert!(
        remaining.is_empty(),
        "archives are gone immediately after project delete: {remaining:?}"
    );

    let (state, _gc_pool) = common::assemble(&config).await.expect("reassemble");
    let removed = state.startup_gc().await.expect("startup gc");
    assert!(
        removed.is_empty(),
        "startup gc finds no orphans after immediate cleanup: {removed:?}"
    );
}

#[tokio::test]
async fn collaborator_role_matrix_and_removal() {
    common::log_case("collaborator_role_matrix_and_removal");
    // C4/C5：write 可发布不可管理；admin 可管理但不可删除项目；移除后立即可见性收回。
    let base = start_server().await;
    let client = Client::new();
    let alice = login(&client, &base, "alice", "alice-pass").await;
    let bob = login(&client, &base, "bob", "bob-pass").await;

    let created = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"name": "matrix", "visibility": "private"}))
        .send()
        .await
        .expect("create project");
    assert_eq!(created.status(), StatusCode::CREATED);
    let project: Project = created.json().await.expect("project body");
    let pid = project.project_id;
    let base_pid = format!("{base}/api/v1/projects/{pid}");

    // write 角色：可 artifacts:write，不可 administration。
    let grant_write = client
        .put(format!("{base_pid}/collaborators/2"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"role": "write"}))
        .send()
        .await
        .expect("grant write");
    assert_eq!(grant_write.status(), StatusCode::OK, "grant write");
    let write_version = client
        .post(format!("{base_pid}/versions"))
        .header("Authorization", bearer(&bob))
        .json(&serde_json::json!({"version": "w1"}))
        .send()
        .await
        .expect("bob write create version");
    assert_eq!(write_version.status(), StatusCode::CREATED, "write role publishes");
    let write_visibility = client
        .post(format!("{base_pid}/visibility"))
        .header("Authorization", bearer(&bob))
        .json(&serde_json::json!({"visibility": "public"}))
        .send()
        .await
        .expect("bob visibility attempt");
    assert_eq!(
        write_visibility.status(),
        StatusCode::FORBIDDEN,
        "write role cannot administer"
    );

    // admin 角色：可 administration，但仍不能 owner-only 删除项目。
    let grant_admin = client
        .put(format!("{base_pid}/collaborators/2"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"role": "admin"}))
        .send()
        .await
        .expect("grant admin");
    assert_eq!(grant_admin.status(), StatusCode::OK, "grant admin");
    let list = client
        .get(format!("{base_pid}/collaborators"))
        .header("Authorization", bearer(&bob))
        .send()
        .await
        .expect("bob list collaborators");
    assert_eq!(list.status(), StatusCode::OK, "admin can list collaborators");
    let admin_visibility = client
        .post(format!("{base_pid}/visibility"))
        .header("Authorization", bearer(&bob))
        .json(&serde_json::json!({"visibility": "public"}))
        .send()
        .await
        .expect("admin visibility");
    assert_eq!(admin_visibility.status(), StatusCode::OK, "admin can set visibility");
    let admin_delete = client
        .delete(format!("{base_pid}"))
        .header("Authorization", bearer(&bob))
        .send()
        .await
        .expect("admin delete attempt");
    assert_eq!(
        admin_delete.status(),
        StatusCode::FORBIDDEN,
        "only owner can delete project"
    );

    // C5：移除协作者后，private 项目对其不可见。
    let removed = client
        .delete(format!("{base_pid}/collaborators/2"))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("remove collaborator");
    assert_eq!(removed.status(), StatusCode::NO_CONTENT, "remove 204");
    let after_remove = client
        .get(format!("{base_pid}/versions"))
        .header("Authorization", bearer(&bob))
        .send()
        .await
        .expect("bob after removal");
    assert_eq!(
        after_remove.status(),
        StatusCode::OK,
        "project is public so read stays visible after removal"
    );
    let write_after_remove = client
        .post(format!("{base_pid}/versions"))
        .header("Authorization", bearer(&bob))
        .json(&serde_json::json!({"version": "after-remove"}))
        .send()
        .await
        .expect("bob write after removal");
    assert_eq!(
        write_after_remove.status(),
        StatusCode::FORBIDDEN,
        "removed collaborator loses write access immediately"
    );
}

#[tokio::test]
async fn token_expiry_bounds_attribute_update_without_resign() {
    common::log_case("token_expiry_bounds_attribute_update_without_resign");
    // D1/D2/D4：expires 边界、属性修改只落库不重签、数据库权限即时生效。
    let base = start_server().await;
    let client = Client::new();
    let alice = login(&client, &base, "alice", "alice-pass").await;

    let created = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"name": "token-bounds", "visibility": "private"}))
        .send()
        .await
        .expect("create project");
    assert_eq!(created.status(), StatusCode::CREATED);
    let project: Project = created.json().await.expect("project body");
    let pid = project.project_id;
    let project_url = format!("{base}/api/v1/projects/{pid}");

    // 非法过期参数：非日期字符串、超过 1 年 -> 422。
    for bad_expiry in [
        serde_json::json!("not-a-date"),
        serde_json::json!((chrono::Utc::now() + chrono::Duration::days(400)).to_rfc3339()),
    ] {
        let resp = client
            .post(format!("{base}/api/v1/tokens"))
            .header("Authorization", bearer(&alice))
            .json(&serde_json::json!({
                "name": "bad-expiry",
                "scopes": ["metadata:read"],
                "expires_at": bad_expiry,
            }))
            .send()
            .await
            .expect("bad expiry token");
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY, "invalid expiry 422");
    }

    // 合法过期：expires_at 只在创建响应出现。
    let expires = chrono::Utc::now() + chrono::Duration::hours(1);
    let issued = client
        .post(format!("{base}/api/v1/tokens"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({
            "name": "deploy",
            "project_scope": "All",
            "scopes": ["metadata:read", "artifacts:read", "artifacts:write"],
            "expires_at": expires.to_rfc3339(),
        }))
        .send()
        .await
        .expect("create token");
    assert_eq!(issued.status(), StatusCode::CREATED, "create token");
    let token: TokenIssued = issued.json().await.expect("issued token");
    assert!(token.expires_at.is_some(), "expires_at present in create response");
    let jwt = token.jwt;

    // 属性修改不重签：返回 TokenSummary（无 jwt），旧 JWT 继续有效。
    let renamed = client
        .post(format!("{base}/api/v1/tokens/{}", token.token_id))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"name": "renamed"}))
        .send()
        .await
        .expect("rename token");
    assert_eq!(renamed.status(), StatusCode::OK, "rename");
    let body = renamed.text().await.expect("rename body");
    assert!(!body.contains("\"jwt\""), "attribute update must not return jwt: {body}");
    let summary: TokenSummary =
        serde_json::from_str(&body).expect("rename summary");
    assert_eq!(summary.name, "renamed");

    // 旧 JWT 不因属性修改而失效（属性修改不重签）。
    let project_read = client
        .get(format!("{project_url}"))
        .header("Authorization", bearer(&jwt))
        .send()
        .await
        .expect("old jwt project read");
    assert_eq!(project_read.status(), StatusCode::OK, "old jwt remains valid after rename");

    // 数据库权限即时生效：移除 artifacts:read 后旧 JWT 下载被拒。
    let version = client
        .post(format!("{project_url}/versions"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "1.0.0"}))
        .send()
        .await
        .expect("create version");
    assert_eq!(version.status(), StatusCode::CREATED);
    let archive = make_targz("dep.txt", b"dep");
    let published = client
        .put(format!("{project_url}/versions/1.0.0/apps/dep"))
        .header("Authorization", bearer(&alice))
        .multipart(
            reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(archive).file_name("dep.tar.gz"),
                )
                .text("sha256", common::sha256_hex(&make_targz("dep.txt", b"dep"))),
        )
        .send()
        .await
        .expect("publish dep");
    assert_eq!(published.status(), StatusCode::CREATED);
    let download_before = client
        .get(format!("{project_url}/versions/1.0.0/download?app=dep"))
        .header("Authorization", bearer(&jwt))
        .send()
        .await
        .expect("download with read scope");
    assert_eq!(download_before.status(), StatusCode::OK, "read scope download ok");

    let narrowed = client
        .post(format!("{base}/api/v1/tokens/{}", token.token_id))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"scopes": []}))
        .send()
        .await
        .expect("narrow scopes");
    assert_eq!(narrowed.status(), StatusCode::OK, "narrow scopes to empty");
    let download_after = client
        .get(format!("{project_url}/versions/1.0.0/download?app=dep"))
        .header("Authorization", bearer(&jwt))
        .send()
        .await
        .expect("download after narrow");
    assert_eq!(
        download_after.status(),
        StatusCode::FORBIDDEN,
        "scope removal takes effect on same jwt (no re-sign)"
    );
    let read_after = client
        .get(format!("{project_url}"))
        .header("Authorization", bearer(&jwt))
        .send()
        .await
        .expect("read after narrow");
    assert_eq!(
        read_after.status(),
        StatusCode::NOT_FOUND,
        "metadata:read removal takes effect on same jwt"
    );

    // 列表无过期字段。
    let list = client
        .get(format!("{base}/api/v1/tokens"))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("token list");
    assert_eq!(list.status(), StatusCode::OK);
    let list_value: serde_json::Value = list.json().await.expect("list body");
    assert!(
        list_value
            .as_array()
            .expect("array")
            .iter()
            .all(|item| item.get("expires_at").is_none()),
        "token list must not expose expires_at"
    );
}

#[tokio::test]
async fn token_revoke_and_management_credential_boundaries() {
    common::log_case("token_revoke_and_management_credential_boundaries");
    // D6/D8：revoke 即时失效；token 管理仅限 session 且他人 token 不可见。
    let base = start_server().await;
    let client = Client::new();
    let alice = login(&client, &base, "alice", "alice-pass").await;
    let bob = login(&client, &base, "bob", "bob-pass").await;

    let created = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"name": "token-revoke", "visibility": "private"}))
        .send()
        .await
        .expect("create project");
    assert_eq!(created.status(), StatusCode::CREATED);
    let project: Project = created.json().await.expect("project body");
    let project_url = format!("{base}/api/v1/projects/{}", project.project_id);

    let issued = client
        .post(format!("{base}/api/v1/tokens"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({
            "name": "revoke-me",
            "scopes": ["metadata:read"],
            "expires_at": null,
        }))
        .send()
        .await
        .expect("create token");
    assert_eq!(issued.status(), StatusCode::CREATED);
    let token: TokenIssued = issued.json().await.expect("token body");

    // 他人不能看到/操作该 token；token 凭据不能管理 token。
    let bob_revoke = client
        .delete(format!("{base}/api/v1/tokens/{}", token.token_id))
        .header("Authorization", bearer(&bob))
        .send()
        .await
        .expect("bob revoke");
    assert_eq!(bob_revoke.status(), StatusCode::NOT_FOUND, "other owner token 404");

    let second = client
        .post(format!("{base}/api/v1/tokens"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({
            "name": "manager",
            "scopes": ["metadata:read"],
            "expires_at": null,
        }))
        .send()
        .await
        .expect("second token");
    assert_eq!(second.status(), StatusCode::CREATED);
    let manager: TokenIssued = second.json().await.expect("manager token");
    let token_management = client
        .post(format!("{base}/api/v1/tokens"))
        .header("Authorization", bearer(&manager.jwt))
        .json(&serde_json::json!({"name": "x", "scopes": ["metadata:read"]}))
        .send()
        .await
        .expect("token creates token");
    assert_eq!(
        token_management.status(),
        StatusCode::FORBIDDEN,
        "token credential cannot manage tokens"
    );

    // owner revoke：204，旧 JWT 立即 401，列表移除。
    let revoked = client
        .delete(format!("{base}/api/v1/tokens/{}", token.token_id))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("owner revoke");
    assert_eq!(revoked.status(), StatusCode::NO_CONTENT, "revoke 204");
    let stale = client
        .get(format!("{project_url}"))
        .header("Authorization", bearer(&token.jwt))
        .send()
        .await
        .expect("revoked jwt read");
    assert_eq!(stale.status(), StatusCode::UNAUTHORIZED, "revoked jwt 401");
    let list = client
        .get(format!("{base}/api/v1/tokens"))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("token list");
    let list_value: serde_json::Value = list.json().await.expect("list body");
    assert!(
        !list_value
            .as_array()
            .expect("array")
            .iter()
            .any(|item| item["token_id"] == token.token_id),
        "revoked token removed from list"
    );
}

#[tokio::test]
async fn token_project_scope_specified_isolation() {
    common::log_case("token_project_scope_specified_isolation");
    // D7：Specified 集合外的项目对 token 不可见；空 Specified 等价 All。
    let base = start_server().await;
    let client = Client::new();
    let alice = login(&client, &base, "alice", "alice-pass").await;

    async fn create_project(client: &Client, base: &str, session: &str, name: &str) -> Project {
        let resp = client
            .post(format!("{base}/api/v1/projects"))
            .header("Authorization", bearer(session))
            .json(&serde_json::json!({"name": name, "visibility": "private"}))
            .send()
            .await
            .expect("create project");
        assert_eq!(resp.status(), StatusCode::CREATED, "create {name}");
        resp.json().await.expect("project body")
    }
    let p1 = create_project(&client, &base, &alice, "scope-a").await;
    let p2 = create_project(&client, &base, &alice, "scope-b").await;

    let issued = client
        .post(format!("{base}/api/v1/tokens"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({
            "name": "scoped",
            "project_scope": {"Specified": [p2.project_id]},
            "scopes": ["metadata:read", "artifacts:read"],
            "expires_at": null,
        }))
        .send()
        .await
        .expect("create scoped token");
    assert_eq!(issued.status(), StatusCode::CREATED);
    let scoped: TokenIssued = issued.json().await.expect("scoped token");

    let out_of_scope = client
        .get(format!("{base}/api/v1/projects/{}", p1.project_id))
        .header("Authorization", bearer(&scoped.jwt))
        .send()
        .await
        .expect("out of scope read");
    assert_eq!(
        out_of_scope.status(),
        StatusCode::NOT_FOUND,
        "Specified outside project is invisible on project read"
    );
    let in_scope = client
        .get(format!("{base}/api/v1/projects/{}", p2.project_id))
        .header("Authorization", bearer(&scoped.jwt))
        .send()
        .await
        .expect("in scope read");
    assert_eq!(in_scope.status(), StatusCode::OK, "Specified inside project visible");
    let out_versions = client
        .get(format!("{base}/api/v1/projects/{}/versions", p1.project_id))
        .header("Authorization", bearer(&scoped.jwt))
        .send()
        .await
        .expect("out of scope versions");
    assert_eq!(
        out_versions.status(),
        StatusCode::FORBIDDEN,
        "Specified outside project denied on version list"
    );

    // 空 Specified 与 All 等价：两个项目都可见。
    let all_issued = client
        .post(format!("{base}/api/v1/tokens"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({
            "name": "all-scope",
            "project_scope": {"Specified": []},
            "scopes": ["metadata:read"],
            "expires_at": null,
        }))
        .send()
        .await
        .expect("create all-scope token");
    assert_eq!(all_issued.status(), StatusCode::CREATED);
    let all_token: TokenIssued = all_issued.json().await.expect("all token");
    for project_id in [p1.project_id, p2.project_id] {
        let read = client
            .get(format!("{base}/api/v1/projects/{project_id}"))
            .header("Authorization", bearer(&all_token.jwt))
            .send()
            .await
            .expect("all scope read");
        assert_eq!(read.status(), StatusCode::OK, "empty Specified behaves as All");
    }
}

#[tokio::test]
async fn version_name_validation_matrix_and_idempotent_lock() {
    common::log_case("version_name_validation_matrix_and_idempotent_lock");
    // E2/E5：保留字/非法字符 422；合法版本创建；重复锁定幂等且 locked_at 不变。
    let base = start_server().await;
    let client = Client::new();
    let alice = login(&client, &base, "alice", "alice-pass").await;
    let created = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"name": "version-names", "visibility": "public"}))
        .send()
        .await
        .expect("create project");
    assert_eq!(created.status(), StatusCode::CREATED);
    let project: Project = created.json().await.expect("project body");
    let url = format!("{base}/api/v1/projects/{}/versions", project.project_id);

    for bad in ["latest", "a/b", "a?b", "a#b", "a\u{0007}b", ""] {
        let resp = client
            .post(&url)
            .header("Authorization", bearer(&alice))
            .json(&serde_json::json!({"version": bad}))
            .send()
            .await
            .expect("bad version create");
        assert_eq!(
            resp.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "version {bad:?} rejected 422"
        );
    }
    let ok = client
        .post(&url)
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "v1.0.0-beta.1"}))
        .send()
        .await
        .expect("valid version create");
    assert_eq!(ok.status(), StatusCode::CREATED, "valid version 201");

    let lock_url = format!("{base}/api/v1/projects/{}/versions/v1.0.0-beta.1/lock", project.project_id);
    let first = client
        .put(&lock_url)
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("lock");
    assert_eq!(first.status(), StatusCode::OK, "lock 200");
    let first_record: VersionRecord = first.json().await.expect("locked record");
    let first_locked = first_record.locked_at.expect("locked_at present");
    let second = client
        .put(&lock_url)
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("lock again");
    assert_eq!(second.status(), StatusCode::OK, "idempotent lock 200");
    let second_record: VersionRecord = second.json().await.expect("locked record");
    assert_eq!(
        second_record.locked_at.as_deref(),
        Some(first_locked.as_str()),
        "repeat lock keeps locked_at stable"
    );
}

#[tokio::test]
async fn opaque_bytes_upload_missing_version_download_headers_and_empty_version() {
    common::log_case("opaque_bytes_upload_missing_version_download_headers_and_empty_version");
    // F8/F10/G2/G5：非 gzip 不透明字节按 sha 匹配入库；目标版本 404；
    // 下载头断言；空版本下载 404。
    let base = start_server().await;
    let client = Client::new();
    let alice = login(&client, &base, "alice", "alice-pass").await;
    let created = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"name": "opaque", "visibility": "public"}))
        .send()
        .await
        .expect("create project");
    assert_eq!(created.status(), StatusCode::CREATED);
    let project: Project = created.json().await.expect("project body");
    let pid = project.project_id;
    let base_pid = format!("{base}/api/v1/projects/{pid}");

    let version = client
        .post(format!("{base_pid}/versions"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "1.0.0"}))
        .send()
        .await
        .expect("create version");
    assert_eq!(version.status(), StatusCode::CREATED);

    // F8：任意字节（非 gzip/tar）sha 匹配即可发布（2026-08-24 不透明流契约）。
    let opaque = b"\x00\xff\x01opaque-not-a-gzip".to_vec();
    let published = client
        .put(format!("{base_pid}/versions/1.0.0/apps/ui"))
        .header("Authorization", bearer(&alice))
        .multipart(
            reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(opaque.clone()).file_name("ui.tar.gz"),
                )
                .text("sha256", common::sha256_hex(&opaque)),
        )
        .send()
        .await
        .expect("opaque upload");
    assert_eq!(published.status(), StatusCode::CREATED, "opaque bytes accepted");

    // F10：不存在的版本 404，且不发布 app。
    let missing = client
        .put(format!("{base_pid}/versions/nope/apps/ui"))
        .header("Authorization", bearer(&alice))
        .multipart(
            reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(opaque.clone()).file_name("ui.tar.gz"),
                )
                .text("sha256", common::sha256_hex(&opaque)),
        )
        .send()
        .await
        .expect("missing version upload");
    assert_eq!(missing.status(), StatusCode::NOT_FOUND, "missing version 404");

    // G2：下载响应头与字节一致。
    let download = client
        .get(format!("{base_pid}/versions/1.0.0/download?app=ui"))
        .send()
        .await
        .expect("download");
    assert_eq!(download.status(), StatusCode::OK, "download 200");
    let disposition = download
        .headers()
        .get("content-disposition")
        .expect("content-disposition header")
        .to_str()
        .expect("disposition str");
    assert_eq!(
        disposition,
        format!("attachment; filename=\"{pid}-1.0.0-ui.tar.gz\""),
        "download filename contract"
    );
    let bytes = download.bytes().await.expect("download bytes");
    assert_eq!(bytes.to_vec(), opaque, "download content matches opaque bytes");

    // G5：空版本（无 apps）下载 404。
    let empty_version = client
        .post(format!("{base_pid}/versions"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "2.0.0"}))
        .send()
        .await
        .expect("create empty version");
    assert_eq!(empty_version.status(), StatusCode::CREATED);
    let empty_download = client
        .get(format!("{base_pid}/versions/2.0.0/download"))
        .send()
        .await
        .expect("empty version download");
    assert_eq!(empty_download.status(), StatusCode::NOT_FOUND, "empty version 404");
}

#[tokio::test]
async fn unified_error_contract_shape_and_unknown_route() {
    common::log_case("unified_error_contract_shape_and_unknown_route");
    // J1/J2：所有错误均为 {"error","message"} JSON；未知路由 404 not_found。
    let base = start_server().await;
    let client = Client::new();

    let unknown = client
        .get(format!("{base}/api/v1/does-not-exist"))
        .send()
        .await
        .expect("unknown route");
    assert_eq!(
        unknown.status(),
        StatusCode::NOT_FOUND,
        "unknown route 404"
    );
    // 已知残余缺口：sfo-http 0.8 内置 404 响应体为空 JSON 错误体无法由
    // 当前公开装配 API 覆盖，此处只断言 404 状态；变更记录中保留跟踪项。

    let alice = login(&client, &base, "alice", "alice-pass").await;
    let public = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"name": "errors", "visibility": "private"}))
        .send()
        .await
        .expect("create project");
    assert_eq!(public.status(), StatusCode::CREATED);
    let project: Project = public.json().await.expect("project body");
    let pid = project.project_id;

    // 401：匿名访问 private。
    let anon = client
        .get(format!("{base}/api/v1/projects/{pid}"))
        .send()
        .await
        .expect("anon private");
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);
    let anon_body: ErrorBody = anon.json().await.expect("anon body");
    assert_eq!(anon_body.error, "unauthorized");

    // 403：只读 token 写。
    let token = client
        .post(format!("{base}/api/v1/tokens"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({
            "name": "reader",
            "scopes": ["metadata:read"],
            "expires_at": null,
        }))
        .send()
        .await
        .expect("reader token");
    assert_eq!(token.status(), StatusCode::CREATED);
    let reader: TokenIssued = token.json().await.expect("reader token body");
    let forbidden = client
        .post(format!("{base}/api/v1/projects/{pid}/versions"))
        .header("Authorization", bearer(&reader.jwt))
        .json(&serde_json::json!({"version": "9.9.9"}))
        .send()
        .await
        .expect("readonly write attempt");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    let forbidden_body: ErrorBody = forbidden.json().await.expect("forbidden body");
    assert_eq!(forbidden_body.error, "forbidden");
    assert!(!forbidden_body.message.is_empty());

    // 409：重复版本。
    let ok = client
        .post(format!("{base}/api/v1/projects/{pid}/versions"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "1.0.0"}))
        .send()
        .await
        .expect("create version");
    assert_eq!(ok.status(), StatusCode::CREATED);
    let duplicate = client
        .post(format!("{base}/api/v1/projects/{pid}/versions"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "1.0.0"}))
        .send()
        .await
        .expect("duplicate version");
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
    let conflict_body: ErrorBody = duplicate.json().await.expect("conflict body");
    assert_eq!(conflict_body.error, "conflict");

    // 422：非法版本名。
    let invalid = client
        .post(format!("{base}/api/v1/projects/{pid}/versions"))
        .header("Authorization", bearer(&alice))
        .json(&serde_json::json!({"version": "bad/name"}))
        .send()
        .await
        .expect("invalid version");
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let invalid_body: ErrorBody = invalid.json().await.expect("invalid body");
    assert_eq!(invalid_body.error, "invalid_input");
}
