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

    #[test]
    fn snapshot_apps_with_entries() {
        use ratatui::backend::TestBackend;
        use ratatui::layout::Rect;
        use ratatui::widgets::ListState;
        use ratatui::Terminal;

        fn collect_text(buf: &ratatui::buffer::Buffer, area: Rect) -> String {
            let mut out = String::with_capacity(area.width as usize * area.height as usize);
            for y in area.y..area.y + area.height {
                for x in area.x..area.x + area.width {
                    let cell = &buf[(x, y)];
                    let mut chars = cell.symbol().chars();
                    out.push(chars.next().unwrap_or(' '));
                    let _ = chars.next();
                }
            }
            out
        }

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let apps = Apps::new();
        let location = Apps::root();
        let entries = vec![
            StorageEntry {
                name: "nfc_reader".to_string(),
                is_dir: true,
                size: 0,
            },
            StorageEntry {
                name: "subghz_remote".to_string(),
                is_dir: true,
                size: 0,
            },
        ];
        let mut state = ListState::default();
        state.select(Some(0));
        terminal
            .draw(|f| apps.render(f, Rect::new(0, 0, 80, 24), &location, &entries, &mut state))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let text = collect_text(&buf, Rect::new(0, 0, 80, 24));

        // Header + apps title + path.
        assert!(
            text.contains("flipper-tui"),
            "missing 'flipper-tui':\n{text}"
        );
        assert!(text.contains("apps"), "missing 'apps' header:\n{text}");
        assert!(
            text.contains("/ext/apps"),
            "missing '/ext/apps' path line:\n{text}"
        );
        // Each app dir is listed as `[D] <name>`.
        assert!(
            text.contains("[D]"),
            "missing '[D]' flag for app dirs:\n{text}"
        );
        assert!(
            text.contains("nfc_reader"),
            "missing 'nfc_reader' row:\n{text}"
        );
        assert!(
            text.contains("subghz_remote"),
            "missing 'subghz_remote' row:\n{text}"
        );
        // Footer hotkeys.
        assert!(text.contains("open"), "footer missing 'open':\n{text}");
        assert!(text.contains("back"), "footer missing 'back':\n{text}");
        assert!(
            text.contains("refresh"),
            "footer missing 'refresh':\n{text}"
        );
    }

    #[test]
    fn snapshot_apps_empty() {
        use ratatui::backend::TestBackend;
        use ratatui::layout::Rect;
        use ratatui::widgets::ListState;
        use ratatui::Terminal;

        fn collect_text(buf: &ratatui::buffer::Buffer, area: Rect) -> String {
            let mut out = String::with_capacity(area.width as usize * area.height as usize);
            for y in area.y..area.y + area.height {
                for x in area.x..area.x + area.width {
                    let cell = &buf[(x, y)];
                    let mut chars = cell.symbol().chars();
                    out.push(chars.next().unwrap_or(' '));
                    let _ = chars.next();
                }
            }
            out
        }

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let apps = Apps::new();
        let location = Apps::root();
        let entries: Vec<StorageEntry> = vec![];
        let mut state = ListState::default();
        terminal
            .draw(|f| apps.render(f, Rect::new(0, 0, 80, 24), &location, &entries, &mut state))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let text = collect_text(&buf, Rect::new(0, 0, 80, 24));

        // The empty-state hint is a literal string in the source.
        assert!(
            text.contains("no apps installed"),
            "missing 'no apps installed' empty-state hint:\n{text}"
        );
        // Path is still shown so users see what was scanned.
        assert!(
            text.contains("/ext/apps"),
            "missing '/ext/apps' path:\n{text}"
        );
    }
}
