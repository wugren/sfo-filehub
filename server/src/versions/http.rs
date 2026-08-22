//! versions 的 HTTP 接口：版本创建/锁定、app 发布/更新/删除、列表/单查/latest 与按 app 下载（I-006 修订）。

use http::{Method, StatusCode};
use sfo_http::http_server::{HttpServer, Request, Response};
use std::collections::HashMap;
use std::sync::Arc;

use crate::contract::{api_error_response, empty_response, json_body, json_ok, path_param, ApiError, AuthProvider};
use crate::model::{Principal, ProjectId};
use crate::storage::FileStore;
use crate::versions::{VersionError, VersionErrorKind, VersionService};

pub fn register<S, Req, Resp>(
    server: &mut S,
    versions: Arc<dyn VersionService>,
    files: Arc<dyn FileStore>,
    auth: Arc<AuthProvider>,
) where
    S: sfo_http::http_server::HttpServer<Req, Resp>,
    Req: Request + Sync,
    Resp: Response,
{
    // POST /api/v1/projects/{project_id}/versions —— 显式创建版本。
    let versions_create = versions.clone();
    let auth_create = auth.clone();
    server.serve("/api/v1/projects/{project_id}/versions", Method::POST, move |mut req: Req| {
        let versions = versions_create.clone();
        let auth = auth_create.clone();
        async move {
            let principal = crate::api_try!(auth.current_principal_req(&req).await);
            let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
            let body = crate::api_try!(json_body::<Req, CreateVersionRequest>(&mut req).await);
            match versions.create_version(&project_id, &body.version, &principal).await {
                Ok(record) => json_ok(StatusCode::CREATED, &record),
                Err(e) => api_error_response(&api_err(&e, &principal)),
            }
        }
    });

    // PUT /api/v1/projects/{project_id}/versions/{version}/apps/{app} —— 发布/更新 app。
    let versions_put = versions.clone();
    let files_put = files.clone();
    let auth_put = auth.clone();
    server.serve(
        "/api/v1/projects/{project_id}/versions/{version}/apps/{app}",
        Method::PUT,
        move |mut req: Req| {
            let versions = versions_put.clone();
            let files = files_put.clone();
            let auth = auth_put.clone();
            async move {
                let principal = crate::api_try!(auth.current_principal_req(&req).await);
                let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
                let version = crate::api_try!(path_param::<Req, String>(&req, "version"));
                let app = crate::api_try!(path_param::<Req, String>(&req, "app"));
                let body = match req.body_bytes().await {
                    Ok(bytes) => bytes,
                    Err(e) => return api_error_response(&ApiError::invalid_input(format!("read body failed: {e}"))),
                };
                let boundary = match multipart_boundary(&req) {
                    Some(b) => b,
                    None => return api_error_response(&ApiError::invalid_input("multipart/form-data required with boundary")),
                };
                let parts = match parse_multipart(&body, &boundary) {
                    Ok(parts) => parts,
                    Err(e) => return api_error_response(&ApiError::invalid_input(e)),
                };
                let Some(archive) = parts.get("file") else {
                    return api_error_response(&ApiError::invalid_input("missing .tar.gz file field"));
                };
                let expected_sha256: Option<String> = parts.get("sha256").map(|v| String::from_utf8(v.clone()).ok()).flatten();
                match files.ingest(archive.clone(), expected_sha256.as_deref()).await {
                    Ok(file) => {
                        match versions.publish_app(&project_id, &version, &app, file.clone(), &principal).await {
                            Ok(outcome) => {
                                let status = if outcome.created { StatusCode::CREATED } else { StatusCode::OK };
                                json_ok(status, &outcome.record)
                            }
                            Err(e) => {
                                let _ = files.discard(&file.file_id).await;
                                api_error_response(&api_err(&e, &principal))
                            }
                        }
                    }
                    Err(e) => api_error_response(&e.into()),
                }
            }
        },
    );

    // DELETE /api/v1/projects/{project_id}/versions/{version}/apps/{app} —— 删除 app。
    let versions_del = versions.clone();
    let auth_del = auth.clone();
    server.serve(
        "/api/v1/projects/{project_id}/versions/{version}/apps/{app}",
        Method::DELETE,
        move |req: Req| {
            let versions = versions_del.clone();
            let auth = auth_del.clone();
            async move {
                let principal = crate::api_try!(auth.current_principal_req(&req).await);
                let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
                let version = crate::api_try!(path_param::<Req, String>(&req, "version"));
                let app = crate::api_try!(path_param::<Req, String>(&req, "app"));
                match versions.delete_app(&project_id, &version, &app, &principal).await {
                    Ok(()) => empty_response(StatusCode::NO_CONTENT),
                    Err(e) => api_error_response(&api_err(&e, &principal)),
                }
            }
        },
    );

    // PUT /api/v1/projects/{project_id}/versions/{version}/lock —— 不可逆锁定。
    let versions_lock = versions.clone();
    let auth_lock = auth.clone();
    server.serve(
        "/api/v1/projects/{project_id}/versions/{version}/lock",
        Method::PUT,
        move |req: Req| {
            let versions = versions_lock.clone();
            let auth = auth_lock.clone();
            async move {
                let principal = crate::api_try!(auth.current_principal_req(&req).await);
                let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
                let version = crate::api_try!(path_param::<Req, String>(&req, "version"));
                match versions.lock(&project_id, &version, &principal).await {
                    Ok(record) => json_ok(StatusCode::OK, &record),
                    Err(e) => api_error_response(&api_err(&e, &principal)),
                }
            }
        },
    );

    let versions_list = versions.clone();
    let auth_list = auth.clone();
    server.serve("/api/v1/projects/{project_id}/versions", Method::GET, move |req: Req| {
        let versions = versions_list.clone();
        let auth = auth_list.clone();
        async move {
            let principal = crate::api_try!(auth.current_principal_req(&req).await);
            let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
            match versions.list(&project_id, &principal).await {
                Ok(list) => json_ok(StatusCode::OK, &list),
                Err(e) => api_error_response(&api_err(&e, &principal)),
            }
        }
    });

    let versions_get = versions.clone();
    let auth_get = auth.clone();
    server.serve("/api/v1/projects/{project_id}/versions/{version}", Method::GET, move |req: Req| {
        let versions = versions_get.clone();
        let auth = auth_get.clone();
        async move {
            let principal = crate::api_try!(auth.current_principal_req(&req).await);
            let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
            let version = crate::api_try!(path_param::<Req, String>(&req, "version"));
            let version = if version == "latest" { None } else { Some(version.as_str()) };
            match versions.get(&project_id, version, &principal).await {
                Ok(record) => json_ok(StatusCode::OK, &record),
                Err(e) => api_error_response(&api_err(&e, &principal)),
            }
        }
    });

    let versions_download = versions.clone();
    let files_download = files.clone();
    let auth_download = auth.clone();
    server.serve("/api/v1/projects/{project_id}/versions/{version}/download", Method::GET, move |req: Req| {
        let versions = versions_download.clone();
        let files = files_download.clone();
        let auth = auth_download.clone();
        async move {
            let principal = crate::api_try!(auth.current_principal_req(&req).await);
            let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
            let version = crate::api_try!(path_param::<Req, String>(&req, "version"));
            let version = if version == "latest" { None } else { Some(version.as_str()) };
            let query: DownloadQuery = match req.query() {
                Ok(query) => query,
                Err(e) => return api_error_response(&ApiError::invalid_input(format!("invalid query string: {e}"))),
            };
            let record = match versions.get(&project_id, version, &principal).await {
                Ok(record) => record,
                Err(e) => return api_error_response(&api_err(&e, &principal)),
            };
            let app = match query.app.as_deref() {
                Some(name) => match record.apps.iter().find(|a| a.app == name) {
                    Some(app) => app,
                    None => {
                        return api_error_response(&ApiError::not_found(format!(
                            "app {name} not found in version {}",
                            record.version
                        )));
                    }
                },
                None => match record.apps.as_slice() {
                    [single] => single,
                    [] => {
                        return api_error_response(&ApiError::not_found(
                            "version has no apps to download",
                        ));
                    }
                    _ => {
                        return api_error_response(&ApiError::invalid_input(
                            "app query parameter required when version has multiple apps",
                        ));
                    }
                },
            };
            let filename = format!("{}-{}-{}", project_id.0, record.version, app.app);
            crate::storage::http::download_response(&*files, &app.file_id, &filename).await
        }
    });
}

