//! IMAP connect and login routine.

/// Errors returned while connecting and authenticating.
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Network I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to load system root certificates.
    #[error("failed to load system root certificates: {0}")]
    RootCerts(#[from] rustls_native_certs::Error),

    /// Invalid DNS name for TLS verification.
    #[error("invalid DNS name: {0}")]
    InvalidDnsName(String),

    /// IMAP protocol error.
    #[error("IMAP error: {0}")]
    Imap(#[from] async_imap::error::Error),

    /// The server did not send the expected greeting.
    #[error("IMAP server sent no greeting")]
    MissingGreeting,
}

/// Connect to the IMAP server and authenticate using the provided context.
pub(crate) async fn connect_and_login(
    ctx: &crate::ImapClientContext,
) -> Result<async_imap::Session<tokio_rustls::client::TlsStream<tokio::net::TcpStream>>, ConnectError> {
    let addr = (ctx.server.as_str(), ctx.port);
    let tcp_stream = tokio::net::TcpStream::connect(addr).await?;
    let mut root_store = rustls::RootCertStore::empty();
    let rustls_native_certs::CertificateResult { certs, errors, .. } =
        rustls_native_certs::load_native_certs();
    if let Some(err) = errors.into_iter().next() {
        return Err(ConnectError::RootCerts(err));
    }
    let _ = root_store.add_parsable_certificates(certs);
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let tls_connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
    let server_name = rustls::pki_types::ServerName::try_from(ctx.server.clone())
        .map_err(|_| ConnectError::InvalidDnsName(ctx.server.clone()))?;

    let client = match ctx.tls_mode {
        crate::TlsMode::Implicit => {
            let tls_stream = tls_connector.connect(server_name.clone(), tcp_stream).await?;
            let mut client = async_imap::Client::new(tls_stream);
            client
                .read_response()
                .await?
                .ok_or(ConnectError::MissingGreeting)?;
            client
        }
        crate::TlsMode::StartTls => {
            let mut client = async_imap::Client::new(tcp_stream);
            client
                .read_response()
                .await?
                .ok_or(ConnectError::MissingGreeting)?;
            client.run_command_and_check_ok("STARTTLS", None).await?;
            let tcp_stream = client.into_inner();
            let tls_stream = tls_connector.connect(server_name.clone(), tcp_stream).await?;
            async_imap::Client::new(tls_stream)
        }
    };

    let session = client
        .login(&ctx.username, ctx.password.as_str())
        .await
        .map_err(|(err, _client)| err)?;
    Ok(session)
}
