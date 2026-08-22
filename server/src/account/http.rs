//! account 的 HTTP 导出：唯一动作是挂载 sfo-account 的现役接口，
//! 不自写 login/session/refresh handler，也不定义 SessionService。

use sfo_account::AccountServer;
use sfo_http::http_server::{HttpServer, Request, Response};

use super::{AccountModule, FilehubAccount, SqliteAccountStore};
use std::sync::Arc;
use sfo_account::DefaultAccountManager;

impl AccountModule {
    /// 直接导出 sfo-account 的 HTTP 接口：
    /// POST /account/login、POST /account/get_account_info_of_session、
    /// GET /account/get_account_info、POST /account/refresh_session。
    pub fn register_http<S, Req, Resp>(&self, server: &mut S)
    where
        S: sfo_http::http_server::HttpServer<Req, Resp>,
        Req: Request + Sync,
        Resp: Response,
    {
        let manager: Arc<DefaultAccountManager<FilehubAccount, SqliteAccountStore>> =
            self.manager.clone();
        AccountServer::register_server::<
            FilehubAccount,
            DefaultAccountManager<FilehubAccount, SqliteAccountStore>,
            Req,
            Resp,
            S,
        >(server, manager);
    }
}
