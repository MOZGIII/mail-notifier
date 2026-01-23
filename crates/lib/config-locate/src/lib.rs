//! Configuration file loading.

use std::path::{Path, PathBuf};

/// An meta-annotated payload of some kind.
#[derive(Debug)]
pub struct Meta<T> {
    /// The actual payload.
    pub payload: T,

    /// The path.
    pub path: PathBuf,
}

/// Error returned while reading configuration.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// No configuration file found in any of the provided paths.
    #[error("no config file found in paths: {paths:?}")]
    NotFound {
        /// The paths that were tried.
        paths: Vec<PathBuf>,
    },

    /// Failed to read the configuration file from disk.
    #[error("failed to read config file {path}: {source}")]
    Read {
        /// Path to the configuration file.
        path: PathBuf,

        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Try to load configuration contents from the first existing file in the list of paths.
///
/// This function iterates through the provided paths in order, attempts to read each file,
/// and returns the contents of the first one that succeeds. If none can be read, returns an error.
pub async fn read<P>(paths: &[P]) -> Result<Meta<String>, ReadError>
where
    P: AsRef<Path>,
{
    for path in paths {
        let path_ref = path.as_ref();
        match tokio::fs::read_to_string(path_ref).await {
            Ok(contents) => {
                return Ok(Meta {
                    payload: contents,
                    path: path_ref.to_path_buf(),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(ReadError::Read {
                    path: path_ref.to_path_buf(),
                    source,
                });
            }
        }
    }

    Err(ReadError::NotFound {
        paths: paths.iter().map(|p| p.as_ref().to_path_buf()).collect(),
    })
}

/// Error returned while loading configuration.
#[derive(Debug, thiserror::Error)]
pub enum LoadError<LoaderError> {
    /// Failed to read the configuration file.
    #[error(transparent)]
    Read(ReadError),

    /// Failed to parse the configuration contents.
    #[error("failed to load config file {path}: {source}")]
    Load {
        /// Path to the configuration file.
        path: PathBuf,

        /// Underlying loader error.
        #[source]
        source: LoaderError,
    },
}

/// Load configuration by reading from the first existing file and parsing it with the provided loader.
pub async fn load<P, L, T, E>(paths: &[P], loader: L) -> Result<Meta<T>, LoadError<E>>
where
    L: FnOnce(String) -> Result<T, E>,
    P: AsRef<Path>,
{
    let Meta { path, payload } = read(paths).await.map_err(LoadError::Read)?;
    let payload = match (loader)(payload) {
        Ok(val) => val,
        Err(source) => return Err(LoadError::Load { path, source }),
    };
    Ok(Meta { payload, path })
}
