//! Menu module.

use slotmap::SlotMap;
use tray_icon::menu::{Menu, MenuItem};

/// State of a mailbox entry in the tray menu.
#[derive(Debug)]
pub struct EntryState {
    /// Name of the mailbox.
    pub name: String,

    /// Whether the mailbox is active.
    pub active: bool,

    /// Number of unread emails.
    pub unread: u32,

    /// The URL to open when user wants to view this mailbox.
    pub view_url: Option<std::sync::Arc<str>>,
}

/// Build the tray menu from the current entries.
pub fn build_menu(entries: &SlotMap<crate::Key, EntryState>) -> Menu {
    let menu = Menu::new();
    for (key, entry) in entries.iter() {
        let text = if entry.active {
            format!("{}: {} unread", entry.name, entry.unread)
        } else {
            format!("{}: inactive", entry.name)
        };
        let enabled = entry.view_url.is_some();
        menu.append(&MenuItem::with_id(key, text, enabled, None))
            .unwrap();
    }
    menu
}
