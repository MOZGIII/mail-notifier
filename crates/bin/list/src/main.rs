//! Main entrypoint for the IMAP LIST helper.

use futures::TryStreamExt;

/// Run the IMAP LIST helper.
#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config_path: std::path::PathBuf = envfury::must("MAIL_NOTIFIER_CONFIG")?;
    let config = config_yaml::load_from_path(&config_path).await?;

    for server in config.servers {
        let Some(first_mailbox) = server.mailboxes.first() else {
            println!("{}: (no mailboxes configured)", server.name);
            continue;
        };

        let monitor_config = config_bringup::build_monitor_config(&server, first_mailbox);

        tracing::info!(
            server_name = %monitor_config.server_name,
            imap_host = %monitor_config.host,
            imap_port = monitor_config.port,
            imap_tls_mode = ?monitor_config.tls_mode,
            "listing IMAP mailboxes"
        );

        let tcp_stream =
            tokio::net::TcpStream::connect((monitor_config.host.as_str(), monitor_config.port))
                .await?;
        let tls_connector = imap_tls_rustls::connector()?;
        let client = imap_tls::connect(
            tcp_stream,
            &monitor_config.tls_server_name,
            monitor_config.tls_mode,
            tls_connector,
        )
        .await?;

        let mut session = client
            .login(&monitor_config.username, &monitor_config.password)
            .await
            .map_err(|(err, _client)| err)?;

        let mut list_stream = session.list(None, Some("*")).await?;
        println!("{}:", monitor_config.server_name);
        while let Some(name) = list_stream.try_next().await? {
            let name = imap_utf7::ImapUtf7Str::new(name.name())?;
            println!("  {name}");
        }
    }

    Ok(())
}
