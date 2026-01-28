//! Mailbox monitoring entrypoint and configuration.

/// Fully resolved mailbox monitoring configuration.
#[derive(Debug, Clone)]
pub struct Params<'a> {
    /// IMAP session setup params.
    pub session: imap_session::Params<'a>,

    /// Mailbox name (e.g. INBOX).
    pub mailbox: &'a imap_utf7::ImapUtf7Str,

    /// Idle timeout.
    pub idle_timeout: std::time::Duration,
}

/// Errors returned while monitoring a mailbox.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// IMAP session error.
    #[error("IMAP session error: {0}")]
    Session(#[source] imap_session::Error),

    /// IMAP monitor error.
    #[error("IMAP monitor error: {0}")]
    Monitor(#[source] imap_checker::MonitorError),
}

/// Connect and monitor a mailbox based on provided settings.
pub async fn monitor<Notify, NotifyFut>(
    params: Params<'_>,
    notify: Notify,
) -> Result<core::convert::Infallible, Error>
where
    Notify: FnMut(imap_checker::MailboxCounts) -> NotifyFut + Send,
    NotifyFut: std::future::Future<Output = ()> + Send,
{
    let Params {
        session,
        mailbox,
        idle_timeout,
    } = params;

    tracing::info!(
        imap_host = %session.connect.host,
        imap_port = session.connect.port,
        imap_tls_mode = ?session.connect.tls_mode,
        imap_mailbox = %mailbox,
        "starting IMAP monitor"
    );

    let session = imap_session::establish(session)
        .await
        .map_err(Error::Session)?;

    imap_checker::monitor_mailbox_counts(session, mailbox, idle_timeout, notify)
        .await
        .map_err(Error::Monitor)
}
