//! CLI utility for storing a keyring password.

use color_eyre::eyre::{Context, bail, eyre};
use std::io::Read;

/// Store a keyring password for a configured server.
#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 1 {
        bail!("Usage: keyring-set <server-name>");
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

    let keyring = match &server.auth {
        config_core::Auth::Login(config_core::LoginCredentials {
            password: config_core::PasswordSource::Keyring { keyring },
            username,
        }) => config_bringup::keyring::service_account(
            keyring,
            username,
            config_bringup::keyring::DEFAULT_SERVICE,
        ),
        _ => {
            bail!("Server '{server_name}' does not use keyring credentials in config");
        }
    };

    let _guard = keyring_bridge::KeyringGuard::init_default()?;

    let password = read_password_from_stdin()?;
    keyring_password::set(keyring.service, keyring.account, &password)
        .wrap_err("Failed to store password in keyring")?;

    println!(
        "Stored password for server '{}' (service '{}', account '{}')",
        server_name, keyring.service, keyring.account
    );

    Ok(())
}

/// Read a password from stdin, trimming trailing newlines.
fn read_password_from_stdin() -> color_eyre::eyre::Result<String> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .wrap_err("Failed to read password from stdin")?;

    let password = input.trim_end_matches(['\n', '\r']).to_string();
    if password.is_empty() {
        bail!("No password provided on stdin");
    }

    Ok(password)
}
