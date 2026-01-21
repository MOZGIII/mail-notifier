//! Keyring store initialization helpers.

use std::sync::Arc;

use keyring_core::CredentialStore;

/// Errors returned while initializing the default keyring store.
#[derive(Debug, thiserror::Error)]
pub enum KeyringInitError {
    /// The platform does not have a supported keyring store.
    #[error("no supported keyring store for this platform")]
    UnsupportedPlatform,

    /// Failed to initialize the platform keyring store.
    #[error("failed to initialize keyring store: {source}")]
    Store {
        /// Underlying keyring error.
        source: keyring_core::Error,
    },
}

/// Guard that keeps the default keyring store initialized.
#[derive(Debug)]
pub struct KeyringGuard;

impl KeyringGuard {
    /// Initialize the default keyring store for the current platform.
    pub fn init_default() -> Result<Self, KeyringInitError> {
        let store = platform_store()?;
        keyring_core::set_default_store(store);
        Ok(Self)
    }
}

impl Drop for KeyringGuard {
    fn drop(&mut self) {
        keyring_core::unset_default_store();
    }
}

#[cfg(target_os = "linux")]
/// Build the default credential store for the current platform.
fn platform_store() -> Result<Arc<CredentialStore>, KeyringInitError> {
    dbus_secret_service_keyring_store::Store::new()
        .map(|store| store as Arc<CredentialStore>)
        .map_err(|source| KeyringInitError::Store { source })
}

#[cfg(target_os = "freebsd")]
/// Build the default credential store for the current platform.
fn platform_store() -> Result<Arc<CredentialStore>, KeyringInitError> {
    dbus_secret_service_keyring_store::Store::new()
        .map(|store| store as Arc<CredentialStore>)
        .map_err(|source| KeyringInitError::Store { source })
}

#[cfg(target_os = "windows")]
/// Build the default credential store for the current platform.
fn platform_store() -> Result<Arc<CredentialStore>, KeyringInitError> {
    windows_native_keyring_store::Store::new()
        .map(|store| store as Arc<CredentialStore>)
        .map_err(|source| KeyringInitError::Store { source })
}

#[cfg(target_os = "macos")]
/// Build the default credential store for the current platform.
fn platform_store() -> Result<Arc<CredentialStore>, KeyringInitError> {
    apple_native_keyring_store::keychain::Store::new()
        .map(|store| store as Arc<CredentialStore>)
        .map_err(|source| KeyringInitError::Store { source })
}

#[cfg(target_os = "ios")]
/// Build the default credential store for the current platform.
fn platform_store() -> Result<Arc<CredentialStore>, KeyringInitError> {
    apple_native_keyring_store::protected::Store::new()
        .map(|store| store as Arc<CredentialStore>)
        .map_err(|source| KeyringInitError::Store { source })
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "windows",
    target_os = "macos",
    target_os = "ios",
)))]
/// Build the default credential store for the current platform.
fn platform_store() -> Result<Arc<CredentialStore>, KeyringInitError> {
    Err(KeyringInitError::UnsupportedPlatform)
}
