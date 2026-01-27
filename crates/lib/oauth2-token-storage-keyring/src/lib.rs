//! Keyring-based token storage implementation.

use std::sync::Arc;

use oauth2_token_storage_core::{Data, DataRef, TokenStorage};

/// Keyring-based token storage.
pub struct KeyringTokenStorage {
    /// The entry to store the token at.
    pub entry: Arc<keyring_core::Entry>,
}

impl KeyringTokenStorage {
    /// Initialize the keyring token storage for the given serivce/account.
    pub async fn init(
        service: impl AsRef<str> + Send + 'static,
        account: impl AsRef<str> + Send + 'static,
    ) -> Result<Self, keyring_core::Error> {
        tokio::task::spawn_blocking(move || {
            let entry = keyring_core::Entry::new(service.as_ref(), account.as_ref())?;
            let entry = Arc::new(entry);
            Ok(Self { entry })
        })
        .await
        .unwrap()
    }
}

/// Errors from storage operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Keyring operation failed.
    #[error("keyring operation failed: {0}")]
    Keyring(#[source] keyring_core::Error),

    /// JSON serialization failed.
    #[error("JSON serialization failed: {0}")]
    Json(#[source] serde_json::Error),
}

impl TokenStorage for KeyringTokenStorage {
    type StoreError = Error;
    type LoadError = Error;
    type ClearError = Error;

    async fn store<'a>(&'a self, data: DataRef<'a>) -> Result<(), Self::StoreError> {
        let json = serde_json::to_string(&data).map_err(Error::Json)?;
        let entry = Arc::clone(&self.entry);
        tokio::task::spawn_blocking(move || {
            entry.set_password(&json).map_err(Error::Keyring)?;

            Ok(())
        })
        .await
        .unwrap()
    }

    async fn load(&self) -> Result<Data, oauth2_token_storage_core::LoadError<Self::LoadError>> {
        let entry = Arc::clone(&self.entry);
        tokio::task::spawn_blocking(move || {
            let json = entry
                .get_password()
                .map_err(Error::Keyring)
                .map_err(|err| match err {
                    error @ Error::Keyring(keyring_core::Error::NoEntry) => {
                        oauth2_token_storage_core::LoadError::NoData(error)
                    }
                    error => oauth2_token_storage_core::LoadError::Internal(error),
                })?;

            let data: Data = serde_json::from_str(&json)
                .map_err(Error::Json)
                .map_err(oauth2_token_storage_core::LoadError::Internal)?;

            Ok(data)
        })
        .await
        .unwrap()
    }

    async fn clear(&self) -> Result<(), Self::ClearError> {
        let entry = Arc::clone(&self.entry);
        tokio::task::spawn_blocking(move || {
            entry.delete_credential().map_err(Error::Keyring)?;

            Ok(())
        })
        .await
        .unwrap()
    }
}
