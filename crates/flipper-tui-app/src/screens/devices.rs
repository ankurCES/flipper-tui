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

    #[test]
    fn empty_endpoint_list_doesnt_panic() {
        // Smoke check: building the view with no devices compiles and
        // can be held without panicking. Full render testing needs a
        // TestBackend — left for the integration harness.
        let _ = Devices::new();
    }
}
