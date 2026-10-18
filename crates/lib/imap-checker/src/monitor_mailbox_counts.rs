//! Mailbox monitoring routine.
//!
//! Watches a mailbox with IMAP IDLE when the server accepts it, and falls back
//! to periodic polling otherwise.

/// Errors returned by the IMAP monitor.
#[derive(Debug, thiserror::Error)]
pub enum MonitorError {
    /// Mailbox count query error.
    #[error("count query error: {0}")]
    FetchCounts(#[from] crate::FetchCountsError),

    /// IMAP protocol error.
    #[error("IMAP error: {0}")]
    Imap(#[from] async_imap::error::Error),
}

/// How the mailbox is watched for changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorMode {
    /// Blocks in IMAP IDLE until the server reports a change or the timeout elapses.
    Idle,

    /// Queries the counts once per timeout interval.
    Poll,
}

/// Monitor mailbox counts and send updates on change.
///
/// `idle_timeout` bounds a single IDLE round in [`MonitorMode::Idle`] and
/// serves as the poll interval in [`MonitorMode::Poll`].
pub async fn monitor_mailbox_counts<Stream, Notify, NotifyFut>(
    mut session: async_imap::Session<Stream>,
    mailbox: &imap_utf7::ImapUtf7Str,
    idle_timeout: std::time::Duration,
    mut notify: Notify,
) -> Result<core::convert::Infallible, MonitorError>
where
    Stream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + std::fmt::Debug,
    Notify: FnMut(crate::MailboxCounts) -> NotifyFut + Send,
    NotifyFut: std::future::Future<Output = ()> + Send,
{
    session.select(mailbox.as_str()).await?;

    let mode = detect_monitor_mode(&mut session).await?;
    tracing::info!(?mode, "mailbox monitor mode selected");

    let mut last_counts = crate::fetch_counts(&mut session, mailbox).await?;

    notify(last_counts).await;

    loop {
        session = match mode {
            MonitorMode::Idle => idle_round(session, idle_timeout).await?,
            MonitorMode::Poll => {
                tokio::time::sleep(idle_timeout).await;
                session
            }
        };

        let counts = crate::fetch_counts(&mut session, mailbox).await?;
        if counts != last_counts {
            last_counts = counts;
            notify(counts).await;
        }
    }
}

/// Decide how to watch the mailbox.
///
/// Servers that advertise the IDLE capability get IDLE directly. Otherwise
/// IDLE is tried once anyway, since some servers accept commands they do not
/// advertise, and polling is chosen when the server rejects it.
pub async fn detect_monitor_mode<Stream>(
    session: &mut async_imap::Session<Stream>,
) -> Result<MonitorMode, async_imap::error::Error>
where
    Stream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + std::fmt::Debug,
{
    let capabilities = session.capabilities().await?;
    if capabilities.has_str("IDLE") {
        return Ok(MonitorMode::Idle);
    }

    tracing::debug!("server does not advertise IDLE, probing");

    if probe_idle(session).await? {
        Ok(MonitorMode::Idle)
    } else {
        Ok(MonitorMode::Poll)
    }
}

/// Try the IDLE command once and report whether the server entered idling.
///
/// When the server accepts the command, the IDLE is terminated with DONE
/// right away so the session is left in a usable state. Untagged responses
/// received along the way are dropped, since the counts are queried afresh
/// afterwards.
async fn probe_idle<Stream>(
    session: &mut async_imap::Session<Stream>,
) -> Result<bool, async_imap::error::Error>
where
    Stream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + std::fmt::Debug,
{
    use async_imap::imap_proto::{Response, Status};

    let id = session.run_command("IDLE").await?;

    loop {
        let Some(response) = session.read_response().await? else {
            return Err(async_imap::error::Error::ConnectionLost);
        };
        match response.parsed() {
            Response::Continue(_) => break,
            // A tagged completion without a continuation request means the
            // server did not enter idling, whatever the status says.
            Response::Done { tag, .. } if *tag == id => return Ok(false),
            _ => {}
        }
    }

    session.run_command_untagged("DONE").await?;

    loop {
        let Some(response) = session.read_response().await? else {
            return Err(async_imap::error::Error::ConnectionLost);
        };
        if let Response::Done { tag, status, .. } = response.parsed()
            && *tag == id
        {
            return Ok(*status == Status::Ok);
        }
    }
}

/// Run one IDLE round: block until the server reports a change or the timeout elapses.
async fn idle_round<Stream>(
    session: async_imap::Session<Stream>,
    idle_timeout: std::time::Duration,
) -> Result<async_imap::Session<Stream>, async_imap::error::Error>
where
    Stream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + std::fmt::Debug,
{
    let mut idle_handle = session.idle();
    idle_handle.init().await?;
    let (idle_wait, _stop) = idle_handle.wait_with_timeout(idle_timeout);
    let _idle_response = idle_wait.await?;
    idle_handle.done().await
}

#[cfg(test)]
mod tests;
