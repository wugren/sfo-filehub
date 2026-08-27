//! 路由级授权包装：handler 执行前完成 Project 资源动作判定（P-07 修订）。

use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;

use sfo_http::errors::HttpResult;
use sfo_http::http_server::{Endpoint, Request, Response};

use crate::contract::{ApiError, AuthProvider, api_error_response, path_param};
use crate::model::{Principal, ProjectId, Resource};
use crate::permissions::PermissionChecker;

/// 项目资源动作授权端点：先解析 Principal 与路径项目标识，在 handler 执行前
/// 调用 `can_access(Project, action)`；拒绝时返回 401/403，不触碰请求体。
/// inner handler 假定授权已通过（业务层纵深校验仍保留）。
#[derive(Clone)]
pub(crate) struct ProjectAuthzEndpoint<Req, F> {
    auth: Arc<AuthProvider>,
    checker: Arc<dyn PermissionChecker>,
    action: &'static str,
    handler: F,
    _req: PhantomData<fn(Req)>,
}

#[async_trait::async_trait]
impl<Req, Resp, F, Fut> Endpoint<Req, Resp> for ProjectAuthzEndpoint<Req, F>
where
    Req: Request + Sync,
    Resp: Response,
    F: Fn(Principal, Req) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = HttpResult<Resp>> + Send + 'static,
{
    async fn call(&self, req: Req) -> HttpResult<Resp> {
        let principal = match self.auth.current_principal_req(&req).await {
            Ok(principal) => principal,
            Err(error) => return api_error_response(&error),
        };
        let project_id = match path_param::<Req, ProjectId>(&req, "project_id") {
            Ok(id) => id,
            Err(error) => return api_error_response(&error),
        };
        match self
            .checker
            .can_access(&principal, &Resource::Project(project_id), self.action)
            .await
        {
            Ok(true) => (self.handler)(principal, req).await,
            Ok(false) => {
                let error = if matches!(principal, Principal::Anonymous) {
                    ApiError::unauthorized("artifacts:write required")
                } else {
                    ApiError::forbidden("artifacts:write required")
                };
                api_error_response(&error)
            }
            Err(error) => api_error_response(&ApiError::from(error)),
        }
    }
}

pub(crate) fn authz_project_action<Req, F>(
    auth: Arc<AuthProvider>,
    checker: Arc<dyn PermissionChecker>,
    action: &'static str,
    handler: F,
) -> ProjectAuthzEndpoint<Req, F>
where
    Req: Request + Sync,
{
    ProjectAuthzEndpoint {
        auth,
        checker,
        action,
        handler,
        _req: PhantomData,
    }
}
