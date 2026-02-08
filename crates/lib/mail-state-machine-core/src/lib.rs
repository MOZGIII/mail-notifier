//! Core traits for the mail state machine.

/// A workload payload that carries an unread count.
///
/// Implement this trait for workload update types so they can be
/// passed to a state machine for processing.
pub trait HasUnread {
    /// The unread message count from this update.
    fn unread(&self) -> u32;
}
