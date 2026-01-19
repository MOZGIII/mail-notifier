//! Docker-backed IMAP integration tests.

use std::error::Error;

use testcontainers::{ImageExt as _, core::IntoContainerPort as _, runners::AsyncRunner as _};

const IMAP_USER: &str = "test";
const IMAP_PASSWORD: &str = "secret";
const IMAP_PORT: u16 = 3143;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn imap_counts_roundtrip() -> Result<(), Box<dyn Error + Send + Sync>> {
    imap_integration::require_integration_tests_enabled()?;

    let container = testcontainers::GenericImage::new("greenmail/standalone", "latest")
        .with_exposed_port(IMAP_PORT.tcp())
        .with_wait_for(testcontainers::core::WaitFor::message_on_stdout(
            "Starting GreenMail API server at",
        ))
        .with_env_var("GREENMAIL_USERS", format!("{IMAP_USER}:{IMAP_PASSWORD}"))
        .start()
        .await?;

    let host = container.get_host().await?;
    let host_port = container.get_host_port_ipv4(IMAP_PORT).await?;

    let mut session = imap_integration::connect_with_retry(
        &host.to_string(),
        host_port,
        IMAP_USER,
        IMAP_PASSWORD,
    )
    .await?;

    let before = imap_checker::fetch_counts(&mut session, "INBOX").await?;

    session
        .append(
            "INBOX",
            None,
            None,
            b"Subject: Integration Test\r\n\r\nHello from tests.\r\n",
        )
        .await?;
    session.noop().await?;

    let after = imap_checker::fetch_counts(&mut session, "INBOX").await?;

    assert_eq!(after.total, before.total + 1);
    assert!(after.unread >= before.unread);

    session.logout().await?;

    Ok(())
}
