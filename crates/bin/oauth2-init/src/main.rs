//! CLI utility for starting an OAuth 2 session.

use color_eyre::eyre::{Context, bail, eyre};
use oauth2::TokenResponse as _;

/// Start an OAuth 2 session and initialize its keyring for a configured server.
#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 1 {
        bail!("Usage: oauth2-init <server-name>");
    }
    let server_name = args.pop().unwrap();

    let config = config_load::with_default_env_var().await?;

    let mut matches = config
        .servers
        .iter()
        .filter(|server| server.name == server_name);
    let server = matches
        .next()
        .ok_or_else(|| eyre!("No server named '{server_name}' in config"))?;
    if matches.next().is_some() {
        bail!("Multiple servers named '{server_name}' in config");
    }

    let oauth2_session_config = match &server.auth {
        config_core::Auth::OAuth2Session(val) => val,
        _ => {
            bail!("Server '{server_name}' does not use oauth2 session auth in config");
        }
    };

    let oauth2_client_name = &oauth2_session_config.oauth2_client;

    let Some(oauth2_client_config) = config.oauth2_clients.get(oauth2_client_name) else {
        bail!(
            "Server '{server_name}' refers to an oauth2 client '{oauth2_client_name}' that can't be found in config",
        );
    };

    let oauth2_client = config_bringup::oauth2_client(oauth2_client_config)?;

    let oauth2_client = match oauth2_client.auth_uri().cloned() {
        Some(url) => oauth2_client.set_auth_uri(url),
        None => bail!("No auth url for client '{oauth2_client_name}'",),
    };

    let (pkce_challenge, pkce_verifier) = oauth2::PkceCodeChallenge::new_random_sha256();

    let oauth2_code_receiver::Started {
        redirect_url,
        receiver,
    } = oauth2_code_receiver::start().await?;

    let redirect_url = oauth2::RedirectUrl::new(redirect_url)?;

    let scopes = oauth2_session_config
        .oauth2_scopes
        .iter()
        .map(|scope| oauth2::Scope::new(scope.into()));

    let (auth_url, expected_csrf_token) = oauth2_client
        .authorize_url(oauth2::CsrfToken::new_random)
        .add_scopes(scopes)
        .set_pkce_challenge(pkce_challenge)
        .set_redirect_uri(std::borrow::Cow::Borrowed(&redirect_url))
        .url();

    println!("Browse to: {}", auth_url);
    let _ = webbrowser::open(auth_url.as_str());

    let oauth2_code_receiver::RedirectQuery { code, state } = receiver.receive().await?;

    if expected_csrf_token.secret() != &state {
        bail!("Invalid CSRF token; refusing to continue");
    }

    let reqwest_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let res = oauth2_client
        .exchange_code(oauth2::AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .set_redirect_uri(std::borrow::Cow::Owned(redirect_url))
        .request_async(&reqwest_client)
        .await?;

    let Some(refresh_token) = res.refresh_token() else {
        bail!("No refresh token in the OAuth 2 response");
    };

    let service = oauth2_session_config
        .keyring
        .service
        .as_deref()
        .unwrap_or(config_bringup::keyring::OAUTH2_SESSION_SERVICE);
    let account = oauth2_session_config
        .keyring
        .account
        .as_deref()
        .unwrap_or(&oauth2_session_config.user);

    let _guard = keyring_bridge::KeyringGuard::init_default()?;

    let mut storage = oauth2_token_storage_keyring::KeyringTokenStorage::init(
        service.to_owned(),
        account.to_owned(),
    )
    .await?;

    oauth2_session::manage(
        &mut storage,
        oauth2_session::token_storage_core::DataRef {
            access_token: res.access_token().secret(),
            expires_at: res
                .expires_in()
                .map(|expires_in| std::time::SystemTime::now() + expires_in),
            refresh_token: refresh_token.secret(),
        },
    )
    .await
    .wrap_err("Failed to store OAuth2 session in keyring")?;

    println!(
        "Initialized an OAuth 2 session for server '{}' (oauth2 client '{}', service '{}', account '{}')",
        server_name, oauth2_client_name, service, account
    );

    Ok(())
}
