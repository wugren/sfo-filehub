//! v1 API 传输：DTO、HTTP、Bearer 注入与 401 续期重试（一次）。

use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use reqwest::StatusCode;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

pub mod contract;
pub mod error;

use contract::{ApiErrorBody, LoginRequest, LoginResp, ProjectDto, SfoEnvelope, VersionDto};
pub use error::ClientError;

use crate::credential_store::{Credential, CredentialStore, normalize_server};

/// 客户端传输配置；`base_url` 兼容旧调用，按服务器身份归一化。
#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub timeout: Duration,
    pub connect_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            base_url: "https://filehub.example.com".to_string(),
            timeout: Duration::from_secs(60),
            connect_timeout: Duration::from_secs(10),
        }
    }
}

/// v1 API 传输（无持久状态）。
#[derive(Debug, Clone)]
pub struct FilehubClient {
    cfg: Config,
    http: reqwest::Client,
    endpoint_bases: Vec<String>,
}

impl FilehubClient {
    pub fn new(cfg: Config) -> Result<Self, ClientError> {
        let identity = normalize_server(&cfg.base_url).map_err(ClientError::from)?;
        let endpoint_bases = endpoint_candidates(&identity);
        let primary = endpoint_bases
            .first()
            .cloned()
            .ok_or_else(|| ClientError::Local("server has no available endpoints".to_string()))?;
        let http = reqwest::Client::builder()
            .timeout(cfg.timeout)
            .connect_timeout(cfg.connect_timeout)
            .user_agent("filehub-cli/0.1")
            .build()
            .map_err(|e| ClientError::Local(format!("failed to initialize HTTP client: {e}")))?;
        Ok(FilehubClient {
            cfg: Config {
                base_url: primary,
                ..cfg
            },
            http,
            endpoint_bases,
        })
    }

    pub fn base_url(&self) -> &str {
        self.endpoint_bases
            .first()
            .map(String::as_str)
            .unwrap_or(&self.cfg.base_url)
    }

