//! Config structs.

use std::sync::Arc;

/// Fully resolved bringup configuration shared across mailboxes.
#[derive(Debug, Clone)]
pub struct Server {
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
    pub auth: ServerAuth,
}

/// Fully-resolved IMAP authentication config.
#[derive(Debug, Clone)]
pub enum ServerAuth {
    /// Login with username/password.
    Login {
        /// Username for IMAP authentication.
        username: String,

        /// Password for IMAP authentication.
        password: String,
    },

    /// Authenticate with the static OAuth 2 credentials.
    OAuth2Credentials {
        /// Username for OAuth2 IMAP authentication.
        user: String,

        /// Access token for OAuth2 IMAP authentication.
        access_token: String,
    },
}

/// Fully resolved bringup configuration shared across mailboxes.
#[derive(Debug, Clone)]
pub struct Mailbox {
    /// A shared server.
    pub server: Arc<Server>,

    /// Mailbox name (e.g. INBOX).
    pub mailbox: imap_utf7::ImapUtf7String,

    /// Idle timeout.
    pub idle_timeout: std::time::Duration,
}
