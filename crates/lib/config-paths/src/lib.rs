//! Opinionated default configuration file paths for mail-notifier.

use std::path::PathBuf;

use either::Either;

/// Returns an iterator over default configuration file paths.
///
/// The paths are yielded in order of preference:
/// 1. User-specific config directory (XDG standard) - multiple variants
/// 2. User-specific config in home directory (fallback) - multiple variants
/// 3. System-wide config
pub fn defaults() -> impl Iterator<Item = PathBuf> {
    let config_path = dirs::config_dir().into_iter().flat_map(|d| {
        [
            d.join("mail-notifier/config.yaml"),
            d.join("mail-notifier.yaml"),
        ]
    });
    let home_path = dirs::home_dir().into_iter().flat_map(|d| {
        [
            d.join(".mail-notifier.yaml"),
            d.join(".mail-notifier/config.yaml"),
        ]
    });
    let system_path = std::iter::once_with(|| PathBuf::from("/etc/mail-notifier/config.yaml"));

    config_path.chain(home_path).chain(system_path)
}

/// Resolves configuration paths based on environment override or defaults.
///
/// If an environment path is provided, returns an iterator containing only that path.
/// Otherwise, returns the default configuration paths.
pub fn resolve(env_path: Option<PathBuf>) -> impl Iterator<Item = PathBuf> {
    match env_path {
        Some(val) => Either::Left(std::iter::once(val)),
        None => Either::Right(defaults()),
    }
}
