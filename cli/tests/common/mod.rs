//! CLI 集成测试共用装置：进程内 mock filehub-server（v1 契约形状）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// 正常行为：s1/s2/s3 会话与 tok-valid/tok-view token 均有效。
    Normal,
    /// 首次 `GET /api/v1/projects`（s1）返回 401 一次，用于续期重试测试。
    ExpiresOnce,
}

pub struct MockServer {
    pub base: String,
    #[allow(dead_code)]
    pub handle: tokio::task::JoinHandle<()>,
}

impl MockServer {
    /// 无协议身份（`host[:port]`），用于验证 Docker 风格 server 语义。
    #[allow(dead_code)] // dv_tests 编译该共享模块时无需此方法。
    pub fn identity(&self) -> String {
        self.base
            .strip_prefix("http://")
            .expect("mock base uses http://")
            .to_string()
    }
}

#[derive(Default)]
struct State {
    counters: Mutex<HashMap<String, u32>>,
}

struct App {
    mode: Mode,
    payload: Vec<u8>,
    payload_sha: String,
    state: State,
}

const SESSIONS: [&str; 3] = ["s1", "s2", "s3"];
const VALID_TOKENS: [&str; 2] = ["tok-valid", "tok-view"];

impl MockServer {
    /// 启动 mock 服务；`payload` 同时用作版本元数据里的 sha256 与下载流。
    pub async fn start(mode: Mode, payload: Vec<u8>) -> MockServer {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock server");
        let addr = listener.local_addr().expect("mock addr");
        let base = format!("http://{addr}");
        let digest = Sha256::digest(&payload);
        let payload_sha = format!("{digest:x}");
        let app = Arc::new(App {
            mode,
            payload,
            payload_sha,
            state: State::default(),
        });
        let handle = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let app = app.clone();
                tokio::spawn(async move {
                    let _ = serve(stream, app).await;
                });
            }
        });
        MockServer { base, handle }
    }
}

async fn serve(
    mut stream: TcpStream,
    app: Arc<App>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut head = Vec::new();
    let mut chunk = [0u8; 2048];
    let mut body_buffer = Vec::new();
    let delimiter_at;
    loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(());
        }
        head.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_header_end(&head) {
            delimiter_at = pos;
            body_buffer.extend_from_slice(&head[pos + 4..]);
            break;
        }
        // HTTPS 优先请求先打到明文 mock 上：非 HTTP 方法开头（如 TLS ClientHello）
        // 直接返回 400 关闭，避免测试等待 TLS 超时，同时让 HTTP 降级路径快速生效。
        if !head.is_empty() && !looks_like_http_request(&head) {
            let _ = stream
                .write_all(
                    b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await;
            return Ok(());
        }
    }
    let head_text = String::from_utf8_lossy(&head[..delimiter_at]);
    let mut lines = head_text.split("\r\n");
    let request_line = lines.next().unwrap_or("").to_string();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let raw_path = parts.next().unwrap_or("").to_string();
    let path = raw_path.split('?').next().unwrap_or("").to_string();
    let mut headers = HashMap::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    let content_length: usize = headers
        .get("content-length")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let transfer_chunked = headers
        .get("transfer-encoding")
        .map(|value| value.to_ascii_lowercase().contains("chunked"))
        .unwrap_or(false);
    let body = if transfer_chunked {
        decode_chunked(&mut body_buffer, &mut stream, &mut chunk).await?
    } else {
        while body_buffer.len() < content_length {
            let n = stream.read(&mut chunk).await?;
            if n == 0 {
                break;
            }
            body_buffer.extend_from_slice(&chunk[..n]);
        }
        body_buffer.truncate(content_length);
        body_buffer.clone()
    };

    let bearer = headers
        .get("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or("")
        .to_string();
    let boundary = headers.get("content-type").and_then(|value| {
        value.split(';').find_map(|part| {
            let part = part.trim();
            part.strip_prefix("boundary=")
                .map(|value| value.trim_matches('"').to_string())
        })
    });
    let response = route(&app, &method, &path, &bearer, &body, boundary.as_deref());
    write_response(&mut stream, response).await?;
    Ok(())
}

fn find_header_end(head: &[u8]) -> Option<usize> {
    head.windows(4).position(|window| window == b"\r\n\r\n")
}

