//! Main entrypoint.

use std::sync::Arc;

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config = config_load::with_default_env_var().await?;
    let _keyring_guard = config_bringup::init_keyring_if_needed(&config)?;
    let mailboxes = config_bringup::for_monitoring(&config).await?;
    drop(config);

    let mut join_set = tokio::task::JoinSet::new();

    let (mailbox_sender, mut mailbox_receiver) = tokio::sync::mpsc::channel(128);
    let (supervisor_sender, mut supervisor_receiver) = tokio::sync::mpsc::channel(128);

    let mut entries: slotmap::SlotMap<slotmap::DefaultKey, tui_view::EntryState> =
        slotmap::SlotMap::with_key();

    let register_state = |config: &Arc<config_bringup::data::Mailbox>| {
        let label = format!("{} / {}", config.server.server_name, config.mailbox);
        entries.insert(tui_view::EntryState {
            name: label,
            active: false,
            unread: 0,
        })
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

    tui_view::render(&mut terminal, entries.values())?;

    loop {
        tokio::select! {
            Some(input_event) = input_receiver.recv() => {
                match input_event {
                    crossterm::event::Event::Key(key)
                        if matches!(key.code, crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc) => {
                        break;
                    }
                    crossterm::event::Event::Resize(_, _) => {
                        tui_view::render(&mut terminal, entries.values())?;
                    }
                    _ => {}
                }
            }
            Some(update) = mailbox_receiver.recv() => {
                if let Some(entry) = entries.get_mut(update.entry) {
                    entry.unread = update.payload.unread;
                }

                tui_view::render(&mut terminal, entries.values())?;
            }
            Some(update) = supervisor_receiver.recv() => {
                if let Some(entry) = entries.get_mut(update.entry) {
                    entry.active = matches!(update.payload, supervisor::SupervisorEvent::Started);
                }

                tui_view::render(&mut terminal, entries.values())?;
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
