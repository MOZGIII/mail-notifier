//! Exec action — spawns a process in response to an event.

use std::collections::HashMap;

/// Error from executing a process action.
#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    /// Failed to spawn the process.
    #[error("failed to spawn process: {0}")]
    Spawn(#[source] std::io::Error),

    /// The process exited with a non-zero status.
    #[error("process exited with {0}")]
    NonZero(std::process::ExitStatus),
}

/// An exec action that spawns a process when invoked.
#[derive(Debug, Clone)]
pub struct ExecAction {
    /// Program to run.
    pub command: String,

    /// Arguments to pass to the program.
    pub args: Vec<String>,

    /// Extra environment variables configured for the spawned process.
    pub env: HashMap<String, String>,
}

impl<Event> action_core::Action<Event> for ExecAction
where
    Event: action_exec_core::ExecEvent + Sync,
{
    type Error = ExecError;

    async fn invoke(&self, event: &Event) -> Result<(), Self::Error> {
        let status = tokio::process::Command::new(&self.command)
            .args(&self.args)
            .envs(&self.env)
            .envs(event.env())
            .status()
            .await
            .map_err(ExecError::Spawn)?;

        if !status.success() {
            return Err(ExecError::NonZero(status));
        }

        Ok(())
    }
}
