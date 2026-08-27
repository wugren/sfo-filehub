use filehub_server::model::{ServerConfig, UsersConfig};

const VALID_PRIVATE_KEY: &str = "-----BEGIN PRIVATE KEY-----\n\
MC4CAQAwBQYDK2VwBCIEIJGVLyTXHTLSMPclke6+1xCFTfX+TmVRcs6UNiMW35Ok\n\
-----END PRIVATE KEY-----\n";

fn users(session_private_key: &str) -> UsersConfig {
    UsersConfig {
        users: vec![],
        session_private_key: session_private_key.to_string(),
    }
}

#[test]
fn validate_accepts_ed25519_pkcs8_private_key() {
    assert!(users(VALID_PRIVATE_KEY).validate().is_ok());
}

#[test]
fn validate_rejects_non_ed25519_private_key_without_echoing_it() {
    let x25519_pkcs8 = "-----BEGIN PRIVATE KEY-----\n\
MC4CAQAwBQYDK2VuBCIEIAht8hhOTTg3JhKw2cgCB8F51Qt/Qz8dYoTg4ds9UJFt\n\
-----END PRIVATE KEY-----\n";
    for invalid in ["not-a-private-session-key", x25519_pkcs8] {
        let err = users(invalid)
            .validate()
            .err()
            .expect("invalid or non-Ed25519 key must fail validation");
        assert!(
            err.contains("Ed25519 PKCS#8 PEM"),
            "unexpected error: {err}"
        );
        assert!(!err.contains(invalid), "error must not echo private key");
    }
}

#[test]
fn yaml_example_loads_with_documented_defaults() {
    let raw = include_str!("../../config.example.yaml");
    let config: ServerConfig =
        serde_saphyr::from_str(raw).expect("commented YAML example must deserialize");

    assert_eq!(config.server.server_addr, "0.0.0.0");
    assert_eq!(config.server.port, 8080);
    assert!(config.server.allow_origins.is_empty());
    assert!(config.server.allow_methods.is_empty());
    assert!(config.server.allow_headers.is_empty());
    assert!(config.server.expose_headers.is_empty());
    assert_eq!(config.server.max_age, 3600);
    assert!(!config.server.support_credentials);
    assert_eq!(config.server.login_rate_limit_per_minute, 30);
    assert_eq!(config.server.login_rate_limit_window_secs, 60);
    assert_eq!(config.db_path, "filehub.db");
    assert_eq!(config.files.data_dir.to_string_lossy(), "./data/files");
    assert_eq!(config.files.max_archive_bytes, 104_857_600);
    assert_eq!(config.users.users.len(), 1);
    assert_eq!(config.users.users[0].username, "admin");
    assert_eq!(config.users.users[0].password.as_deref(), Some("change-me"));
    assert!(config.users.users[0].password_hash.is_none());
    assert!(config.users.validate().is_ok());
}

#[test]
fn yaml_example_keeps_defaulted_fields_commented_out() {
    let raw = include_str!("../../config.example.yaml");
    for key in [
        "allow_origins",
        "allow_methods",
        "allow_headers",
        "expose_headers",
        "max_age",
        "support_credentials",
        "login_rate_limit_per_minute",
        "login_rate_limit_window_secs",
        "db_path",
    ] {
        assert!(
            raw.lines()
                .any(|line| line.trim_start().starts_with(&format!("# {key}:"))),
            "defaulted field {key} must remain visible as a comment"
        );
        assert!(
            !raw.lines()
                .any(|line| line.trim_start().starts_with(&format!("{key}:"))),
            "defaulted field {key} must not be active in the example"
        );
    }
}

#[test]
fn yaml_config_rejects_missing_required_field() {
    let raw = include_str!("../../config.example.yaml").replace("  port: 8080\n", "");
    let result = serde_saphyr::from_str::<ServerConfig>(&raw);
    assert!(result.is_err(), "server.port must remain required");
}
