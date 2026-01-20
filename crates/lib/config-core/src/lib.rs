//! Shared configuration types for mail-notifier.

/// Root configuration.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// IMAP servers to monitor.
    pub servers: Vec<ServerConfig>,
}

/// A monitored IMAP server.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
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

    /// Credentials for authentication.
    pub credentials: Credentials,

    /// Mailboxes to monitor on this server.
    pub mailboxes: Vec<MailboxConfig>,

    /// Idle timeout override for this server (seconds).
    pub idle_timeout_secs: Option<u64>,
}

/// TLS configuration for a server.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
#[derive(Debug, Clone, PartialEq)]
pub struct TlsConfig {
    /// TLS mode.
    pub mode: TlsMode,

    /// Optional override for the TLS server name (SNI).
    pub server_name: Option<String>,
}

/// Supported TLS modes.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TlsMode {
    /// Implicit TLS (usually port 993).
    Implicit,

    /// STARTTLS upgrade (usually port 143).
    #[cfg_attr(
        feature = "serde",
        serde(alias = "starttls", alias = "start_tls", alias = "start-tls")
    )]
    StartTls,
}

/// Credentials for IMAP authentication.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
#[derive(Debug, Clone, PartialEq)]
pub struct Credentials {
    /// Username for IMAP authentication.
    pub username: String,

    /// Password for IMAP authentication.
    pub password: String,
}

/// A mailbox to monitor.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
#[derive(Debug, Clone, PartialEq)]
pub struct MailboxConfig {
    /// Mailbox name (e.g. INBOX).
    pub name: String,

    /// Idle timeout override for this mailbox (seconds).
    pub idle_timeout_secs: Option<u64>,
}
