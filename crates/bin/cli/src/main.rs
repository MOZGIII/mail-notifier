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

    monitoring_engine::spawn_monitors::<monitoring_workload_imap::Mailbox, _, _, _, _, _, _>(
        monitoring_engine::SpawnMonitorsParams {
            workload_items: &mailboxes,
            register_state: |config: &Arc<config_bringup::Mailbox>| {
                Arc::new(Entry {
                    label: format!("{} / {}", config.server.server_name, config.mailbox),
                    view_url: config.view_url.clone(),
                })
            },
            join_set: &mut join_set,
            workload_notify: |update: monitoring_engine::WorkloadUpdate<
                Arc<Entry>,
                monitoring_workload_imap::Mailbox,
            >| async move {
                tracing::info!(
                    label = %update.entry.label,
                    total = %update.payload.total,
                    unread = %update.payload.unread,
                    view_url = %update.entry.view_url.as_deref().unwrap_or_default(),
                    "mailbox counts update"
                );
            },
            supervisor_notify: move |update: monitoring_engine::SupervisorUpdate<
                Arc<Entry>,
                monitoring_workload_imap::Mailbox,
            >| async move {
                tracing::info!(label = %update.entry.label, status = ?update.payload, "supervisor event");
            },
        },
    );

    while let Some(result) = join_set.join_next().await {
        result.unwrap();
    }

    Ok(())
}

/// The entry to maintain to display the mailbox.
struct Entry {
    /// The label of the mailbox.
    pub label: String,

    /// The view URL of the mailbox.
    pub view_url: Option<Arc<str>>,
}
