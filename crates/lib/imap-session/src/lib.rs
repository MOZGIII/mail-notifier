//! High-level IMAP session utilities.

/// The effective session type we use.
pub type Session = async_imap::Session<imap_tls_rustls::TlsStream>;

/// IMAP session params.
#[derive(Debug, Clone, PartialEq)]
pub struct SetupParams<'a> {
    /// Hostname or IP address of the IMAP server.
    pub host: &'a str,

    /// IMAP port.
    pub port: u16,

    /// TLS mode.
    pub tls_mode: imap_tls::TlsMode,

    /// TLS server name (SNI).
    pub tls_server_name: &'a str,

    /// Username for IMAP authentication.
    pub username: &'a str,

    /// Password for IMAP authentication.
    pub password: &'a str,
}

/// Errors returned while monitoring a mailbox.
#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    /// TCP connection error.
    #[error("TCP connection error: {0}")]
    TcpConnect(#[source] std::io::Error),

    /// IMAP TLS connector error.
    #[error("IMAP TLS connector error: {0}")]
    ImapTlsConnector(#[source] imap_tls_rustls::TlsConnectError),

    /// IMAP TLS connection error.
    #[error("IMAP TLS connection error: {0}")]
    ImapTlsConnect(#[source] imap_tls::ConnectError<imap_tls_rustls::TlsConnectError>),

    /// IMAP login error.
    #[error("IMAP login error: {0}")]
    Login(#[source] async_imap::error::Error),
}

/// Connect and login to set up an IMAP session.
pub async fn setup(params: SetupParams<'_>) -> Result<Session, SetupError> {
    let SetupParams {
        host,
        port,
        tls_mode,
        tls_server_name,
        username,
        password,
    } = params;

    tracing::debug!(
        imap_host = %host,
        imap_port = port,
        imap_tls_mode = ?tls_mode,
        "setting up IMAP session"
    );

    let tcp_stream = tokio::net::TcpStream::connect((host, port))
        .await
        .map_err(SetupError::TcpConnect)?;
    let tls_connector = imap_tls_rustls::connector().map_err(SetupError::ImapTlsConnector)?;
    let client = imap_tls::connect(tcp_stream, tls_server_name, tls_mode, tls_connector)
        .await
        .map_err(SetupError::ImapTlsConnect)?;

    let session = client
        .login(username, password)
        .await
        .map_err(|(err, _client)| err)
        .map_err(SetupError::Login)?;

    Ok(session)
}
