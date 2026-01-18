//! Connection context and settings.

/// How to secure the IMAP connection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TlsMode {
    /// Implicit TLS (usually port 993).
    Implicit,

    /// Start with plaintext and upgrade using STARTTLS (usually port 143).
    StartTls,
}

/// Configuration required to connect to an IMAP server.
#[derive(Clone, Debug)]
pub struct ImapClientContext {
    /// IMAP server hostname.
    pub server: String,

    /// IMAP server port.
    pub port: u16,

    /// Username for authentication.
    pub username: String,

    /// Password for authentication.
    pub password: crate::Password,

    /// Mailbox to monitor (usually `INBOX`).
    pub mailbox: String,

    /// TLS mode for the connection.
    pub tls_mode: TlsMode,

    /// IDLE keepalive timeout. Use less than 29 minutes to avoid server timeouts.
    pub idle_timeout: std::time::Duration,
}
