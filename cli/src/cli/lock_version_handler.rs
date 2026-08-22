//! lock-version 命令编排：不可逆锁定项目版本。

use std::path::Path;

use super::args::{LockVersionArgs, parse_server_project_version};
use super::{CliError, build_auth_client};

pub async fn run(config: Option<&Path>, args: LockVersionArgs) -> Result<i32, CliError> {
    let (server, project, version) =
        parse_server_project_version(&args.target).map_err(CliError::InvalidInput)?;
    let (auth, server_url) = build_auth_client(config, Some(&server))?;

    let resolved = auth
        .run_auth(Some(&server_url), |bearer| {
            let client = auth.transport.clone();
            let project = project.clone();
            async move { client.resolve_project(&bearer, &project).await }
        })
        .await?;

    let record = auth
        .run_auth(Some(&server_url), |bearer| {
            let client = auth.transport.clone();
            let project_id = resolved.project_id;
            let version = version.clone();
            async move { client.lock_version(&bearer, project_id, &version).await }
        })
        .await?;

    match record.locked_at {
        Some(_) => println!("version locked: {}:{}", project, record.version),
        None => return Err(CliError::InvalidInput("服务端未返回锁定状态".to_string())),
    }
    Ok(0)
}
