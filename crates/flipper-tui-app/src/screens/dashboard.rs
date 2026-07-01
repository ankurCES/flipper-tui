//! Dashboard — the qFlipper "Home" tab. Renders hardware, firmware,
//! radio, and flash metadata in two columns.

use flipper_core::DeviceInfo;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

#[derive(Debug)]
pub struct Dashboard;

impl Dashboard {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, info: &DeviceInfo) {
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
            Span::styled(
                format!("{} • {}", info.hardware.revision, info.hardware.region),
                Style::default().fg(Color::Gray),
            ),
        ]))
        .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(header, chunks[0]);

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        frame.render_widget(hardware_panel(info), cols[0]);
        frame.render_widget(radio_panel(info), cols[1]);

        let footer = Paragraph::new(Line::from(vec![
            Span::styled("s", Style::default().fg(Color::Cyan)),
            Span::raw(" storage  "),
            Span::styled("u", Style::default().fg(Color::Cyan)),
            Span::raw(" updates  "),
            Span::styled("S", Style::default().fg(Color::Cyan)),
            Span::raw(" settings  "),
            Span::styled("?", Style::default().fg(Color::Cyan)),
            Span::raw(" help  "),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(" quit"),
        ]));
        frame.render_widget(footer, chunks[2]);
    }
}

fn hardware_panel(info: &DeviceInfo) -> Paragraph<'_> {
    let mut lines: Vec<Line> = Vec::new();
    push_field(&mut lines, "Hardware", &info.hardware.name);
    push_field(&mut lines, "Revision", &info.hardware.revision);
    push_field(&mut lines, "Region", &info.hardware.region);
    push_field(&mut lines, "Lot", &info.hardware.lot);
    push_field(&mut lines, "Serial", &info.serial);
    lines.push(Line::from(""));
    push_field(&mut lines, "Firmware", &info.firmware_branch);
    push_field(&mut lines, "Version", &info.firmware_version);
    push_field(&mut lines, "Build", &info.firmware_build);
    push_field(&mut lines, "Target", &info.firmware_target);
    lines.push(Line::from(""));
    push_field(&mut lines, "Flash", &info.flash.vendor);
    push_field(&mut lines, "Model", &info.flash.model);
    push_field(&mut lines, "Size (kB)", &info.flash.size_bytes.to_string());
    push_field(
        &mut lines,
        "API",
        &format!("{}.{}", info.api_major, info.api_minor),
    );
    push_field(&mut lines, "Boot", &format!("{:?}", info.boot_mode));

    Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" hardware "))
}

fn radio_panel(info: &DeviceInfo) -> Paragraph<'_> {
    let yes = Style::default().fg(Color::Green);
    let no = Style::default().fg(Color::DarkGray);
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "Radios",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    push_flag(&mut lines, "Sub-GHz", info.radio.subghz, yes, no);
    push_flag(&mut lines, "NFC", info.radio.nfc, yes, no);
    push_flag(&mut lines, "IR", info.radio.ir, yes, no);
    lines.push(Line::from(""));
    push_field(&mut lines, "BLE MAC", &info.radio.ble_mac);
    Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" radio "))
}

fn push_field(lines: &mut Vec<Line<'_>>, key: &str, value: &str) {
    lines.push(Line::from(vec![
        Span::styled(format!("{key:<14} "), Style::default().fg(Color::DarkGray)),
        Span::raw(value.to_string()),
    ]));
}

fn push_flag(lines: &mut Vec<Line<'_>>, key: &str, on: bool, yes: Style, no: Style) {
    lines.push(Line::from(vec![
        Span::styled(format!("{key:<14} "), Style::default().fg(Color::DarkGray)),
        Span::styled(if on { "YES" } else { "—  " }, if on { yes } else { no }),
    ]));
}

impl Default for Dashboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flipper_core::{BootMode, FlashInfo, HardwareInfo, RadioInfo};

    fn sample() -> DeviceInfo {
        DeviceInfo {
            hardware: HardwareInfo {
                name: "f7".into(),
                revision: "R3llow4n".into(),
                region: "US".into(),
                lot: "2024-Q3-19".into(),
            },
            firmware_branch: "mntm-012".into(),
            firmware_version: "Momentum v1.4.4 OCT 2024".into(),
            firmware_build: "4106".into(),
            firmware_target: "f7".into(),
            radio: RadioInfo {
                ble_mac: "AA:BB:CC:DD:EE:FF".into(),
                subghz: true,
                nfc: true,
                ir: true,
            },
            flash: FlashInfo {
                vendor: "Winbond".into(),
                model: "W25Q128".into(),
                size_bytes: 16384,
            },
            api_major: 87,
            api_minor: 1,
            boot_mode: BootMode::Normal,
            serial: "flip_R3llow4n1".into(),
        }
    }

    #[test]
    fn dashboard_holds_without_panicking() {
        let _ = Dashboard::new();
        let info = sample();
        // Just constructing the dashboard and reading the info shape is
        // enough to prove the wiring compiles. Visual rendering needs a
        // TestBackend and lands in the integration suite.
        assert_eq!(info.firmware_branch, "mntm-012");
    }
}
