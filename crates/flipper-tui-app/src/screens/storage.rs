//! Storage browser. Mirrors qFlipper's `FileManager` tab.
//!
//! Renders a list of `StorageEntry` items for the currently-viewed path
//! (e.g. `/ext`, `/ext/apps`). Directories are prefixed `[D]`, files
//! `[F]`, matching the Flipper CLI's own output so the user can read
//! the same shape they see in the serial console.
//!
//! v0.1 is read-only: pressing `Enter` on a directory navigates into
//! it, `Esc` backs out one level, `r` refreshes. File selection
//! displays size + type only — viewing / downloading / uploading /
//! deleting is gated behind the pyflipper safety list and lands in
//! v0.2 with the qFlipper-style confirmation dialog.

use flipper_core::StorageEntry;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

/// Where the browser is currently pointing. The string is the path
/// the TUI last sent to the device (`/ext`, `/ext/apps`, etc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageLocation {
    pub path: String,
}

impl StorageLocation {
    pub fn root() -> Self {
        Self {
            path: "/ext".to_string(),
        }
    }

    /// `cd` into a child directory relative to the current path. Joins
    /// with a single `/` separator; the caller is responsible for not
    /// passing `..` or absolute paths (we trust the parsed CLI output).
    #[must_use]
    pub fn descend(&self, child: &str) -> Self {
        let path = if self.path.ends_with('/') {
            format!("{}{}", self.path, child)
        } else {
            format!("{}/{}", self.path, child)
        };
        Self { path }
    }

    /// Pop one directory component off the end. Returns `None` if
    /// already at the root (`/ext`); the caller should leave the
    /// screen rather than wrap around.
    #[must_use]
    pub fn ascend(&self) -> Option<Self> {
        let trimmed = self.path.trim_end_matches('/');
        let (parent, _name) = trimmed.rsplit_once('/')?;
        if parent.is_empty() {
            return None;
        }
        Some(Self {
            path: parent.to_string(),
        })
    }
}

#[derive(Debug)]
pub struct Storage;

impl Storage {
    pub fn new() -> Self {
        Self
    }

    /// Render the browser. `location` is the path currently displayed
    /// (rendered in the header); `entries` is the result of the most
    /// recent `parse_storage_list` for that path.
    pub fn render(
        &self,
        frame: &mut Frame,
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
            Span::styled("storage", Style::default().fg(Color::Gray)),
            Span::raw("  "),
            Span::styled(&location.path, Style::default().fg(Color::Yellow)),
        ]))
        .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(header, chunks[0]);

        let items: Vec<ListItem> = if entries.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  (empty directory)",
                Style::default().fg(Color::DarkGray),
            )))]
        } else {
            entries
                .iter()
                .map(|e| {
                    let flag = if e.is_dir { "[D] " } else { "[F] " };
                    let size = if e.is_dir {
                        "<dir>".to_string()
                    } else {
                        human_size(e.size)
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(flag, Style::default().fg(Color::Cyan)),
                        Span::raw(format!("{:<24}", e.name)),
                        Span::styled(size, Style::default().fg(Color::DarkGray)),
                    ]))
                })
                .collect()
        };

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" contents "))
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
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}

/// Render a byte count as `B` / `KiB` / `MiB`. Matches qFlipper's
/// `FileManagerDelegate` size column format closely enough that the
/// TUI size and the CLI's `ls -la` line are visually comparable.
fn human_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * 1024;
    if bytes >= MIB {
        format!("{} MiB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{} KiB", bytes / KIB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_storage_root_with_entries() {
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
        let storage = Storage::new();
        let location = StorageLocation::root();
        let entries = vec![
            StorageEntry {
                name: "apps".to_string(),
                is_dir: true,
                size: 0,
            },
            StorageEntry {
                name: "Manifest".to_string(),
                is_dir: false,
                size: 4096,
            },
        ];
        let mut state = ListState::default();
        state.select(Some(0));
        terminal
            .draw(|f| storage.render(f, Rect::new(0, 0, 80, 24), &location, &entries, &mut state))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let text = collect_text(&buf, Rect::new(0, 0, 80, 24));

        // Header + panel title + path.
        assert!(
            text.contains("flipper-tui"),
            "missing 'flipper-tui':\n{text}"
        );
        assert!(
            text.contains("storage"),
            "missing 'storage' header:\n{text}"
        );
        assert!(text.contains("/ext"), "missing '/ext' path line:\n{text}");
        assert!(
            text.contains("contents"),
            "missing 'contents' panel title:\n{text}"
        );
        // Directory row uses [D] flag.
        assert!(
            text.contains("[D]"),
            "missing '[D]' flag in entries:\n{text}"
        );
        assert!(text.contains("apps"), "missing 'apps' row:\n{text}");
        // File row uses [F] flag and a human-readable size.
        assert!(
            text.contains("[F]"),
            "missing '[F]' flag for files:\n{text}"
        );
        assert!(text.contains("Manifest"), "missing 'Manifest' row:\n{text}");
        assert!(text.contains("4 KiB"), "missing size '4 KiB':\n{text}");
        // Footer hotkeys.
        assert!(text.contains("open"), "footer missing 'open':\n{text}");
        assert!(text.contains("back"), "footer missing 'back':\n{text}");
        assert!(
            text.contains("refresh"),
            "footer missing 'refresh':\n{text}"
        );
    }

    #[test]
    fn snapshot_storage_empty() {
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
        let storage = Storage::new();
        let location = StorageLocation {
            path: "/ext/lol_no_such_dir".to_string(),
        };
        let entries: Vec<StorageEntry> = vec![];
        let mut state = ListState::default();
        terminal
            .draw(|f| storage.render(f, Rect::new(0, 0, 80, 24), &location, &entries, &mut state))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let text = collect_text(&buf, Rect::new(0, 0, 80, 24));

        assert!(
            text.contains("empty directory"),
            "missing empty-state hint:\n{text}"
        );
    }

    // ---- Behavior tests for StorageLocation::descend / ascend / human_size ----
    // These were originally added in M5a and are restored here after the
    // snapshot-test rewrite accidentally dropped them. The snapshot tests
    // cover the render path; these tests cover the pure path-string logic.

    #[test]
    fn descend_appends_with_single_slash() {
        let loc = StorageLocation::root();
        let child = loc.descend("apps");
        assert_eq!(child.path, "/ext/apps");
        let grandchild = child.descend("nfc");
        assert_eq!(grandchild.path, "/ext/apps/nfc");
    }

    #[test]
    fn ascend_pops_one_component() {
        let loc = StorageLocation {
            path: "/ext/apps/nfc".to_string(),
        };
        let parent = loc.ascend().unwrap();
        assert_eq!(parent.path, "/ext/apps");
        let grandparent = parent.ascend().unwrap();
        assert_eq!(grandparent.path, "/ext");
        assert!(grandparent.ascend().is_none(), "ascending /ext yields None");
    }

    #[test]
    fn ascend_on_root_returns_none() {
        let loc = StorageLocation::root();
        assert!(loc.ascend().is_none());
    }

    #[test]
    fn human_size_renders_known_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2 KiB");
        assert_eq!(human_size(5 * 1024 * 1024), "5 MiB");
    }
}
