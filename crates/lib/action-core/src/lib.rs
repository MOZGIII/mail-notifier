//! Core trait for actions that respond to events.

/// An action that can be invoked in response to an event.
pub trait Action<Event> {
    /// The error type returned by this action.
    type Error;

    /// Invoke the action with the given event.
    fn invoke(&self, event: &Event) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
