//! http 子模块：AppState 与 register_api 装配（P-07 fh-server-http）。

pub mod auth;
pub mod authz;
pub mod router;

use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

use crate::account::AccountModule;
use crate::account::rate_limit::FixedWindowLoginLimiter;
use crate::contract::AuthProvider;
use crate::model::{FileId, ServerConfig};
use crate::permissions::{PermissionChecker, PermissionsModule, model::ProjectAccess};
use crate::projects::ProjectModule;
use crate::projects::ProjectService;
use crate::projects::service::SqliteProjectAccess;
use crate::storage::{FileModule, FileStore};
use crate::tokens::{TokenService, TokensModule};
use crate::versions::{VersionModule, VersionService};

pub struct AppState {
    pub account: Arc<AccountModule>,
    pub permissions_module: Arc<PermissionsModule>,
    pub permissions: Arc<dyn PermissionChecker>,
    pub tokens: Arc<dyn TokenService>,
    pub files: Arc<dyn FileStore>,
    pub versions: Arc<dyn VersionService>,
    pub projects: Arc<dyn ProjectService>,
    pub max_archive_bytes: u64,
}

impl AppState {
    /// 按依赖顺序装配全部子模块（I-001~I-008 的组合入口）。
    pub async fn assemble(config: &ServerConfig, db: &SqlitePool) -> Result<Self, String> {
        let login_rate_limiter = if config.server.login_rate_limit_per_minute == 0 {
            None
        } else {
            Some(Arc::new(FixedWindowLoginLimiter::new(
                config.server.login_rate_limit_window_secs.max(1),
                config.server.login_rate_limit_per_minute,
            )) as Arc<dyn sfo_account::LoginRateLimiter>)
        };
        let account = Arc::new(
            AccountModule::init(&config.users, login_rate_limiter, db).await?,
        );

        // 项目只读端口提前创建，注入权限核心（projects 表归属仍为 projects 子模块）。
        let project_access: Arc<dyn ProjectAccess> = Arc::new(SqliteProjectAccess::new(db.clone()));
        let permissions_module = Arc::new(PermissionsModule::init(db, project_access).await?);
        let permissions = permissions_module.checker();

        let tokens = TokensModule::init(db).await?;
        let files = FileModule::init(
            db,
            config.files.data_dir.clone(),
            config.files.max_archive_bytes,
        )
        .await?;
        let versions = VersionModule::init(db, permissions.clone(), files.store()).await?;
        let projects = ProjectModule::init(db, permissions.clone(), files.store()).await?;

        Ok(Self {
            account,
            permissions_module,
            permissions,
            tokens: tokens.service(),
            files: files.store(),
            versions: versions.service(),
            projects: projects.service(),
            max_archive_bytes: config.files.max_archive_bytes,
        })
    }

    /// 启动回收：全部已发布版本引用的文件保留，其余孤儿文件清理。
    pub async fn startup_gc(&self) -> Result<Vec<FileId>, String> {
        let keep = self
            .versions
            .referenced_file_ids()
            .await
            .map_err(|e| format!("collect referenced files failed: {e}"))?;
        self.files
            .gc_orphans(&keep)
            .await
            .map_err(|e| format!("gc orphans failed: {e}"))
    }

    pub fn auth_provider(&self) -> Arc<AuthProvider> {
        Arc::new(AuthProvider {
            session_auth: Arc::new(auth::SessionAuthWrapper {
                account: self.account.clone(),
            }),
            token_auth: Arc::new(auth::TokenAuthWrapper {
                tokens: self.tokens.clone(),
            }),
        })
    }
}

/// sfo-http 装配唯一入口：把全部子模块 handler 注册进 HttpServer。
pub async fn register_api<S, Req, Resp>(server: &mut S, state: AppState)
where
    S: sfo_http::http_server::HttpServer<Req, Resp> + 'static,
    Req: sfo_http::http_server::Request + Sync,
    Resp: sfo_http::http_server::Response,
{
    let state = Arc::new(state);
    router::register_all::<S, Req, Resp>(server, state);
}
