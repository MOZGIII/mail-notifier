//! Main entrypoint for the CLI logger.

use std::sync::Arc;

/// Run the CLI logger.
#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config = config_load::with_default_env_var().await?;
    let _keyring_guard = config_bringup::init_keyring_if_needed(&config)?;
    let mailboxes = config_bringup::for_monitoring(&config).await?;
    drop(config);

    let mut join_set = tokio::task::JoinSet::new();

    monitoring_engine::spawn_monitors(monitoring_engine::SpawnMonitorsParams {
        mailboxes: &mailboxes,
        register_state: |config: &config_bringup::data::Mailbox| {
            Arc::new(format!(
                "{} / {}",
                config.server.server_name, config.mailbox
            ))
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
