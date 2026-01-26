//! Parts to represent fully-resolved config.

/// Fully-resolved IMAP authentication config.
#[derive(Debug, Clone)]
pub enum Auth {
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
