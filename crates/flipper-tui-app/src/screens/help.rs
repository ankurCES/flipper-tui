//! Help screen — one-screen cheat sheet of every binding.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

#[derive(Debug)]
pub struct Help;

impl Help {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let header = Paragraph::new(Line::from(vec![
            Span::styled("flipper-tui", Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled("help", Style::default().fg(Color::Gray)),
        ]))
        .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(header, chunks[0]);

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(Span::styled(
            "Navigation",
            Style::default().fg(Color::Yellow),
        )));
        push(&mut lines, "Tab", "cycle focus");
        push(&mut lines, "↑/↓  k/j", "move selection");
        push(&mut lines, "Enter", "activate");
        push(&mut lines, "Esc", "back / cancel");
        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled(
            "Global",
            Style::default().fg(Color::Yellow),
        )));
        push(&mut lines, "q", "quit");
        push(&mut lines, "r", "refresh / rescan");
        push(&mut lines, "?", "toggle this help");
        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled(
            "Dashboard tabs",
            Style::default().fg(Color::Yellow),
        )));
        push(&mut lines, "s", "Storage");
        push(&mut lines, "u", "Updates");
        push(&mut lines, "S", "Settings");

        let body =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" help "));
        frame.render_widget(body, chunks[1]);
    }
}

fn push(lines: &mut Vec<Line>, key: &str, desc: &str) {
    lines.push(Line::from(vec![
        Span::styled(format!("  {key:<14}"), Style::default().fg(Color::Cyan)),
        Span::raw(desc.to_string()),
    ]));
}

impl Default for Help {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
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
    fn snapshot_help() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let help = Help::new();
        terminal
            .draw(|f| help.render(f, Rect::new(0, 0, 80, 24)))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let text = collect_text(&buf, Rect::new(0, 0, 80, 24));

        // Header + body title.
        assert!(
            text.contains("flipper-tui"),
            "header missing 'flipper-tui' in snapshot:\n{text}"
        );
        assert!(
            text.contains("help"),
            "title missing 'help' in snapshot:\n{text}"
        );

        // Section headings.
        assert!(
            text.contains("Navigation"),
            "section missing 'Navigation' in snapshot:\n{text}"
        );
        assert!(
            text.contains("Global"),
            "section missing 'Global' in snapshot:\n{text}"
        );
        assert!(
            text.contains("Dashboard tabs"),
            "section missing 'Dashboard tabs' in snapshot:\n{text}"
        );

        // At least one of the documented bindings must be present.
        assert!(
            text.contains("cycle focus"),
            "binding 'cycle focus' missing in snapshot:\n{text}"
        );
        assert!(
            text.contains("back / cancel"),
            "binding 'back / cancel' missing in snapshot:\n{text}"
        );
    }
}
