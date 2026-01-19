//! Integration test harness crate.

/// Returns Ok when integration tests should run, otherwise logs a hint and returns an error.
pub fn require_integration_tests_enabled() -> Result<(), &'static str> {
    if std::env::var("RUN_IMAP_INTEGRATION_TESTS").is_ok() {
        return Ok(());
    }

    eprintln!("skipping IMAP integration tests; set RUN_IMAP_INTEGRATION_TESTS=true to run");

    Err("integration tests disabled")
}