fn looks_like_http_request(head: &[u8]) -> bool {
    const METHODS: &[&str] = &["GET ", "POST ", "PUT ", "DELETE ", "OPTIONS ", "HEAD "];
    METHODS
        .iter()
        .any(|method| head.starts_with(method.as_bytes()))
}

struct Response {
    status: &'static str,
    content_type: String,
    body: Vec<u8>,
}

fn json_response(status: &'static str, payload: serde_json::Value) -> Response {
    Response {
        status,
        content_type: "application/json".to_string(),
        body: serde_json::to_vec(&payload).unwrap_or_default(),
    }
}

fn error_response(status: &'static str, code: &str, message: &str) -> Response {
    json_response(
        status,
        serde_json::json!({"error": code, "message": message}),
    )
}

fn route(
    app: &App,
    method: &str,
    path: &str,
    bearer: &str,
    body: &[u8],
    boundary: Option<&str>,
) -> Response {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match (method, segments.as_slice()) {
        ("POST", ["account", "login"]) => {
            let parsed: Option<serde_json::Value> = serde_json::from_slice(body).ok();
            let password = parsed
                .as_ref()
                .and_then(|value| value.get("password"))
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if password == "alice-pass" {
                json_response(
                    "200 OK",
                    serde_json::json!({"err": 0, "result": {"session": "s1", "refresh_session": "r1"}}),
                )
            } else {
                error_response("401 Unauthorized", "unauthorized", "bad credentials")
            }
        }
        ("POST", ["account", "refresh_session"]) => match bearer {
            "r1" => json_response(
                "200 OK",
                serde_json::json!({"err": 0, "result": {"session": "s2", "refresh_session": "r2"}}),
            ),
            "r2" => json_response(
                "200 OK",
                serde_json::json!({"err": 0, "result": {"session": "s3", "refresh_session": "r3"}}),
            ),
            _ => error_response(
                "401 Unauthorized",
                "unauthorized",
                "invalid refresh session",
            ),
        },
        ("GET", ["api", "v1", "projects"]) => {
            if is_auth_invalid(app, bearer) {
                return error_response("401 Unauthorized", "unauthorized", "invalid credential");
            }
            if app.mode == Mode::ExpiresOnce
                && bearer == "s1"
                && take_once(&app.state, "s1-projects")
            {
                return error_response("401 Unauthorized", "unauthorized", "session expired once");
            }
            json_response(
                "200 OK",
                serde_json::json!([
                    {"project_id": 1, "name": "demo", "visibility": "public", "owner": 1},
                    {"project_id": 2, "name": "refresh-once", "visibility": "public", "owner": 1}
                ]),
            )
        }
        ("POST", ["api", "v1", "projects", _id, "versions"]) => {
            if is_auth_invalid(app, bearer) {
                return error_response("401 Unauthorized", "unauthorized", "invalid credential");
            }
            if bearer == "tok-view" {
                return error_response(
                    "403 Forbidden",
                    "forbidden",
                    "readonly token cannot create version",
                );
            }
            let parsed: Option<serde_json::Value> = serde_json::from_slice(body).ok();
            let version_text = parsed
                .as_ref()
                .and_then(|value| value.get("version"))
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string();
            if version_text == "exists" {
                return error_response("409 Conflict", "conflict", "version 已存在");
            }
            if version_text.is_empty() {
                return error_response(
                    "422 Unprocessable Entity",
                    "invalid_input",
                    "missing version",
                );
            }
            json_response("201 Created", version_created_json(&app, &version_text))
        }
        (
            "PUT",
            [
                "api",
                "v1",
                "projects",
                _id,
                "versions",
                version_name,
                "apps",
                app_name,
            ],
        ) => {
            if is_auth_invalid(app, bearer) {
                return error_response("401 Unauthorized", "unauthorized", "invalid credential");
            }
            if bearer == "tok-view" {
                return error_response(
                    "403 Forbidden",
                    "forbidden",
                    "readonly token cannot publish",
                );
            }
            if *version_name == "missing" {
                return error_response("404 Not Found", "not_found", "version not found");
            }
            if *version_name == "exists" || *version_name == "locked" {
                return error_response("409 Conflict", "conflict", "version is locked");
            }
            let parsed = multipart_fields(body, boundary);
            let has_file = parsed.get("file").is_some();
            if !has_file {
                return error_response("422 Unprocessable Entity", "invalid_input", "missing file");
            }
            let created = *app_name != "exists";
            let version_text = version_name.to_string();
            let app_text = app_name.to_string();
            json_response(
                if created { "201 Created" } else { "200 OK" },
                version_with_app_json(&app, &app_text, &version_text),
            )
        }
        (
            "DELETE",
            [
                "api",
                "v1",
                "projects",
                _id,
                "versions",
                version_name,
                "apps",
                app_name,
            ],
        ) => {
            if is_auth_invalid(app, bearer) {
                return error_response("401 Unauthorized", "unauthorized", "invalid credential");
            }
            if bearer == "tok-view" {
                return error_response(
                    "403 Forbidden",
                    "forbidden",
                    "readonly token cannot delete app",
                );
            }
            if *version_name == "locked" || *version_name == "locked-v1" {
                return error_response("409 Conflict", "conflict", "version is locked");
            }
            if *app_name == "missing" {
                return error_response("404 Not Found", "not_found", "app not found");
            }
            Response {
                status: "204 No Content",
                content_type: "application/json".to_string(),
                body: Vec::new(),
            }
        }
        ("PUT", ["api", "v1", "projects", _id, "versions", version, "lock"]) => {
            if is_auth_invalid(app, bearer) {
                return error_response("401 Unauthorized", "unauthorized", "invalid credential");
            }
            if bearer == "tok-view" {
                return error_response("403 Forbidden", "forbidden", "readonly token cannot lock");
            }
            if *version == "missing" {
                return error_response("404 Not Found", "not_found", "version not found");
            }
            let mut value = version_json(&app, version);
            value["locked_at"] = serde_json::json!("2026-08-21T00:00:00+00:00");
            json_response("200 OK", value)
        }
        ("GET", ["api", "v1", "projects", _id, "versions"]) => {
            if is_auth_invalid(app, bearer) {
                return error_response("401 Unauthorized", "unauthorized", "invalid credential");
            }
            json_response(
                "200 OK",
                serde_json::json!([version_json(&app, "v1"), version_json(&app, "v2")]),
            )
        }
        ("GET", ["api", "v1", "projects", _id, "versions", version]) => {
            if is_auth_invalid(app, bearer) {
                return error_response("401 Unauthorized", "unauthorized", "invalid credential");
            }
            if *version == "missing" {
                error_response("404 Not Found", "not_found", "version not found")
            } else if *version == "latest" {
                json_response("200 OK", version_json(&app, "v2"))
            } else {
                json_response("200 OK", version_json(&app, version))
            }
        }
        (
            "GET",
            [
                "api",
                "v1",
                "projects",
                _id,
                "versions",
                version,
                "download",
            ],
        ) => {
            if is_auth_invalid(app, bearer) {
                return error_response("401 Unauthorized", "unauthorized", "invalid credential");
            }
            if bearer == "s1" && *version == "refresh-once" && take_once(&app.state, "s1-download")
            {
                return error_response("401 Unauthorized", "unauthorized", "session expired once");
            }
            let body = if *version == "corrupt" {
                b"not-a-real-gzip".to_vec()
            } else {
                app.payload.clone()
            };
            Response {
                status: "200 OK",
                content_type: "application/gzip".to_string(),
                body,
            }
        }
        _ => error_response("404 Not Found", "not_found", "route not found"),
    }
}

