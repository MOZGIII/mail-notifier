//! Main entrypoint.

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config_path: std::path::PathBuf = envfury::must("MAIL_NOTIFIER_CONFIG")?;
    let config = config_yaml::load_from_path(&config_path).await?;
    let _keyring_guard = config_bringup::init_keyring_if_needed(&config)?;

    let mut join_set = tokio::task::JoinSet::new();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(128);

    let mut entries: slotmap::SlotMap<slotmap::DefaultKey, tui_view::EntryState> =
        slotmap::SlotMap::with_key();

    for server in &config.servers {
        for mailbox in &server.mailboxes {
            let label = format!("{} / {}", server.name, mailbox.name);
            let entry_key = entries.insert(tui_view::EntryState {
                name: label,
                unread: 0,
            });

            let sender = sender.clone();
            let config = config_bringup::build_monitor_config(server, mailbox).await?;
            join_set.spawn(async move {
                let notify = move |counts| {
                    let sender = sender.clone();
                    async move {
                        let _ = sender.send(MailboxUpdate { entry_key, counts }).await;
                    }
                };

                mailbox_monitor::monitor_mailbox_counts(config, notify).await
            });
        }
    }
    drop(config);

    drop(sender);

    let _guard = tui_crossterm_guard::TerminalGuard::enter()?;
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
            Some(update) = receiver.recv() => {
                if let Some(entry) = entries.get_mut(update.entry_key) {
                    entry.unread = update.counts.unread;
                }

                tui_view::render(&mut terminal, entries.values())?;
            }
            Some(result) = join_set.join_next() => {
                result??;
            }
            else => break,
        }
    }

    Ok(())
}

/// Update payload for a mailbox.
#[derive(Debug, Clone)]
struct MailboxUpdate {
    /// Key for the UI entry.
    entry_key: slotmap::DefaultKey,

    /// Mailbox counts.
    counts: imap_checker::MailboxCounts,
}
