//! Main entrypoint for the CLI logger.

/// Run the CLI logger.
#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config_path: std::path::PathBuf = envfury::must("MAIL_NOTIFIER_CONFIG")?;
    let config = config_yaml::load_from_path(&config_path).await?;
    let _keyring_guard = config_bringup::init_keyring_if_needed(&config)?;
    let monitor_configs = config_bringup::build_monitor_configs(&config).await?;
    drop(config);

    let mut join_set = tokio::task::JoinSet::new();

    for config in monitor_configs {
        let label = format!("{} / {}", config.server_name, config.mailbox);
        join_set.spawn(async move {
            let label = label.as_str();
            let notify = move |counts: imap_checker::MailboxCounts| async move {
                println!("{} total={} unread={}", label, counts.total, counts.unread);
            };

            mailbox_monitor::monitor_mailbox_counts(config, notify).await
        });
    }
    while let Some(result) = join_set.join_next().await {
        result??;
    }

    Ok(())
}
