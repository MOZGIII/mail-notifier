//! Mailbox monitoring entrypoint and configuration.

use std::time::Duration;

/// Fully resolved mailbox monitoring configuration.
#[derive(Debug, Clone, PartialEq)]
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

    /// Username for IMAP authentication.
    pub username: String,

    /// Password for IMAP authentication.
    pub password: String,

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
pub async fn monitor<F, Fut>(
    config: &Config,
    notify: F,
) -> Result<core::convert::Infallible, MonitorError>
where
    F: FnMut(imap_checker::MailboxCounts) -> Fut + Send,
    Fut: std::future::Future<Output = ()> + Send,
{
    tracing::info!(
        server_name = %config.server_name,
        imap_host = %config.host,
        imap_port = config.port,
        imap_mailbox = %config.mailbox,
        imap_tls_mode = ?config.tls_mode,
        "starting IMAP monitor"
    );

    let session = imap_session::setup(imap_session::SetupParams {
        host: &config.host,
        port: config.port,
        tls_mode: config.tls_mode,
        tls_server_name: &config.tls_server_name,
        username: &config.username,
        password: &config.password,
    })
    .await
    .map_err(MonitorError::SessionSetup)?;

    imap_checker::monitor_mailbox_counts(session, &config.mailbox, config.idle_timeout, notify)
        .await
        .map_err(MonitorError::Monitor)
}
