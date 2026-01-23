//! YAML configuration loading for mail-notifier.

pub use serde_yaml_bw::Error;

/// Parse configuration directly from a YAML string.
pub fn parse_str(contents: &str) -> Result<config_core::Config, Error> {
    serde_yaml_bw::from_str(contents)
}
