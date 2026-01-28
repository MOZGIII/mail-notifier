//! High-level IMAP connection utilities.

/// The effective data stream type we use.
pub type Stream = imap_tls_rustls::TlsStream;

/// The effective client type we use.
pub type Client = async_imap::Client<Stream>;

/// IMAP connect params.
#[derive(Debug, Clone, PartialEq)]
pub struct Params<'a> {
    /// Hostname or IP address of the IMAP server.
    pub host: &'a str,

    /// IMAP port.
    pub port: u16,

    /// TLS mode.
    pub tls_mode: imap_tls::TlsMode,

    /// TLS server name (SNI).
    pub tls_server_name: &'a str,
}

/// Errors returned while connecting to an IMAP server.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// TCP connection error.
    #[error("TCP connection error: {0}")]
    TcpConnect(#[source] std::io::Error),

    /// IMAP TLS connector error.
    #[error("IMAP TLS connector error: {0}")]
    ImapTlsConnector(#[source] imap_tls_rustls::TlsConnectError),

    /// IMAP TLS connection error.
    #[error("IMAP TLS connection error: {0}")]
    ImapTlsConnect(#[source] imap_tls::ConnectError<imap_tls_rustls::TlsConnectError>),
}

/// Connect to an IMAP server and produce an IMAP client.
pub async fn connect(params: Params<'_>) -> Result<Client, Error> {
    let Params {
        host,
        port,
        tls_mode,
        tls_server_name,
    } = params;

    tracing::debug!(
        imap_host = %host,
        imap_port = port,
        imap_tls_mode = ?tls_mode,
        tls_server_name = %tls_server_name,
        "connecting to an IMAP server"
    );

    let tcp_stream = tokio::net::TcpStream::connect((host, port))
        .await
        .map_err(Error::TcpConnect)?;
    let tls_connector = imap_tls_rustls::connector().map_err(Error::ImapTlsConnector)?;
    let client = imap_tls::connect(tcp_stream, tls_server_name, tls_mode, tls_connector)
        .await
        .map_err(Error::ImapTlsConnect)?;

    Ok(client)
}