    /// 发送请求；同一逻辑请求在主端点连接失败时按候选顺序降级（HTTPS -> loopback HTTP）。
    async fn send_with_fallback<F, Fut>(
        &self,
        fail_prefix: &str,
        mut build: F,
    ) -> Result<reqwest::Response, ClientError>
    where
        F: FnMut(&str) -> Fut,
        Fut: Future<Output = Result<reqwest::RequestBuilder, ClientError>>,
    {
        let mut last_error: Option<reqwest::Error> = None;
        for base in &self.endpoint_bases {
            let builder = build(base).await?;
            match builder.send().await {
                Ok(response) => return Ok(response),
                Err(error) => last_error = Some(error),
            }
        }
        Err(ClientError::Transport(format!(
            "{fail_prefix} (network/connection): {}",
            last_error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "no available endpoints".to_string())
        )))
    }

    pub async fn login_password(
        &self,
        user: &str,
        password: &str,
    ) -> Result<LoginResp, ClientError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let body = LoginRequest {
            user_name: user,
            password,
            timestamp,
        };
        let response = self
            .send_with_fallback("sign-in request failed", |base| {
                let base = base.to_string();
                let body = body.clone();
                async move { Ok(self.http.post(format!("{base}/account/login")).json(&body)) }
            })
            .await?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| ClientError::Transport(format!("failed to read sign-in response: {e}")))?;
        if !status.is_success() {
            return Err(classify_status(status, &bytes));
        }
        let wrapped: SfoEnvelope<LoginResp> = serde_json::from_slice(&bytes).map_err(|e| {
            ClientError::Transport(format!("failed to parse sign-in response: {e}"))
        })?;
        if wrapped.err != 0 {
            return Err(ClientError::Auth(format!(
                "sign-in failed (server err={})",
                wrapped.err
            )));
        }
        wrapped.result.ok_or_else(|| {
            ClientError::Transport("sign-in response is missing the result field".to_string())
        })
    }

    pub async fn refresh_session(&self, refresh: &str) -> Result<LoginResp, ClientError> {
        let response = self
            .send_with_fallback("session refresh request failed", |base| {
                let base = base.to_string();
                async move {
                    Ok(self
                        .http
                        .post(format!("{base}/account/refresh_session"))
                        .bearer_auth(refresh))
                }
            })
            .await?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|e| {
            ClientError::Transport(format!("failed to read session refresh response: {e}"))
        })?;
        if !status.is_success() {
            return Err(classify_status(status, &bytes));
        }
        let wrapped: SfoEnvelope<LoginResp> = serde_json::from_slice(&bytes).map_err(|e| {
            ClientError::Transport(format!("failed to parse session refresh response: {e}"))
        })?;
        if wrapped.err != 0 {
            return Err(ClientError::Auth(
                "session refresh failed because the refresh flag is invalid".to_string(),
            ));
        }
        wrapped.result.ok_or_else(|| {
            ClientError::Transport(
                "session refresh response is missing the result field".to_string(),
            )
        })
    }

    /// 拉取全部可见项目：按 `?limit/offset` 分页循环，用 `X-Total-Count` 驱动；
    /// 旧服务端/无总量头时回退单页（等价于既有语义）。
    pub async fn list_projects(&self, bearer: &str) -> Result<Vec<ProjectDto>, ClientError> {
        const PAGE_LIMIT: u32 = 500;
        let mut projects = Vec::new();
        let mut offset = 0u32;
        loop {
            let suffix = format!("/api/v1/projects?limit={PAGE_LIMIT}&offset={offset}");
            let (status, bytes, total) = self.get_json_page(&suffix, bearer).await?;
            let body = ensure_success(status, bytes.to_vec())?;
            let page: Vec<ProjectDto> = serde_json::from_slice(&body).map_err(|e| {
                ClientError::Transport(format!("failed to parse project list: {e}"))
            })?;
            let page_len = page.len() as u32;
            projects.extend(page);
            let Some(total) = total else {
                break;
            };
            if page_len == 0 || projects.len() as u64 >= total {
                break;
            }
            offset = offset.saturating_add(page_len);
        }
        Ok(projects)
    }

    /// 按项目名精确匹配解析项目；未找到/重名按 InvalidInput 处理。
    pub async fn resolve_project(
        &self,
        bearer: &str,
        name: &str,
    ) -> Result<ProjectDto, ClientError> {
        let projects = self.list_projects(bearer).await?;
        let mut matches: Vec<ProjectDto> =
            projects.into_iter().filter(|p| p.name == name).collect();
        if matches.is_empty() {
            return Err(ClientError::InvalidInput(format!(
                "project {name} does not exist or is not visible to the current identity"
            )));
        }
        if matches.len() > 1 {
            return Err(ClientError::InvalidInput(format!(
                "multiple projects named {name} exist on the server; target is ambiguous"
            )));
        }
        Ok(matches.remove(0))
    }

    /// 显式创建版本：JSON `{"version"}`；409 为终态冲突。
    pub async fn create_version(
        &self,
        bearer: &str,
        project_id: i64,
        version: &str,
    ) -> Result<VersionDto, ClientError> {
        let response = self
            .send_with_fallback("create-version request failed", |base| {
                let base = base.to_string();
                async move {
                    Ok(self
                        .http
                        .post(format!("{base}/api/v1/projects/{project_id}/versions"))
                        .bearer_auth(bearer)
                        .json(&serde_json::json!({ "version": version })))
                }
            })
            .await?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|e| {
            ClientError::Transport(format!("failed to read create-version response: {e}"))
        })?;
        let body = ensure_success(status, bytes.to_vec())?;
        serde_json::from_slice(&body).map_err(|e| {
            ClientError::Transport(format!("failed to parse create-version response: {e}"))
        })
    }

    /// multipart 发布/更新 app：`PUT .../versions/{version}/apps/{app}` + `file` + `sha256`；
    /// 版本不存在 404，版本已锁定 409。
    pub async fn publish_app(
        &self,
        bearer: &str,
        project_id: i64,
        version: &str,
        app: &str,
        archive: &Path,
        sha256: &str,
    ) -> Result<VersionDto, ClientError> {
        let file_name = archive
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "payload.tar.gz".to_string());
        let response = self
            .send_with_fallback("publish request failed", |base| {
                let base = base.to_string();
                let file_name = file_name.clone();
                async move {
                    let file = tokio::fs::File::open(archive).await.map_err(|e| {
                        ClientError::Local(format!("failed to open archive for publishing: {e}"))
                    })?;
                    let part = reqwest::multipart::Part::stream(file)
                        .file_name(file_name)
                        .mime_str("application/gzip")
                        .map_err(|e| {
                            ClientError::Local(format!(
                                "failed to construct multipart request: {e}"
                            ))
                        })?;
                    let form = reqwest::multipart::Form::new()
                        .text("sha256", sha256.to_string())
                        .part("file", part);
                    Ok(self
                        .http
                        .put(format!(
                            "{base}/api/v1/projects/{project_id}/versions/{version}/apps/{app}"
                        ))
                        .bearer_auth(bearer)
                        .multipart(form))
                }
            })
            .await?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| ClientError::Transport(format!("failed to read publish response: {e}")))?;
        let body = ensure_success(status, bytes.to_vec())?;
        serde_json::from_slice(&body)
            .map_err(|e| ClientError::Transport(format!("failed to parse publish response: {e}")))
    }

    /// 删除版本内 app。
    pub async fn delete_app(
        &self,
        bearer: &str,
        project_id: i64,
        version: &str,
        app: &str,
    ) -> Result<(), ClientError> {
        let response = self
            .send_with_fallback("delete-app request failed", |base| {
                let base = base.to_string();
                async move {
                    Ok(self
                        .http
                        .delete(format!(
                            "{base}/api/v1/projects/{project_id}/versions/{version}/apps/{app}"
                        ))
                        .bearer_auth(bearer))
                }
            })
            .await?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|e| {
            ClientError::Transport(format!("failed to read delete-app response: {e}"))
        })?;
        let _ = ensure_success(status, bytes.to_vec())?;
        Ok(())
    }

    /// 锁定版本（不可逆）。
    pub async fn lock_version(
        &self,
        bearer: &str,
        project_id: i64,
        version: &str,
    ) -> Result<VersionDto, ClientError> {
        let response = self
            .send_with_fallback("lock-version request failed", |base| {
                let base = base.to_string();
                async move {
                    Ok(self
                        .http
                        .put(format!(
                            "{base}/api/v1/projects/{project_id}/versions/{version}/lock"
                        ))
                        .bearer_auth(bearer))
                }
            })
            .await?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|e| {
            ClientError::Transport(format!("failed to read lock-version response: {e}"))
        })?;
        let body = ensure_success(status, bytes.to_vec())?;
        serde_json::from_slice(&body).map_err(|e| {
            ClientError::Transport(format!("failed to parse lock-version response: {e}"))
        })
    }

    /// 版本元数据（`None` = latest，服务端最近发布语义）。
    pub async fn get_version(
        &self,
        bearer: &str,
        project_id: i64,
        version: Option<&str>,
    ) -> Result<VersionDto, ClientError> {
        let tail = version.unwrap_or("latest");
        let (status, bytes) = self
            .get_json(
                &format!("/api/v1/projects/{project_id}/versions/{tail}"),
                bearer,
            )
            .await?;
        let body = ensure_success(status, bytes)?;
        serde_json::from_slice(&body).map_err(|e| {
            ClientError::Transport(format!("failed to parse version information: {e}"))
        })
    }

    /// 版本列表。
    pub async fn list_versions(
        &self,
        bearer: &str,
        project_id: i64,
    ) -> Result<Vec<VersionDto>, ClientError> {
        let (status, bytes) = self
            .get_json(&format!("/api/v1/projects/{project_id}/versions"), bearer)
            .await?;
        let body = ensure_success(status, bytes)?;
        serde_json::from_slice(&body)
            .map_err(|e| ClientError::Transport(format!("failed to parse version list: {e}")))
    }

    /// 流式下载到临时文件；失败清理由调用方负责。
    pub async fn download(
        &self,
        bearer: &str,
        project_id: i64,
        version: Option<&str>,
        app: &str,
        tmp: &Path,
    ) -> Result<(), ClientError> {
        let tail = version.unwrap_or("latest");
        let response = self
            .send_with_fallback("download request failed", |base| {
                let base = base.to_string();
                async move {
                    Ok(self
                        .http
                        .get(format!(
                            "{base}/api/v1/projects/{project_id}/versions/{tail}/download"
                        ))
                        .bearer_auth(bearer)
                        .query(&[("app", app)]))
                }
            })
            .await?;
        let status = response.status();
        if !status.is_success() {
            let bytes = response.bytes().await.unwrap_or_default();
            return Err(classify_status(status, &bytes));
        }
        let mut body = response
            .error_for_status()
            .map_err(|e| ClientError::Transport(format!("download transport failed: {e}")))?
            .bytes_stream();
        let mut file = tokio::fs::File::create(tmp).await.map_err(|e| {
            ClientError::Local(format!("failed to create temporary download file: {e}"))
        })?;
        use futures_util::StreamExt;
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(|e| {
                ClientError::Transport(format!("failed to read download stream: {e}"))
            })?;
            file.write_all(&chunk).await.map_err(|e| {
                ClientError::Local(format!("failed to write temporary download file: {e}"))
            })?;
        }
        file.flush().await.map_err(|e| {
            ClientError::Local(format!("failed to flush temporary download file: {e}"))
        })?;
        Ok(())
    }

    async fn get_json(
        &self,
        suffix: &str,
        bearer: &str,
    ) -> Result<(StatusCode, Vec<u8>), ClientError> {
        let response = self
            .send_with_fallback("request failed", |base| {
                let base = base.to_string();
                async move { Ok(self.http.get(format!("{base}{suffix}")).bearer_auth(bearer)) }
            })
            .await?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| ClientError::Transport(format!("failed to read response: {e}")))?;
        Ok((status, bytes.to_vec()))
    }

    /// 与 `get_json` 相同，额外解析 `X-Total-Count` 响应头（缺失/非法为 None）。
    async fn get_json_page(
        &self,
        suffix: &str,
        bearer: &str,
    ) -> Result<(StatusCode, Vec<u8>, Option<u64>), ClientError> {
        let response = self
            .send_with_fallback("request failed", |base| {
                let base = base.to_string();
                async move { Ok(self.http.get(format!("{base}{suffix}")).bearer_auth(bearer)) }
            })
            .await?;
        let total = response
            .headers()
            .get("x-total-count")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| ClientError::Transport(format!("failed to read response: {e}")))?;
        Ok((status, bytes.to_vec(), total))
    }
}

