//! filehub 发布客户端（filehub-cli）库入口。
//!
//! 模块边界按任务 003 设计冻结：`cli`（命令装配）依赖 `apiclient`、
//! `credential_store` 与 `archive`；`apiclient` 依赖 `credential_store`。

pub mod apiclient;
pub mod archive;
pub mod cli;
pub mod credential_store;
