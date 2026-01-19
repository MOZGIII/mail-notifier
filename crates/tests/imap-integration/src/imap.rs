//! IMAP connectivity helpers.

use std::time::Duration;

use async_imap::Client;
use tokio::net::TcpStream;

/// Connects to the IMAP server with retries until it is ready.
pub async fn connect_with_retry(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
) -> Result<async_imap::Session<TcpStream>, std::io::Error> {
    let try_connect = || async move {
        let stream = TcpStream::connect((host, port)).await?;

        let mut client = Client::new(stream);

        let Some(_) = client.read_response().await? else {
            return Err(std::io::Error::other("missing IMAP greeting"));
        };

        let session = client
            .login(user, password)
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
