//! The once sender.

/// A wrapper around [`tokio::sync::oneshot::Sender`] that doesn't consume the sender
/// on send but still allows sending only once.
pub struct OnceSender<T>(atomic_take::AtomicTake<tokio::sync::oneshot::Sender<T>>);

/// An error while sending.
#[derive(thiserror::Error)]
pub enum SendError<T> {
    /// Payload has already been sent.
    #[error("already sent")]
    AlreadySent(T),

    /// The receiving channel was closed before we managed to send the payload.
    #[error("receiver gone")]
    ReceiverGone(T),
}

impl<T> OnceSender<T> {
    /// Create a new [`OnceSender`] for a given [`tokio::sync::oneshot::Sender`].
    pub fn new(tx: tokio::sync::oneshot::Sender<T>) -> Self {
        Self(atomic_take::AtomicTake::new(tx))
    }

    /// Send a
    pub fn send(&self, payload: T) -> Result<(), SendError<T>> {
        let Some(tx) = self.0.take() else {
            return Err(SendError::AlreadySent(payload));
        };

        tx.send(payload).map_err(SendError::ReceiverGone)
    }
}

impl<T> core::fmt::Debug for OnceSender<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OnceSender").field(&self.0).finish()
    }
}
