//! YAML configuration loading for mail-notifier.

use std::path::{Path, PathBuf};

use config_core::Config;

/// Errors returned while loading YAML configuration.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// Failed to read the YAML file from disk.
    #[error("failed to read config file {path}: {source}")]
    Read {
        /// Path to the configuration file.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to parse the YAML contents.
    #[error("failed to parse YAML config {path}: {source}")]
    Parse {
        /// Path to the configuration file.
        path: PathBuf,
        /// Underlying YAML parse error.
        source: serde_yaml_bw::Error,
    },
}

/// Load configuration from a YAML file on disk.
pub async fn load_from_path<P>(path: P) -> Result<Config, LoadError>
where
    P: AsRef<Path>,
{
    let path_ref = path.as_ref();
    let contents = tokio::fs::read_to_string(path_ref)
        .await
        .map_err(|source| LoadError::Read {
            path: path_ref.to_path_buf(),
            source,
        })?;

    let config = serde_yaml_bw::from_str(&contents).map_err(|source| LoadError::Parse {
        path: path_ref.to_path_buf(),
        source,
    })?;

    Ok(config)
}

/// Parse configuration directly from a YAML string.
pub fn parse_yaml_str(contents: &str) -> Result<Config, serde_yaml_bw::Error> {
    serde_yaml_bw::from_str(contents)
}
