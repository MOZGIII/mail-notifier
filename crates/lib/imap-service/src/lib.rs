//! Mailbox monitoring entrypoint and configuration.

/// Errors returned while monitoring a mailbox.
#[derive(Debug, thiserror::Error)]
pub enum MonitorMailboxError {
    /// IMAP session error.
    #[error("IMAP session error: {0}")]
    Session(#[source] imap_session::Error),

    /// IMAP monitor error.
    #[error("IMAP monitor error: {0}")]
    Monitor(#[source] imap_checker::MonitorError),
}

/// Connect and monitor a mailbox based on provided settings.
pub async fn monitor_mailbox<Notify, NotifyFut>(
    mailbox: &config_bringup::Mailbox,
    notify: Notify,
) -> Result<core::convert::Infallible, MonitorMailboxError>
where
    Notify: FnMut(imap_checker::MailboxCounts) -> NotifyFut + Send,
    NotifyFut: std::future::Future<Output = ()> + Send,
{
    let config_bringup::Mailbox {
        server,
        mailbox,
        idle_timeout,
    } = mailbox;

    let session = connect_to_server(server.as_ref())
        .await
        .map_err(MonitorMailboxError::Session)?;

    imap_checker::monitor_mailbox_counts(session, mailbox, *idle_timeout, notify)
        .await
        .map_err(MonitorMailboxError::Monitor)
}

/// Connect to a server based on provided settings.
pub async fn connect_to_server(
    server: &config_bringup::Server,
) -> Result<imap_session::Session, imap_session::Error> {
    let config_bringup::Server {
        server_name: _,
        host,
        port,
        tls_mode,
        tls_server_name,
        auth,
    } = server;

    let connect = imap_connect::Params {
        host,
        port: *port,
        tls_mode: *tls_mode,
        tls_server_name,
    };

    let auth = match auth {
        config_bringup::ServerAuth::Login { username, password } => {
            imap_auth::Params::Login { username, password }
        }
        config_bringup::ServerAuth::OAuth2Credentials { user, access_token } => {
            imap_auth::Params::OAuth2 { user, access_token }
        }
        config_bringup::ServerAuth::OAuth2Session { .. } => todo!(),
    };

    let session = imap_session::Params { connect, auth };

    imap_session::establish(session).await
}