#[derive(serde::Deserialize)]
pub struct CreateVersionRequest {
    pub version: String,
}

#[derive(serde::Deserialize)]
pub struct DownloadQuery {
    #[serde(default)]
    pub app: Option<String>,
}

fn api_err(err: &VersionError, principal: &Principal) -> ApiError {
    if matches!(principal, Principal::Anonymous) && err.kind == VersionErrorKind::Forbidden {
        ApiError::unauthorized(&err.message)
    } else {
        version_error_to_api(err)
    }
}

fn version_error_to_api(err: &VersionError) -> ApiError {
    match err.kind {
        VersionErrorKind::NotFound => ApiError::not_found(&err.message),
        VersionErrorKind::Forbidden => ApiError::forbidden(&err.message),
        VersionErrorKind::Conflict => ApiError::conflict(&err.message),
        VersionErrorKind::InvalidInput => ApiError::invalid_input(&err.message),
        VersionErrorKind::Db => ApiError::server(&err.message),
    }
}

pub fn multipart_boundary<Req: Request>(req: &Req) -> Option<String> {
    let header = req.header(http::header::CONTENT_TYPE)?;
    let content_type = header.to_str().ok()?;
    if !content_type.to_ascii_lowercase().starts_with("multipart/form-data") {
        return None;
    }
    for param in split_semicolons(content_type) {
        if let Some(value) = param.trim().strip_prefix("boundary=") {
            let value = value.trim_matches('"');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn split_semicolons(value: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, byte) in value.bytes().enumerate() {
        if byte == b';' {
            out.push(&value[start..i]);
            start = i + 1;
        }
    }
    out.push(&value[start..]);
    out
}

fn parse_multipart(body: &[u8], boundary: &str) -> Result<HashMap<String, Vec<u8>>, String> {
    let delimiter = format!("\r\n--{boundary}");
    let delimiter_bytes = delimiter.as_bytes();
    let opening = format!("--{boundary}\r\n");
    let first = find_subslice(body, opening.as_bytes())
        .ok_or_else(|| "multipart boundary not found".to_string())?;
    let mut parts: HashMap<String, Vec<u8>> = HashMap::new();
    let mut pos = first + opening.len();
    loop {
        let Some(rel_end) = find_subslice(&body[pos..], delimiter_bytes) else {
            break;
        };
        let part_end = pos + rel_end;
        let block = &body[pos..part_end];
        let (headers, content) = split_headers_content(block);
        let name = header_param(&headers, "name")?;
        parts.insert(name, content.to_vec());
        pos = part_end + delimiter_bytes.len();
        if body.len() >= pos + 2 && &body[pos..pos + 2] == b"--" {
            break;
        }
    }
    if parts.is_empty() {
        return Err("multipart body has no parts".to_string());
    }
    Ok(parts)
}

fn split_headers_content(block: &[u8]) -> (Vec<String>, &[u8]) {
    if let Some(idx) = find_subslice(block, b"\r\n\r\n") {
        let headers = String::from_utf8_lossy(&block[..idx])
            .lines()
            .map(|l| l.to_string())
            .collect();
        let content_start = idx + 4;
        let mut end = block.len();
        while end > content_start && (block[end - 1] == b'\r' || block[end - 1] == b'\n') {
            end -= 1;
        }
        (headers, &block[content_start..end])
    } else {
        (Vec::new(), block)
    }
}

fn header_param(headers: &[String], key: &str) -> Result<String, String> {
    for header in headers {
        if header.to_ascii_lowercase().starts_with("content-disposition:") {
            let value = header.splitn(2, ':').nth(1).unwrap_or("");
            for part in split_semicolons(value) {
                let part = part.trim();
                if let Some(v) = part.strip_prefix(&format!("{key}=")) {
                    let v = v.trim_matches('"');
                    if v.is_empty() {
                        return Err(format!("empty {key}"));
                    }
                    return Ok(v.to_string());
                }
            }
        }
    }
    Err(format!("missing {key} in multipart part header"))
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
