//! Parts to represent fully-resolved config.

/// Fully-resolved IMAP authentication config.
#[derive(Debug)]
pub enum Auth<OAuth2SessionAccessTokenProvider> {
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
        access_token_provider: OAuth2SessionAccessTokenProvider,
    },
}
