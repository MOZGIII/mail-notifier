//! Keyring password resolution helpers.

/// Errors returned while resolving passwords from a keyring.
#[derive(Debug, thiserror::Error)]
pub enum GetError {
    /// Failed to resolve the keyring entry.
    #[error(
        "failed to resolve keyring entry for service '{service}' and account '{account}': {source}"
    )]
    Resolve {
        /// Keyring service name.
        service: String,

        /// Keyring account name.
        account: String,

        /// Underlying keyring error.
        source: keyring_core::Error,
    },
}

/// Errors returned while storing passwords in a keyring.
#[derive(Debug, thiserror::Error)]
pub enum SetError {
    /// Failed to store the keyring entry.
    #[error(
        "failed to store keyring entry for service '{service}' and account '{account}': {source}"
    )]
    Store {
        /// Keyring service name.
        service: String,

        /// Keyring account name.
        account: String,

        /// Underlying keyring error.
        source: keyring_core::Error,
    },
}

/// Get a password from the keyring for the given service/account pair.
pub fn get(service: &str, account: &str) -> Result<String, GetError> {
    let entry = keyring_core::Entry::new(service, account).map_err(|source| GetError::Resolve {
        service: service.to_string(),
        account: account.to_string(),
        source,
    })?;

    entry.get_password().map_err(|source| GetError::Resolve {
        service: service.to_string(),
        account: account.to_string(),
        source,
    })
}

/// Store a password in the keyring for the given service/account pair.
pub fn set(service: &str, account: &str, password: &str) -> Result<(), SetError> {
    let entry = keyring_core::Entry::new(service, account).map_err(|source| SetError::Store {
        service: service.to_string(),
        account: account.to_string(),
        source,
    })?;

    entry
        .set_password(password)
        .map_err(|source| SetError::Store {
            service: service.to_string(),
            account: account.to_string(),
            source,
        })
}

#[cfg(test)]
mod tests;
