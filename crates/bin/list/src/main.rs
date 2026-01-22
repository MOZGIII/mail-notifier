//! Main entrypoint for the IMAP LIST helper.

use futures::TryStreamExt;

/// Run the IMAP LIST helper.
#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config_path: std::path::PathBuf = envfury::must("MAIL_NOTIFIER_CONFIG")?;
    let config = config_yaml::load_from_path(&config_path).await?;
    let _keyring_guard = config_bringup::init_keyring_if_needed(&config)?;

    for server in &config.servers {
        let server_config = config_bringup::bringup_server_config(server).await?;

        tracing::info!(
            server_name = %server_config.server_name,
            imap_host = %server_config.host,
            imap_port = server_config.port,
            imap_tls_mode = ?server_config.tls_mode,
            "listing IMAP mailboxes"
        );

        let mut session = imap_session::setup(imap_session::SetupParams {
            host: &server_config.host,
            port: server_config.port,
            tls_mode: server_config.tls_mode,
            tls_server_name: &server_config.tls_server_name,
            username: &server_config.username,
            password: &server_config.password,
        })
        .await?;

        let mut list_stream = session.list(None, Some("*")).await?;
        println!("{}:", server_config.server_name);
        while let Some(name) = list_stream.try_next().await? {
            let name = imap_utf7::ImapUtf7Str::new(name.name())?;
            println!("  {name}");
        }
    }
    drop(config);

    Ok(())
}
