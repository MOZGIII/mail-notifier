//! Authentication.

/// An auth error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Login failed.
    #[error("login: {0}")]
    Login(async_imap::error::Error),

    /// OAuth2 failed.
    #[error("oauth2: {0}")]
    OAuth2(async_imap::error::Error),
}

/// Auth params.
#[derive(Debug, Clone, PartialEq)]
pub enum Params<'a> {
    /// Username/password login.
    Login {
        /// Username for IMAP authentication.
        ///
        /// Typically an email address.
        username: &'a str,

        /// Password for IMAP authentication.
        password: &'a str,
    },

    /// OAuth 2 authnetication.
    OAuth2 {
        /// The user for IMAP authentication.
        ///
        /// Typically an email address.
        user: &'a str,

        /// The access token for IMAP authentication.
        access_token: &'a str,
    },
}

/// Authenticate to the client to obtain a session.
pub(crate) async fn execute(
    client: crate::Client,
    auth: Params<'_>,
) -> Result<crate::Session, Error> {
    match auth {
        Params::Login { username, password } => client
            .login(username, password)
            .await
            .map_err(|(err, _client)| err)
            .map_err(Error::Login),
        Params::OAuth2 { user, access_token } => client
            .authenticate("XOAUTH2", OAuth2Authenticator { user, access_token })
            .await
            .map_err(|(err, _client)| err)
            .map_err(Error::OAuth2),
    }
}

/// An internal OAuth 2 authenticator for provided credentials.
struct OAuth2Authenticator<'a> {
    /// User.
    user: &'a str,

    /// Access token.
    access_token: &'a str,
}

impl<'a> async_imap::Authenticator for OAuth2Authenticator<'a> {
    type Response = String;

    fn process(&mut self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}
