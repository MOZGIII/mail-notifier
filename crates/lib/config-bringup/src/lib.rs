//! Lift raw config into mailbox monitor config.

/// Default IDLE timeout (seconds) when not specified in config.
const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300;

/// Default keyring service name.
const DEFAULT_KEYRING_SERVICE: &str = "mail-notifier";

/// Initialize the default keyring store when the config references keyring credentials.
pub fn init_keyring_if_needed(
    config: &config_core::Config,
) -> Result<Option<keyring_bridge::KeyringGuard>, keyring_bridge::KeyringInitError> {
    let needs_keyring = config.servers.iter().any(|server| {
        matches!(
            server.auth,
            config_core::Auth::Login(config_core::LoginCredentials {
                password: config_core::PasswordSource::Keyring { .. },
                ..
            })
        )
    });

    if !needs_keyring {
        return Ok(None);
    }

    keyring_bridge::KeyringGuard::init_default().map(Some)
}

/// Convert config TLS mode to IMAP TLS mode.
fn map_tls_mode(mode: config_core::TlsMode) -> imap_tls::TlsMode {
    match mode {
        config_core::TlsMode::Implicit => imap_tls::TlsMode::Implicit,
        config_core::TlsMode::StartTls => imap_tls::TlsMode::StartTls,
    }
}

/// Default IMAP port for the given TLS mode.
fn default_port(mode: imap_tls::TlsMode) -> u16 {
    match mode {
        imap_tls::TlsMode::Implicit => 993,
        imap_tls::TlsMode::StartTls => 143,
    }
}

/// Fully resolved bringup configuration shared across mailboxes.
#[derive(Debug, Clone)]
pub struct BringupServerConfig {
    /// Human-friendly name for logging and identification.
    pub server_name: String,

    /// Hostname or IP address of the IMAP server.
    pub host: String,

    /// IMAP port.
    pub port: u16,

    /// TLS mode.
    pub tls_mode: imap_tls::TlsMode,

    /// TLS server name (SNI).
    pub tls_server_name: String,

    /// IMAP authentication.
    pub auth: imap_monitor::config::Auth,
}

/// Build a resolved bringup server config from config-core types.
pub async fn bringup_server_config(
    server: &config_core::ServerConfig,
) -> Result<BringupServerConfig, ResolveCredentialsError> {
    let tls_mode = map_tls_mode(server.tls.mode);
    let port = server.port.unwrap_or_else(|| default_port(tls_mode));
    let tls_server_name = server
        .tls
        .server_name
        .clone()
        .unwrap_or_else(|| server.host.clone());

    let auth = bringup_server_auth_config(&server.auth).await?;

    Ok(BringupServerConfig {
        server_name: server.name.clone(),
        host: server.host.clone(),
        port,
        tls_mode,
        tls_server_name,
        auth,
    })
}

/// Bringup the auth config.
async fn bringup_server_auth_config(
    auth: &config_core::Auth,
) -> Result<imap_monitor::config::Auth, ResolveCredentialsError> {
    Ok(match auth {
        config_core::Auth::Login(credentials) => {
            let password = resolve_password(credentials).await?;

            imap_monitor::config::Auth::Login {
                username: credentials.username.clone(),
                password,
            }
        }
        config_core::Auth::OAuth2Credentials(oauth2) => {
            imap_monitor::config::Auth::OAuth2Credentials {
                user: oauth2.user.clone(),
                access_token: oauth2.access_token.clone(),
            }
        }
    })
}

/// Build a resolved mailbox monitor config from config-core types.
pub async fn bringup_monitor_config(
    server: &config_core::ServerConfig,
    mailbox: &config_core::MailboxConfig,
) -> Result<imap_monitor::Config, ResolveCredentialsError> {
    let server_config = bringup_server_config(server).await?;
    let idle_timeout_secs = mailbox
        .idle_timeout_secs
        .or(server.idle_timeout_secs)
        .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECS);

    Ok(imap_monitor::Config {
        server_name: server_config.server_name,
        host: server_config.host,
        port: server_config.port,
        tls_mode: server_config.tls_mode,
        tls_server_name: server_config.tls_server_name,
        auth: server_config.auth,
        mailbox: imap_utf7::ImapUtf7String::from_utf8(&mailbox.name),
        idle_timeout: std::time::Duration::from_secs(idle_timeout_secs),
    })
}

/// Build resolved mailbox monitor configs for all servers/mailboxes in a config.
pub async fn bringup_monitor_configs(
    config: &config_core::Config,
) -> Result<Vec<imap_monitor::Config>, ResolveCredentialsError> {
    let mut configs = Vec::new();

    for server in &config.servers {
        for mailbox in &server.mailboxes {
            configs.push(bringup_monitor_config(server, mailbox).await?);
        }
    }

    Ok(configs)
}

/// Resolve the password from config, including keyring lookups.
async fn resolve_password(
    credentials: &config_core::LoginCredentials,
) -> Result<String, ResolveCredentialsError> {
    match &credentials.password {
        config_core::PasswordSource::Plain(password) => Ok(password.clone()),
        config_core::PasswordSource::Keyring { keyring } => {
            let keyring = keyring_service_account(keyring, &credentials.username);
            let service = keyring.service.to_owned();
            let account = keyring.account.to_owned();

            let password =
                tokio::task::spawn_blocking(move || keyring_password::get(&service, &account))
                    .await
                    .unwrap()?;
            Ok(password)
        }
    }
}

/// Keyring service/account pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyringServiceAccount<'a> {
    /// Keyring service name.
    pub service: &'a str,

    /// Keyring account name.
    pub account: &'a str,
}

/// Resolve the keyring service/account for the given keyring reference.
pub fn keyring_service_account<'a>(
    keyring: &'a config_core::KeyringRef,
    username: &'a str,
) -> KeyringServiceAccount<'a> {
    let service = keyring
        .service
        .as_deref()
        .unwrap_or(DEFAULT_KEYRING_SERVICE);
    let account = keyring.account.as_deref().unwrap_or(username);
    KeyringServiceAccount { service, account }
}

/// Errors returned while resolving credentials.
#[derive(Debug, thiserror::Error)]
pub enum ResolveCredentialsError {
    /// Failed to read the password from the keyring.
    #[error(transparent)]
    Keyring(#[from] keyring_password::GetError),
}
