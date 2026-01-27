//! Mailbox monitoring entrypoint and configuration.

use std::time::Duration;

pub mod config;

/// Fully resolved mailbox monitoring configuration.
#[derive(Debug, Clone)]
pub struct Config {
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
    pub auth: config::Auth,

    /// Mailbox name (e.g. INBOX).
    pub mailbox: imap_utf7::ImapUtf7String,

    /// Idle timeout.
    pub idle_timeout: Duration,
}

/// Errors returned while monitoring a mailbox.
#[derive(Debug, thiserror::Error)]
pub enum MonitorError {
    /// TLS connector error.
    #[error("IMAP session setup error: {0}")]
    SessionSetup(#[source] imap_session::SetupError),

    /// IMAP monitor error.
    #[error("IMAP monitor error: {0}")]
    Monitor(#[source] imap_checker::MonitorError),
}

/// Connect and monitor a mailbox based on provided settings.
pub async fn monitor<Notify, NotifyFut>(
    config: &Config,
    notify: Notify,
) -> Result<core::convert::Infallible, MonitorError>
where
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

    let auth = match &config.auth {
        config::Auth::Login { username, password } => {
            imap_session::auth::Params::Login { username, password }
        }
        config::Auth::OAuth2Credentials { user, access_token } => {
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
    .map_err(MonitorError::SessionSetup)?;

    imap_checker::monitor_mailbox_counts(session, &config.mailbox, config.idle_timeout, notify)
        .await
        .map_err(MonitorError::Monitor)
}
