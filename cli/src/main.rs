//! filehub-cli 可执行入口：只负责取退出码并退出。

use filehub_cli::cli::run_cli;

#[tokio::main]
async fn main() {
    let code = run_cli().await;
    std::process::exit(code);
}
