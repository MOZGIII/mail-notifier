//! OAuth 2 session manager crate.
//!
//! This crate provides an OAuth2 session manager that handles token refresh and storage.

use oauth2::TokenResponse as _;

/// OAuth 2 Session Manager.
pub struct Manager<
    TokenStorage,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
> where
    HasAuthUrl: oauth2::EndpointState,
    HasDeviceAuthUrl: oauth2::EndpointState,
    HasIntrospectionUrl: oauth2::EndpointState,
    HasRevocationUrl: oauth2::EndpointState,
{
    /// The OAuth2 client for refreshing the token.
    pub oauth2_client: oauth2::basic::BasicClient<
        HasAuthUrl,
        HasDeviceAuthUrl,
        HasIntrospectionUrl,
        HasRevocationUrl,
        oauth2::EndpointSet,
    >,

    /// The HTTP client for refreshing the token.
    pub http_client: reqwest::Client,

    /// OAuth 2 token storage.
    pub storage: TokenStorage,

    /// If the token expires in less than this duration - refresh it.
    pub expiration_immenance_tolerance: std::time::Duration,
}

/// An error that can occir while getting a token.
#[derive(Debug, thiserror::Error)]
pub enum GetTokenError<TokenStorage: oauth2_token_storage_core::TokenStorage> {
    /// Loading token from storage failed.
    #[error("unable to load token from storage: {0}")]
    StorageLoad(oauth2_token_storage_core::LoadError<TokenStorage::LoadError>),

    /// Exchanging refresh token failed.
    #[error("unable to exchange refresh token: {0}")]
    ExchangeRefreshToken(
        oauth2::RequestTokenError<
            oauth2::HttpClientError<reqwest::Error>,
            oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>,
        >,
    ),

    /// Exchanging refresh token didn't produce a new refresh token.
    #[error("no refresh token in exchange refresh token response")]
    NoRefreshTokenInResponse,

    /// Unable to store a newly refreshed data.
    #[error("no refresh token in exchange refresh token response")]
    StorageStore(TokenStorage::StoreError),
}

impl<
    TokenStorage: oauth2_token_storage_core::TokenStorage,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
> Manager<TokenStorage, HasAuthUrl, HasDeviceAuthUrl, HasIntrospectionUrl, HasRevocationUrl>
where
    HasAuthUrl: oauth2::EndpointState,
    HasDeviceAuthUrl: oauth2::EndpointState,
    HasIntrospectionUrl: oauth2::EndpointState,
    HasRevocationUrl: oauth2::EndpointState,
{
    /// Get an up-to-date access token.
    pub async fn get_access_token(&mut self) -> Result<String, GetTokenError<TokenStorage>> {
        let mut data = self
            .storage
            .load()
            .await
            .map_err(GetTokenError::StorageLoad)?;

        if let Some(expires_at) = data.expires_at
            && std::time::SystemTime::now() + self.expiration_immenance_tolerance > expires_at
        {
            let res = self
                .oauth2_client
                .exchange_refresh_token(&oauth2::RefreshToken::new(data.refresh_token))
                .request_async(&self.http_client)
                .await
                .map_err(GetTokenError::ExchangeRefreshToken)?;

            let Some(refresh_token) = res.refresh_token() else {
                return Err(GetTokenError::NoRefreshTokenInResponse);
            };

            data = oauth2_token_storage_core::Data {
                access_token: res.access_token().secret().clone(),
                expires_at: res
                    .expires_in()
                    .map(|expires_in| std::time::SystemTime::now() + expires_in),
                refresh_token: refresh_token.secret().clone(),
            };

            self.storage
                .store(data.as_ref())
                .await
                .map_err(GetTokenError::StorageStore)?;
        }

        Ok(data.access_token)
    }
}

/// Manage a new session with a given storage.
pub async fn manage<TokenStorage: oauth2_token_storage_core::TokenStorage>(
    storage: &mut TokenStorage,
    data: oauth2_token_storage_core::DataRef<'_>,
) -> Result<(), TokenStorage::StoreError> {
    storage.store(data).await
}
