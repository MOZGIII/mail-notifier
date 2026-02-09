//! The new-mail event type passed to actions.

/// Event emitted when new mail is detected.
///
/// Implements [`action_exec_core::ExecEvent`] so it can be used with
/// [`action_exec::ExecAction`].
#[derive(Debug, Clone, Copy)]
pub struct NewMailEvent;

impl action_exec_core::ExecEvent for NewMailEvent {}
