//! Main entrypoint for the CLI logger.

use std::sync::Arc;

/// Run the CLI logger.
#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config_path: std::path::PathBuf = envfury::must("MAIL_NOTIFIER_CONFIG")?;
    let config = config_yaml::load_from_path(&config_path).await?;
    let _keyring_guard = config_bringup::init_keyring_if_needed(&config)?;
    let monitor_configs = config_bringup::bringup_monitor_configs(&config).await?;
    drop(config);

    let mut join_set = tokio::task::JoinSet::new();

    monitoring_engine::spawn_monitors(monitoring_engine::SpawnMonitorsParams {
        monitor_configs: &monitor_configs,
        register_state: |config: &imap_monitor::Config| {
            Arc::new(format!("{} / {}", config.server_name, config.mailbox))
        },
        join_set: &mut join_set,
        mailbox_notify: |update: monitoring_engine::MailboxUpdate<Arc<String>>| async move {
            tracing::info!(
                label = %update.entry,
                total = %update.payload.total,
                unread = %update.payload.unread,
                "mailbox counts update"
            );
        },
        supervisor_notify: move |update: monitoring_engine::SupervisorUpdate<Arc<String>>| async move {
            tracing::info!(label = %update.entry, status = ?update.payload, "supervisor event");
        },
    });

    while let Some(result) = join_set.join_next().await {
        result.unwrap();
    }

    Ok(())
}
