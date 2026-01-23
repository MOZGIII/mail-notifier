//! Tray menu for mail notifier.

use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent, menu::MenuEvent};

mod key;
mod menu;

use key::Key;

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<core::convert::Infallible> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config_path: std::path::PathBuf = envfury::must("MAIL_NOTIFIER_CONFIG")?;
    let config = config_yaml::load_from_path(&config_path).await?;
    let _keyring_guard = config_bringup::init_keyring_if_needed(&config)?;
    let monitor_configs = config_bringup::bringup_monitor_configs(&config).await?;
    drop(config);

    let mut join_set = tokio::task::JoinSet::new();

    let mut entries = slotmap::SlotMap::<Key, menu::EntryState>::with_key();

    let register_state = |config: &imap_monitor::Config| {
        let label = format!("{} / {}", config.server_name, config.mailbox);
        entries.insert(menu::EntryState {
            name: label,
            active: false,
            unread: 0,
        })
    };

    let event_loop = tao::event_loop::EventLoopBuilder::<UserEvent>::with_user_event().build();

    monitoring_engine::spawn_monitors(monitoring_engine::SpawnMonitorsParams {
        monitor_configs: &monitor_configs,
        register_state,
        join_set: &mut join_set,
        mailbox_notify: {
            let proxy = event_loop.create_proxy();
            move |update| {
                let proxy = proxy.clone();
                async move {
                    tokio::task::spawn_blocking(move || {
                        let _ = proxy.send_event(UserEvent::MailboxUpdate(update));
                    })
                    .await
                    .unwrap()
                }
            }
        },
        supervisor_notify: {
            let proxy = event_loop.create_proxy();
            move |update| {
                let proxy = proxy.clone();
                async move {
                    tokio::task::spawn_blocking(move || {
                        let _ = proxy.send_event(UserEvent::SupervisorUpdate(update));
                    })
                    .await
                    .unwrap()
                }
            }
        },
    });

    tracing::info!(message = "Starting tray...");

    let proxy = event_loop.create_proxy();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::TrayIcon(event));
    }));

    let proxy = event_loop.create_proxy();
    tray_icon::menu::MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Menu(event));
    }));

    let mut tray_icon = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = tao::event_loop::ControlFlow::Wait;

        match event {
            tao::event::Event::NewEvents(tao::event::StartCause::Init) => {
                let icon = load_icon();
                let menu = menu::build_menu(&entries);
                tray_icon = Some(
                    TrayIconBuilder::new()
                        .with_menu(Box::new(menu))
                        .with_icon(icon)
                        .build()
                        .unwrap(),
                );
            }
            tao::event::Event::UserEvent(UserEvent::MailboxUpdate(update)) => {
                if let Some(entry) = entries.get_mut(update.entry) {
                    entry.unread = update.payload.unread;
                }
                update_tray_menu(&mut tray_icon, &entries);
            }
            tao::event::Event::UserEvent(UserEvent::SupervisorUpdate(update)) => {
                if let Some(entry) = entries.get_mut(update.entry) {
                    entry.active = matches!(update.payload, supervisor::SupervisorEvent::Started);
                }
                update_tray_menu(&mut tray_icon, &entries);
            }
            tao::event::Event::UserEvent(UserEvent::TrayIcon(_event)) => {
                // Handle tray icon events if needed
            }
            tao::event::Event::UserEvent(UserEvent::Menu(event)) => {
                if let Ok(key) = event.id.try_into()
                    && let Some(entry) = entries.get(key)
                {
                    tracing::info!("Menu item clicked: {}", entry.name);
                }
            }
            tao::event::Event::WindowEvent {
                event: tao::event::WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = tao::event_loop::ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

/// User events for the event loop.
#[derive(Debug)]
#[allow(dead_code)]
enum UserEvent {
    /// Mailbox update event.
    MailboxUpdate(monitoring_engine::MailboxUpdate<Key>),

    /// Supervisor update event.
    SupervisorUpdate(monitoring_engine::SupervisorUpdate<Key>),

    /// Tray icon event.
    TrayIcon(TrayIconEvent),

    /// Menu event.
    Menu(MenuEvent),
}

/// Load a simple icon for the tray.
fn load_icon() -> tray_icon::Icon {
    // For now, create a simple icon
    let rgba = vec![255u8; 32 * 32 * 4];
    tray_icon::Icon::from_rgba(rgba, 32, 32).unwrap()
}

/// Update the tray icon's menu with the current entries.
fn update_tray_menu(
    tray_icon: &mut Option<TrayIcon>,
    entries: &slotmap::SlotMap<Key, menu::EntryState>,
) {
    if let Some(tray_icon) = tray_icon {
        let menu = menu::build_menu(entries);
        tray_icon.set_menu(Some(Box::new(menu)));
    }
}
