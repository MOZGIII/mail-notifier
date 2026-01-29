//! Config types.

use std::sync::Arc;

/// Fully resolved bringup configuration shared across mailboxes.
#[derive(Debug)]
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

/// The alias for the [`oauth2_session::Manager`] with the generic parameters specified.
pub type OAuth2SessionManager = oauth2_session::Manager<
    oauth2_token_storage_keyring::KeyringTokenStorage,
    oauth2::EndpointMaybeSet,
    oauth2::EndpointMaybeSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
>;

/// Fully-resolved IMAP authentication config.
#[derive(Debug)]
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
        /// Username for OAuth 2 IMAP authentication.
        user: String,

        /// Access token for OAuth 2 IMAP authentication.
        access_token: String,
    },

    /// Authenticate with the static OAuth 2 credentials.
    OAuth2Session {
        /// Username for OAuth 2 IMAP authentication.
        user: String,

        /// Token provider for the OAuth 2 IMAP authentication.
        session_manager: Box<tokio::sync::Mutex<OAuth2SessionManager>>,
    },
}

/// Fully resolved bringup configuration shared across mailboxes.
#[derive(Debug)]
pub struct Mailbox {
    /// A shared server.
    pub server: Arc<Server>,

    /// Mailbox name (e.g. INBOX).
    pub mailbox: imap_utf7::ImapUtf7String,

    /// Idle timeout.
    pub idle_timeout: std::time::Duration,
}
