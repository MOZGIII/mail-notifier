//! Lift raw config into mailbox monitor config.

use std::sync::Arc;

/// Type alias for OAuth2 client with auth and token endpoints configured.
pub type OAuth2Client = oauth2::basic::BasicClient<
    oauth2::EndpointMaybeSet,
    oauth2::EndpointMaybeSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointSet,
>;

struct OAuth2Context {
    oauth2_clients: std::collections::HashMap<String, OAuth2Client>,
    reqwest_client: reqwest::Client,
}

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
#[derive(Debug)]
pub struct BringupServerConfig<OAuth2SessionAccessTokenProvider> {
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
    pub auth: imap_monitor::config::Auth<OAuth2SessionAccessTokenProvider>,
}

/// Build a resolved bringup server config from config-core types.
pub async fn bringup_server_config(
    server: &config_core::ServerConfig,
    oauth2_context: &OAuth2Context,
) -> Result<BringupServerConfig, BringupAuthError> {
    let tls_mode = map_tls_mode(server.tls.mode);
    let port = server.port.unwrap_or_else(|| default_port(tls_mode));
    let tls_server_name = server
        .tls
        .server_name
        .clone()
        .unwrap_or_else(|| server.host.clone());

    let auth = bringup_server_auth_config(&server.auth, oauth2_context).await?;

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
    oauth2_context: &OAuth2Context,
) -> Result<imap_monitor::config::Auth, BringupAuthError> {
    Ok(match auth {
        config_core::Auth::Login(credentials) => {
            let password = resolve_password(credentials)
                .await
                .map_err(BringupAuthError::CredentialResolving)?;

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
        config_core::Auth::OAuth2Session(oauth2) => {
            let oauth2_client = oauth2_context
                .oauth2_clients
                .get(&oauth2.oauth2_client)
                .ok_or_else(|| BringupAuthError::OAuth2ClientNotFound {
                    oauth2_client: oauth2.oauth2_client,
                })?;

            let stirage = oauth2_token_storage_keyring::KeyringTokenStorage::init(
                oauth2.keyring.service,
                oauth2.keyring.account,
            )
            .await?;

            let session_manager = oauth2_session::Manager {
                expiration_immenance_tolerance: std::time::Duration::from_secs(
                    oauth2
                        .expiration_immenance_tolerance_secs
                        .unwrap_or(60 * 60),
                ),
                oauth2_client: oauth2_client.clone(),
                http_client: reqwest::Client::new(),
                storage,
            };

            imap_monitor::config::Auth::OAuth2Session {
                user: oauth2.user.clone(),
                session_manager,
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

/// Build resolved OAuth2 clients from config.
pub async fn bringup_oauth2_clients(
    oauth2_clients: &std::collections::HashMap<String, config_core::OAuth2ClientConfig>,
) -> Result<std::collections::HashMap<String, Arc<ConfiguredOAuth2Client>>, BringupOAuth2Error> {
    let mut clients = std::collections::HashMap::new();

    for (name, config) in oauth2_clients {
        let client_id = oauth2::ClientId::new(config.client_id.clone());
        let client_secret = oauth2::ClientSecret::new(config.client_secret.clone());

        let token_url = oauth2::TokenUrl::new(config.token_url.clone())
            .map_err(BringupOAuth2Error::InvalidTokenUrl)?;

        let auth_url = config
            .auth_url
            .clone()
            .map(oauth2::AuthUrl::new)
            .transpose()
            .map_err(BringupOAuth2Error::InvalidAuthUrl)?;

        let device_authorization_url = config
            .device_authorization_url
            .clone()
            .map(oauth2::DeviceAuthorizationUrl::new)
            .transpose()
            .map_err(BringupOAuth2Error::DeviceAuthorizationUrl)?;

        let oauth2_client = oauth2::basic::BasicClient::new(client_id)
            .set_client_secret(client_secret)
            .set_token_uri(token_url)
            .set_auth_uri_option(auth_url)
            .set_device_authorization_url_option(device_authorization_url);

        clients.insert(name.clone(), Arc::new(oauth2_client));
    }

    Ok(clients)
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

/// Errors returned while in auth bringup.
#[derive(Debug, thiserror::Error)]
pub enum BringupAuthError {
    /// Failed to resolve credentials.
    #[error(transparent)]
    CredentialResolving(ResolveCredentialsError),

    /// Failed to resolve credentials.
    #[error("oauth2 client {oauth2_client} not found")]
    OAuth2ClientNotFound { oauth2_client: String },
}

/// Errors returned while resolving credentials.
#[derive(Debug, thiserror::Error)]
pub enum ResolveCredentialsError {
    /// Failed to read the password from the keyring.
    #[error(transparent)]
    Keyring(#[from] keyring_password::GetError),
}

/// Errors returned while bringing up OAuth2 clients.
#[derive(Debug, thiserror::Error)]
pub enum BringupOAuth2Error {
    /// Invalid token URL.
    #[error("invalid token URL: {0}")]
    InvalidTokenUrl(#[source] oauth2::url::ParseError),

    /// Invalid auth URL.
    #[error("invalid auth URL: {0}")]
    InvalidAuthUrl(#[source] oauth2::url::ParseError),

    /// Invalid device authorization URL.
    #[error("invalid device authorization URL: {0}")]
    DeviceAuthorizationUrl(#[source] oauth2::url::ParseError),

    /// Failed to initialize keyring storage.
    #[error("failed to initialize keyring storage: {0}")]
    KeyringInit(#[source] keyring_core::Error),
}
