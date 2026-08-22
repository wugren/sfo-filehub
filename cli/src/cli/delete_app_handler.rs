//! delete-app 命令编排：从项目版本中删除指定应用。

use std::path::Path;

use super::args::{DeleteAppArgs, parse_server_project_version_name};
use super::{CliError, build_auth_client};

pub async fn run(config: Option<&Path>, args: DeleteAppArgs) -> Result<i32, CliError> {
    let (server, project, version, app) =
        parse_server_project_version_name(&args.target).map_err(CliError::InvalidInput)?;
    let (auth, server_url) = build_auth_client(config, Some(&server))?;

    let resolved = auth
        .run_auth(Some(&server_url), |bearer| {
            let client = auth.transport.clone();
            let project = project.clone();
            async move { client.resolve_project(&bearer, &project).await }
        })
        .await?;

    auth.run_auth(Some(&server_url), |bearer| {
        let client = auth.transport.clone();
        let project_id = resolved.project_id;
        let version = version.clone();
        let app = app.clone();
        async move { client.delete_app(&bearer, project_id, &version, &app).await }
    })
    .await?;

    println!("app deleted: {}:{}:{}", project, version, app);
    Ok(0)
}
