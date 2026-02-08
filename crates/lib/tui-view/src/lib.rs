//! Terminal UI rendering.

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

/// Snapshot of a mailbox entry for rendering.
#[derive(Debug, Clone, Copy)]
pub struct EntryView<'a> {
    /// Display name for the mailbox.
    pub name: &'a str,

    /// Unread message count, or [`None`] if no data has been received yet.
    pub unread: Option<u32>,

    /// Whether the mailbox is active or not.
    pub active: bool,
}

/// Render the main UI frame.
pub fn render<'a, B, I>(terminal: &mut ratatui::Terminal<B>, entries: I) -> Result<(), B::Error>
where
    B: ratatui::backend::Backend,
    I: IntoIterator<Item = EntryView<'a>>,
{
    let entries: Vec<EntryView<'a>> = entries.into_iter().collect();
    terminal.draw(|frame| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(frame.area());

        let header = Paragraph::new("Mail Notifier — press q to quit")
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(header, chunks[0]);

        let items: Vec<ListItem> = if entries.is_empty() {
            vec![ListItem::new("No mailboxes configured")]
        } else {
            entries
                .iter()
                .map(|entry| {
                    let count = entry.unread.unwrap_or(0);
                    ListItem::new(format!("{} — {} new", entry.name, count)).style({
                        let mut s = Style::new();
                        if !entry.active {
                            s = s.italic();
                        }
                        s
                    })
                })
                .collect()
        };

        let list =
            List::new(items).block(Block::default().borders(Borders::ALL).title("Mailboxes"));
        frame.render_widget(list, chunks[1]);
    })?;

    Ok(())
}
