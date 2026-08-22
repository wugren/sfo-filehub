//! clap 命令面：push/pull 与全命令 `server/...` 目标形态（017-cli-slash-target-separator）。

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "filehub",
    version,
    about = "filehub 文件集散发布客户端",
    after_help = "退出码：0 成功 / 1 用法错误 / 2 认证失败 / 3 授权失败 / 4 冲突 / 5 输入无效 / 6 网络/传输 / 7 内容完整性 / 8 本地文件系统",
    subcommand_required = true,
    arg_required_else_help = true
)]
pub struct CliArgs {
    /// 覆盖凭据与配置文件路径（默认平台用户配置目录 config.toml）。
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 账号密码或 token 登录并保存本地凭据
    Login(LoginArgs),
    /// 清除指定/默认服务器的本地凭据
    Logout(LogoutArgs),
    /// 把文件或目录发布为 `<server/project/version/name>` 应用（统一 .tar.gz）
    Push(PushArgs),
    /// 按 `<server/project/version/name>` 拉取 .tar.gz 到指定文件路径
    Pull(PullArgs),
    /// 查询 `<server/project>` 的版本信息（文本或 JSON）
    Versions(VersionsArgs),
    /// 显式创建项目版本（版本已存在时失败）
    #[command(name = "new-version")]
    NewVersion(NewVersionArgs),
    /// 不可逆锁定项目版本
    #[command(name = "lock-version")]
    LockVersion(LockVersionArgs),
    /// 从项目版本中删除指定应用
    #[command(name = "delete-app")]
    DeleteApp(DeleteAppArgs),
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    /// filehub 服务地址（host[:port]，无需协议头；HTTPS 优先，loopback 可降级 HTTP）
    pub server: Option<String>,
    /// 账号密码登录用户名（缺省交互提示）
    #[arg(short, long, conflicts_with = "token_stdin")]
    pub username: Option<String>,
    /// 密码从 stdin 读取（剥离末尾换行）
    #[arg(long, conflicts_with = "token_stdin")]
    pub password_stdin: bool,
    /// token 从 stdin 读取；与账号密码选项互斥
    #[arg(long)]
    pub token_stdin: bool,
}

#[derive(Debug, Args)]
pub struct LogoutArgs {
    /// 服务器地址 host[:port]（缺省按默认服务器解析；无需协议头）
    pub server: Option<String>,
}

#[derive(Debug, Args)]
pub struct PushArgs {
    /// <server/project/version/name>（server 为 host[:port]，含端口/IPv6）
    pub target: String,
    /// 待发布的文件或目录
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct PullArgs {
    /// <server/project/version/name>（server 为 host[:port]，含端口/IPv6）
    pub target: String,
    /// 下载归档的精确输出文件路径
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct VersionsArgs {
    /// <server/project>
    pub target: String,
    /// 输出到指定文件（缺省 stdout）
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// 输出格式：text 或 json（缺省 text）
    #[arg(long, value_parser = ["text", "json"], default_value = "text")]
    pub format: String,
}

#[derive(Debug, Args)]
pub struct NewVersionArgs {
    /// <server/project/version>（必须为尚未创建的版本）
    pub target: String,
}

#[derive(Debug, Args)]
pub struct LockVersionArgs {
    /// <server/project/version>（锁定后不可逆）
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
        return Err(format!("目标必须为 {form}；实际输入：{value}"));
    }
    let server = raw[0].trim().to_string();
    if server.is_empty() {
        return Err(format!("server 不能为空，必须为 {form}；实际输入：{value}"));
    }
    let mut fields = Vec::with_capacity(expected_segments - 1);
    for segment in &raw[1..] {
        let field = segment.trim().to_string();
        if field.is_empty() {
            return Err(format!(
                "目标字段不能为空，必须为 {form}；实际输入：{value}"
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
