//! Shared configuration types for mail-notifier.

/// Root configuration.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// IMAP servers to monitor.
    pub servers: Vec<ServerConfig>,

    /// OAuth 2 client configurations.
    #[cfg_attr(feature = "serde", serde(default))]
    pub oauth2_clients: std::collections::HashMap<String, OAuth2ClientConfig>,
}

/// A monitored IMAP server.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Debug, Clone, PartialEq)]
pub struct ServerConfig {
    /// Human-friendly name for logging and identification.
    pub name: String,

    /// Hostname or IP address of the IMAP server.
    pub host: String,

    /// Optional port override.
    pub port: Option<u16>,

    /// TLS settings.
    pub tls: TlsConfig,

    /// Authentication settings.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub auth: Auth,

    /// Mailboxes to monitor on this server.
    pub mailboxes: Vec<MailboxConfig>,

    /// Idle timeout override for this server (seconds).
    pub idle_timeout_secs: Option<u64>,
}

/// TLS configuration for a server.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Debug, Clone, PartialEq)]
pub struct TlsConfig {
    /// TLS mode.
    pub mode: TlsMode,

    /// Optional override for the TLS server name (SNI).
    pub server_name: Option<String>,
}

/// Supported TLS modes.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TlsMode {
    /// Implicit TLS (usually port 993).
    Implicit,

    /// STARTTLS upgrade (usually port 143).
    #[cfg_attr(feature = "serde", serde(rename = "starttls", alias = "start_tls"))]
    StartTls,
}

/// IMAP authentication settings.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Debug, Clone, PartialEq)]
pub enum Auth {
    /// Login via username/password.
    Login(LoginCredentials),

    /// Authneticate via OAuth 2 credentials.
    #[cfg_attr(feature = "serde", serde(rename = "oauth2_credentials"))]
    OAuth2Credentials(OAuth2Credentials),

    /// Authenticate via a managed OAuth 2 session.
    #[cfg_attr(feature = "serde", serde(rename = "oauth2_session"))]
    OAuth2Session(OAuth2Session),
}

/// Login credentials for IMAP authentication.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Debug, Clone, PartialEq)]
pub struct LoginCredentials {
    /// Username for IMAP authentication.
    pub username: String,

    /// Password for IMAP authentication.
    pub password: PasswordSource,
}

/// OAuth 2 credentials for IMAP authentication.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Debug, Clone, PartialEq)]
pub struct OAuth2Credentials {
    /// Username for OAuth 2 IMAP authentication.
    pub user: String,

    /// Access token for OAuth 2 IMAP authentication.
    pub access_token: String,
}

/// Managed OAuth 2 session for IMAP authentication.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Debug, Clone, PartialEq)]
pub struct OAuth2Session {
    /// Username for OAuth 2 IMAP authentication.
    pub user: String,

    /// OAuth 2 client to use for IMAP authentication.
    pub oauth2_client: String,

    /// The keyring to use for the OAuth 2 credentials.
    pub keyring: KeyringRef,

    /// If the token expires in less than this duration - refresh it (secs).
    pub expiration_immenance_tolerance_secs: Option<u64>,
}

/// OAuth 2 client configuration.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Debug, Clone, PartialEq)]
pub struct OAuth2ClientConfig {
    /// OAuth 2 client ID.
    pub client_id: String,

    /// OAuth 2 client secret.
    pub client_secret: String,

    /// OAuth 2 token URL.
    pub token_url: String,

    /// OAuth 2 authorization URL.
    pub auth_url: Option<String>,

    /// OAuth 2 device authorization URL.
    pub device_authorization_url: Option<String>,
}

/// Source for a password value.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[derive(Debug, Clone, PartialEq)]
pub enum PasswordSource {
    /// Plaintext password stored directly in config.
    Plain(String),

    /// Reference to a keyring entry nested under a `keyring` field.
    Keyring {
        /// Keyring reference for resolving a password.
        keyring: KeyringRef,
    },
}

/// Keyring reference for resolving a password or secret.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Debug, Clone, PartialEq)]
pub struct KeyringRef {
    /// Keyring service name. Defaults to the application service.
    pub service: Option<String>,

    /// Keyring account name. Defaults to the credentials username.
    pub account: Option<String>,
}

/// A mailbox to monitor.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Debug, Clone, PartialEq)]
pub struct MailboxConfig {
    /// Mailbox name (e.g. INBOX).
    pub name: String,

    /// Idle timeout override for this mailbox (seconds).
    pub idle_timeout_secs: Option<u64>,
}
