//! Tests for the mailbox monitor against a scripted fake IMAP server.

use std::time::Duration;

use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _};

use super::*;

/// How the fake server answers the IDLE command.
#[derive(Debug, Clone, Copy)]
enum IdleSupport {
    /// Enters idling and immediately reports a mailbox change.
    Accepted,

    /// Rejects the command with BAD, like a server without IDLE.
    Rejected,
}

/// Serve a scripted IMAP session that does not advertise IDLE.
///
/// The first STATUS reports an empty mailbox, every later one reports a
/// single unread message.
async fn fake_server(stream: tokio::io::DuplexStream, idle: IdleSupport) {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut lines = tokio::io::BufReader::new(reader).lines();

    writer
        .write_all(b"* OK fake server ready\r\n")
        .await
        .unwrap();

    let mut status_calls = 0u32;

    while let Ok(Some(line)) = lines.next_line().await {
        let mut parts = line.splitn(3, ' ');
        let tag = parts.next().unwrap_or_default();
        let command = parts.next().unwrap_or_default().to_ascii_uppercase();

        let reply = match command.as_str() {
            "LOGIN" => format!("{tag} OK LOGIN completed\r\n"),
            "CAPABILITY" => {
                format!("* CAPABILITY IMAP4rev1\r\n{tag} OK CAPABILITY completed\r\n")
            }
            "SELECT" => format!(
                "* 0 EXISTS\r\n* 0 RECENT\r\n* FLAGS ()\r\n{tag} OK [READ-WRITE] SELECT completed\r\n"
            ),
            "STATUS" => {
                let count = status_calls.min(1);
                status_calls += 1;
                format!(
                    "* STATUS INBOX (MESSAGES {count} UNSEEN {count})\r\n{tag} OK STATUS completed\r\n"
                )
            }
            "IDLE" => match idle {
                IdleSupport::Rejected => format!("{tag} BAD Unknown command\r\n"),
                IdleSupport::Accepted => {
                    writer
                        .write_all(b"+ idling\r\n* 1 EXISTS\r\n")
                        .await
                        .unwrap();
                    let done = lines.next_line().await.unwrap();
                    assert_eq!(done.as_deref(), Some("DONE"));
                    format!("{tag} OK IDLE terminated\r\n")
                }
            },
            _ => format!("{tag} BAD unexpected command\r\n"),
        };

        writer.write_all(reply.as_bytes()).await.unwrap();
    }
}

/// Spin up the fake server and log into it.
async fn connect(idle: IdleSupport) -> async_imap::Session<tokio::io::DuplexStream> {
    let (client_stream, server_stream) = tokio::io::duplex(4096);
    tokio::spawn(fake_server(server_stream, idle));

    let mut client = async_imap::Client::new(client_stream);
    client.read_response().await.unwrap().unwrap();
    client
        .login("user", "secret")
        .await
        .map_err(|(err, _)| err)
        .unwrap()
}

/// Run the monitor against the fake server and return the update stream.
fn spawn_monitor(
    session: async_imap::Session<tokio::io::DuplexStream>,
) -> (
    tokio::sync::mpsc::UnboundedReceiver<crate::MailboxCounts>,
    tokio::task::JoinHandle<Result<core::convert::Infallible, MonitorError>>,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mailbox = imap_utf7::ImapUtf7Str::new("INBOX").unwrap();
        monitor_mailbox_counts(session, mailbox, Duration::from_millis(10), move |counts| {
            let tx = tx.clone();
            async move {
                tx.send(counts).unwrap();
            }
        })
        .await
    });
    (rx, task)
}

/// Collect the first two updates and stop the monitor.
async fn first_two_updates(idle: IdleSupport) -> [crate::MailboxCounts; 2] {
    let session = connect(idle).await;
    let (mut rx, task) = spawn_monitor(session);

    let updates = tokio::time::timeout(Duration::from_secs(5), async {
        let first = rx.recv().await.unwrap();
        let second = rx.recv().await.unwrap();
        [first, second]
    })
    .await
    .unwrap();

    task.abort();
    updates
}

#[tokio::test]
async fn detect_mode_falls_back_to_poll_when_idle_is_rejected() {
    let mut session = connect(IdleSupport::Rejected).await;
    session.select("INBOX").await.unwrap();

    let mode = detect_monitor_mode(&mut session).await.unwrap();

    assert_eq!(mode, MonitorMode::Poll);
}

#[tokio::test]
async fn detect_mode_uses_idle_when_unadvertised_but_accepted() {
    let mut session = connect(IdleSupport::Accepted).await;
    session.select("INBOX").await.unwrap();

    let mode = detect_monitor_mode(&mut session).await.unwrap();

    assert_eq!(mode, MonitorMode::Idle);
    // The session is still usable after the probe.
    session.noop().await.unwrap();
}

#[tokio::test]
async fn poll_mode_delivers_updates() {
    let updates = first_two_updates(IdleSupport::Rejected).await;

    assert_eq!(
        updates,
        [
            crate::MailboxCounts {
                total: 0,
                unread: 0
            },
            crate::MailboxCounts {
                total: 1,
                unread: 1
            },
        ]
    );
}

#[tokio::test]
async fn idle_mode_delivers_updates() {
    let updates = first_two_updates(IdleSupport::Accepted).await;

    assert_eq!(
        updates,
        [
            crate::MailboxCounts {
                total: 0,
                unread: 0
            },
            crate::MailboxCounts {
                total: 1,
                unread: 1
            },
        ]
    );
}
