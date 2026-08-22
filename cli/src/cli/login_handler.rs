//! login / logout 命令编排（fh-cli-login）。

use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;

use super::args::{LoginArgs, LogoutArgs};
use super::{CliError, open_store};
use crate::apiclient::Config;
use crate::apiclient::FilehubClient;

pub async fn run_login(config: Option<&Path>, args: LoginArgs) -> Result<i32, CliError> {
    let mut store = open_store(config)?;
    let mut env_server = std::env::var("FILEHUB_SERVER").ok();
    if env_server
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .is_empty()
    {
        env_server = None;
    }
    let server = store.resolve_server(args.server.as_deref(), env_server.as_deref())?;

    let stdin_is_terminal = std::io::stdin().is_terminal();
    let (mode, username, secret) = collect_login_inputs(&args, stdin_is_terminal)?;

    let base_url = server.clone();
    let client = FilehubClient::new(Config {
        base_url,
        ..Config::default()
    })?;
    match mode {
        LoginMode::Password => {
            let login = client.login_password(&username, &secret).await?;
            store.save_session(&server, &username, &login.session, &login.refresh_session)?;
            store.flush()?;
            log::info!("账号密码登录成功，凭据已保存（不含明文日志）");
            println!("Login Succeeded（{server}，用户 {username}）");
        }
        LoginMode::Token => {
            // 登录时用受保护只读接口验证 token 有效性；无效不发版、不写凭据。
            client
                .list_projects(&secret)
                .await
                .map_err(|e| CliError::Auth(format!("token 校验失败：{e}")))?;
            store.save_token(&server, &secret)?;
            store.flush()?;
            log::info!("token 登录成功，凭据已保存（不含明文日志）");
            println!("Login Succeeded（{server}，token 已保存）");
        }
    }
    Ok(0)
}

