//! Main entrypoint for the IMAP LIST helper.

use futures::TryStreamExt;

/// Run the IMAP LIST helper.
#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config = config_load::with_default_env_var().await?;
    let _keyring_guard = config_bringup::init_keyring_if_needed(&config)?;
    let servers = config_bringup::servers_only(&config).await?;
    drop(config);

    for server in &servers {
        tracing::info!(
            server_name = %server.server_name,
            imap_host = %server.host,
            imap_port = server.port,
            imap_tls_mode = ?server.tls_mode,
            "listing IMAP mailboxes"
        );

        let mut session = imap_session::setup(server.to_imap_session_params()).await?;

        let mut list_stream = session.list(None, Some("*")).await?;
        println!("{}:", server.server_name);
        while let Some(name) = list_stream.try_next().await? {
            let name = imap_utf7::ImapUtf7Str::new(name.name())?;
            println!("  {name}");
        }
    }

    Ok(())
}
