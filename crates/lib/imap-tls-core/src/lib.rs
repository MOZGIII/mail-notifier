//! Core TLS connector trait for IMAP clients.

/// Connector for upgrading a TCP stream to a secured IMAP stream.
pub trait TlsConnector {
    /// Secured stream type.
    type Stream;

    /// Error type returned by the connector.
    type Error;

    /// Connect using the provided server name and TCP stream.
    fn connect<'a>(
        &'a self,
        tls_server_name: &'a str,
        tcp_stream: tokio::net::TcpStream,
    ) -> impl std::future::Future<Output = Result<Self::Stream, Self::Error>> + Send + 'a;
}

impl<S, E, F, Fut> TlsConnector for F
where
    F: Fn(&str, tokio::net::TcpStream) -> Fut,
    Fut: std::future::Future<Output = Result<S, E>> + Send + 'static,
{
    type Stream = S;
    type Error = E;

    fn connect<'a>(
        &'a self,
        tls_server_name: &'a str,
        tcp_stream: tokio::net::TcpStream,
    ) -> impl std::future::Future<Output = Result<Self::Stream, Self::Error>> + Send + 'a {
        (self)(tls_server_name, tcp_stream)
    }
}
