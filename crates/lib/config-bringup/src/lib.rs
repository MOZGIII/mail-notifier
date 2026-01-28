//! Lift raw config into mailbox monitor config.

use std::sync::Arc;

pub mod data;
pub mod keyring;

/// Default IDLE timeout (seconds) when not specified in config.
const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300;

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

/// Bringup the server config.
async fn server(
    server: &config_core::ServerConfig,
) -> Result<data::Server, ResolveCredentialsError> {
    let tls_mode = match server.tls.mode {
        config_core::TlsMode::Implicit => imap_tls::TlsMode::Implicit,
        config_core::TlsMode::StartTls => imap_tls::TlsMode::StartTls,
    };

    let port = server.port.unwrap_or(match tls_mode {
        imap_tls::TlsMode::Implicit => 993,
        imap_tls::TlsMode::StartTls => 143,
    });

    let tls_server_name = server
        .tls
        .server_name
        .clone()
        .unwrap_or_else(|| server.host.clone());

    let auth = server_auth(&server.auth).await?;

    Ok(data::Server {
        server_name: server.name.clone(),
        host: server.host.clone(),
        port,
        tls_mode,
        tls_server_name,
        auth,
    })
}

/// Bringup the server auth config.
async fn server_auth(
    auth: &config_core::Auth,
) -> Result<data::ServerAuth, ResolveCredentialsError> {
    Ok(match auth {
        config_core::Auth::Login(credentials) => {
            let password = resolve_password(credentials).await?;

            data::ServerAuth::Login {
                username: credentials.username.clone(),
                password,
            }
        }
        config_core::Auth::OAuth2Credentials(oauth2) => data::ServerAuth::OAuth2Credentials {
            user: oauth2.user.clone(),
            access_token: oauth2.access_token.clone(),
        },
    })
}

/// Build an IMAP mailbox config.
fn mailbox(
    bringup_server: Arc<data::Server>,
    core_server: &config_core::ServerConfig,
    core_mailbox: &config_core::MailboxConfig,
) -> data::Mailbox {
    let idle_timeout_secs = core_mailbox
        .idle_timeout_secs
        .or(core_server.idle_timeout_secs)
        .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECS);

    data::Mailbox {
        server: bringup_server,
        mailbox: imap_utf7::ImapUtf7String::from_utf8(&core_mailbox.name),
        idle_timeout: std::time::Duration::from_secs(idle_timeout_secs),
    }
}

/// Bringup the full config for monitoring purposes.
pub async fn for_monitoring(
    core_config: &config_core::Config,
) -> Result<Vec<data::Mailbox>, ResolveCredentialsError> {
    let mut list = Vec::new();

    for core_server in &core_config.servers {
        let bringup_server = server(core_server).await?;
        let bringup_server = Arc::new(bringup_server);

        for core_mailbox in &core_server.mailboxes {
            let bringup_server = Arc::clone(&bringup_server);
            let bringup_mailbox = mailbox(bringup_server, core_server, core_mailbox);

            list.push(bringup_mailbox);
        }
    }

    Ok(list)
}

/// Bringup the partial config for server operations.
pub async fn servers_only(
    core_config: &config_core::Config,
) -> Result<Vec<data::Server>, ResolveCredentialsError> {
    let mut list = Vec::new();

    for core_server in &core_config.servers {
        let bringup_server = server(core_server).await?;

        list.push(bringup_server);
    }

    Ok(list)
}

/// Resolve the password from config, including keyring lookups.
async fn resolve_password(
    credentials: &config_core::LoginCredentials,
) -> Result<String, ResolveCredentialsError> {
    match &credentials.password {
        config_core::PasswordSource::Plain(password) => Ok(password.clone()),
        config_core::PasswordSource::Keyring { keyring } => {
            let keyring =
                keyring::service_account(keyring, &credentials.username, keyring::DEFAULT_SERVICE);
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

/// Errors returned while resolving credentials.
#[derive(Debug, thiserror::Error)]
pub enum ResolveCredentialsError {
    /// Failed to read the password from the keyring.
    #[error(transparent)]
    Keyring(#[from] keyring_password::GetError),
}
