use std::env;
use std::path::PathBuf;

use filehub_server::account::store::connect_pool;
use filehub_server::http::{AppState, register_api};
use filehub_server::model::ServerConfig;
use sfo_http::actix_server::ActixHttpServer;
use sfo_http::http_server::HttpServerConfig;

#[tokio::main]
async fn main() -> Result<(), String> {
    sfo_log::Logger::new("filehub-server")
        .set_log_level("info")
        .start()
        .map_err(|e| format!("init logger failed: {e}"))?;

    let config_path = env::args()
        .nth(1)
        .or_else(|| env::var("FH_CONFIG").ok())
        .unwrap_or_else(|| "filehub-server.yaml".to_string());
    let raw = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("read config {config_path} failed: {e}"))?;
    let config: ServerConfig = serde_saphyr::from_str(&raw).map_err(|error| {
        if let Some(location) = error.location() {
            format!(
                "parse config {config_path} failed: invalid YAML at line {}, column {}",
                location.line(),
                location.column()
            )
        } else {
            format!("parse config {config_path} failed: invalid YAML")
        }
    })?;
    config
        .users
        .validate()
        .map_err(|e| format!("invalid config: {e}"))?;

    let db_path = config.db_path.clone();
    if let Some(parent) = PathBuf::from(&db_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create db parent failed: {e}"))?;
        }
    }
    let db = connect_pool(&db_path, 5)
        .await
        .map_err(|e| format!("open sqlite failed: {e}"))?;
    let state = AppState::assemble(&config, &db).await?;
    log::info!(
        "startup gc removed {} orphan files",
        state.startup_gc().await?.len()
    );

    let mut server = ActixHttpServer::new(
        HttpServerConfig::new(config.server.server_addr.clone(), config.server.port)
            .allow_origins(config.server.allow_origins.clone())
            .allow_methods(config.server.allow_methods.clone())
            .allow_headers(config.server.allow_headers.clone())
            .expose_headers(config.server.expose_headers.clone())
            .max_age(config.server.max_age)
            .support_credentials(config.server.support_credentials),
    );
    register_api(&mut server, state).await;
    server
        .run()
        .await
        .map_err(|e| format!("http server failed: {e}"))?;
    Ok(())
}
