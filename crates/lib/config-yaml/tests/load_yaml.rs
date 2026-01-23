//! Tests for YAML config loading.

#[test]
fn loads_yaml_fixture() {
    let yaml = include_str!("fixtures/sample.yml");

    let config = config_yaml::parse_str(yaml).expect("config should parse");

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

#[test]
fn parses_keyring_password_with_defaults() {
    let yaml = "\nservers:\n  - name: primary\n    host: imap.example.com\n    tls:\n      mode: implicit\n    credentials:\n      username: user@example.com\n      password:\n        keyring: {}\n    mailboxes:\n      - name: INBOX\n";

    let config = config_yaml::parse_str(yaml).expect("config should parse");
    let server = &config.servers[0];

    assert!(matches!(
        server.credentials.password,
        config_core::PasswordSource::Keyring { ref keyring }
            if keyring.service.is_none() && keyring.account.is_none()
    ));
}

#[test]
fn parses_keyring_password_with_overrides() {
    let yaml = "\nservers:\n  - name: primary\n    host: imap.example.com\n    tls:\n      mode: implicit\n    credentials:\n      username: user@example.com\n      password:\n        keyring:\n          service: mail-notifier\n          account: user@example.com\n    mailboxes:\n      - name: INBOX\n";

    let config = config_yaml::parse_str(yaml).expect("config should parse");
    let server = &config.servers[0];

    match &server.credentials.password {
        config_core::PasswordSource::Keyring { keyring } => {
            assert_eq!(keyring.service.as_deref(), Some("mail-notifier"));
            assert_eq!(keyring.account.as_deref(), Some("user@example.com"));
        }
        other => panic!("unexpected password source: {other:?}"),
    }
}

#[test]
fn parses_starttls_aliases() {
    let yaml = "\nservers:\n  - name: primary\n    host: imap.example.com\n    tls:\n      mode: starttls\n    credentials:\n      username: user@example.com\n      password: secret\n    mailboxes:\n      - name: INBOX\n";

    let config = config_yaml::parse_str(yaml).expect("config should parse");
    let server = &config.servers[0];

    assert_eq!(server.tls.mode, config_core::TlsMode::StartTls);
}

#[test]
fn parses_server_idle_timeout() {
    let yaml = "\nservers:\n  - name: primary\n    host: imap.example.com\n    tls:\n      mode: implicit\n    idle-timeout-secs: 120\n    credentials:\n      username: user@example.com\n      password: secret\n    mailboxes:\n      - name: INBOX\n";

    let config = config_yaml::parse_str(yaml).expect("config should parse");
    let server = &config.servers[0];

    assert_eq!(server.idle_timeout_secs, Some(120));
}

#[test]
fn rejects_invalid_yaml() {
    let yaml = r#"servers: ["#;

    let error = config_yaml::parse_str(yaml).expect_err("invalid yaml should error");
    let message = error.to_string();
    assert!(!message.is_empty());
}