/// 已准备请求：目标服务器 + Bearer + 凭据类型（供 401 续期判断）。
pub struct Prepared {
    pub server: String,
    pub bearer: String,
    pub credential: Credential,
}

/// 带凭据与 401 续期重试的认证客户端。
pub struct AuthClient {
    pub transport: FilehubClient,
    pub store: Arc<RwLock<CredentialStore>>,
}

impl AuthClient {
    pub fn new(transport: FilehubClient, store: Arc<RwLock<CredentialStore>>) -> Self {
        AuthClient { transport, store }
    }

    /// 解析服务器并取 Bearer（token > session）。
    pub async fn prepare(&self, server: Option<&str>) -> Result<Prepared, ClientError> {
        let env_server = std::env::var("FILEHUB_SERVER").ok();
        let store = self.store.read().await;
        let server_url = store.resolve_server(server, env_server.as_deref())?;
        let credential = store.current_credential(&server_url).ok_or_else(|| {
            ClientError::Auth(format!(
                "not signed in to {server_url}; run filehub login or provide a valid token"
            ))
        })?;
        let bearer = match &credential {
            Credential::PasswordSession { session, .. } => session.clone(),
            Credential::Token { token } => token.clone(),
        };
        Ok(Prepared {
            server: server_url,
            bearer,
            credential,
        })
    }

