//! TLS connector helpers for IMAP clients.

/// TLS stream type used for IMAP connections.
pub type TlsStream = tokio_rustls::client::TlsStream<tokio::net::TcpStream>;

/// Errors returned while preparing or establishing a TLS connection.
#[derive(Debug, thiserror::Error)]
pub enum TlsConnectError {
    /// Failed to load system root certificates.
    #[error("failed to load system root certificates: {0}")]
    RootCerts(#[from] rustls_native_certs::Error),

    /// Invalid DNS name for TLS verification.
    #[error("invalid DNS name: {0}")]
    InvalidDnsName(String),

    /// TLS handshake or I/O error.
    #[error("TLS I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Build a rustls connector configured with system root certificates.
pub fn connector() -> Result<tokio_rustls::TlsConnector, TlsConnectError> {
    let mut root_store = rustls::RootCertStore::empty();
    let rustls_native_certs::CertificateResult { certs, errors, .. } =
        rustls_native_certs::load_native_certs();
    if let Some(err) = errors.into_iter().next() {
        return Err(TlsConnectError::RootCerts(err));
    }
    let _ = root_store.add_parsable_certificates(certs);
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    Ok(tokio_rustls::TlsConnector::from(std::sync::Arc::new(
        config,
    )))
}

/// Establish a TLS connection over an existing TCP stream.
pub async fn connect(
    connector: &tokio_rustls::TlsConnector,
    server: &str,
    stream: tokio::net::TcpStream,
) -> Result<TlsStream, TlsConnectError> {
    let server_name = rustls::pki_types::ServerName::try_from(server.to_string())
        .map_err(|_| TlsConnectError::InvalidDnsName(server.to_string()))?;
    let tls_stream = connector.connect(server_name, stream).await?;
    Ok(tls_stream)
}
