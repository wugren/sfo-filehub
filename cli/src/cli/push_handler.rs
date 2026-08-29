//! push 命令编排（fh-cli-push）。

use std::path::Path;

use super::args::{PushArgs, parse_server_project_version_name};
use super::{CliError, build_auth_client};
use crate::archive;

pub async fn run(config: Option<&Path>, args: PushArgs) -> Result<i32, CliError> {
    let (server, project, version, app) =
        parse_server_project_version_name(&args.target).map_err(CliError::InvalidInput)?;
    if !args.path.exists() {
        return Err(CliError::Local(format!(
            "path to publish does not exist: {}",
            args.path.display()
        )));
    }

    let packed = archive::pack_tar_gz(&args.path)?;
    let mut cleanup = CleanupGuard::new(packed.path.clone());
    let (auth, server_url) = build_auth_client(config, Some(&server))?;

    // 解析项目名 -> project_id；401 时按 session 续期一次。
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
            let archive_path = packed.path.clone();
            let sha256 = packed.sha256.clone();
            let project_id = resolved.project_id;
            let version = version.clone();
            let app = app.clone();
            async move {
                client
                    .publish_app(&bearer, project_id, &version, &app, &archive_path, &sha256)
                    .await
            }
        })
        .await?;

    cleanup.disarm();
    let sha = record
        .apps
        .iter()
        .find(|a| a.app == app)
        .map(|a| a.sha256.clone())
        .unwrap_or_default();
    println!(
        "push succeeded: {}:{}:{} (sha256 {})",
        project, record.version, app, sha
    );
    Ok(0)
}

/// 临时归档清理守卫：成功/失败路径都删除。
struct CleanupGuard {
    path: std::path::PathBuf,
    armed: bool,
}

impl CleanupGuard {
    fn new(path: std::path::PathBuf) -> Self {
        CleanupGuard { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
