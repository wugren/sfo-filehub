//! versions 命令编排（fh-cli-versions）。

use std::path::Path;

use super::args::{VersionsArgs, parse_server_project};
use super::{CliError, build_auth_client};

pub async fn run(config: Option<&Path>, args: VersionsArgs) -> Result<i32, CliError> {
    let (server, project) = parse_server_project(&args.target).map_err(CliError::InvalidInput)?;
    let (auth, server_url) = build_auth_client(config, Some(&server))?;
    let resolved = auth
        .run_auth(Some(&server_url), |bearer| {
            let client = auth.transport.clone();
            let project = project.clone();
            async move { client.resolve_project(&bearer, &project).await }
        })
        .await?;
    let versions = auth
        .run_auth(Some(&server_url), |bearer| {
            let client = auth.transport.clone();
            let project_id = resolved.project_id;
            async move { client.list_versions(&bearer, project_id).await }
        })
        .await?;

    let body = if args.format == "json" {
        serde_json::to_vec_pretty(&versions)
            .map_err(|e| CliError::Local(format!("序列化版本 JSON 失败：{e}")))?
    } else {
        render_text(&versions).into_bytes()
    };

    match &args.output {
        Some(path) => {
            write_output_file(path, &body)?;
            println!("versions written: {}", path.display());
        }
        None => {
            use std::io::Write as _;
            let mut stdout = std::io::stdout();
            stdout
                .write_all(&body)
                .map_err(|e| CliError::Local(format!("输出 stdout 失败：{e}")))?;
            stdout
                .write_all(b"\n")
                .map_err(|e| CliError::Local(format!("输出 stdout 失败：{e}")))?;
        }
    }
    Ok(0)
}

fn render_text(versions: &[crate::apiclient::contract::VersionDto]) -> String {
    let mut out = String::new();
    out.push_str("VERSION\tPUBLISHED_AT\tLOCKED\tAPPS\n");
    for version in versions {
        let locked = if version.locked_at.is_some() {
            "yes"
        } else {
            "no"
        };
        let apps = if version.apps.is_empty() {
            "-".to_string()
        } else {
            version
                .apps
                .iter()
                .map(|app| format!("{}:{}", app.app, app.size))
                .collect::<Vec<_>>()
                .join(",")
        };
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            version.version, version.published_at, locked, apps
        ));
    }
    out
}

/// 路径安全校验 + 原子写入（先写隐藏临时文件再 rename）。
fn write_output_file(path: &Path, body: &[u8]) -> Result<(), CliError> {
    let value = path.to_string_lossy();
    if value.contains('\0') {
        return Err(CliError::InvalidInput("输出路径包含 NUL 字符".to_string()));
    }
    if path.is_dir() {
        return Err(CliError::InvalidInput(format!(
            "输出路径是目录而不是文件：{}",
            path.display()
        )));
    }
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => std::path::PathBuf::from("."),
    };
    std::fs::create_dir_all(&parent)
        .map_err(|e| CliError::Local(format!("创建输出目录失败：{e}")))?;
    let tmp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "versions.txt".to_string()),
        std::process::id()
    ));
    std::fs::write(&tmp, body).map_err(|e| CliError::Local(format!("写入版本输出失败：{e}")))?;
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| CliError::Local(format!("替换旧输出失败：{e}")))?;
    }
    std::fs::rename(&tmp, path).map_err(|e| CliError::Local(format!("落盘版本输出失败：{e}")))?;
    Ok(())
}
