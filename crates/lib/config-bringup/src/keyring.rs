//! Keyring bringup utils.

/// Default keyring service name.
pub const DEFAULT_SERVICE: &str = "mail-notifier";

/// Keyring service/account pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceAccount<'a> {
    /// Keyring service name.
    pub service: &'a str,

    /// Keyring account name.
    pub account: &'a str,
}

/// Resolve the keyring service/account for the given keyring reference.
pub fn service_account<'a>(
    keyring: &'a config_core::KeyringRef,
    username: &'a str,
    default_service: &'a str,
) -> ServiceAccount<'a> {
    let service = keyring.service.as_deref().unwrap_or(default_service);
    let account = keyring.account.as_deref().unwrap_or(username);
    ServiceAccount { service, account }
}
