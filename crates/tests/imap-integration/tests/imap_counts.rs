//! Docker-backed IMAP integration tests.

use std::error::Error;

const IMAP_USER: &str = "test";
const IMAP_PASSWORD: &str = "secret";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn imap_counts_roundtrip() -> Result<(), Box<dyn Error + Send + Sync>> {
    imap_integration::require_integration_tests_enabled()?;

    let container = imap_integration::start_greenmail(IMAP_USER, IMAP_PASSWORD).await?;

    let host = container.get_host().await?;
    let host_port = container
        .get_host_port_ipv4(imap_integration::IMAP_PORT)
        .await?;

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
