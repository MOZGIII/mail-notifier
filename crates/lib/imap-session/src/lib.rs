//! High-level IMAP session utilities.

pub use imap_auth::Session;

/// IMAP session params.
#[derive(Debug, Clone, PartialEq)]
pub struct Params<'a> {
    /// Connect params.
    pub connect: imap_connect::Params<'a>,

    /// Auth params.
    pub auth: imap_auth::Params<'a>,
}

/// Errors returned while establishing a session.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// IMAP connection error.
    #[error("connect: {0}")]
    Connect(#[source] imap_connect::Error),

    /// IMAP auth error.
    #[error("auth: {0}")]
    Auth(#[source] imap_auth::Error),
}

/// Connect and login to establish an IMAP session.
pub async fn establish(params: Params<'_>) -> Result<Session, Error> {
    let Params { connect, auth } = params;

    let client = imap_connect::connect(connect)
        .await
        .map_err(Error::Connect)?;

    let session = imap_auth::auth(client, auth).await.map_err(Error::Auth)?;

    Ok(session)
}
