//! YAML configuration parsing.
//!
//! The primary value of this crate lies in its comprehensive test suite
//! that validates parsing behavior across various scenarios.

/// The error type.
pub type Error = serde_path_to_error::Error<serde_yaml_bw::Error>;

/// Parse a YAML string into a Config.
pub fn parse_yaml(yaml: &str) -> Result<config_core::Config, Error> {
    serde_path_to_error::deserialize(serde_yaml_bw::Deserializer::from_str(yaml))
}
