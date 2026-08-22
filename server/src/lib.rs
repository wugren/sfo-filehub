//! filehub-server：文件集散服务后台（filehub 产品 001 任务）。
//! 七个履职子模块 + 共享 model/contract，http 为唯一装配层。

pub mod account;
pub mod contract;
pub mod http;
pub mod model;
pub mod permissions;
pub mod projects;
pub mod storage;
pub mod tokens;
pub mod versions;

pub use http::register_api;

/// 把产生 ApiError 的表达式转换为 handler 提前返回（配合统一 JSON 错误响应）。
#[macro_export]
macro_rules! api_try {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => return crate::contract::api_error_response(&e),
        }
    };
}
