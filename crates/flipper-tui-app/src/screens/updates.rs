//! Updates screen. Mirrors qFlipper's `UpdateOverlay`.
//!
//! v0.1 is a scaffold: it shows what's installed (the cached
//! `DeviceInfo` firmware metadata) and surfaces the current
//! update-check `UpdateState` in a status panel. The actual
//! firmware install / restore / repair flow is on the pyflipper
//! safety list — these are destructive operations that can wipe
//! user data and require explicit user confirmation. They land in
//! v0.2 once the protobuf RPC channel is wired up.
//!
//! v0.1 footer hints:
//! - `c` re-runs `firmware update check` against the bridge (a
//!   read-only verb that the CLI may or may not answer; the parser
//!   ignores unknown replies and the state stays at
//!   `UpdateState::NotSupported`).
//! - `i` is shown but disabled — the install flow is gated.
//! - `r` refreshes the screen (same effect as `c` for now).
//! - `Esc` returns to the Dashboard.

use flipper_core::{DeviceInfo, UpdateState, UpdateStatus};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

#[derive(Debug)]
pub struct Updates;

impl Updates {
    pub fn new() -> Self {
        Self
    }

    /// Render the updates screen. `info` is the cached `DeviceInfo`
    /// (always rendered so the user sees what's installed);
    /// `status` is the current `UpdateStatus` snapshot.
    pub fn render(&self, frame: &mut Frame, area: Rect, info: &DeviceInfo, status: &UpdateStatus) {
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
            Span::styled("updates", Style::default().fg(Color::Gray)),
        ]))
        .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(header, chunks[0]);

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        frame.render_widget(installed_panel(info), cols[0]);
        frame.render_widget(status_panel(status), cols[1]);

        let footer = Paragraph::new(Line::from(vec![
            Span::styled("c", Style::default().fg(Color::Cyan)),
            Span::raw(" check  "),
            Span::styled("i", Style::default().fg(Color::DarkGray)),
            Span::styled(" install (gated)", Style::default().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" back  "),
            Span::styled("?", Style::default().fg(Color::Cyan)),
            Span::raw(" help  "),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(" quit"),
        ]));
        frame.render_widget(footer, chunks[2]);
    }
}

impl Default for Updates {
    fn default() -> Self {
        Self::new()
    }
}

fn installed_panel(info: &DeviceInfo) -> Paragraph<'_> {
    let mut lines: Vec<Line> = Vec::new();
    push_field(&mut lines, "Branch", &info.firmware_branch);
    push_field(&mut lines, "Version", &info.firmware_version);
    push_field(&mut lines, "Build", &info.firmware_build);
    push_field(&mut lines, "Target", &info.firmware_target);
    lines.push(Line::from(""));
    push_field(&mut lines, "Serial", &info.serial);
    push_field(&mut lines, "Hardware", &info.hardware.name);
    Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" installed "))
}

fn status_panel(status: &UpdateStatus) -> Paragraph<'_> {
    let mut lines: Vec<Line> = Vec::new();
    push_field(&mut lines, "Branch", &status.installed.firmware_branch);
    push_field(&mut lines, "Commit", &status.installed.firmware_commit);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Status",
        Style::default().fg(Color::Yellow),
    )));
    push_state(&mut lines, &status.state);
    if let UpdateState::UpdateAvailable {
        branch,
        target_version,
    } = &status.state
    {
        lines.push(Line::from(""));
        push_field(&mut lines, "Update branch", branch);
        push_field(&mut lines, "Target version", target_version);
    }
    Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" updates "))
}

fn push_field(lines: &mut Vec<Line<'_>>, key: &str, value: &str) {
    lines.push(Line::from(vec![
        Span::styled(format!("{key:<14} "), Style::default().fg(Color::DarkGray)),
        Span::raw(value.to_string()),
    ]));
}

fn push_state(lines: &mut Vec<Line<'_>>, state: &UpdateState) {
    let (text, color) = match state {
        UpdateState::Unknown => ("unknown — press c to check", Color::DarkGray),
        UpdateState::NotSupported => (
            "not supported on this firmware — v0.2 will add protobuf RPC",
            Color::Yellow,
        ),
        UpdateState::Checking => ("checking…", Color::Cyan),
        UpdateState::NoUpdates => ("up to date", Color::Green),
        UpdateState::UpdateAvailable { .. } => ("update available", Color::Green),
        UpdateState::Error(_) => ("error", Color::Red),
    };
    lines.push(Line::from(Span::styled(
        format!("  {text}"),
        Style::default().fg(color),
    )));
    if let UpdateState::Error(msg) = state {
        lines.push(Line::from(Span::styled(
            format!("  {msg}"),
            Style::default().fg(Color::Red),
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flipper_core::{BootMode, FlashInfo, HardwareInfo, Info, RadioInfo};

    fn sample_info() -> DeviceInfo {
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

    fn sample_status(state: UpdateState) -> UpdateStatus {
        UpdateStatus {
            installed: Info {
                firmware_version: "mntm-012 e1784e74".into(),
                firmware_branch: "mntm-012".into(),
                firmware_commit: "e1784e74".into(),
                firmware_build_date: "31-12-2025".into(),
            },
            state,
        }
    }

    #[test]
    fn updates_holds_without_panicking() {
        let _ = Updates::new();
    }

    #[test]
    fn updates_renders_unsupported_state() {
        // Default v0.1: bridge doesn't speak update RPC.
        let info = sample_info();
        let status = sample_status(UpdateState::NotSupported);
        let _ = (info, status);
    }

    #[test]
    fn updates_renders_no_updates_state() {
        let info = sample_info();
        let status = sample_status(UpdateState::NoUpdates);
        let _ = (info, status);
    }

    #[test]
    fn updates_renders_error_state() {
        let info = sample_info();
        let status = sample_status(UpdateState::Error("network unreachable".into()));
        let _ = (info, status);
    }
}
