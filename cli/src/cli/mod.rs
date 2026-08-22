//! 命令装配：解析、编排、稳定退出码与输出。

use clap::Parser;

pub mod args;
pub mod delete_app_handler;
pub mod lock_version_handler;
pub mod login_handler;
pub mod new_version_handler;
pub mod pull_handler;
pub mod push_handler;
pub mod versions_handler;

use crate::apiclient::{AuthClient, Config, FilehubClient};
use crate::credential_store::CredentialStore;

pub use args::{CliArgs, Command};

/// CLI 错误：携带稳定退出码（0/1/2/3/4/5/6/7/8）。
#[derive(Debug)]
pub enum CliError {
    Usage(String),
    Auth(String),
    Forbidden(String),
    Conflict(String),
    InvalidInput(String),
    Network(String),
    Integrity(String),
    Local(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Usage(_) => 1,
            CliError::Auth(_) => 2,
            CliError::Forbidden(_) => 3,
            CliError::Conflict(_) => 4,
            CliError::InvalidInput(_) => 5,
            CliError::Network(_) => 6,
            CliError::Integrity(_) => 7,
            CliError::Local(_) => 8,
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Usage(message) => write!(f, "用法错误：{message}"),
            CliError::Auth(message) => write!(f, "{message}"),
            CliError::Forbidden(message) => write!(f, "权限不足：{message}"),
            CliError::Conflict(message) => write!(f, "{message}"),
            CliError::InvalidInput(message) => write!(f, "输入无效：{message}"),
            CliError::Network(message) => write!(f, "网络/传输：{message}"),
            CliError::Integrity(message) => write!(f, "内容完整性：{message}"),
            CliError::Local(message) => write!(f, "本地文件系统：{message}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<crate::apiclient::ClientError> for CliError {
    fn from(value: crate::apiclient::ClientError) -> Self {
        match value {
            crate::apiclient::ClientError::Auth(message) => CliError::Auth(message),
            crate::apiclient::ClientError::Forbidden(message) => CliError::Forbidden(message),
            crate::apiclient::ClientError::NotFound(message) => CliError::InvalidInput(message),
            crate::apiclient::ClientError::Conflict(message) => CliError::Conflict(message),
            crate::apiclient::ClientError::InvalidInput(message) => CliError::InvalidInput(message),
            crate::apiclient::ClientError::Transport(message) => CliError::Network(message),
            crate::apiclient::ClientError::Integrity(message) => CliError::Integrity(message),
            crate::apiclient::ClientError::Local(message) => CliError::Local(message),
        }
    }
}

impl From<crate::credential_store::CredentialStoreError> for CliError {
    fn from(value: crate::credential_store::CredentialStoreError) -> Self {
        match value {
            crate::credential_store::CredentialStoreError::NotLoggedIn(message)
            | crate::credential_store::CredentialStoreError::NoServer(message) => {
                CliError::Auth(message)
            }
            credential_store_error => CliError::Local(credential_store_error.to_string()),
        }
    }
}

impl From<crate::archive::ArchiveError> for CliError {
    fn from(value: crate::archive::ArchiveError) -> Self {
        match value {
            crate::archive::ArchiveError::Unsupported(message)
            | crate::archive::ArchiveError::Integrity(message) => CliError::Integrity(message),
            crate::archive::ArchiveError::LocalFs(message) => CliError::Local(message),
        }
    }
}

/// CLI 主入口：初始化日志、解析命令并分发，返回稳定退出码。
pub async fn run_cli() -> i32 {
    init_logger();
    let args = match CliArgs::try_parse() {
        Ok(args) => args,
        Err(error) => {
            let _ = error.print();
            return 1;
        }
    };
    let result = run_command(args).await;
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("filehub: {error}");
            error.exit_code()
        }
    }
}

async fn run_command(args: CliArgs) -> Result<i32, CliError> {
    match args.command {
        Command::Login(login) => login_handler::run_login(args.config.as_deref(), login).await,
        Command::Logout(logout) => login_handler::run_logout(args.config.as_deref(), logout).await,
        Command::Push(push) => push_handler::run(args.config.as_deref(), push).await,
        Command::Pull(pull) => pull_handler::run(args.config.as_deref(), pull).await,
        Command::Versions(versions) => {
            versions_handler::run(args.config.as_deref(), versions).await
        }
        Command::NewVersion(new_version) => {
            new_version_handler::run(args.config.as_deref(), new_version).await
        }
        Command::LockVersion(lock) => lock_version_handler::run(args.config.as_deref(), lock).await,
        Command::DeleteApp(delete) => delete_app_handler::run(args.config.as_deref(), delete).await,
    }
}

/// 构造带凭据的 apiclient（共享内存 store 可被 401 续期落盘）。
/// 构造带凭据的 apiclient：服务器按 显式 SERVER > FILEHUB_SERVER > 默认/唯一已存 解析。
pub(crate) fn build_auth_client(
    config: Option<&std::path::Path>,
    server: Option<&str>,
) -> Result<(AuthClient, String), CliError> {
    let store = open_store(config)?;
    let env_server = std::env::var("FILEHUB_SERVER").ok();
    let server_url = store
        .resolve_server(server, env_server.as_deref())
        .map_err(CliError::from)?;
    let client = FilehubClient::new(Config {
        base_url: server_url.clone(),
        ..Config::default()
    })?;
    Ok((
        AuthClient::new(client, std::sync::Arc::new(tokio::sync::RwLock::new(store))),
        server_url,
    ))
}

/// 打开凭据存储；注意 login/logout 需要 `&mut` 所有权，这里返回 store。
pub(crate) fn open_store(config: Option<&std::path::Path>) -> Result<CredentialStore, CliError> {
    let path = CredentialStore::config_path(config);
    CredentialStore::open(&path).map_err(CliError::from)
}

fn init_logger() {
    // 默认 warn 级：正常命令输出走 stdout/stderr 直接输出，日志不污染脚本 stdout。
    let _ = sfo_log::Logger::new("filehub-cli")
        .set_log_level("warn")
        .start();
}