    /// 执行一次带 Bearer 的请求；session 凭据遇 401 时续期一次并重试。
    /// 流式下载的中途失败不重试由调用方控制（见 pull_handler）。
    pub async fn run_auth<T, F, Fut>(
        &self,
        server: Option<&str>,
        mut call: F,
    ) -> Result<T, ClientError>
    where
        F: FnMut(String) -> Fut,
        Fut: Future<Output = Result<T, ClientError>>,
    {
        let prepared = self.prepare(server).await?;
        match call(prepared.bearer.clone()).await {
            Err(ClientError::Auth(message)) => match &prepared.credential {
                Credential::PasswordSession {
                    refresh_session, ..
                } => {
                    let renewed = self.transport.refresh_session(refresh_session).await?;
                    let mut store = self.store.write().await;
                    store.update_session(
                        &prepared.server,
                        &renewed.session,
                        &renewed.refresh_session,
                    )?;
                    store.flush()?;
                    drop(store);
                    log::info!("session refreshed and request retried once");
                    call(renewed.session).await
                }
                Credential::Token { .. } => Err(ClientError::Auth(message)),
            },
            other => other,
        }
    }
}

/// 构造请求端点候选：HTTPS 优先；loopback 身份额外加入 HTTP 降级（Docker 默认）。
fn endpoint_candidates(identity: &str) -> Vec<String> {
    let mut endpoints = vec![format!("https://{identity}")];
    if is_loopback_identity(identity) {
        endpoints.push(format!("http://{identity}"));
    }
    endpoints
}

