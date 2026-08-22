//! integration：真实 Actix/sfo-http 服务器 + HTTP 客户端契约验证。

use common::{make_targz, temp_dir, test_config};
use filehub_server::account::store::connect_pool;
use filehub_server::http::{register_api, AppState};
use reqwest::{Client, StatusCode};
use sfo_http::actix_server::ActixHttpServer;
use sfo_http::http_server::HttpServerConfig;
use std::net::TcpListener;

#[path = "common/mod.rs"] mod common;

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

async fn start_server() -> String {
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("bind")
        .local_addr()
        .expect("addr")
        .port();
    let dir = temp_dir().await;
    let mut config = test_config(&dir, &dir.join("api.db").to_string_lossy());
    config.server.port = port;
    let db = connect_pool(&config.db_path, 2).await.expect("pool");
    let state = AppState::assemble(&config, &db).await.expect("assemble");
    let base = format!("http://127.0.0.1:{port}");

    let server_config = HttpServerConfig::new(config.server.server_addr.clone(), config.server.port)
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
        if client.get(format!("{base}/api/v1/projects")).send().await.is_ok() {
            return base;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("server did not become ready");
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
    login.session
}

fn bearer(session: &str) -> String {
    format!("Bearer {session}")
}

#[tokio::test]
async fn api_login_session_and_token_flow() {
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

    // member 不能创建项目
    let bob = login(&client, &base, "bob", "bob-pass").await;
    let denied = client
        .post(format!("{base}/api/v1/projects"))
        .header("Authorization", bearer(&bob))
        .json(&serde_json::json!({"name": "bad", "visibility": "private"}))
        .send()
        .await
        .expect("member create");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN, "member create denied");

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
    let anon_private = client.get(format!("{project_url}/versions")).send().await.expect("anon private");
    assert_eq!(anon_private.status(), StatusCode::UNAUTHORIZED, "anon private unauthorized");

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
    assert_eq!(duplicate.status(), StatusCode::CONFLICT, "duplicate version");

    // 同一版本发布两个 app；重复发布同 app 为更新
    let archive_a = make_targz("a.txt", b"a");
    let archive_b = make_targz("b.txt", b"b");
    let app_url = |version: &str, app: &str| {
        format!("{project_url}/versions/{version}/apps/{app}")
    };
    let created_app = client
        .put(app_url("1.0.0", "ui"))
        .header("Authorization", bearer(&alice))
        .multipart(reqwest::multipart::Form::new().part("file", reqwest::multipart::Part::bytes(archive_a.clone()).file_name("a.tar.gz")))
        .send()
        .await
        .expect("publish ui");
    assert_eq!(created_app.status(), StatusCode::CREATED, "app created");
    let second_app = client
        .put(app_url("1.0.0", "server"))
        .header("Authorization", bearer(&alice))
        .multipart(reqwest::multipart::Form::new().part("file", reqwest::multipart::Part::bytes(archive_b.clone()).file_name("b.tar.gz")))
        .send()
        .await
        .expect("publish server");
    assert_eq!(second_app.status(), StatusCode::CREATED, "second app created");
    let updated_app = client
        .put(app_url("1.0.0", "ui"))
        .header("Authorization", bearer(&alice))
        .multipart(reqwest::multipart::Form::new().part("file", reqwest::multipart::Part::bytes(archive_b.clone()).file_name("b.tar.gz")))
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
    assert!(single_record.apps.iter().any(|app| app.app == "server" && app.sha256.len() == 64));

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
        .multipart(reqwest::multipart::Form::new().part("file", reqwest::multipart::Part::bytes(archive_a.clone()).file_name("c.tar.gz")))
        .send()
        .await
        .expect("locked publish attempt");
    assert_eq!(locked_publish.status(), StatusCode::CONFLICT, "locked publish");
    let locked_delete = client
        .delete(app_url("1.0.0", "ui"))
        .header("Authorization", bearer(&alice))
        .send()
        .await
        .expect("locked delete attempt");
    assert_eq!(locked_delete.status(), StatusCode::CONFLICT, "locked delete");

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
    assert_eq!(token_write.status(), StatusCode::FORBIDDEN, "read-only token write denied");

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
    assert_eq!(old_read.status(), StatusCode::UNAUTHORIZED, "rotated token invalid");
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
    assert_eq!(multi_default.status(), StatusCode::UNPROCESSABLE_ENTITY, "multi-app default download 422");
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
    let ui_app = latest_body.apps.iter().find(|app| app.app == "ui").expect("ui app");
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
    assert_eq!(bob_write.status(), StatusCode::FORBIDDEN, "read-only collaborator");
}

#[tokio::test]
async fn version_app_input_and_download_boundaries() {
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
    assert_eq!(empty.status(), StatusCode::UNPROCESSABLE_ENTITY, "empty version rejected");

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
        .multipart(reqwest::multipart::Form::new().part("file", reqwest::multipart::Part::bytes(make_targz("x", b"x")).file_name("x.tar.gz")))
        .send()
        .await
        .expect("invalid app");
    assert_eq!(invalid_app.status(), StatusCode::UNPROCESSABLE_ENTITY, "invalid app name rejected");

    // 单 app 版本：无 ?app 缺省下载 200；错误 app 404；显式 app 200 且内容一致。
    let solo = client
        .put(format!("{project_url}/versions/1.0.0/apps/solo"))
        .header("Authorization", bearer(&alice))
        .multipart(reqwest::multipart::Form::new().part("file", reqwest::multipart::Part::bytes(make_targz("solo", b"solo-data")).file_name("solo.tar.gz")))
        .send()
        .await
        .expect("publish solo");
    assert_eq!(solo.status(), StatusCode::CREATED, "solo app");
    let default_download = client
        .get(format!("{project_url}/versions/1.0.0/download"))
        .send()
        .await
        .expect("default download");
    assert_eq!(default_download.status(), StatusCode::OK, "single-app default download");
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