pub async fn run_logout(config: Option<&Path>, args: LogoutArgs) -> Result<i32, CliError> {
    let mut store = open_store(config)?;
    let env_server = std::env::var("FILEHUB_SERVER").ok();
    let server = store.logout(args.server.as_deref(), env_server.as_deref())?;
    store.flush()?;
    println!("Logout Succeeded（{server}）");
    Ok(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoginMode {
    Password,
    Token,
}

/// 按「显式选项 > 环境变量 > 交互提示」收集登录输入。
fn collect_login_inputs<'a>(
    args: &'a LoginArgs,
    stdin_is_terminal: bool,
) -> Result<(LoginMode, String, String), CliError> {
    let env_username = non_empty_env("FILEHUB_USERNAME");
    let env_password = non_empty_env("FILEHUB_PASSWORD");
    let env_token = non_empty_env("FILEHUB_TOKEN");

    let password_mode = args.username.is_some()
        || args.password_stdin
        || env_password.is_some()
        || env_username.is_some();
    let token_mode = args.token_stdin;
    if password_mode && token_mode {
        return Err(CliError::Usage(
            "--password-stdin/--username/密码环境变量与 --token-stdin/token 登录互斥".to_string(),
        ));
    }

    let mode = if token_mode {
        LoginMode::Token
    } else if password_mode {
        LoginMode::Password
    } else if env_token.is_some() {
        LoginMode::Token
    } else if !stdin_is_terminal {
        return Err(CliError::Usage(
            "stdin 非终端时必须显式使用 --password-stdin 或 --token-stdin".to_string(),
        ));
    } else {
        // 交互终端且未指定登录方式：提示用户先选择账号密码或 token。
        prompt_login_mode()?
    };

    match mode {
        LoginMode::Token => {
            let token = if args.token_stdin {
                if stdin_is_terminal {
                    println!("Token: ");
                    read_hidden()
                } else {
                    read_stdin_line()
                }
            } else if let Some(token) = env_token {
                token
            } else {
                // 交互选择 token 模式：终端下隐藏输入。
                println!("Token: ");
                read_hidden()
            };
            Ok((LoginMode::Token, String::new(), token))
        }
        LoginMode::Password => {
            password_inputs(&args, stdin_is_terminal, env_username, env_password)
        }
    }
}

/// 交互模式未指定登录方式时，用 ↑/↓ 高亮选择并回车确认。
fn prompt_login_mode() -> Result<LoginMode, CliError> {
    let selection = dialoguer::Select::new()
        .with_prompt("请选择登录方式")
        .items(["账号密码", "Token"])
        .default(0)
        .interact()
        .map_err(|e| CliError::Local(format!("读取登录方式选择失败：{e}")))?;
    login_mode_from_select_index(selection)
}

/// Select 候选项下标映射：0=账号密码，1=Token；其它值视为选择异常。
fn login_mode_from_select_index(index: usize) -> Result<LoginMode, CliError> {
    match index {
        0 => Ok(LoginMode::Password),
        1 => Ok(LoginMode::Token),
        _ => Err(CliError::Usage(
            "登录方式选择无效：请用上下键选择账号密码或 Token".to_string(),
        )),
    }
}

/// 账号密码模式的用户名与密码收集（交互提示不回显）。
fn password_inputs(
    args: &LoginArgs,
    stdin_is_terminal: bool,
    env_username: Option<String>,
    env_password: Option<String>,
) -> Result<(LoginMode, String, String), CliError> {
    let username = match args.username.as_deref() {
        Some(name) if !name.trim().is_empty() => name.trim().to_string(),
        Some(_) => return Err(CliError::Usage("用户名不能为空".to_string())),
        None => env_username.unwrap_or_default(),
    };
    let username = if username.is_empty() {
        if !stdin_is_terminal {
            return Err(CliError::Usage(
                "stdin 非终端时用户名必须经 -u/--username 或 FILEHUB_USERNAME 提供".to_string(),
            ));
        }
        print!("Username: ");
        std::io::stdout()
            .flush()
            .map_err(|e| CliError::Local(e.to_string()))?;
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|e| CliError::Local(format!("读取用户名失败：{e}")))?;
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            return Err(CliError::Usage("用户名不能为空".to_string()));
        }
        trimmed
    } else {
        username.to_string()
    };

    let password = if args.password_stdin {
        if stdin_is_terminal {
            print!("Password: ");
            std::io::stdout()
                .flush()
                .map_err(|e| CliError::Local(e.to_string()))?;
            read_hidden()
        } else {
            read_stdin_line()
        }
    } else if let Some(password) = env_password {
        password
    } else if stdin_is_terminal {
        print!("Password: ");
        std::io::stdout()
            .flush()
            .map_err(|e| CliError::Local(e.to_string()))?;
        read_hidden()
    } else {
        return Err(CliError::Usage(
            "stdin 非终端且未给 --password-stdin 时无法输入密码".to_string(),
        ));
    };
    if password.is_empty() {
        return Err(CliError::Auth("密码不能为空".to_string()));
    }
    Ok((LoginMode::Password, username, password))
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_stdin_line() -> String {
    let mut line = String::new();
    let _ = std::io::stdin().lock().read_line(&mut line);
    line.trim_end_matches(['\r', '\n']).to_string()
}

fn read_hidden() -> String {
    rpassword::read_password().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_terminal_without_mode_is_usage_error() {
        let args = LoginArgs {
            server: None,
            username: Some("alice".to_string()),
            password_stdin: false,
            token_stdin: false,
        };
        let error = collect_login_inputs(&args, false).unwrap_err();
        assert!(matches!(error, CliError::Usage(_)));
    }

    #[test]
    fn token_stdin_from_env_when_present() {
        // Rust 2024：环境变量修改为 unsafe；仅测试进程内使用。
        unsafe { std::env::set_var("FILEHUB_TOKEN", "env-token") };
        let args = LoginArgs {
            server: None,
            username: None,
            password_stdin: false,
            token_stdin: false,
        };
        let result = collect_login_inputs(&args, true).unwrap();
        assert_eq!(result.0, LoginMode::Token);
        assert_eq!(result.2, "env-token");
        unsafe { std::env::remove_var("FILEHUB_TOKEN") };
    }

    #[test]
    fn login_mode_from_select_index_maps_items() {
        assert_eq!(
            login_mode_from_select_index(0).unwrap(),
            LoginMode::Password
        );
        assert_eq!(login_mode_from_select_index(1).unwrap(), LoginMode::Token);
    }

    #[test]
    fn login_mode_from_select_index_rejects_unexpected_index() {
        let error = login_mode_from_select_index(2).unwrap_err();
        assert!(matches!(error, CliError::Usage(_)));
    }
}
