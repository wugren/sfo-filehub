//! projects 的 HTTP 接口（I-007）。

use http::{Method, StatusCode};
use sfo_http::http_server::{HttpServer, Request, Response};
use serde::Deserialize;
use std::sync::Arc;

use crate::contract::{api_error_response, empty_response, json_body, json_ok, path_param, ApiError, AuthProvider};
use crate::model::{ProjectId, Visibility};

use super::{ProjectError, ProjectErrorKind, ProjectService};

pub fn register<S, Req, Resp>(server: &mut S, projects: Arc<dyn ProjectService>, auth: Arc<AuthProvider>)
where
    S: sfo_http::http_server::HttpServer<Req, Resp>,
    Req: Request + Sync,
    Resp: Response,
{
    let projects_create = projects.clone();
    let auth_create = auth.clone();
    server.serve("/api/v1/projects", Method::POST, move |mut req: Req| {
        let projects = projects_create.clone();
        let auth = auth_create.clone();
        async move {
            let principal = crate::api_try!(auth.require_user_or_token_req(&req).await);
            let body: CreateProjectRequest = crate::api_try!(json_body(&mut req).await);
            match projects.create(&principal, &body.name, body.visibility).await {
                Ok(record) => json_ok(StatusCode::CREATED, &record),
                Err(e) => api_error_response(&project_error_to_api(&e)),
            }
        }
    });

    let projects_list = projects.clone();
    let auth_list = auth.clone();
    server.serve("/api/v1/projects", Method::GET, move |req: Req| {
        let projects = projects_list.clone();
        let auth = auth_list.clone();
        async move {
            let principal = crate::api_try!(auth.current_principal_req(&req).await);
            match projects.list(&principal).await {
                Ok(list) => json_ok(StatusCode::OK, &list),
                Err(e) => api_error_response(&project_error_to_api(&e)),
            }
        }
    });

    let projects_patch = projects.clone();
    let auth_patch = auth.clone();
    server.serve("/api/v1/projects/{project_id}/visibility", Method::POST, move |mut req: Req| {
        let projects = projects_patch.clone();
        let auth = auth_patch.clone();
        async move {
            let principal = crate::api_try!(auth.require_user_or_token_req(&req).await);
            let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
            let body: VisibilityRequest = crate::api_try!(json_body(&mut req).await);
            match projects.set_visibility(&project_id, &principal, body.visibility).await {
                Ok(()) => {
                    let record = match projects.list(&principal).await {
                        Ok(list) => list.into_iter().find(|p| p.project_id == project_id),
                        Err(e) => return api_error_response(&project_error_to_api(&e)),
                    };
                    match record {
                        Some(record) => json_ok(StatusCode::OK, &record),
                        None => api_error_response(&ApiError::not_found("project not found")),
                    }
                }
                Err(e) => api_error_response(&project_error_to_api(&e)),
            }
        }
    });

    let projects_delete = projects.clone();
    let auth_delete = auth.clone();
    server.serve("/api/v1/projects/{project_id}", Method::DELETE, move |req: Req| {
        let projects = projects_delete.clone();
        let auth = auth_delete.clone();
        async move {
            let principal = crate::api_try!(auth.require_user_or_token_req(&req).await);
            let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
            match projects.delete(&project_id, &principal).await {
                Ok(()) => empty_response(StatusCode::NO_CONTENT),
                Err(e) => api_error_response(&project_error_to_api(&e)),
            }
        }
    });

    // GET /api/v1/projects/{project_id}：匿名(public)/session/token，metadata:read。
    let projects_get = projects.clone();
    let auth_get = auth.clone();
    server.serve("/api/v1/projects/{project_id}", Method::GET, move |req: Req| {
        let projects = projects_get.clone();
        let auth = auth_get.clone();
        async move {
            let principal = crate::api_try!(auth.current_principal_req(&req).await);
            let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
            match projects.list(&principal).await {
                Ok(list) => match list.into_iter().find(|p| p.project_id == project_id) {
                    Some(record) => json_ok(StatusCode::OK, &record),
                    None => {
                        if matches!(principal, crate::model::Principal::Anonymous) {
                            api_error_response(&ApiError::unauthorized("private project requires login or the project does not exist"))
                        } else {
                            api_error_response(&ApiError::not_found("project not found"))
                        }
                    }
                },
                Err(e) => api_error_response(&project_error_to_api(&e)),
            }
        }
    });
}

fn project_error_to_api(err: &ProjectError) -> ApiError {
    match err.kind {
        ProjectErrorKind::NotFound => ApiError::not_found(&err.message),
        ProjectErrorKind::Forbidden => ApiError::forbidden(&err.message),
        ProjectErrorKind::Conflict => ApiError::conflict(&err.message),
        ProjectErrorKind::InvalidInput => ApiError::invalid_input(&err.message),
        ProjectErrorKind::Db => ApiError::server(&err.message),
    }
}

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    #[serde(default = "default_visibility")]
    pub visibility: Visibility,
}

fn default_visibility() -> Visibility {
    Visibility::Private
}

#[derive(Deserialize)]
pub struct VisibilityRequest {
    pub visibility: Visibility,
}
