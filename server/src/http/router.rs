//! 路由装配：account 直接导出 + permissions/tokens/projects/versions 路由（I-008）。

use std::sync::Arc;

use sfo_http::http_server::{HttpServer, Request, Response};

use super::AppState;

pub fn register_all<S, Req, Resp>(server: &mut S, state: Arc<AppState>)
where
    S: sfo_http::http_server::HttpServer<Req, Resp>,
    Req: Request + Sync,
    Resp: Response,
{
    let auth = state.auth_provider();
    state.account.register_http::<S, Req, Resp>(server);
    crate::permissions::http::register::<S, Req, Resp>(server, state.permissions.clone(), auth.clone());
    crate::tokens::http::register::<S, Req, Resp>(server, state.tokens.clone(), auth.clone());
    crate::projects::http::register::<S, Req, Resp>(server, state.projects.clone(), auth.clone());
    crate::versions::http::register::<S, Req, Resp>(server, state.versions.clone(), state.files.clone(), auth.clone());
}
