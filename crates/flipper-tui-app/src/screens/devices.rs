//! Devices picker. Mirrors qFlipper's first-launch device picker.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

/// Stateless view: callers pass in a snapshot of detected endpoints and
/// a `ListState` to drive selection.
#[derive(Debug)]
pub struct Devices;

impl Devices {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        endpoints: &[String],
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
            Span::styled("devices", Style::default().fg(Color::Gray)),
        ]))
        .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(header, chunks[0]);

        let items: Vec<ListItem> = if endpoints.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  (no Flipper detected on USB — plug one in and press `r`)",
                Style::default().fg(Color::DarkGray),
            )))]
        } else {
            endpoints
                .iter()
                .map(|p| ListItem::new(Line::from(p.as_str())))
                .collect()
        };

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" detected "))
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, chunks[1], state);

        let footer = Paragraph::new(Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" connect  "),
            Span::styled("r", Style::default().fg(Color::Cyan)),
            Span::raw(" rescan  "),
            Span::styled("?", Style::default().fg(Color::Cyan)),
            Span::raw(" help  "),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(" quit"),
        ]));
        frame.render_widget(footer, chunks[2]);
    }
}

impl Default for Devices {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn snapshot_devices_empty() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let devices = Devices::new();
        let endpoints: Vec<String> = vec![];
        let mut state = ListState::default();
        terminal
            .draw(|f| devices.render(f, Rect::new(0, 0, 80, 24), &endpoints, &mut state))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let text = collect_text(&buf, Rect::new(0, 0, 80, 24));

        // Header + panel title.
        assert!(
            text.contains("flipper-tui"),
            "missing 'flipper-tui':\n{text}"
        );
        assert!(
            text.contains("devices"),
            "missing 'devices' header:\n{text}"
        );
        assert!(
            text.contains("detected"),
            "missing 'detected' panel title:\n{text}"
        );
        // Empty-state hint is the exact string from the source.
        assert!(
            text.contains("no Flipper detected on USB"),
            "missing empty-state hint:\n{text}"
        );
        // Footer hotkeys.
        assert!(
            text.contains("connect"),
            "footer missing 'connect':\n{text}"
        );
        assert!(text.contains("rescan"), "footer missing 'rescan':\n{text}");
        assert!(text.contains("help"), "footer missing 'help':\n{text}");
        assert!(text.contains("quit"), "footer missing 'quit':\n{text}");
    }

    #[test]
    fn snapshot_devices_with_endpoints() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let devices = Devices::new();
        let endpoints = vec![
            "/dev/tty.usbmodemflip_R3llow4n1".to_string(),
            "/dev/cu.usbmodemflip_R3llow4n1".to_string(),
        ];
        let mut state = ListState::default();
        state.select(Some(0));
        terminal
            .draw(|f| devices.render(f, Rect::new(0, 0, 80, 24), &endpoints, &mut state))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let text = collect_text(&buf, Rect::new(0, 0, 80, 24));

        assert!(
            text.contains("flipper-tui"),
            "missing 'flipper-tui':\n{text}"
        );
        assert!(
            text.contains("detected"),
            "missing 'detected' panel title:\n{text}"
        );
        // One endpoint per line must be present in the rendered buffer.
        assert!(
            text.contains("/dev/tty.usbmodemflip_R3llow4n1"),
            "missing first endpoint line:\n{text}"
        );
        assert!(
            text.contains("/dev/cu.usbmodemflip_R3llow4n1"),
            "missing second endpoint line:\n{text}"
        );
    }
}
