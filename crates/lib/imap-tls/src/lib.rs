//! IMAP connect helpers.

/// Errors returned while connecting to the IMAP server.
#[derive(Debug, thiserror::Error)]
pub enum ConnectError<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// Network I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// TLS connector error.
    #[error("TLS error: {0}")]
    Tls(E),

    /// IMAP protocol error.
    #[error("IMAP error: {0}")]
    Imap(#[from] async_imap::error::Error),

    /// The server did not send the expected greeting.
    #[error("IMAP server sent no greeting")]
    MissingGreeting,
}

/// How to secure the IMAP connection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TlsMode {
    /// Implicit TLS (usually port 993).
    Implicit,

    /// Start with plaintext and upgrade using STARTTLS (usually port 143).
    StartTls,
}

/// Connect to the IMAP server using the provided connector.
pub async fn connect<C>(
    tcp_stream: tokio::net::TcpStream,
    tls_server_name: &str,
    tls_mode: TlsMode,
    connector: C,
) -> Result<async_imap::Client<C::Stream>, ConnectError<C::Error>>
where
    C: imap_tls_core::TlsConnector,
    C::Stream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + std::fmt::Debug,
    C::Error: std::error::Error + Send + Sync + 'static,
{
    let client = match tls_mode {
        TlsMode::Implicit => {
            let stream = connector
                .connect(tls_server_name, tcp_stream)
                .await
                .map_err(ConnectError::Tls)?;
            let mut client = async_imap::Client::new(stream);
            client
                .read_response()
                .await?
                .ok_or(ConnectError::MissingGreeting)?;
            client
        }
        TlsMode::StartTls => {
            let mut client = async_imap::Client::new(tcp_stream);
            client
                .read_response()
                .await?
                .ok_or(ConnectError::MissingGreeting)?;
            client.run_command_and_check_ok("STARTTLS", None).await?;
            let tcp_stream = client.into_inner();
            let stream = connector
                .connect(tls_server_name, tcp_stream)
                .await
                .map_err(ConnectError::Tls)?;
            async_imap::Client::new(stream)
        }
    };

    Ok(client)
}
