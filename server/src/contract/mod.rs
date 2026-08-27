//! contract 子模块：v1 DTO/错误映射/统一响应与凭据解析（P-07 fh-server-http）。

use http::{HeaderName, HeaderValue, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use sfo_http::errors::{ErrorCode, HttpResult};
use sfo_http::http_server::{Request, Response};
use sfo_result::Error as SfoError;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::str::FromStr;

use crate::permissions::model::{PermissionError, PermissionErrorKind};

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthorized", message)
    }
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", message)
    }
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_input", message)
    }
    pub fn server(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "server_error", message)
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}: {}",
            self.status.as_u16(),
            self.code,
            self.message
        )
    }
}

impl Error for ApiError {}

impl From<PermissionError> for ApiError {
    fn from(value: PermissionError) -> Self {
        match value.kind {
            PermissionErrorKind::NotFound => ApiError::not_found(value.message),
            PermissionErrorKind::Forbidden => ApiError::forbidden(value.message),
            PermissionErrorKind::Db => ApiError::server(value.message),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(value: sqlx::Error) -> Self {
        ApiError::server(value.to_string())
    }
}

/// 把错误渲染为 ApiError 的统一 JSON body。
pub fn api_error_response<Resp: Response>(err: &ApiError) -> HttpResult<Resp> {
    let mut resp = Resp::new(err.status);
    resp.set_content_type("application/json").map_err(|e| {
        SfoError::new(
            ErrorCode::ServerError,
            format!("set content type failed: {e}"),
        )
    })?;
    let body =
        serde_json::to_vec(&json!({ "error": err.code, "message": err.message })).map_err(|e| {
            SfoError::new(
                ErrorCode::ServerError,
                format!("encode error body failed: {e}"),
            )
        })?;
    resp.set_body(body);
    Ok(resp)
}

/// 成功 JSON 响应。
pub fn json_ok<Resp: Response, T: Serialize>(status: StatusCode, value: &T) -> HttpResult<Resp> {
    let body = serde_json::to_vec(value).map_err(|e| {
        SfoError::new(
            ErrorCode::ServerError,
            format!("encode response failed: {e}"),
        )
    })?;
    let mut resp = Resp::new(status);
    resp.set_content_type("application/json").map_err(|e| {
        SfoError::new(
            ErrorCode::ServerError,
            format!("set content type failed: {e}"),
        )
    })?;
    resp.set_body(body);
    Ok(resp)
}

/// 空 body/特定状态响应（如 204）。
pub fn empty_response<Resp: Response>(status: StatusCode) -> HttpResult<Resp> {
    let resp = Resp::new(status);
    Ok(resp)
}

/// 从路径参数解析值。
pub fn path_param<Req: Request, T: FromStr>(req: &Req, key: &str) -> Result<T, ApiError> {
    let raw = req
        .param(key)
        .map_err(|_| ApiError::not_found(format!("missing path parameter {key}")))?;
    raw.parse::<T>()
        .map_err(|_| ApiError::invalid_input(format!("invalid path parameter {key}")))
}

/// 解析 JSON body。
pub async fn json_body<Req: Request, T: DeserializeOwned>(req: &mut Req) -> Result<T, ApiError> {
    req.body_json::<T>()
        .await
        .map_err(|e| ApiError::invalid_input(format!("invalid JSON body: {e}")))
}

/// 从 Authorization: Bearer 头取凭据；无头返回 None，非 Bearer 返回 401。
pub fn optional_bearer<Req: Request>(req: &Req) -> Result<Option<String>, ApiError> {
    let Some(header) = req.header(HeaderName::from_static("authorization")) else {
        return Ok(None);
    };
    let Ok(value) = header.to_str() else {
        return Err(ApiError::unauthorized("invalid authorization header"));
    };
    let value = value.trim();
    let Some(rest) = value.strip_prefix("Bearer ") else {
        return Err(ApiError::unauthorized(
            "authorization header must use Bearer scheme",
        ));
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return Err(ApiError::unauthorized("empty bearer credential"));
    }
    Ok(Some(rest.to_string()))
}

#[allow(dead_code)]
fn _marker_misc<T>() -> PhantomData<T> {
    PhantomData
}

pub struct Bearer(pub String);
#[async_trait::async_trait]
pub trait SessionAuth: 'static + Send + Sync {
    async fn decode_user(&self, bearer: &str) -> Option<crate::model::UserId>;
}

#[async_trait::async_trait]
pub trait TokenAuth: 'static + Send + Sync {
    async fn resolve_token(&self, bearer: &str) -> Option<crate::model::Principal>;
}

/// 认证桥：由 http 装配层实现，业务子模块只依赖本 provider 解析 Principal。
pub struct AuthProvider {
    pub session_auth: std::sync::Arc<dyn SessionAuth>,
    pub token_auth: std::sync::Arc<dyn TokenAuth>,
}

