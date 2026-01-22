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
    /// Network I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// TLS connector error.
    #[error("TLS connector error: {0}")]
    Connector(#[from] imap_tls_rustls::TlsConnectError),

    /// IMAP connection error.
    #[error("IMAP connection error: {0}")]
    Connect(#[from] imap_tls::ConnectError<imap_tls_rustls::TlsConnectError>),

    /// IMAP login error.
    #[error("IMAP login error: {0}")]
    Login(#[from] async_imap::error::Error),

    /// IMAP monitor error.
    #[error("IMAP monitor error: {0}")]
    Monitor(#[from] imap_checker::MonitorError),
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

    let tcp_stream = tokio::net::TcpStream::connect((host, port)).await?;
    let tls_connector = imap_tls_rustls::connector()?;
    let client = imap_tls::connect(tcp_stream, tls_server_name, tls_mode, tls_connector).await?;

    let session = client
        .login(username, password)
        .await
        .map_err(|(err, _client)| err)?;

    Ok(session)
}
