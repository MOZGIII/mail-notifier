//! Main entrypoint.

use std::sync::Arc;

/// Local UI state for a mailbox entry.
#[derive(Debug, Clone)]
struct EntryState {
    /// Display name for the mailbox.
    name: String,
}

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config = config_load::with_default_env_var().await?;
    let _keyring_guard = config_bringup::init_keyring_if_needed(&config)?;
    let mailboxes = config_bringup::for_monitoring(&config).await?;
    let actions_engine = Arc::new(config_bringup::actions_engine(config.events.as_ref()));
    drop(config);

    let mut join_set = tokio::task::JoinSet::new();

    let (mailbox_sender, mut mailbox_receiver) = tokio::sync::mpsc::channel(128);
    let (supervisor_sender, mut supervisor_receiver) = tokio::sync::mpsc::channel(128);

    let mut state = mail_state_machine::State::<slotmap::DefaultKey, EntryState>::new();

    let register_state = |config: &Arc<config_bringup::Mailbox>| {
        let label = format!("{} / {}", config.server.server_name, config.mailbox);
        state.insert(EntryState { name: label })
    };

    monitoring_engine::spawn_monitors::<monitoring_workload_imap::Mailbox, _, _, _, _, _, _>(
        monitoring_engine::SpawnMonitorsParams {
            workload_items: &mailboxes,
            register_state,
            join_set: &mut join_set,
            workload_notify: move |update| {
                let mailbox_sender = mailbox_sender.clone();
                async move {
                    let _ = mailbox_sender.send(update).await;
                }
            },
            supervisor_notify: move |update| {
                let supervisor_sender = supervisor_sender.clone();
                async move {
                    let _ = supervisor_sender.send(update).await;
                }
            },
        },
    );

    tracing::info!(message = "Entering UI...");

    let terminal_guard = tui_crossterm_guard::TerminalGuard::enter()?;
    let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;
    terminal.clear()?;

    let (input_sender, mut input_receiver) = tokio::sync::mpsc::channel(32);
    tokio::task::spawn_blocking(move || {
        while let Ok(evt) = crossterm::event::read() {
            if input_sender.blocking_send(evt).is_err() {
                break;
            }
        }
    });

    tui_view::render(&mut terminal, entry_views(&state))?;

    loop {
        tokio::select! {
            Some(input_event) = input_receiver.recv() => {
                match input_event {
                    crossterm::event::Event::Key(key)
                        if matches!(key.code, crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc) => {
                        break;
                    }
                    crossterm::event::Event::Resize(_, _) => {
                        tui_view::render(&mut terminal, entry_views(&state))?;
                    }
                    _ => {}
                }
            }
            Some(update) = mailbox_receiver.recv() => {
                if let Some(mut proc) = state.process_update(update.entry) {
                    let effects = proc.workload(&update.payload);
                    tokio::spawn({
                        let actions_engine = Arc::clone(&actions_engine);
                        async move { actions_engine.process(&effects).await }
                    });
                }

                tui_view::render(&mut terminal, entry_views(&state))?;
            }
            Some(update) = supervisor_receiver.recv() => {
                if let Some(mut proc) = state.process_update(update.entry) {
                    proc.supervisor(&update.payload);
                }

                tui_view::render(&mut terminal, entry_views(&state))?;
            }
            Some(result) = join_set.join_next() => {
                result.unwrap();
            }
            else => break,
        }
    }

    drop(terminal_guard);

    tracing::info!(message = "Exiting...");

    Ok(())
}

/// Build [`tui_view::EntryView`] items from the unified state.
fn entry_views(
    state: &mail_state_machine::State<slotmap::DefaultKey, EntryState>,
) -> impl Iterator<Item = tui_view::EntryView<'_>> {
    state.entries().values().map(|entry| tui_view::EntryView {
        name: &entry.user_data.name,
        active: entry.tracked().active,
        unread: entry.tracked().unread,
    })
}
