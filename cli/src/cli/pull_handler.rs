//! pull 命令编排（fh-cli-pull）。

use std::path::Path;

use super::args::{PullArgs, parse_server_project_version_name};
use super::{CliError, build_auth_client};
use crate::apiclient::error::ClientError;
use crate::archive;
use crate::credential_store::Credential;

pub async fn run(config: Option<&Path>, args: PullArgs) -> Result<i32, CliError> {
    let (server, project, version, app) =
        parse_server_project_version_name(&args.target).map_err(CliError::InvalidInput)?;
    if args.path.is_dir() {
        return Err(CliError::InvalidInput(format!(
            "pull destination must be a file path, not a directory: {}",
            args.path.display()
        )));
    }
    let final_path = args.path.as_path();
    let file_name = final_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "pull.tar.gz".to_string());
    let tmp = final_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".{file_name}.{}.tmp", std::process::id()));
    let parent = final_path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.as_os_str().is_empty() {
        std::fs::create_dir_all(parent).map_err(|e| {
            CliError::Local(format!(
                "failed to create output directory {}: {e}",
                parent.display()
            ))
        })?;
    }

    let (auth, server_url) = build_auth_client(config, Some(&server))?;
    let prepared = auth.prepare(Some(&server_url)).await?;

    // 先解析项目，再取版本元数据（含 SHA-256 与真实版本号）。
    let resolved_project = auth
        .run_auth(Some(&server_url), |bearer| {
            let client = auth.transport.clone();
            let project = project.clone();
            async move { client.resolve_project(&bearer, &project).await }
        })
        .await?;
    let project_id = resolved_project.project_id;
    let version_info = auth
        .run_auth(Some(&server_url), |bearer| {
            let client = auth.transport.clone();
            let version = version.clone();
            async move {
                client
                    .get_version(&bearer, project_id, Some(&version))
                    .await
            }
        })
        .await?;
    let actual_version = version_info.version.clone();
    let app_info = version_info
        .apps
        .iter()
        .find(|a| a.app == app)
        .ok_or_else(|| {
            CliError::InvalidInput(format!(
                "app {app} does not exist in version {actual_version}"
            ))
        })?;
    let expected_sha256 = app_info.sha256.clone();

    // 下载到临时文件；预检通过后开始流式写。中途出错不重试。
    let mut bearer = prepared.bearer.clone();
    let mut refreshed = false;
    loop {
        let result = auth
            .transport
            .download(&bearer, project_id, Some(&version), &app, &tmp)
            .await;
        match result {
            Ok(()) => break,
            Err(ClientError::Auth(_)) if !refreshed => {
                if tmp.exists() && std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0) > 0 {
                    let _ = std::fs::remove_file(&tmp);
                    return Err(CliError::Network(
                        "received 401 after the download stream started; partial downloads are not retried, so run pull again"
                            .to_string(),
                    ));
                }
                // 尚未产生部分字节：session 续期一次后重试。
                let renewed = match &prepared.credential {
                    Credential::PasswordSession {
                        refresh_session, ..
                    } => auth.transport.refresh_session(refresh_session).await?,
                    Credential::Token { .. } => {
                        return Err(CliError::Auth("token is invalid or expired".to_string()));
                    }
                };
                let mut store = auth.store.write().await;
                store.update_session(
                    &prepared.server,
                    &renewed.session,
                    &renewed.refresh_session,
                )?;
                store.flush()?;
                drop(store);
                bearer = renewed.session;
                refreshed = true;
            }
            Err(error) => {
                let _ = std::fs::remove_file(&tmp);
                return Err(CliError::from(error));
            }
        }
    }

    if let Err(error) = archive::finalize_download(&tmp, final_path, &expected_sha256) {
        let _ = std::fs::remove_file(&tmp);
        return Err(CliError::from(error));
    }
    println!("pull succeeded: {}", final_path.display());
    Ok(0)
}
