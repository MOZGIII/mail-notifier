//! Main entrypoint.

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
    let (mailbox_sender, mut mailbox_receiver) = tokio::sync::mpsc::channel(128);
    let (supervisor_sender, mut supervisor_receiver) = tokio::sync::mpsc::channel(128);

    let mut entries: slotmap::SlotMap<slotmap::DefaultKey, tui_view::EntryState> =
        slotmap::SlotMap::with_key();

    for config in monitor_configs {
        let label = format!("{} / {}", config.server_name, config.mailbox);
        let entry_key = entries.insert(tui_view::EntryState {
            name: label,
            active: false,
            unread: 0,
        });

        let mailbox_sender = mailbox_sender.clone();
        let supervisor_sender = supervisor_sender.clone();
        join_set.spawn(async move {
            let work = move || {
                let mailbox_sender = mailbox_sender.clone();
                let config = config.clone();
                let mailbox_notify = move |counts| {
                    let mailbox_sender = mailbox_sender.clone();
                    async move {
                        let _ = mailbox_sender
                            .send(MailboxUpdate { entry_key, counts })
                            .await;
                    }
                };

                std::panic::AssertUnwindSafe(async move {
                    imap_monitor::monitor(&config, mailbox_notify).await
                })
            };

            let retries_backoff = exp_backoff::State {
                factor: 2,
                max: core::time::Duration::from_secs(30),
                value: core::time::Duration::from_secs(1),
            };

            let supervisor_notify = move |event| {
                let supervisor_sender = supervisor_sender.clone();
                async move {
                    let _ = supervisor_sender
                        .send(SupervisorUpdate { entry_key, event })
                        .await;
                }
            };

            supervisor::run(supervisor::Params {
                work,
                notifier: supervisor_notify,
                sleep: tokio::time::sleep,
                retries_backoff,
            })
            .await
        });
    }

    drop(mailbox_sender);
    drop(supervisor_sender);

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
                if let Some(entry) = entries.get_mut(update.entry_key) {
                    entry.unread = update.counts.unread;
                }

                tui_view::render(&mut terminal, entries.values())?;
            }
            Some(update) = supervisor_receiver.recv() => {
                if let Some(entry) = entries.get_mut(update.entry_key) {
                    entry.active = matches!(update.event, supervisor::SupervisorEvent::Started);
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

/// Update payload for a mailbox.
#[derive(Debug)]
struct MailboxUpdate {
    /// Key for the UI entry.
    entry_key: slotmap::DefaultKey,

    /// Mailbox counts.
    counts: imap_checker::MailboxCounts,
}

/// Update payload for a supervised task.
#[derive(Debug)]
struct SupervisorUpdate {
    /// Key for the UI entry.
    entry_key: slotmap::DefaultKey,

    /// The supervisor event.
    event: supervisor::SupervisorEvent<core::convert::Infallible, imap_monitor::MonitorError>,
}
