//! Mailbox monitoring entrypoint and configuration.

use std::time::Duration;

pub mod config;

/// Fully resolved mailbox monitoring configuration.
#[derive(Debug)]
pub struct Config<OAuth2SessionAccessTokenProvider> {
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
    pub auth: config::Auth<OAuth2SessionAccessTokenProvider>,

    /// Mailbox name (e.g. INBOX).
    pub mailbox: imap_utf7::ImapUtf7String,

    /// Idle timeout.
    pub idle_timeout: Duration,
}

/// Errors returned while monitoring a mailbox.
#[derive(Debug, thiserror::Error)]
pub enum MonitorError<OAuth2SessionAccessTokenProviderError> {
    /// OAuth 2 session error.
    #[error("getting OAuth 2 token for the session error: {0}")]
    OAuth2SessionGetToken(#[source] OAuth2SessionAccessTokenProviderError),

    /// IMAP session setup error.
    #[error("IMAP session setup error: {0}")]
    ImapSessionSetup(#[source] imap_session::SetupError),

    /// IMAP monitor error.
    #[error("IMAP monitor error: {0}")]
    Monitor(#[source] imap_checker::MonitorError),
}

/// Connect and monitor a mailbox based on provided settings.
pub async fn monitor<
    OAuth2SessionAccessTokenProvider,
    OAuth2SessionAccessTokenProviderFut,
    OAuth2SessionAccessTokenProviderError,
    Notify,
    NotifyFut,
>(
    config: &Config<OAuth2SessionAccessTokenProvider>,
    notify: Notify,
) -> Result<core::convert::Infallible, MonitorError<OAuth2SessionAccessTokenProviderError>>
where
    OAuth2SessionAccessTokenProvider: Fn() -> OAuth2SessionAccessTokenProviderFut + Send,
    OAuth2SessionAccessTokenProviderFut:
        std::future::Future<Output = Result<String, OAuth2SessionAccessTokenProviderError>> + Send,
    OAuth2SessionAccessTokenProviderError: std::fmt::Debug,
    Notify: FnMut(imap_checker::MailboxCounts) -> NotifyFut + Send,
    NotifyFut: std::future::Future<Output = ()> + Send,
{
    tracing::info!(
        server_name = %config.server_name,
        imap_host = %config.host,
        imap_port = config.port,
        imap_mailbox = %config.mailbox,
        imap_tls_mode = ?config.tls_mode,
        "starting IMAP monitor"
    );

    // Used to keep the oauth2 session access token value around until
    // the imap session initializes.
    let mut oauth2_session_access_token = None;

    let auth = match config.auth {
        config::Auth::Login {
            ref username,
            ref password,
        } => imap_session::auth::Params::Login { username, password },
        config::Auth::OAuth2Credentials {
            ref user,
            ref access_token,
        } => imap_session::auth::Params::OAuth2 { user, access_token },
        config::Auth::OAuth2Session {
            ref user,
            ref access_token_provider,
        } => {
            let access_token = (access_token_provider)()
                .await
                .map_err(MonitorError::OAuth2SessionGetToken)?;

            let access_token = oauth2_session_access_token.insert(access_token);

            imap_session::auth::Params::OAuth2 { user, access_token }
        }
    };

    let session = imap_session::setup(imap_session::SetupParams {
        host: &config.host,
        port: config.port,
        tls_mode: config.tls_mode,
        tls_server_name: &config.tls_server_name,
        auth,
    })
    .await
    .map_err(MonitorError::ImapSessionSetup)?;

    drop(oauth2_session_access_token);

    imap_checker::monitor_mailbox_counts(session, &config.mailbox, config.idle_timeout, notify)
        .await
        .map_err(MonitorError::Monitor)
}
