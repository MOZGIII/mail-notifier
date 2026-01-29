//! Error types.

/// Config bringup error.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Failed to bringup OAuth 2 clients.
    #[error("oauth2 clients: {0}")]
    OAuth2Clients(#[source] OAuth2ClientError),

    /// Failed to bringup server.
    #[error("server: {0}")]
    Server(#[source] ServerError),
}

/// Sever bringup error.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// Failed to bringup server auth.
    #[error("server auth: {0}")]
    ServerAuth(#[source] ServerAuthError),
}

/// Server auth bringup error.
#[derive(Debug, thiserror::Error)]
pub enum ServerAuthError {
    /// Failed to read the password from the keyring.
    #[error("resolve password: {0}")]
    ResolvePassword(#[source] ResolvePasswordError),

    /// The OAuth 2 client with the given name was not found.
    #[error("oauth2 client \"{name}\" not found")]
    OAuth2ClientNotFound {
        /// The name of the client that was missing.
        name: String,
    },

    /// OAuth 2 session keyring init error.
    #[error("oauth2 session keyring: {0}")]
    OAuth2KeyringInit(oauth2_token_storage_keyring::keyring_core::Error),
}

/// Errors returned while resolving password.
#[derive(Debug, thiserror::Error)]
pub enum ResolvePasswordError {
    /// Failed to read the password from the keyring.
    #[error("keyring: {0}")]
    Keyring(#[source] keyring_password::GetError),
}

/// OAuth 2 client bringup error.
#[derive(Debug, thiserror::Error)]
pub enum OAuth2ClientError {
    /// The token URL is not valid.
    #[error("token url: {0}")]
    TokenUrl(oauth2::url::ParseError),

    /// The auth URL is not valid.
    #[error("auth url: {0}")]
    AuthUrl(oauth2::url::ParseError),

    /// The device authorization URL is not valid.
    #[error("device authorization url: {0}")]
    DeviceAuthorizationUrl(oauth2::url::ParseError),
}
