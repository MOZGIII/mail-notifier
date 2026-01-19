//! Docker-backed IMAP integration tests.

use std::error::Error;
use std::time::Duration;

use async_imap::Client;
use testcontainers::{ImageExt as _, core::IntoContainerPort as _, runners::AsyncRunner as _};
use tokio::net::TcpStream;

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

    let mut session = connect_with_retry(&host.to_string(), host_port).await?;

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

async fn connect_with_retry(
    host: &str,
    port: u16,
) -> Result<async_imap::Session<TcpStream>, std::io::Error> {
    let try_connect = || async move {
        let stream = TcpStream::connect((host, port)).await?;

        let mut client = Client::new(stream);

        let Some(_) = client.read_response().await? else {
            return Err(std::io::Error::other("missing IMAP greeting"));
        };

        let session = client
            .login(IMAP_USER, IMAP_PASSWORD)
            .await
            .map_err(|(err, _)| std::io::Error::other(err))?;

        Ok(session)
    };

    let mut attempts = 60u8;
    loop {
        let err = match try_connect().await {
            Ok(session) => return Ok(session),
            Err(err) => err,
        };

        let Some(attempts_left) = attempts.checked_sub(1) else {
            return Err(err);
        };

        attempts = attempts_left;
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}
