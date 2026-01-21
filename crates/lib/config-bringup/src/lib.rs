//! Lift raw config into mailbox monitor config.

/// Default IDLE timeout (seconds) when not specified in config.
const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300;

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

/// Build a resolved mailbox monitor config from config-core types.
pub fn build_monitor_config(
    server: config_core::ServerConfig,
    mailbox: config_core::MailboxConfig,
) -> mailbox_monitor::MailboxMonitorConfig {
    let tls_mode = map_tls_mode(server.tls.mode);
    let port = server.port.unwrap_or_else(|| default_port(tls_mode));
    let tls_server_name = server
        .tls
        .server_name
        .unwrap_or_else(|| server.host.clone());
    let idle_timeout_secs = mailbox
        .idle_timeout_secs
        .or(server.idle_timeout_secs)
        .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECS);

    mailbox_monitor::MailboxMonitorConfig {
        server_name: server.name,
        host: server.host,
        port,
        tls_mode,
        tls_server_name,
        username: server.credentials.username,
        password: server.credentials.password,
        mailbox: mailbox.name,
        idle_timeout: std::time::Duration::from_secs(idle_timeout_secs),
    }
}
