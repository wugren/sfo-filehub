//! 配置文件的持久数据模型（schema_version 前向宽容）。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 单服务器凭据记录；token 与 session 字段由写入方保证互斥。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerCredential {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// 顶层配置文档。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDocument {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_server: Option<String>,
    #[serde(default)]
    pub server: HashMap<String, ServerCredential>,
}
