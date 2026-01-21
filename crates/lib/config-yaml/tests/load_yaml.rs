//! Tests for YAML config loading.

use std::path::Path;

#[tokio::test]
async fn loads_yaml_fixture() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.yml");
    let config = config_yaml::load_from_path(&fixture_path)
        .await
        .expect("config should load");

    assert_eq!(config.servers.len(), 1);
    let server = &config.servers[0];
    assert_eq!(server.name, "primary");
    assert_eq!(server.host, "imap.example.com");
    assert_eq!(server.port, Some(993));
    assert_eq!(server.tls.mode, config_core::TlsMode::Implicit);
    assert_eq!(server.tls.server_name.as_deref(), Some("imap.example.com"));
    assert_eq!(server.credentials.username, "user@example.com");
    assert!(matches!(
        server.credentials.password,
        config_core::PasswordSource::Plain(ref value) if value == "secret"
    ));
    assert_eq!(server.mailboxes.len(), 2);
    assert_eq!(server.mailboxes[0].name, "INBOX");
    assert_eq!(server.mailboxes[0].idle_timeout_secs, Some(300));
    assert_eq!(server.mailboxes[1].name, "Alerts");
    assert_eq!(server.mailboxes[1].idle_timeout_secs, None);
}
