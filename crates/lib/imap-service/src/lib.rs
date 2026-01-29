//! Mailbox monitoring entrypoint and configuration.

/// Errors returned while connecting to the server.
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// OAuth 2 session error.
    #[error("OAuth 2 session error: {0}")]
    OAuth2Session(
        #[source] oauth2_session::GetTokenError<oauth2_token_storage_keyring::KeyringTokenStorage>,
    ),

    /// IMAP session error.
    #[error("IMAP session error: {0}")]
    ImapSession(#[source] imap_session::Error),
}

/// Errors returned while monitoring a mailbox.
#[derive(Debug, thiserror::Error)]
pub enum MonitorMailboxError {
    /// Connection error.
    #[error(transparent)]
    Connect(ConnectError),

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
        .map_err(MonitorMailboxError::Connect)?;

    imap_checker::monitor_mailbox_counts(session, mailbox, *idle_timeout, notify)
        .await
        .map_err(MonitorMailboxError::Monitor)
}

/// Connect to a server based on provided settings.
pub async fn connect_to_server(
    server: &config_bringup::Server,
) -> Result<imap_session::Session, ConnectError> {
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

    let mut oauth2_session_access_token = None;

    let auth = match auth {
        config_bringup::ServerAuth::Login { username, password } => {
            imap_auth::Params::Login { username, password }
        }
        config_bringup::ServerAuth::OAuth2Credentials { user, access_token } => {
            imap_auth::Params::OAuth2 { user, access_token }
        }
        config_bringup::ServerAuth::OAuth2Session {
            user,
            session_manager,
        } => {
            let mut session_manager = session_manager.lock().await;
            let access_token = session_manager
                .get_access_token()
                .await
                .map_err(ConnectError::OAuth2Session)?;
            let access_token = oauth2_session_access_token.insert(access_token);
            imap_auth::Params::OAuth2 { user, access_token }
        }
    };

    let session = imap_session::Params { connect, auth };

    imap_session::establish(session)
        .await
        .map_err(ConnectError::ImapSession)
}
