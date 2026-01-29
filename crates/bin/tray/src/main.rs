//! Tray menu for mail notifier.

use std::sync::Arc;

use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent, menu::MenuEvent};

mod icon;
mod key;
mod menu;

use key::Key;

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<core::convert::Infallible> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config = config_load::with_default_env_var().await?;
    let _keyring_guard = config_bringup::init_keyring_if_needed(&config)?;
    let mailboxes = config_bringup::for_monitoring(&config).await?;
    drop(config);

    let mut join_set = tokio::task::JoinSet::new();

    let mut entries = slotmap::SlotMap::<Key, menu::EntryState>::with_key();

    let register_state = |config: &Arc<config_bringup::Mailbox>| {
        let label = format!("{} / {}", config.server.server_name, config.mailbox);
        entries.insert(menu::EntryState {
            name: label,
            active: false,
            unread: 0,
        })
    };

    let event_loop = tao::event_loop::EventLoopBuilder::<UserEvent>::with_user_event().build();

    #[cfg(target_os = "macos")]
    let event_loop = {
        let mut event_loop = event_loop;

        use tao::platform::macos::EventLoopExtMacOS as _;
        event_loop.set_dock_visibility(false);
        event_loop.set_activation_policy(tao::platform::macos::ActivationPolicy::Accessory);

        event_loop
    };

    monitoring_engine::spawn_monitors::<monitoring_workload_imap::Mailbox, _, _, _, _, _, _>(
        monitoring_engine::SpawnMonitorsParams {
            workload_items: &mailboxes,
            register_state,
            join_set: &mut join_set,
            workload_notify: {
                let proxy = event_loop.create_proxy();
                move |update| {
                    let proxy = proxy.clone();
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let _ = proxy.send_event(UserEvent::WorkloadUpdate(update));
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
        },
    );

    let (new_icon_text_sender, mut new_icon_text_receiver) = tokio::sync::mpsc::channel(128);

    let proxy = event_loop.create_proxy();
    tokio::task::spawn_blocking(move || {
        icon_render_loop::run(icon_render_loop::Params {
            width: icon::WIDTH,
            height: icon::HEIGHT,
            render_task_receiver: move || new_icon_text_receiver.blocking_recv(),
            rendered_data_sender: move |icon| {
                let result = proxy.send_event(UserEvent::NewIcon(icon));
                match result {
                    Ok(()) => std::ops::ControlFlow::Continue(()),
                    Err(_) => std::ops::ControlFlow::Break(()),
                }
            },
        })
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
    let mut total_cache = None;

    tokio::task::block_in_place(move || {
        event_loop.run(move |event, _, control_flow| {
            *control_flow = tao::event_loop::ControlFlow::Wait;

            match event {
                tao::event::Event::NewEvents(tao::event::StartCause::Init) => {
                    let icon = icon::from_render_loop_data(icon::idle()).unwrap();
                    let menu = menu::build_menu(&entries);
                    tray_icon = Some(
                        TrayIconBuilder::new()
                            .with_menu(Box::new(menu))
                            .with_icon(icon)
                            .build()
                            .unwrap(),
                    );
                    update_total(&entries, &mut total_cache, new_icon_text_sender.clone());
                }
                tao::event::Event::UserEvent(UserEvent::WorkloadUpdate(update)) => {
                    if let Some(entry) = entries.get_mut(update.entry) {
                        entry.unread = update.payload.unread;
                    }
                    update_tray_menu(&mut tray_icon, &entries);
                    update_total(&entries, &mut total_cache, new_icon_text_sender.clone());
                }
                tao::event::Event::UserEvent(UserEvent::SupervisorUpdate(update)) => {
                    if let Some(entry) = entries.get_mut(update.entry) {
                        entry.active =
                            matches!(update.payload, supervisor::SupervisorEvent::Started);
                    }
                    update_tray_menu(&mut tray_icon, &entries);
                    update_total(&entries, &mut total_cache, new_icon_text_sender.clone());
                }
                tao::event::Event::UserEvent(UserEvent::NewIcon(icon)) => {
                    update_tray_icon(&mut tray_icon, icon)
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
        })
    })
}

/// User events for the event loop.
#[derive(Debug)]
#[allow(dead_code)]
enum UserEvent {
    /// Workload update event.
    WorkloadUpdate(monitoring_engine::WorkloadUpdate<Key, monitoring_workload_imap::Mailbox>),

    /// Supervisor update event.
    SupervisorUpdate(monitoring_engine::SupervisorUpdate<Key, monitoring_workload_imap::Mailbox>),

    /// New icon is ready.
    NewIcon(icon_render_loop::Data),

    /// Tray icon event.
    TrayIcon(TrayIconEvent),

    /// Menu event.
    Menu(MenuEvent),
}

/// Update the tray icon's menu with the current entries.
fn update_tray_menu(
    tray_icon: &mut Option<TrayIcon>,
    entries: &slotmap::SlotMap<Key, menu::EntryState>,
) {
    let Some(tray_icon) = tray_icon else {
        return;
    };

    let menu = menu::build_menu(entries);
    tray_icon.set_menu(Some(Box::new(menu)));
}

/// Update the total number.
fn update_total(
    entries: &slotmap::SlotMap<Key, menu::EntryState>,
    total_cache: &mut Option<u32>,
    new_icon_image_requester: tokio::sync::mpsc::Sender<String>,
) {
    let total = entries.values().map(|state| state.unread).sum();

    let should_redraw = total_cache.map(|cache| cache != total).unwrap_or(true);

    if should_redraw {
        *total_cache = Some(total);
        let _ = new_icon_image_requester.blocking_send(total.to_string());
    }
}

/// Update the tray icon's actual icon.
fn update_tray_icon(tray_icon: &mut Option<TrayIcon>, data: icon_render_loop::Data) {
    let Some(tray_icon) = tray_icon else {
        return;
    };

    let icon = match icon::from_render_loop_data(data) {
        Ok(val) => val,
        Err(error) => {
            tracing::debug!(?error, "unable to prepare new icon");
            return;
        }
    };

    if let Err(error) = tray_icon.set_icon(Some(icon)) {
        tracing::debug!(?error, "unable to set new icon");
    }
}
