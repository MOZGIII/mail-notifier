//! OAuth 2 authorization code receiver.

pub use axum_redirect_receiver::ReceiveError;

use axum_redirect_receiver::{
    State, StateFor,
    axum::{extract::Query, routing::get},
};

/// OAuth 2 code receiver.
pub type Receiver = axum_redirect_receiver::RedirectReceiver<RedirectQuery>;

/// The query params we're interested in.
#[derive(serde::Deserialize)]
pub struct RedirectQuery {
    /// The code.
    pub code: String,

    /// The state.
    pub state: String,
}

/// Start a new receiver.
pub async fn start() -> Result<Started, std::io::Error> {
    let callback =
        |State(tx): StateFor<RedirectQuery>,
         Query(query): axum_redirect_receiver::axum::extract::Query<RedirectQuery>| async move {
            if let Err(error) = tx.send(query) {
                return error.to_string();
            }

            "Ok".to_owned()
        };

    let router = axum_redirect_receiver::axum::Router::new().route("/callback", get(callback));

    let (addr, receiver) = axum_redirect_receiver::RedirectReceiver::start(router).await?;

    let redirect_url = format!("http://localhost:{}/callback", addr.port());

    Ok(Started {
        redirect_url,
        receiver,
    })
}

/// The started receiver.
pub struct Started {
    /// The redirect URL to use.
    pub redirect_url: String,

    /// The receiver instance.
    pub receiver: Receiver,
}
