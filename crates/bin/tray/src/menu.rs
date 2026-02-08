//! Menu module.

use tray_icon::menu::{Menu, MenuItem};

/// State of a mailbox entry in the tray menu.
#[derive(Debug)]
pub struct EntryState {
    /// Name of the mailbox.
    pub name: String,

    /// The URL to open when user wants to view this mailbox.
    pub view_url: Option<std::sync::Arc<str>>,
}

/// Build the tray menu from the current state.
pub fn build_menu(state: &mail_state_machine::State<crate::Key, EntryState>) -> Menu {
    let menu = Menu::new();
    for (key, entry) in state.iter() {
        let unread = entry.tracked().unread.unwrap_or(0);
        let text = if entry.tracked().active {
            format!("{}: {unread} unread", entry.user_data.name)
        } else {
            format!("{}: {unread} unread (inactive)", entry.user_data.name)
        };
        let enabled = entry.user_data.view_url.is_some();
        menu.append(&MenuItem::with_id(key, text, enabled, None))
            .unwrap();
    }
    menu
}
