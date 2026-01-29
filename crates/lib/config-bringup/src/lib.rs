//! Bringup the config.

use std::sync::Arc;

mod error;
pub mod keyring;
mod types;

pub(crate) mod internal;

pub use self::error::*;
pub use self::types::*;

/// Default IDLE timeout (seconds) when not specified in config.
const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300;

/// Initialize the default keyring store when the config references keyring credentials.
pub fn init_keyring_if_needed(
    config: &config_core::Config,
) -> Result<Option<keyring_bridge::KeyringGuard>, keyring_bridge::KeyringInitError> {
    let needs_keyring = config.servers.iter().any(|server| {
        matches!(
            server.auth,
            config_core::Auth::Login(config_core::LoginCredentials {
                password: config_core::PasswordSource::Keyring { .. },
                ..
            }) | config_core::Auth::OAuth2Session(..)
        )
    });

    if !needs_keyring {
        return Ok(None);
    }

    keyring_bridge::KeyringGuard::init_default().map(Some)
}

/// Bringup the full config for monitoring purposes.
pub async fn for_monitoring(
    core_config: &config_core::Config,
) -> Result<Vec<Arc<types::Mailbox>>, ConfigError> {
    let oauth2_context = internal::oauth2_context(core_config)?;

    let mut list = Vec::new();

    for core_server in &core_config.servers {
        let bringup_server = internal::server(core_server, &oauth2_context)
            .await
            .map_err(ConfigError::Server)?;
        let bringup_server = Arc::new(bringup_server);

        for core_mailbox in &core_server.mailboxes {
            let bringup_server = Arc::clone(&bringup_server);
            let bringup_mailbox = internal::mailbox(bringup_server, core_server, core_mailbox);
            let bringup_mailbox = Arc::new(bringup_mailbox);
            list.push(bringup_mailbox);
        }
    }

    Ok(list)
}

/// Bringup the partial config for server operations.
pub async fn servers_only(
    core_config: &config_core::Config,
) -> Result<Vec<types::Server>, ConfigError> {
    let oauth2_context = internal::oauth2_context(core_config)?;

    let mut servers = Vec::new();

    for core_server in &core_config.servers {
        let bringup_server = internal::server(core_server, &oauth2_context)
            .await
            .map_err(ConfigError::Server)?;

        servers.push(bringup_server);
    }

    Ok(servers)
}
