//! Mailbox monitoring helpers.

use std::time::Duration;

/// Default IDLE timeout (seconds) when not specified in config.
pub const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300;

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

/// Connect and monitor a mailbox based on configured server settings.
pub async fn monitor_mailbox_counts<F, Fut>(
    server: config_core::ServerConfig,
    mailbox: config_core::MailboxConfig,
    notify: F,
) -> color_eyre::eyre::Result<()>
where
    F: FnMut(imap_checker::MailboxCounts) -> Fut + Send,
    Fut: std::future::Future<Output = ()> + Send,
{
    let tls_mode = map_tls_mode(server.tls.mode);
    let port = server.port.unwrap_or_else(|| default_port(tls_mode));
    let tls_server_name = server.tls.server_name.as_deref().unwrap_or(&server.host);
    let idle_timeout_secs = mailbox
        .idle_timeout_secs
        .or(server.idle_timeout_secs)
        .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECS);
    let idle_timeout = Duration::from_secs(idle_timeout_secs);

    tracing::info!(
        server_name = %server.name,
        imap_host = %server.host,
        imap_port = port,
        imap_mailbox = %mailbox.name,
        imap_tls_mode = ?tls_mode,
        "starting IMAP monitor"
    );

    let tcp_stream = tokio::net::TcpStream::connect((server.host.as_str(), port)).await?;
    let tls_connector = imap_tls_rustls::connector()?;
    let client = imap_tls::connect(tcp_stream, tls_server_name, tls_mode, tls_connector).await?;

    let session = client
        .login(&server.credentials.username, &server.credentials.password)
        .await
        .map_err(|(err, _client)| err)?;

    imap_checker::monitor_mailbox_counts(session, mailbox.name, idle_timeout, notify).await?;

    Ok(())
}