/// 解码 HTTP chunked 传输编码（测试 mock 用，支持普通与 chunk 块）。
async fn decode_chunked(
    buf: &mut Vec<u8>,
    stream: &mut TcpStream,
    tmp: &mut [u8; 2048],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut pos = 0usize;
    let mut decoded = Vec::new();
    loop {
        let mut line = None;
        while pos <= buf.len() && line.is_none() {
            if let Some(idx) = buf[pos..].iter().position(|byte| *byte == b'\n') {
                line = Some(
                    String::from_utf8_lossy(&buf[pos..pos + idx])
                        .trim()
                        .to_string(),
                );
                pos += idx + 1;
            } else {
                let n = stream.read(tmp).await?;
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
            }
        }
        let Some(line) = line else { break };
        if line.is_empty() {
            continue;
        }
        let size_text = line.split(';').next().unwrap_or("").trim();
        let size = match usize::from_str_radix(size_text, 16) {
            Ok(size) => size,
            Err(_) => return Ok(decoded),
        };
        if size == 0 {
            break;
        }
        while buf.len() - pos < size + 2 {
            let n = stream.read(tmp).await?;
            if n == 0 {
                return Ok(decoded);
            }
            buf.extend_from_slice(&tmp[..n]);
        }
        decoded.extend_from_slice(&buf[pos..pos + size]);
        pos += size + 2;
    }
    Ok(decoded)
}

