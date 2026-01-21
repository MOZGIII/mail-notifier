//! IMAP IDLE monitoring routine.

/// Errors returned by the IMAP monitor.
#[derive(Debug, thiserror::Error)]
pub enum MonitorError {
    /// Mailbox count query error.
    #[error("count query error: {0}")]
    FetchCounts(#[from] crate::FetchCountsError),

    /// The server does not advertise the IDLE capability.
    #[error("IMAP server does not advertise IDLE capability")]
    IdleNotSupported,

    /// IMAP protocol error during IDLE.
    #[error("IMAP error: {0}")]
    Imap(#[from] async_imap::error::Error),
}

/// Monitor mailbox counts and send updates on change.
pub async fn monitor_mailbox_counts<S, F, Fut>(
    mut session: async_imap::Session<S>,
    mailbox: &imap_utf7::ImapUtf7Str,
    idle_timeout: std::time::Duration,
    mut notify: F,
) -> Result<(), MonitorError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + std::fmt::Debug,
    F: FnMut(crate::MailboxCounts) -> Fut + Send,
    Fut: std::future::Future<Output = ()> + Send,
{
    let capabilities = session.capabilities().await?;
    if !capabilities.has_str("IDLE") {
        return Err(MonitorError::IdleNotSupported);
    }

    session.select(mailbox.as_str()).await?;
    let mut last_counts = crate::fetch_counts(&mut session, mailbox).await?;

    notify(last_counts).await;

    loop {
        let mut idle_handle = session.idle();
        idle_handle.init().await?;
        let (idle_wait, _stop) = idle_handle.wait_with_timeout(idle_timeout);
        let _idle_response = idle_wait.await?;
        session = idle_handle.done().await?;

        let counts = crate::fetch_counts(&mut session, mailbox).await?;
        if counts != last_counts {
            last_counts = counts;
            notify(counts).await;
        }
    }
}
