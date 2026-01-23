//! Tests for the config-paths crate.

use config_paths::defaults;

#[test]
fn test_default_paths_are_absolute() {
    let paths = defaults();

    for path in paths {
        assert!(
            path.is_absolute(),
            "Path {} is not absolute",
            path.display()
        );
    }
}
