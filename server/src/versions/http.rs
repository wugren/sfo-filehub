//! versions 的 HTTP 接口：版本创建/锁定、app 发布/更新/删除、列表/单查/latest 与按 app 下载（I-006 修订）。

use http::{Method, StatusCode};
use http_body_util::BodyExt;
use sfo_http::http_server::{Request, Response};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

use crate::contract::{
    ApiError, AuthProvider, api_error_response, empty_response, json_body, json_ok, path_param,
};
use crate::http::authz::authz_project_action;
use crate::model::{Principal, ProjectId};
use crate::permissions::PermissionChecker;
use crate::permissions::checker::ACTION_ARTIFACTS_WRITE;
use crate::storage::FileStore;
use crate::storage::UploadStream;
use crate::versions::upload::{MultipartEvent, MultipartParser, UploadLimits};
use crate::versions::{VersionError, VersionErrorKind, VersionService};

pub fn register<S, Req, Resp>(
    server: &mut S,
    versions: Arc<dyn VersionService>,
    files: Arc<dyn FileStore>,
    auth: Arc<AuthProvider>,
    checker: Arc<dyn PermissionChecker>,
    max_archive_bytes: u64,
) where
    S: sfo_http::http_server::HttpServer<Req, Resp>,
    Req: Request + Sync,
    Resp: Response,
{
    let upload_limits = UploadLimits {
        max_archive_bytes,
        max_field_bytes: 256,
        max_header_bytes: 8 * 1024,
        max_total_bytes: max_archive_bytes.saturating_add(1024 * 1024),
    };
    // POST /api/v1/projects/{project_id}/versions —— 显式创建版本。
    let versions_create = versions.clone();
    let auth_create = auth.clone();
    server.serve(
        "/api/v1/projects/{project_id}/versions",
        Method::POST,
        move |mut req: Req| {
            let versions = versions_create.clone();
            let auth = auth_create.clone();
            async move {
                let principal = crate::api_try!(auth.current_principal_req(&req).await);
                let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
                let body = crate::api_try!(json_body::<Req, CreateVersionRequest>(&mut req).await);
                match versions
                    .create_version(&project_id, &body.version, &principal)
                    .await
                {
                    Ok(record) => json_ok(StatusCode::CREATED, &record),
                    Err(e) => api_error_response(&api_err(&e, &principal)),
                }
            }
        },
    );

    // PUT /api/v1/projects/{project_id}/versions/{version}/apps/{app} —— 发布/更新 app。
    let versions_put = versions.clone();
    let files_put = files.clone();
    let auth_put = auth.clone();
    let checker_put = checker.clone();
    let upload_hook = authz_project_action(auth_put, checker_put, ACTION_ARTIFACTS_WRITE, {
        let versions = versions_put.clone();
        let files = files_put.clone();
        let limits = upload_limits;
        move |principal, mut req: Req| {
            let versions = versions.clone();
            let files = files.clone();
            let limits = limits;
            async move {
                let project_id = match path_param::<Req, ProjectId>(&req, "project_id") {
                    Ok(id) => id,
                    Err(error) => return api_error_response(&error),
                };
                let version = match path_param::<Req, String>(&req, "version") {
                    Ok(v) => v,
                    Err(error) => return api_error_response(&error),
                };
                let app = match path_param::<Req, String>(&req, "app") {
                    Ok(v) => v,
                    Err(error) => return api_error_response(&error),
                };
                // Content-Length 预检：超大请求在读取 body 前直接拒绝。
                if let Some(len) = req
                    .header(http::header::CONTENT_LENGTH)
                    .and_then(|v| v.to_str().ok().map(|s| s.to_string()))
                    .and_then(|s| s.parse::<u64>().ok())
                {
                    if len > limits.max_total_bytes {
                        return api_error_response(&ApiError::invalid_input(format!(
                            "request body exceeds upload limit ({})",
                            limits.max_total_bytes
                        )));
                    }
                }
                let mut body = match req.take_http_body() {
                    Ok(body) => body,
                    Err(e) => {
                        return api_error_response(&ApiError::invalid_input(format!(
                            "read body failed: {e}"
                        )));
                    }
                };
                let boundary = match multipart_boundary(&req) {
                    Some(boundary) => boundary,
                    None => {
                        return api_error_response(&ApiError::invalid_input(
                            "multipart/form-data required with boundary",
                        ));
                    }
                };
                let mut parser = MultipartParser::new(&boundary, limits);
                let (reader, mut writer) = tokio::io::duplex(64 * 1024);
                let ingest_task = tokio::spawn({
                    let files = files.clone();
                    async move { files.ingest(UploadStream::from_reader(reader), None).await }
                });

                let mut sha256: Option<String> = None;
                let mut upload_error: Option<String> = None;
                'feed: while let Some(frame) = body.frame().await {
                    let frame = match frame {
                        Ok(frame) => frame,
                        Err(e) => {
                            upload_error = Some(format!("read body failed: {e}"));
                            break;
                        }
                    };
                    if !frame.is_data() {
                        upload_error =
                            Some("trailer frames are not supported for uploads".to_string());
                        break;
                    }
                    let bytes = match frame.into_data() {
                        Ok(bytes) => bytes,
                        Err(_) => {
                            upload_error = Some("invalid body data frame".to_string());
                            break;
                        }
                    };
                    let events = match parser.feed(&bytes) {
                        Ok(events) => events,
                        Err(e) => {
                            upload_error = Some(e);
                            break;
                        }
                    };
                    for event in events {
                        match event {
                            MultipartEvent::FileChunk(chunk) => {
                                if let Err(e) = writer.write_all(&chunk).await {
                                    upload_error = Some(format!("upload stream closed: {e}"));
                                    break 'feed;
                                }
                            }
                            MultipartEvent::Field { name, value } if name == "sha256" => {
                                sha256 = Some(value);
                            }
                            MultipartEvent::Field { .. } => {}
                        }
                    }
                    if upload_error.is_some() {
                        break;
                    }
                }
                if upload_error.is_none() {
                    if let Err(e) = parser.finish() {
                        upload_error = Some(e);
                    }
                }
                drop(writer);
                let joined = match ingest_task.await {
                    Ok(outcome) => outcome,
                    Err(e) => {
                        return api_error_response(&ApiError::server(format!(
                            "ingest task failed: {e}"
                        )));
                    }
                };
                if let Some(message) = upload_error {
                    // 文件流可能已在 ingest 中完成落库（joined == Ok(file)），
                    // 请求解析失败时同样需要 discard，避免残留孤儿文件与 DB 记录。
                    if let Ok(file) = &joined {
                        let _ = files.discard(&file.file_id).await;
                    }
                    return api_error_response(&ApiError::invalid_input(message));
                }
                let file = match joined {
                    Ok(file) => file,
                    Err(e) => return api_error_response(&e.into()),
                };
                let expected = match sha256 {
                    Some(value) if is_sha256_hex(&value) => value,
                    _ => {
                        let _ = files.discard(&file.file_id).await;
                        return api_error_response(&ApiError::invalid_input(
                            "sha256 field is required (64 hex chars)",
                        ));
                    }
                };
                if !file.sha256.eq_ignore_ascii_case(&expected) {
                    let _ = files.discard(&file.file_id).await;
                    return api_error_response(&ApiError::invalid_input("sha256 mismatch"));
                }
                match versions
                    .publish_app(&project_id, &version, &app, file.clone(), &principal)
                    .await
                {
                    Ok(outcome) => {
                        let status = if outcome.created {
                            StatusCode::CREATED
                        } else {
                            StatusCode::OK
                        };
                        json_ok(status, &outcome.record)
                    }
                    Err(e) => {
                        let _ = files.discard(&file.file_id).await;
                        api_error_response(&api_err(&e, &principal))
                    }
                }
            }
        }
    });
    server.serve(
        "/api/v1/projects/{project_id}/versions/{version}/apps/{app}",
        Method::PUT,
        upload_hook,
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
                match versions
                    .delete_app(&project_id, &version, &app, &principal)
                    .await
                {
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
    server.serve(
        "/api/v1/projects/{project_id}/versions",
        Method::GET,
        move |req: Req| {
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
        },
    );

    let versions_get = versions.clone();
    let auth_get = auth.clone();
    server.serve(
        "/api/v1/projects/{project_id}/versions/{version}",
        Method::GET,
        move |req: Req| {
            let versions = versions_get.clone();
            let auth = auth_get.clone();
            async move {
                let principal = crate::api_try!(auth.current_principal_req(&req).await);
                let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
                let version = crate::api_try!(path_param::<Req, String>(&req, "version"));
                let version = if version == "latest" {
                    None
                } else {
                    Some(version.as_str())
                };
                match versions.get(&project_id, version, &principal).await {
                    Ok(record) => json_ok(StatusCode::OK, &record),
                    Err(e) => api_error_response(&api_err(&e, &principal)),
                }
            }
        },
    );

    let versions_download = versions.clone();
    let files_download = files.clone();
    let auth_download = auth.clone();
    server.serve(
        "/api/v1/projects/{project_id}/versions/{version}/download",
        Method::GET,
        move |req: Req| {
            let versions = versions_download.clone();
            let files = files_download.clone();
            let auth = auth_download.clone();
            async move {
                let principal = crate::api_try!(auth.current_principal_req(&req).await);
                let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
                let version = crate::api_try!(path_param::<Req, String>(&req, "version"));
                let version = if version == "latest" {
                    None
                } else {
                    Some(version.as_str())
                };
                let query: DownloadQuery = match req.query() {
                    Ok(query) => query,
                    Err(e) => {
                        return api_error_response(&ApiError::invalid_input(format!(
                            "invalid query string: {e}"
                        )));
                    }
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
        },
    );
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
    if !content_type
        .to_ascii_lowercase()
        .starts_with("multipart/form-data")
    {
        return None;
    }
    for param in content_type.split(';') {
        if let Some(value) = param.trim().strip_prefix("boundary=") {
            let value = value.trim_matches('"');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// 上传协议必填的 sha256 字段：必须恰为 64 位十六进制字符。
fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