/// loopback 身份判定：localhost、127.0.0.0/8、::1（含 `[::1]:port` 形态）。
fn is_loopback_identity(identity: &str) -> bool {
    let host = identity
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']').map(|(host, _)| host))
        .unwrap_or_else(|| identity.split(':').next().unwrap_or(identity));
    let host = host.to_ascii_lowercase();
    host == "localhost" || host == "::1" || host.starts_with("127.")
}

fn ensure_success(status: StatusCode, body: Vec<u8>) -> Result<Vec<u8>, ClientError> {
    if status.is_success() {
        Ok(body)
    } else {
        Err(classify_status(status, &body))
    }
}

fn classify_status(status: StatusCode, body: &[u8]) -> ClientError {
    let parsed: Option<ApiErrorBody> = serde_json::from_slice(body).ok();
    let message = parsed
        .as_ref()
        .map(|api| api.message.clone())
        .unwrap_or_else(|| {
            String::from_utf8_lossy(body)
                .trim()
                .chars()
                .take(240)
                .collect::<String>()
        });
    let fallback = |kind: ClientError| {
        if message.is_empty() {
            kind
        } else {
            match kind {
                ClientError::Auth(_) => ClientError::Auth(message.clone()),
                ClientError::Forbidden(_) => ClientError::Forbidden(message.clone()),
                ClientError::NotFound(_) => ClientError::NotFound(message.clone()),
                ClientError::Conflict(_) => ClientError::Conflict(message.clone()),
                ClientError::InvalidInput(_) => ClientError::InvalidInput(message.clone()),
                _ => kind,
            }
        }
    };
    match status.as_u16() {
        401 => fallback(ClientError::Auth(
            "authentication failed (401): credentials are invalid or expired".to_string(),
        )),
        403 => fallback(ClientError::Forbidden(
            "the server rejected this operation (403)".to_string(),
        )),
        404 => fallback(ClientError::NotFound(
            "resource not found (404)".to_string(),
        )),
        409 => fallback(ClientError::Conflict(
            "request conflicts with existing server state (409)".to_string(),
        )),
        422 => fallback(ClientError::InvalidInput(
            "the server rejected the input (422)".to_string(),
        )),
        code => ClientError::Transport(format!("server transport/error (HTTP {code}): {message}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_prefer_https_and_loopback_gets_http_fallback() {
        assert_eq!(
            endpoint_candidates("registry.example.com"),
            vec!["https://registry.example.com".to_string()]
        );
        assert_eq!(
            endpoint_candidates("127.0.0.1:8080"),
            vec![
                "https://127.0.0.1:8080".to_string(),
                "http://127.0.0.1:8080".to_string()
            ]
        );
        assert_eq!(
            endpoint_candidates("localhost"),
            vec![
                "https://localhost".to_string(),
                "http://localhost".to_string()
            ]
        );
        assert_eq!(
            endpoint_candidates("[::1]:8080"),
            vec![
                "https://[::1]:8080".to_string(),
                "http://[::1]:8080".to_string()
            ]
        );
    }
}
