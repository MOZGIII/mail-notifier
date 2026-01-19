//! Main entrypoint.

use std::time::Duration;

/// Parse IMAP TLS mode from an environment value.
fn parse_tls_mode(value: &str) -> color_eyre::eyre::Result<imap_tls::TlsMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "implicit" | "imaps" => Ok(imap_tls::TlsMode::Implicit),
        "starttls" | "start_tls" | "start-tls" => Ok(imap_tls::TlsMode::StartTls),
        other => Err(color_eyre::eyre::eyre!(
            "unsupported IMAP_TLS_MODE '{other}', use 'implicit' or 'starttls'"
        )),
    }
}

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let host: String = envfury::must("IMAP_HOST")?;
    let username: String = envfury::must("IMAP_USERNAME")?;
    let password: String = envfury::must("IMAP_PASSWORD")?;
    let mailbox: String = envfury::or_parse("IMAP_MAILBOX", "INBOX")?;
    let tls_mode_raw: String = envfury::or_parse("IMAP_TLS_MODE", "implicit")?;
    let tls_mode = parse_tls_mode(&tls_mode_raw)?;
    let default_port = match tls_mode {
        imap_tls::TlsMode::Implicit => 993,
        imap_tls::TlsMode::StartTls => 143,
    };
    let port: u16 = envfury::or("IMAP_PORT", default_port)?;
    let tls_server_name: String = envfury::or_parse("IMAP_TLS_SERVER_NAME", &host)?;
    let idle_timeout_secs: u64 = envfury::or_parse("IMAP_IDLE_TIMEOUT_SECS", "300")?;
    let idle_timeout = Duration::from_secs(idle_timeout_secs);

    tracing::info!(
        imap_host = %host,
        imap_port = port,
        imap_mailbox = %mailbox,
        imap_tls_mode = ?tls_mode,
        "starting IMAP monitor"
    );

    let tcp_stream = tokio::net::TcpStream::connect((host.as_str(), port)).await?;
    let tls_connector = imap_tls_rustls::connector()?;
    let client = imap_tls::connect(tcp_stream, &tls_server_name, tls_mode, tls_connector).await?;

    let session = client
        .login(&username, &password)
        .await
        .map_err(|(err, _client)| err)?;

    imap_checker::monitor_new_mail(session, &mailbox, idle_timeout).await?;

    Ok(())
}
