//! Mailbox count data.

/// Mailbox counters for logging.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MailboxCounts {
    /// Total messages in the mailbox.
    pub total: u32,

    /// Messages that are unread.
    pub unread: u32,
}
