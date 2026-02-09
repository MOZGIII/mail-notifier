//! Actions engine — dispatches actions in response to mail state machine events.
//!
//! The engine holds a set of pre-built actions for each event type and
//! invokes them when matching [`mail_state_machine::WorkloadEffects`]
//! are observed.

use action_core::Action;

mod new_mail_event;

pub use self::new_mail_event::NewMailEvent;

/// A runtime action that can be invoked in response to a new-mail event.
#[derive(Debug)]
pub enum NewMailAction {
    /// Spawn a process.
    Exec(action_exec::ExecAction),
}

/// The actions engine.
///
/// Holds pre-built actions and dispatches them when
/// [`mail_state_machine::WorkloadEffects`] indicate relevant events.
#[derive(Debug, Default)]
pub struct Engine {
    /// Actions to run when new mail is detected.
    pub new_mail_actions: Vec<NewMailAction>,
}

impl Engine {
    /// Process workload effects and invoke matching actions.
    ///
    /// When [`WorkloadEffects::new_mail`](mail_state_machine::WorkloadEffects::new_mail)
    /// is `true`, all configured new-mail actions are invoked.
    /// Errors from individual actions are logged but do not prevent
    /// remaining actions from running.
    pub async fn process(&self, effects: &mail_state_machine::WorkloadEffects) {
        if effects.new_mail {
            self.run_new_mail_actions().await;
        }
    }

    /// Run all new-mail actions.
    async fn run_new_mail_actions(&self) {
        let event = NewMailEvent;
        for action in &self.new_mail_actions {
            match action {
                NewMailAction::Exec(exec) => {
                    if let Err(err) = exec.invoke(&event).await {
                        tracing::error!(error = %err, "new-mail exec action failed");
                    }
                }
            }
        }
    }
}
