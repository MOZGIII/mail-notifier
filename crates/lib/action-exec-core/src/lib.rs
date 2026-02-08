//! Traits that events must implement to supply information to the exec action.

use std::ffi::OsStr;

/// Information an event provides for process-spawn actions.
///
/// Any event type used with the exec action must implement this trait
/// so the spawned process can receive context about the event.
pub trait ExecEvent {
    /// Extra environment variables to set on the spawned process.
    fn env(&self) -> impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)> {
        std::iter::empty::<(&OsStr, &OsStr)>()
    }
}
