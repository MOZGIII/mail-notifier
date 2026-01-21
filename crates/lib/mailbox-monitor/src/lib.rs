//! Mailbox monitoring entrypoint and configuration.

use std::time::Duration;

/// Fully resolved mailbox monitoring configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct MailboxMonitorConfig {
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
pub enum MonitorMailboxError {
    /// Network I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// TLS connector error.
    #[error("TLS connector error: {0}")]
    Connector(#[from] imap_tls_rustls::TlsConnectError),

    /// IMAP connection error.
    #[error("IMAP connection error: {0}")]
    Connect(#[from] imap_tls::ConnectError<imap_tls_rustls::TlsConnectError>),

    /// IMAP login error.
    #[error("IMAP login error: {0}")]
    Login(#[from] async_imap::error::Error),

    /// IMAP monitor error.
    #[error("IMAP monitor error: {0}")]
    Monitor(#[from] imap_checker::MonitorError),
}

/// Connect and monitor a mailbox based on provided settings.
pub async fn monitor_mailbox_counts<F, Fut>(
    config: MailboxMonitorConfig,
    notify: F,
) -> Result<(), MonitorMailboxError>
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

    let tcp_stream = tokio::net::TcpStream::connect((config.host.as_str(), config.port)).await?;
    let tls_connector = imap_tls_rustls::connector()?;
    let client = imap_tls::connect(
        tcp_stream,
        &config.tls_server_name,
        config.tls_mode,
        tls_connector,
    )
    .await?;

    let session = client
        .login(&config.username, &config.password)
        .await
        .map_err(|(err, _client)| err)?;

    imap_checker::monitor_mailbox_counts(session, &config.mailbox, config.idle_timeout, notify)
        .await?;

    Ok(())
}
