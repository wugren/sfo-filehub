//! tokens 的 HTTP 接口（sfo-http；P-03）。

use http::{Method, StatusCode};
use sfo_http::http_server::{HttpServer, Request, Response};
use serde::Deserialize;
use std::sync::Arc;

use crate::contract::{api_error_response, empty_response, json_body, json_ok, path_param, AuthProvider, ApiError};
use crate::model::{TokenId, UserId};
use super::model::{TokenCreateRequest, TokenError, TokenErrorKind, TokenUpdateRequest};
use super::TokenService;

pub fn register<S, Req, Resp>(server: &mut S, tokens: Arc<dyn TokenService>, auth: Arc<AuthProvider>)
where
    S: sfo_http::http_server::HttpServer<Req, Resp>,
    Req: Request + Sync,
    Resp: Response,
{
    let tokens_create = tokens.clone();
    let auth_create = auth.clone();
    server.serve("/api/v1/tokens", Method::POST, move |req: Req| {
        let tokens = tokens_create.clone();
        let auth = auth_create.clone();
        async move {
            let principal = crate::api_try!(auth.require_user_req(&req).await);
            let owner = crate::api_try!(require_owner(&principal));
            let mut req = req;
            let body: TokenCreateRequest = crate::api_try!(json_body(&mut req).await);
            let req = TokenCreateRequest { owner, ..body };
            match tokens.create(req).await {
                Ok(issued) => json_ok(StatusCode::CREATED, &issued),
                Err(e) => api_error_response(&token_error_to_api(&e)),
            }
        }
    });

    let tokens_list = tokens.clone();
    let auth_list = auth.clone();
    server.serve("/api/v1/tokens", Method::GET, move |req: Req| {
        let tokens = tokens_list.clone();
        let auth = auth_list.clone();
        async move {
            let principal = crate::api_try!(auth.require_user_req(&req).await);
            let owner = crate::api_try!(require_owner(&principal));
            match tokens.list(&owner).await {
                Ok(list) => json_ok(StatusCode::OK, &list),
                Err(e) => api_error_response(&token_error_to_api(&e)),
            }
        }
    });

    let tokens_patch = tokens.clone();
    let auth_patch = auth.clone();
    server.serve("/api/v1/tokens/{token_id}", Method::POST, move |req: Req| {
        let tokens = tokens_patch.clone();
        let auth = auth_patch.clone();
        async move {
            let principal = crate::api_try!(auth.require_user_req(&req).await);
            let owner = crate::api_try!(require_owner(&principal));
            let token_id = crate::api_try!(path_param::<Req, TokenId>(&req, "token_id"));
            let mut req = req;
            let patch: TokenUpdateRequest = crate::api_try!(json_body(&mut req).await);
            match tokens.update(&token_id, &owner, patch).await {
                Ok(Some(issued)) => json_ok(StatusCode::OK, &issued),
                Ok(None) => {
                    let mut list = match tokens.list(&owner).await {
                        Ok(list) => list,
                        Err(e) => return api_error_response(&token_error_to_api(&e)),
                    };
                    list.retain(|t| t.token_id == token_id);
                    match list.into_iter().next() {
                        Some(summary) => json_ok(StatusCode::OK, &summary),
                        None => api_error_response(&ApiError::not_found("token not found")),
                    }
                }
                Err(e) => api_error_response(&token_error_to_api(&e)),
            }
        }
    });

    let tokens_rotate = tokens.clone();
    let auth_rotate = auth.clone();
    server.serve("/api/v1/tokens/{token_id}/rotate", Method::POST, move |req: Req| {
        let tokens = tokens_rotate.clone();
        let auth = auth_rotate.clone();
        async move {
            let principal = crate::api_try!(auth.require_user_req(&req).await);
            let owner = crate::api_try!(require_owner(&principal));
            let token_id = crate::api_try!(path_param::<Req, TokenId>(&req, "token_id"));
            match tokens.rotate(&token_id, &owner).await {
                Ok(issued) => json_ok(StatusCode::OK, &issued),
                Err(e) => api_error_response(&token_error_to_api(&e)),
            }
        }
    });

    let tokens_revoke = tokens.clone();
    let auth_revoke = auth.clone();
    server.serve("/api/v1/tokens/{token_id}", Method::DELETE, move |req: Req| {
        let tokens = tokens_revoke.clone();
        let auth = auth_revoke.clone();
        async move {
            let principal = crate::api_try!(auth.require_user_req(&req).await);
            let owner = crate::api_try!(require_owner(&principal));
            let token_id = crate::api_try!(path_param::<Req, TokenId>(&req, "token_id"));
            match tokens.revoke(&token_id, &owner).await {
                Ok(()) => empty_response(StatusCode::NO_CONTENT),
                Err(e) => api_error_response(&token_error_to_api(&e)),
            }
        }
    });
}

fn require_owner(principal: &crate::model::Principal) -> Result<UserId, ApiError> {
    match principal {
        crate::model::Principal::User { user_id, .. } => Ok(*user_id),
        _ => Err(ApiError::forbidden("token management requires a user session")),
    }
}

fn token_error_to_api(err: &TokenError) -> ApiError {
    match err.kind {
        TokenErrorKind::NotFound => ApiError::not_found(&err.message),
        TokenErrorKind::InvalidInput => ApiError::invalid_input(&err.message),
        TokenErrorKind::Db => ApiError::server(&err.message),
    }
}
