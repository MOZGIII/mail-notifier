//! Configuration loading orchestration for mail-notifier.

use std::path::PathBuf;

use config_core::Config;

/// Load configuration using the standard mail-notifier configuration loading process.
#[cfg(feature = "env")]
pub async fn with_default_env_var() -> Result<Config, WithDefaultEnvVarError> {
    let env_path = envfury::maybe("MAIL_NOTIFIER_CONFIG").map_err(WithDefaultEnvVarError::Env)?;
    with(env_path)
        .await
        .map_err(WithDefaultEnvVarError::Resolver)
}

/// Errors that can occur during configuration loading.
#[cfg(feature = "env")]
#[derive(Debug, thiserror::Error)]
pub enum WithDefaultEnvVarError {
    /// Env variable reading error.
    #[error("config path env var read: {0}")]
    Env(#[source] envfury::Error<envfury::ValueError<<PathBuf as std::str::FromStr>::Err>>),

    /// Resolving configuration error.
    #[error(transparent)]
    Resolver(#[from] config_resolver::LoadError<YamlError>),
}

/// Load configuration using the standard mail-notifier configuration loading process but
/// with a custom env path value.
pub async fn with(
    env_path: Option<PathBuf>,
) -> Result<Config, config_resolver::LoadError<YamlError>> {
    let paths: Vec<PathBuf> = config_paths::resolve(env_path).collect();
    let meta_config = config_resolver::load(&paths, |s| serde_yaml_bw::from_str(&s)).await?;
    Ok(meta_config.payload)
}

/// A convenience type-alias for the YAML parser error type.
pub type YamlError = serde_yaml_bw::Error;
