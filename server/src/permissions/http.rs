//! permissions 的协作者管理 HTTP 接口（sfo-http；基于 PermissionChecker）。

use http::StatusCode;
use sfo_http::http::Method;
use sfo_http::http_server::{Request, Response};
use std::sync::Arc;

use crate::contract::{AuthProvider, api_error_response, json_body, json_ok, path_param};
use crate::model::{ProjectId, ProjectRole, UserId};

use super::PermissionChecker;

pub fn register<S, Req, Resp>(
    server: &mut S,
    checker: Arc<dyn PermissionChecker>,
    auth: Arc<AuthProvider>,
) where
    S: sfo_http::http_server::HttpServer<Req, Resp>,
    Req: Request + Sync,
    Resp: Response,
{
    let checker_get = checker.clone();
    let auth_get = auth.clone();
    server.serve(
        "/api/v1/projects/{project_id}/collaborators",
        Method::GET,
        move |req: Req| {
            let checker = checker_get.clone();
            let auth = auth_get.clone();
            async move {
                let principal = crate::api_try!(auth.current_principal_req(&req).await);
                let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
                match checker.list_collaborators(&project_id, &principal).await {
                    Ok(list) => json_ok(StatusCode::OK, &list),
                    Err(e) => api_error_response(&e.into()),
                }
            }
        },
    );

    let checker_put = checker.clone();
    let auth_put = auth.clone();
    server.serve(
        "/api/v1/projects/{project_id}/collaborators/{user_id}",
        Method::PUT,
        move |mut req: Req| {
            let checker = checker_put.clone();
            let auth = auth_put.clone();
            async move {
                let principal = crate::api_try!(auth.current_principal_req(&req).await);
                let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
                let user_id = crate::api_try!(path_param::<Req, UserId>(&req, "user_id"));
                let role =
                    crate::api_try!(json_body::<Req, ProjectRoleRequest>(&mut req).await).role;
                match checker
                    .grant_collaborator(&project_id, &principal, &user_id, role)
                    .await
                {
                    Ok(()) => {
                        let record = crate::model::Collaborator { user_id, role };
                        json_ok(StatusCode::OK, &record)
                    }
                    Err(e) => api_error_response(&e.into()),
                }
            }
        },
    );

    let checker_del = checker.clone();
    let auth_del = auth.clone();
    server.serve(
        "/api/v1/projects/{project_id}/collaborators/{user_id}",
        Method::DELETE,
        move |req: Req| {
            let checker = checker_del.clone();
            let auth = auth_del.clone();
            async move {
                let principal = crate::api_try!(auth.current_principal_req(&req).await);
                let project_id = crate::api_try!(path_param::<Req, ProjectId>(&req, "project_id"));
                let user_id = crate::api_try!(path_param::<Req, UserId>(&req, "user_id"));
                match checker
                    .remove_collaborator(&project_id, &principal, &user_id)
                    .await
                {
                    Ok(()) => {
                        let mut resp = Resp::new(StatusCode::NO_CONTENT);
                        let _ = &mut resp;
                        Ok(resp)
                    }
                    Err(e) => api_error_response(&e.into()),
                }
            }
        },
    );
}

#[derive(serde::Deserialize)]
pub struct ProjectRoleRequest {
    pub role: ProjectRole,
}
