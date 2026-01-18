//! Mailbox counter query.

/// Errors returned while querying mailbox counts.
#[derive(Debug, thiserror::Error)]
pub enum FetchCountsError {
    /// IMAP protocol error.
    #[error("IMAP error: {0}")]
    Imap(#[from] async_imap::error::Error),
}

/// Query the current total and unread counts for the mailbox.
pub(crate) async fn fetch_counts<S>(
    session: &mut async_imap::Session<S>,
    mailbox: &str,
) -> Result<crate::MailboxCounts, FetchCountsError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + std::fmt::Debug,
{
    let status = session.status(mailbox, "(MESSAGES UNSEEN)").await?;
    Ok(crate::MailboxCounts {
        total: status.exists,
        unread: status.unseen.unwrap_or(0),
    })
}
