//! Internal utils.

use std::sync::Arc;

use crate::*;

/// Type alias for OAuth 2 client with auth and token endpoints configured.
pub type OAuth2Client = oauth2::basic::BasicClient<
    oauth2::EndpointMaybeSet,
    oauth2::EndpointMaybeSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointSet,
>;

/// The internal OAuth 2 context.
pub struct OAuth2Context {
    /// OAuth 2 clietns.
    pub oauth2_clients: std::collections::HashMap<String, Arc<OAuth2Client>>,

    /// HTTP client.
    pub reqwest_client: reqwest::Client,
}

/// Build resolved OAuth2 clients from config.
pub fn oauth2_clients(
    oauth2_clients: &std::collections::HashMap<String, config_core::OAuth2ClientConfig>,
) -> Result<std::collections::HashMap<String, Arc<OAuth2Client>>, OAuth2ClientError> {
    let mut clients = std::collections::HashMap::new();

    for (name, config) in oauth2_clients {
        let client_id = oauth2::ClientId::new(config.client_id.clone());
        let client_secret = oauth2::ClientSecret::new(config.client_secret.clone());

        let token_url =
            oauth2::TokenUrl::new(config.token_url.clone()).map_err(OAuth2ClientError::TokenUrl)?;

        let auth_url = config
            .auth_url
            .clone()
            .map(oauth2::AuthUrl::new)
            .transpose()
            .map_err(OAuth2ClientError::AuthUrl)?;

        let device_authorization_url = config
            .device_authorization_url
            .clone()
            .map(oauth2::DeviceAuthorizationUrl::new)
            .transpose()
            .map_err(OAuth2ClientError::DeviceAuthorizationUrl)?;

        let oauth2_client = oauth2::basic::BasicClient::new(client_id)
            .set_client_secret(client_secret)
            .set_token_uri(token_url)
            .set_auth_uri_option(auth_url)
            .set_device_authorization_url_option(device_authorization_url);

        clients.insert(name.clone(), Arc::new(oauth2_client));
    }

    Ok(clients)
}

/// Bringup the OAuth 2 context.
pub fn oauth2_context(
    core_config: &config_core::Config,
) -> Result<OAuth2Context, crate::ConfigError> {
    let oauth2_clients =
        oauth2_clients(&core_config.oauth2_clients).map_err(crate::ConfigError::OAuth2Clients)?;

    let oauth2_context = OAuth2Context {
        oauth2_clients,
        reqwest_client: Default::default(),
    };

    Ok(oauth2_context)
}

/// Bringup the server config.
pub async fn server(
    server: &config_core::ServerConfig,
    oauth2_context: &internal::OAuth2Context,
) -> Result<types::Server, ServerError> {
    let tls_mode = match server.tls.mode {
        config_core::TlsMode::Implicit => imap_tls::TlsMode::Implicit,
        config_core::TlsMode::StartTls => imap_tls::TlsMode::StartTls,
    };

    let port = server.port.unwrap_or(match tls_mode {
        imap_tls::TlsMode::Implicit => 993,
        imap_tls::TlsMode::StartTls => 143,
    });

    let tls_server_name = server
        .tls
        .server_name
        .clone()
        .unwrap_or_else(|| server.host.clone());

    let auth = server_auth(&server.auth, oauth2_context)
        .await
        .map_err(ServerError::ServerAuth)?;

    Ok(types::Server {
        server_name: server.name.clone(),
        host: server.host.clone(),
        port,
        tls_mode,
        tls_server_name,
        auth,
    })
}

/// Bringup the server auth config.
pub async fn server_auth(
    auth: &config_core::Auth,
    oauth2_context: &OAuth2Context,
) -> Result<ServerAuth, ServerAuthError> {
    Ok(match auth {
        config_core::Auth::Login(credentials) => {
            let password = resolve_password(credentials)
                .await
                .map_err(ServerAuthError::ResolvePassword)?;

            ServerAuth::Login {
                username: credentials.username.clone(),
                password,
            }
        }
        config_core::Auth::OAuth2Credentials(oauth2) => ServerAuth::OAuth2Credentials {
            user: oauth2.user.clone(),
            access_token: oauth2.access_token.clone(),
        },
        config_core::Auth::OAuth2Session(oauth2) => {
            let oauth2_client = oauth2_context
                .oauth2_clients
                .get(&oauth2.oauth2_client)
                .ok_or_else(|| ServerAuthError::OAuth2ClientNotFound {
                    name: oauth2.oauth2_client.clone(),
                })?;

            let storage = oauth2_token_storage_keyring::KeyringTokenStorage::init(
                oauth2
                    .keyring
                    .service
                    .as_deref()
                    .map(|val| std::borrow::Cow::<'static, str>::Owned(val.to_owned()))
                    .unwrap_or_else(|| {
                        std::borrow::Cow::Borrowed::<'static, _>(keyring::OAUTH2_SESSION_SERVICE)
                    }),
                oauth2
                    .keyring
                    .account
                    .as_deref()
                    .unwrap_or(&oauth2.user)
                    .to_owned(),
            )
            .await
            .map_err(ServerAuthError::OAuth2KeyringInit)?;

            let session_manager = oauth2_session::Manager {
                expiration_immenance_tolerance: std::time::Duration::from_secs(
                    oauth2
                        .expiration_immenance_tolerance_secs
                        .unwrap_or(60 * 60),
                ),
                oauth2_client: Arc::as_ref(oauth2_client).clone(),
                http_client: oauth2_context.reqwest_client.clone(),
                storage,
            };

            ServerAuth::OAuth2Session {
                user: oauth2.user.clone(),
                session_manager: Box::new(session_manager),
            }
        }
    })
}

/// Build an IMAP mailbox config.
pub fn mailbox(
    bringup_server: Arc<Server>,
    core_server: &config_core::ServerConfig,
    core_mailbox: &config_core::MailboxConfig,
) -> Mailbox {
    let idle_timeout_secs = core_mailbox
        .idle_timeout_secs
        .or(core_server.idle_timeout_secs)
        .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECS);

    Mailbox {
        server: bringup_server,
        mailbox: imap_utf7::ImapUtf7String::from_utf8(&core_mailbox.name),
        idle_timeout: std::time::Duration::from_secs(idle_timeout_secs),
    }
}

/// Resolve the password from config, including keyring lookups.
pub async fn resolve_password(
    credentials: &config_core::LoginCredentials,
) -> Result<String, ResolvePasswordError> {
    match &credentials.password {
        config_core::PasswordSource::Plain(password) => Ok(password.clone()),
        config_core::PasswordSource::Keyring { keyring } => {
            let keyring =
                keyring::service_account(keyring, &credentials.username, keyring::DEFAULT_SERVICE);
            let service = keyring.service.to_owned();
            let account = keyring.account.to_owned();

            let password =
                tokio::task::spawn_blocking(move || keyring_password::get(&service, &account))
                    .await
                    .unwrap()
                    .map_err(ResolvePasswordError::Keyring)?;
            Ok(password)
        }
    }
}
