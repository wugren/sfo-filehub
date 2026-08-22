//! 测试公共工具：临时配置、内存 SQLite、AppState 装配与 .tar.gz 夹具。

use std::io::{Cursor, Write};
use std::path::PathBuf;

use filehub_server::account::store::connect_pool;
use filehub_server::http::AppState;
use filehub_server::model::{
    FileId, FilesConfig, HttpConfigSeed, ServerConfig, UserConfig, UsersConfig, Visibility,
};
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqlitePool;
use sfo_account::AccountStore;

pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

pub fn test_config(data_dir: &std::path::Path, db_path: &str) -> ServerConfig {
    ServerConfig {
        server: HttpConfigSeed {
            server_addr: "127.0.0.1".to_string(),
            port: 0,
            allow_origins: vec![],
            allow_methods: vec![],
            allow_headers: vec![],
            expose_headers: vec![],
            max_age: 3600,
            support_credentials: false,
        },
        users: UsersConfig {
            users: vec![
                UserConfig {
                    username: "alice".to_string(),
                    password: Some("alice-pass".to_string()),
                    password_hash: None,
                    role: Some("owner".to_string()),
                },
                UserConfig {
                    username: "bob".to_string(),
                    password: Some("bob-pass".to_string()),
                    password_hash: None,
                    role: Some("member".to_string()),
                },
            ],
            session_key: "test-session-key-please-change".to_string(),
        },
        files: FilesConfig {
            data_dir: data_dir.to_path_buf(),
            max_archive_bytes: 1024 * 1024,
        },
        db_path: db_path.to_string(),
    }
}

pub async fn assemble(config: &ServerConfig) -> Result<(AppState, SqlitePool), String> {
    let db = connect_pool(&config.db_path, 1)
        .await
        .map_err(|e| format!("open db: {e}"))?;
    let state = AppState::assemble(config, &db).await?;
    Ok((state, db))
}

pub async fn user_id(state: &AppState, username: &str) -> filehub_server::model::UserId {
    state
        .account
        .store()
        .get_account_by_name(username)
        .await
        .expect("account exists")
        .expect("account row")
        .id
}

pub async fn login_session(state: &AppState, username: &str, password: &str) -> String {
    use sfo_account::AccountManager;
    let manager = state.account.manager();
    let (session, _refresh) = manager
        .login(username, password, 1700000000, None)
        .await
        .expect("login ok");
    session
}

pub fn make_targz(name: &str, content: &[u8]) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut encoder = GzEncoder::new(cursor, Compression::default());
    {
        let mut archive_buf = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut archive_buf);
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, name, content).expect("tar entry");
            builder.finish().expect("tar finish");
        }
        encoder.write_all(&archive_buf).expect("gz write");
    }
    encoder.finish().expect("gz finish").into_inner()
}

pub fn make_project(project_id: i64, name: &str, visibility: Visibility, owner: i64) -> filehub_server::model::ProjectRecord {
    filehub_server::model::ProjectRecord {
        project_id: filehub_server::model::ProjectId(project_id),
        name: name.to_string(),
        visibility,
        owner: filehub_server::model::UserId(owner),
    }
}

pub fn file_record(file_id: &str, sha256: &str, size: u64) -> filehub_server::model::FileRecord {
    filehub_server::model::FileRecord {
        file_id: FileId::new(file_id.to_string()),
        sha256: sha256.to_string(),
        size,
    }
}

pub async fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "filehub-test-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}
