//! OAuth2 token storage interface.

/// The owned token storage data item.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Data {
    /// The access token.
    pub access_token: String,

    /// When access token expires.
    pub expires_at: Option<std::time::SystemTime>,

    /// The one-time use refresh token.
    pub refresh_token: String,
}

/// The ref token storage data item.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DataRef<'a> {
    /// The access token.
    pub access_token: &'a str,

    /// When access token expires.
    pub expires_at: Option<std::time::SystemTime>,

    /// The one-time use refresh token.
    pub refresh_token: &'a str,
}

impl Data {
    /// Return a [`DataRef`] to the data.
    pub fn as_ref(&self) -> DataRef<'_> {
        DataRef {
            access_token: &self.access_token,
            expires_at: self.expires_at,
            refresh_token: &self.refresh_token,
        }
    }
}

impl<'a> From<DataRef<'a>> for Data {
    fn from(data_ref: DataRef<'a>) -> Self {
        Data {
            access_token: data_ref.access_token.to_string(),
            expires_at: data_ref.expires_at,
            refresh_token: data_ref.refresh_token.to_string(),
        }
    }
}

/// The load error that can communicate the absence of data.
#[derive(Debug, thiserror::Error)]
pub enum LoadError<Error> {
    /// There is no data available to load.
    #[error("no data: {0}")]
    NoData(Error),

    /// Internal error has occurred
    #[error(transparent)]
    Internal(Error),
}

/// Abstract token storage interface.
pub trait TokenStorage: Send + Sync {
    /// The error type for store operation.
    type StoreError;

    /// The error type for load operation.
    type LoadError;

    /// The error type for clear operation.
    type ClearError;

    /// Store the data.
    fn store<'a>(
        &'a self,
        data: DataRef<'a>,
    ) -> impl std::future::Future<Output = Result<(), Self::StoreError>> + Send + 'a;

    /// Load stored data.
    fn load<'a>(
        &'a self,
    ) -> impl std::future::Future<Output = Result<Data, LoadError<Self::LoadError>>> + Send + 'a;

    /// Clear stored data.
    fn clear<'a>(
        &'a self,
    ) -> impl std::future::Future<Output = Result<(), Self::ClearError>> + Send + 'a;
}
