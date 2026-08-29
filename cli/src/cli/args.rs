//! clap 命令面：push/pull 与全命令 `server/...` 目标形态（017-cli-slash-target-separator）。

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "filehub",
    version,
    about = "filehub artifact publishing client",
    after_help = "Exit codes: 0 success / 1 usage error / 2 authentication failure / 3 authorization failure / 4 conflict / 5 invalid input / 6 network/transport / 7 content integrity / 8 local filesystem",
    subcommand_required = true,
    arg_required_else_help = true
)]
pub struct CliArgs {
    /// Override the credential and configuration file path
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Sign in with a password or token and save the credentials locally
    Login(LoginArgs),
    /// Remove locally stored credentials for the specified or default server
    Logout(LogoutArgs),
    /// Publish a file or directory as a .tar.gz app at `<server/project/version/name>`
    Push(PushArgs),
    /// Download a .tar.gz app from `<server/project/version/name>` to a file
    Pull(PullArgs),
    /// List version information for `<server/project>` as text or JSON
    Versions(VersionsArgs),
    /// Create a project version, failing if it already exists
    #[command(name = "new-version")]
    NewVersion(NewVersionArgs),
    /// Permanently lock a project version
    #[command(name = "lock-version")]
    LockVersion(LockVersionArgs),
    /// Delete an app from a project version
    #[command(name = "delete-app")]
    DeleteApp(DeleteAppArgs),
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    /// filehub server address as host[:port] (HTTPS preferred; loopback may use HTTP)
    pub server: Option<String>,
    /// Username for password sign-in (prompted when omitted)
    #[arg(short, long, conflicts_with = "token_stdin")]
    pub username: Option<String>,
    /// Read the password from stdin, stripping the trailing newline
    #[arg(long, conflicts_with = "token_stdin")]
    pub password_stdin: bool,
    /// Read the token from stdin; conflicts with password sign-in options
    #[arg(long)]
    pub token_stdin: bool,
}

#[derive(Debug, Args)]
pub struct LogoutArgs {
    /// Server address as host[:port] (uses the default server when omitted)
    pub server: Option<String>,
}

#[derive(Debug, Args)]
pub struct PushArgs {
    /// `<server/project/version/name>` where server is host[:port], including IPv6
    pub target: String,
    /// File or directory to publish
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct PullArgs {
    /// `<server/project/version/name>` where server is host[:port], including IPv6
    pub target: String,
    /// Exact output file path for the downloaded archive
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct VersionsArgs {
    /// <server/project>
    pub target: String,
    /// Write output to a file instead of stdout
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// Output format: text or json
    #[arg(long, value_parser = ["text", "json"], default_value = "text")]
    pub format: String,
}

#[derive(Debug, Args)]
pub struct NewVersionArgs {
    /// `<server/project/version>` for a version that does not yet exist
    pub target: String,
}

#[derive(Debug, Args)]
pub struct LockVersionArgs {
    /// `<server/project/version>` (locking cannot be undone)
    pub target: String,
}

#[derive(Debug, Args)]
pub struct DeleteAppArgs {
    /// <server/project/version/name>
    pub target: String,
}

/// 目标串统一解析：按 `/` 分段，段数必须严格等于字段数。
///
/// server 段允许包含端口与 IPv6 冒号（如 `127.0.0.1:8080`、`[::1]:8080`、
/// `::1:8080`），因此用 `/` 作为目标字段分隔符可避免与 server 内部冒号歧义；
/// 缺段、多余段与空段均明确报错。
fn parse_target(
    value: &str,
    expected_segments: usize,
    form: &str,
) -> Result<(String, Vec<String>), String> {
    let value = value.trim();
    // server 与 login/logout 保持一致的旧习惯兼容：显式 http(s):// 前缀先剥离，
    // 身份仍为 host[:port]；随后按 / 严格分段。
    let value = value
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(value);
    let raw: Vec<&str> = value.split('/').collect();
    if raw.len() != expected_segments {
        return Err(format!("target must match {form}; received: {value}"));
    }
    let server = raw[0].trim().to_string();
    if server.is_empty() {
        return Err(format!(
            "server cannot be empty and target must match {form}; received: {value}"
        ));
    }
    let mut fields = Vec::with_capacity(expected_segments - 1);
    for segment in &raw[1..] {
        let field = segment.trim().to_string();
        if field.is_empty() {
            return Err(format!(
                "target fields cannot be empty and target must match {form}; received: {value}"
            ));
        }
        fields.push(field);
    }
    Ok((server, fields))
}

/// 解析 `<server/project>`。
pub fn parse_server_project(value: &str) -> Result<(String, String), String> {
    let (server, mut fields) = parse_target(value, 2, "<server/project>")?;
    Ok((server, fields.remove(0)))
}

/// 解析 `<server/project/version>`。
pub fn parse_server_project_version(value: &str) -> Result<(String, String, String), String> {
    let (server, mut fields) = parse_target(value, 3, "<server/project/version>")?;
    Ok((server, fields.remove(0), fields.remove(0)))
}

/// 解析 `<server/project/version/name>`。
pub fn parse_server_project_version_name(
    value: &str,
) -> Result<(String, String, String, String), String> {
    let (server, mut fields) = parse_target(value, 4, "<server/project/version/name>")?;
    Ok((server, fields.remove(0), fields.remove(0), fields.remove(0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_server_with_port_project_version_name() {
        assert_eq!(
            parse_server_project_version_name("127.0.0.1:8080/test/1.1.0/logs").unwrap(),
            (
                "127.0.0.1:8080".to_string(),
                "test".to_string(),
                "1.1.0".to_string(),
                "logs".to_string()
            )
        );
        assert_eq!(
            parse_server_project_version_name("http://127.0.0.1:8080/test/1.1.0/logs").unwrap(),
            (
                "127.0.0.1:8080".to_string(),
                "test".to_string(),
                "1.1.0".to_string(),
                "logs".to_string()
            )
        );
    }

    #[test]
    fn parses_server_without_port_forms() {
        assert_eq!(
            parse_server_project("hub.example.com/demo").unwrap(),
            ("hub.example.com".to_string(), "demo".to_string())
        );
        assert_eq!(
            parse_server_project_version("hub.example.com/demo/1.0.0").unwrap(),
            (
                "hub.example.com".to_string(),
                "demo".to_string(),
                "1.0.0".to_string()
            )
        );
    }

    #[test]
    fn parses_ipv6_server_forms() {
        assert_eq!(
            parse_server_project_version_name("::1:8080/test/1.1.0/logs").unwrap(),
            (
                "::1:8080".to_string(),
                "test".to_string(),
                "1.1.0".to_string(),
                "logs".to_string()
            )
        );
        assert_eq!(
            parse_server_project_version_name("[::1]:8080/test/1.1.0/logs").unwrap(),
            (
                "[::1]:8080".to_string(),
                "test".to_string(),
                "1.1.0".to_string(),
                "logs".to_string()
            )
        );
    }

    #[test]
    fn rejects_missing_or_empty_fields() {
        assert!(parse_server_project("hub.example.com").is_err());
        assert!(parse_server_project("hub.example.com/demo/extra").is_err());
        assert!(parse_server_project_version("hub.example.com/demo").is_err());
        assert!(parse_server_project_version_name("hub.example.com/demo/1.0.0").is_err());
        assert!(parse_server_project_version_name("hub.example.com/demo//1.0.0/logs").is_err());
        assert!(parse_server_project_version("/demo/1.0.0").is_err());
        assert!(
            parse_server_project_version_name("hub.example.com/demo/1.0.0/logs/extra").is_err()
        );
    }
}
