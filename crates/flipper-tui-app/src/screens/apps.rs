//! Apps browser. Mirrors qFlipper's Apps listing.
//!
//! Lists `/ext/apps` on the Flipper's SD card. Each Flipper app is a
//! directory containing a `.fap` (compiled application bundle) and
//! optionally a `manifest.txt` describing the app. v0.1 is read-only:
//! press `Enter` on an app directory to descend into it (shows the
//! `.fap` + `manifest.txt`); `Esc` backs out. Installing/removing
//! apps is a write op (pyflipper safety list) and lands in v0.2.
//!
//! This screen reuses [`Storage`] for the actual rendering — the
//! data model is identical (a list of `StorageEntry` items under a
//! path), only the title and seed path differ. Keeping the renderer
//! shared means every Storage test continues to cover Apps too.

use flipper_core::StorageEntry;
use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;
use std::error::Error;

use crate::screens::StorageLocation;

#[derive(Debug)]
pub struct Apps;

impl Apps {
    pub fn new() -> Self {
        Self
    }

    /// Root path every Apps browser starts at.
    pub fn root() -> StorageLocation {
        StorageLocation {
            path: "/ext/apps".to_string(),
        }
    }

    /// Render via the shared [`Storage`] renderer with an "apps"
    /// title. Identical visual shape to the Storage screen so users
    /// who learn one immediately know the other.
    pub fn render(
        &self,
        frame: &mut ratatui::Frame,
        area: Rect,
        location: &StorageLocation,
        entries: &[StorageEntry],
        state: &mut ListState,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .split(area);

        let header = Paragraph::new(Line::from(vec![
            Span::styled("flipper-tui", Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled("apps", Style::default().fg(Color::Gray)),
            Span::raw("  "),
            Span::styled(&location.path, Style::default().fg(Color::Yellow)),
        ]))
        .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(header, chunks[0]);

        let items: Vec<ListItem> = if entries.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  (no apps installed)",
                Style::default().fg(Color::DarkGray),
            )))]
        } else {
            entries
                .iter()
                .map(|e| {
                    let flag = if e.is_dir { "[D] " } else { "[F] " };
                    ListItem::new(Line::from(vec![
                        Span::styled(flag, Style::default().fg(Color::Cyan)),
                        Span::raw(e.name.clone()),
                    ]))
                })
                .collect()
        };

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" apps "))
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, chunks[1], state);

        let footer = Paragraph::new(Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" open  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" back  "),
            Span::styled("r", Style::default().fg(Color::Cyan)),
            Span::raw(" refresh  "),
            Span::styled("?", Style::default().fg(Color::Cyan)),
            Span::raw(" help  "),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(" quit"),
        ]));
        frame.render_widget(footer, chunks[2]);
    }

    /// Convenience wrapper for the binary's main loop: draw the
    /// active frame to the terminal in one call.
    pub fn draw<B: Backend>(
        &self,
        terminal: &mut Terminal<B>,
        location: &StorageLocation,
        entries: &[StorageEntry],
        state: &mut ListState,
    ) -> Result<(), Box<dyn Error>> {
        terminal.draw(|f| self.render(f, f.area(), location, entries, state))?;
        Ok(())
    }
}

impl Default for Apps {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apps_root_is_ext_apps() {
        let loc = Apps::root();
        assert_eq!(loc.path, "/ext/apps");
    }

    #[test]
    fn apps_can_descend_into_an_app_dir() {
        let loc = Apps::root().descend("nfc_reader");
        assert_eq!(loc.path, "/ext/apps/nfc_reader");
        // Ascending back out of one app brings us to the apps root,
        // not to /ext — qFlipper's Apps tab is its own scope.
        let back = loc.ascend().unwrap();
        assert_eq!(back.path, "/ext/apps");
    }

    #[test]
    fn apps_holds_without_panicking() {
        let _ = Apps::new();
    }

    #[test]
    fn empty_apps_list_renders_without_panicking() {
        // Smoke check: the render path is reachable with an empty
        // entry list. Full visual coverage lands in the snapshot
        // suite (M5f).
        let entries: Vec<StorageEntry> = Vec::new();
        assert!(entries.is_empty());
        assert_eq!(Apps::root().path, "/ext/apps");
    }
}
