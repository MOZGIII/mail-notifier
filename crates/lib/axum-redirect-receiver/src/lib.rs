//! Redirect receiver.

pub mod once_sender;

use std::sync::Arc;

pub use self::once_sender::OnceSender;

pub use axum;

pub use axum::extract::State;

/// Redirect receiver.
pub struct RedirectReceiver<Payload> {
    /// A handle to control the background task.
    server_handle: tokio_util::task::AbortOnDropHandle<()>,

    /// The axum server graceful shutdown signal.
    stop: tokio_util::sync::DropGuard,

    /// The channel to transfer the redirect payload over.
    rx: tokio::sync::oneshot::Receiver<Payload>,
}

/// The router state for a given payload.
pub type StateFor<Payload> = State<Arc<OnceSender<Payload>>>;

impl<Payload> RedirectReceiver<Payload>
where
    Payload: Send + 'static,
{
    /// Start a new receiver.
    pub async fn start(
        router: axum::Router<Arc<OnceSender<Payload>>>,
    ) -> Result<(std::net::SocketAddr, Self), std::io::Error> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let tx = OnceSender::new(tx);

        let app = router.with_state(Arc::new(tx));

        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;

        let local_addr = listener.local_addr()?;

        let stop = tokio_util::sync::CancellationToken::new();
        let graceful_termination_signal = stop.clone().cancelled_owned();
        let stop = stop.drop_guard();

        let server_handle = tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, app)
                .with_graceful_shutdown(graceful_termination_signal)
                .await
            {
                tracing::error!(?error, "axum serve errored");
            }
        });
        let server_handle = tokio_util::task::AbortOnDropHandle::new(server_handle);

        let this = Self {
            server_handle,
            stop,
            rx,
        };

        Ok((local_addr, this))
    }

    /// Receive the redirect.
    pub async fn receive(self) -> Result<Payload, ReceiveError> {
        let Self {
            server_handle,
            stop,
            rx,
        } = self;

        let payload = rx.await.map_err(ReceiveError::Rx)?;

        drop(stop);

        server_handle.await.unwrap();

        Ok(payload)
    }
}

/// An error when receiving the code.
#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    /// Channel closed without sending the code.
    #[error("receiving the redirect failed")]
    Rx(tokio::sync::oneshot::error::RecvError),
}