impl AuthProvider {
    pub async fn current_principal(
        &self,
        bearer: Option<&str>,
    ) -> Result<crate::model::Principal, ApiError> {
        match bearer {
            None => Ok(crate::model::Principal::Anonymous),
            Some(bearer) => {
                if let Some(user_id) = self.session_auth.decode_user(bearer).await {
                    return Ok(crate::model::Principal::User { user_id });
                }
                match self.token_auth.resolve_token(bearer).await {
                    Some(principal) => Ok(principal),
                    None => Err(ApiError::unauthorized(
                        "invalid session or token credential",
                    )),
                }
            }
        }
    }

    pub async fn current_principal_req<Req: Request>(
        &self,
        req: &Req,
    ) -> Result<crate::model::Principal, ApiError> {
        let bearer = optional_bearer(req)?;
        self.current_principal(bearer.as_deref()).await
    }

    pub async fn require_user_req<Req: Request>(
        &self,
        req: &Req,
    ) -> Result<crate::model::Principal, ApiError> {
        match self.current_principal_req(req).await? {
            principal @ crate::model::Principal::User { .. } => Ok(principal),
            crate::model::Principal::Anonymous => Err(ApiError::unauthorized("login required")),
            crate::model::Principal::Token { .. } => Err(ApiError::forbidden(
                "user session required for this operation",
            )),
        }
    }

    pub async fn require_user_or_token_req<Req: Request>(
        &self,
        req: &Req,
    ) -> Result<crate::model::Principal, ApiError> {
        match self.current_principal_req(req).await? {
            crate::model::Principal::Anonymous => Err(ApiError::unauthorized("login required")),
            principal => Ok(principal),
        }
    }
}

/// 与设计签名一致：异步取 Authorization Bearer 凭据（内部无阻塞 IO）。
pub async fn extract_bearer<Req: Request>(req: &Req) -> Result<Bearer, ApiError> {
    Ok(Bearer(optional_bearer(req)?.unwrap_or_default()))
}

pub fn set_download_headers<Resp: Response>(resp: &mut Resp, name: &str) -> Result<(), ApiError> {
    resp.insert_header(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("application/gzip"),
    );
    resp.insert_header(
        HeaderName::from_static("content-disposition"),
        HeaderValue::from_str(&format!(
            "attachment; filename=\"{}.tar.gz\"",
            escape_disposition_filename(name)
        ))
            .map_err(|e| ApiError::server(format!("invalid disposition header: {e}")))?,
    );
    Ok(())
}

/// 按 RFC 9110 quoted-string 规则转义下载文件名：`\` -> `\\`、`"` -> `\"`，
/// 并移除 HeaderValue 无法承载的控制字符。正常名称输出与原始值逐字一致。
fn escape_disposition_filename(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

pub fn header_value(value: &str) -> HeaderValue {
    HeaderValue::from_str(value).unwrap_or_else(|_| HeaderValue::from_static(""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sfo_http::http_server::HttpBody;
    use tokio::io::AsyncRead;

    struct TestResponse {
        disposition: Option<HeaderValue>,
    }

    impl TestResponse {
        fn empty() -> Self {
            Self { disposition: None }
        }
    }

    impl Response for TestResponse {
        fn from_result<
            T: Serialize,
            C: std::fmt::Debug + Copy + Sync + Send + 'static + Into<u16>,
        >(
            _ret: sfo_result::Result<T, C>,
        ) -> Self {
            Self::empty()
        }

        fn new(_status: StatusCode) -> Self {
            Self::empty()
        }

        fn insert_header(&mut self, name: HeaderName, value: HeaderValue) {
            if name == HeaderName::from_static("content-disposition") {
                self.disposition = Some(value);
            }
        }

        fn set_content_type(&mut self, _content_type: &str) -> HttpResult<()> {
            Ok(())
        }

        fn set_body(&mut self, _body: Vec<u8>) {}

        fn set_body_read<R: AsyncRead + Send + Unpin + 'static>(&mut self, _reader: R) {}

        fn set_http_body(&mut self, _body: HttpBody) {}
    }

    fn disposition(resp: &TestResponse) -> &HeaderValue {
        resp.disposition
            .as_ref()
            .expect("content-disposition header present")
    }

    #[test]
    fn download_disposition_escapes_quoted_string_metacharacters() {
        let mut resp = TestResponse::empty();
        set_download_headers(&mut resp, r#"1.0"x\y"#).expect("disposition header generated");
        assert_eq!(
            disposition(&resp).to_str().expect("header str"),
            r#"attachment; filename="1.0\"x\\y.tar.gz""#
        );
    }

    #[test]
    fn download_disposition_drops_control_characters_and_keeps_normal_names() {
        let mut weird = TestResponse::empty();
        set_download_headers(&mut weird, "1.0\ndev\x7f").expect("disposition header generated");
        assert_eq!(
            disposition(&weird).to_str().expect("header str"),
            r#"attachment; filename="1.0dev.tar.gz""#
        );

        let mut normal = TestResponse::empty();
        set_download_headers(&mut normal, "123-1.0.0-ui").expect("disposition header generated");
        assert_eq!(
            disposition(&normal).to_str().expect("header str"),
            r#"attachment; filename="123-1.0.0-ui.tar.gz""#
        );
    }
}
