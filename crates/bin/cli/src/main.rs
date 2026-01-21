//! Main entrypoint for the CLI logger.

/// Run the CLI logger.
#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config_path: std::path::PathBuf = envfury::must("MAIL_NOTIFIER_CONFIG")?;
    let config = config_yaml::load_from_path(&config_path).await?;

    let mut join_set = tokio::task::JoinSet::new();

    for server in config.servers {
        for mailbox in &server.mailboxes {
            let label = format!("{} / {}", server.name, mailbox.name);
            let server = server.clone();
            let mailbox = mailbox.clone();
            let label_clone = label.clone();

            join_set.spawn(async move {
                let notify = move |counts: imap_checker::MailboxCounts| {
                    let label = label_clone.clone();
                    async move {
                        println!("{} total={} unread={}", label, counts.total, counts.unread);
                    }
                };

                let config = config_bringup::build_monitor_config(server, mailbox);
                mailbox_monitor::monitor_mailbox_counts(config, notify).await
            });
        }
    }

    while let Some(result) = join_set.join_next().await {
        result??;
    }

    Ok(())
}
