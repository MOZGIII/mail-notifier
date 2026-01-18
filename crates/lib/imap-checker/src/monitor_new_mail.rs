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

/// Monitor new email using IMAP IDLE and log unread and total counts.
pub async fn monitor_new_mail<S>(
    mut session: async_imap::Session<S>,
    mailbox: &str,
    idle_timeout: std::time::Duration,
) -> Result<(), MonitorError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + std::fmt::Debug,
{
    let capabilities = session.capabilities().await?;
    if !capabilities.has_str("IDLE") {
        return Err(MonitorError::IdleNotSupported);
    }

    session.select(mailbox).await?;
    let mut last_counts = crate::fetch_counts(&mut session, mailbox).await?;
    tracing::info!(
        mailbox = %mailbox,
        total = last_counts.total,
        unread = last_counts.unread,
        "initial mailbox counts"
    );

    loop {
        let mut idle_handle = session.idle();
        idle_handle.init().await?;
        let (idle_wait, _stop) = idle_handle.wait_with_timeout(idle_timeout);
        let idle_response = idle_wait.await?;
        session = idle_handle.done().await?;

        match idle_response {
            async_imap::extensions::idle::IdleResponse::Timeout => {
                tracing::debug!("idle timeout elapsed, re-issuing IDLE");
            }
            async_imap::extensions::idle::IdleResponse::ManualInterrupt => {
                tracing::debug!("idle interrupted, re-issuing IDLE");
            }
            async_imap::extensions::idle::IdleResponse::NewData(_) => {
                tracing::debug!("idle notified of new data");
            }
        }

        let counts = crate::fetch_counts(&mut session, mailbox).await?;
        if counts != last_counts {
            if counts.total > last_counts.total || counts.unread > last_counts.unread {
                tracing::info!(
                    mailbox = %mailbox,
                    total = counts.total,
                    unread = counts.unread,
                    "new mail available"
                );
            } else {
                tracing::info!(
                    mailbox = %mailbox,
                    total = counts.total,
                    unread = counts.unread,
                    "mailbox counts updated"
                );
            }
            last_counts = counts;
        }
    }
}
