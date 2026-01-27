//! Tests for config YAML parsing.

use config_core::*;

fn must_parse(yaml: &str) -> Config {
    config_yaml::parse_yaml(yaml).expect("Failed to parse YAML")
}

fn base_server() -> ServerConfig {
    ServerConfig {
        name: "test server".to_string(),
        host: "imap.example.com".to_string(),
        port: None,
        tls: TlsConfig {
            mode: TlsMode::Implicit,
            server_name: None,
        },
        auth: Auth::Login(LoginCredentials {
            username: "user@example.com".to_string(),
            password: PasswordSource::Plain("secret".to_string()),
        }),
        mailboxes: vec![MailboxConfig {
            name: "INBOX".to_string(),
            idle_timeout_secs: None,
        }],
        idle_timeout_secs: None,
    }
}

#[test]
fn test_basic_config_parsing() {
    let yaml = include_str!("fixtures/basic.yml");
    let config = must_parse(yaml);

    let expected = Config {
        servers: vec![base_server()],
        oauth2_clients: Default::default(),
    };

    assert_eq!(config, expected);
}

#[test]
fn test_oauth2_config_parsing() {
    let yaml = include_str!("fixtures/oauth2_simple.yml");
    let config = must_parse(yaml);

    let expected = Config {
        servers: vec![ServerConfig {
            auth: Auth::OAuth2Credentials(OAuth2Credentials {
                user: "user@example.com".to_string(),
                access_token: "token123".to_string(),
            }),
            ..base_server()
        }],
        oauth2_clients: Default::default(),
    };

    assert_eq!(config, expected);
}

#[test]
fn test_keyring_config_parsing() {
    let yaml = include_str!("fixtures/keyring.yml");
    let config = must_parse(yaml);

    let expected = Config {
        servers: vec![ServerConfig {
            auth: Auth::Login(LoginCredentials {
                username: "user@example.com".to_string(),
                password: PasswordSource::Keyring {
                    keyring: KeyringRef {
                        service: None,
                        account: None,
                    },
                },
            }),
            ..base_server()
        }],
        oauth2_clients: Default::default(),
    };

    assert_eq!(config, expected);
}

#[test]
fn test_starttls_config_parsing() {
    let yaml = include_str!("fixtures/starttls.yml");
    let config = must_parse(yaml);

    let expected = Config {
        servers: vec![ServerConfig {
            tls: TlsConfig {
                mode: TlsMode::StartTls,
                ..base_server().tls
            },
            ..base_server()
        }],
        oauth2_clients: Default::default(),
    };

    assert_eq!(config, expected);
}

#[test]
fn test_starttls_alias_config_parsing() {
    let yaml = include_str!("fixtures/starttls_alias.yml");
    let config = must_parse(yaml);

    let expected = Config {
        servers: vec![ServerConfig {
            tls: TlsConfig {
                mode: TlsMode::StartTls,
                ..base_server().tls
            },
            ..base_server()
        }],
        oauth2_clients: Default::default(),
    };

    assert_eq!(config, expected);
}

#[test]
fn test_idle_timeout_config_parsing() {
    let yaml = include_str!("fixtures/idle_timeout.yml");
    let config = must_parse(yaml);

    let expected = Config {
        servers: vec![ServerConfig {
            idle_timeout_secs: Some(120),
            ..base_server()
        }],
        oauth2_clients: Default::default(),
    };

    assert_eq!(config, expected);
}

#[test]
fn test_keyring_overrides_config_parsing() {
    let yaml = include_str!("fixtures/keyring_overrides.yml");
    let config = must_parse(yaml);

    let expected = Config {
        servers: vec![ServerConfig {
            auth: Auth::Login(LoginCredentials {
                username: "user@example.com".to_string(),
                password: PasswordSource::Keyring {
                    keyring: KeyringRef {
                        service: Some("mail-notifier".to_string()),
                        account: Some("user@example.com".to_string()),
                    },
                },
            }),
            ..base_server()
        }],
        oauth2_clients: Default::default(),
    };

    assert_eq!(config, expected);
}