fn version_json(app: &App, version: &str) -> serde_json::Value {
    serde_json::json!({
        "project_id": 1,
        "version": version,
        "published_at": "2026-08-20T00:00:00+00:00",
        "locked_at": null,
        "apps": [app_json(&app, "default")]
    })
}

fn version_created_json(_app: &App, version: &str) -> serde_json::Value {
    serde_json::json!({
        "project_id": 1,
        "version": version,
        "published_at": "2026-08-20T00:00:00+00:00",
        "locked_at": null,
        "apps": []
    })
}

fn version_with_app_json(app: &App, app_name: &str, version: &str) -> serde_json::Value {
    serde_json::json!({
        "project_id": 1,
        "version": version,
        "published_at": "2026-08-20T00:00:00+00:00",
        "locked_at": null,
        "apps": [app_json(&app, app_name)]
    })
}

fn app_json(app: &App, app_name: &str) -> serde_json::Value {
    serde_json::json!({
        "app": app_name,
        "file_id": "file-1",
        "sha256": app.payload_sha,
        "size": app.payload.len(),
        "updated_at": "2026-08-20T00:00:00+00:00"
    })
}

fn is_auth_invalid(app: &App, bearer: &str) -> bool {
    if SESSIONS.contains(&bearer) || VALID_TOKENS.contains(&bearer) {
        return false;
    }
    // auth 检查本身也可被 ExpiresOnce 触发（见上方路由）。
    let _ = app;
    true
}

fn take_once(state: &State, key: &str) -> bool {
    let mut counters = state.counters.lock().unwrap();
    let count = counters.entry(key.to_string()).or_insert(0);
    if *count == 0 {
        *count += 1;
        true
    } else {
        false
    }
}

fn multipart_fields(body: &[u8], boundary: Option<&str>) -> HashMap<String, Vec<u8>> {
    let mut fields = HashMap::new();
    let text = String::from_utf8_lossy(body);
    let Some(boundary_marker) = boundary.map(str::to_string) else {
        return fields;
    };
    if boundary_marker.is_empty() {
        return fields;
    }
    let delimiter = format!("--{boundary_marker}");
    for block in text.split(&format!("\r\n{delimiter}")) {
        let Some((headers, content)) = block.split_once("\r\n\r\n") else {
            continue;
        };
        let name = headers
            .lines()
            .find_map(|line| {
                let rest = line
                    .split("; ")
                    .find_map(|part| part.strip_prefix("name="))?;
                Some(rest.trim_matches('"').to_string())
            })
            .unwrap_or_default();
        if !name.is_empty() {
            fields.insert(name, content.trim_end_matches("\r\n--").as_bytes().to_vec());
        }
    }
    fields
}

async fn write_response(
    stream: &mut TcpStream,
    response: Response,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let head = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(&response.body).await?;
    stream.flush().await?;
    Ok(())
}

/// 生成一个可被 CLI 校验通过的 `.tar.gz` 载荷。
pub fn make_payload(name: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let encoder = flate2::write::GzEncoder::new(&mut bytes, flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);
        let content = format!("hello {name}");
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::file());
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_cksum();
        builder
            .append_data(&mut header, "artifact.txt", content.as_bytes())
            .unwrap();
        let encoder = builder.into_inner().unwrap();
        encoder.finish().unwrap();
    }
    bytes
}

/// 测试用例共用的临时目录与配置路径。
pub struct TestEnv {
    pub dir: tempfile::TempDir,
    pub config: std::path::PathBuf,
}

impl TestEnv {
    pub fn new() -> TestEnv {
        let dir = tempfile::tempdir().expect("temp dir");
        let config = dir.path().join("filehub-config.toml");
        TestEnv { dir, config }
    }
}

/// 环境变量互斥守卫：FILEHUB_* 测试用例串行执行，避免并行污染。
pub static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 获取环境变量守卫（容忍先前测试 panic 留下的 poisoned 状态）。
pub fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    match ENV_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
