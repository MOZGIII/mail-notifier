//! Mailbox monitoring entrypoint and configuration.

/// Fully resolved mailbox monitoring configuration.
#[derive(Debug, Clone)]
pub struct MonitorParams<'a> {
    /// IMAP session setup params.
    pub imap_session: imap_session::SetupParams<'a>,

    /// Mailbox name (e.g. INBOX).
    pub mailbox: &'a imap_utf7::ImapUtf7Str,

    /// Idle timeout.
    pub idle_timeout: std::time::Duration,
}

/// Errors returned while monitoring a mailbox.
#[derive(Debug, thiserror::Error)]
pub enum MonitorError {
    /// TLS connector error.
    #[error("IMAP session setup error: {0}")]
    SessionSetup(#[source] imap_session::SetupError),

    /// IMAP monitor error.
    #[error("IMAP monitor error: {0}")]
    Monitor(#[source] imap_checker::MonitorError),
}

/// Connect and monitor a mailbox based on provided settings.
pub async fn monitor<Notify, NotifyFut>(
    params: MonitorParams<'_>,
    notify: Notify,
) -> Result<core::convert::Infallible, MonitorError>
where
    Notify: FnMut(imap_checker::MailboxCounts) -> NotifyFut + Send,
    NotifyFut: std::future::Future<Output = ()> + Send,
{
    let MonitorParams {
        imap_session,
        mailbox,
        idle_timeout,
    } = params;

    tracing::info!(
        imap_host = %imap_session.host,
        imap_port = imap_session.port,
        imap_mailbox = %mailbox,
        imap_tls_mode = ?imap_session.tls_mode,
        "starting IMAP monitor"
    );

    let session = imap_session::setup(imap_session)
        .await
        .map_err(MonitorError::SessionSetup)?;

    imap_checker::monitor_mailbox_counts(session, mailbox, idle_timeout, notify)
        .await
        .map_err(MonitorError::Monitor)
}
