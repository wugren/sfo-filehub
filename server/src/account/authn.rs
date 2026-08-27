//! session 凭据解析辅助：只复用 AccountModule::decode_session，构造后续认证所需的用户身份。

use crate::model::Principal;

use super::AccountModule;

/// 尝试把 Bearer 凭据作为登录 session 解码；失败返回 None（由调用方回退到 token 解析）。
pub async fn try_user_principal(account: &AccountModule, bearer: &str) -> Option<Principal> {
    match account.decode_session(bearer).await {
        Ok(account) => Some(Principal::User {
            user_id: account.id,
        }),
        Err(_) => None,
    }
}
