//! Settings screen. Mirrors qFlipper's `HomeOverlay` `DeviceInfo` +
//! `DeviceActions` panels.
//!
//! v0.1 is display-only: shows the cached `DeviceInfo` (hardware,
//! firmware, radio, flash, API version, boot mode, serial) plus the
//! fresh `StorageInfo` snapshot for `/ext` (label, free/total bytes,
//! filesystem type) when the device responds to `storage info`. Any
//! write op (changing name, screen brightness, LED color, etc.) is a
//! destructive firmware setting and lives on the pyflipper safety
//! list — those land in v0.2 with explicit confirmation dialogs.
//!
//! The screen falls back to rendering just `DeviceInfo` if the
//! `storage info` fetch fails or returns empty, so users always see
//! *something* useful.

use flipper_core::{DeviceInfo, StorageInfo};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

#[derive(Debug)]
pub struct Settings;

impl Settings {
    pub fn new() -> Self {
        Self
    }

    /// Render the settings screen. `storage` is `Some` when the live
    /// `storage info /ext` snapshot loaded cleanly; `None` when the
    /// fetch failed or returned empty.
    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        info: &DeviceInfo,
        storage: Option<&StorageInfo>,
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
            Span::styled("settings", Style::default().fg(Color::Gray)),
        ]))
        .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(header, chunks[0]);

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        frame.render_widget(device_panel(info), cols[0]);
        frame.render_widget(storage_panel(storage), cols[1]);

        let footer = Paragraph::new(Line::from(vec![
            Span::styled("r", Style::default().fg(Color::Cyan)),
            Span::raw(" refresh  "),
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

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

fn device_panel(info: &DeviceInfo) -> Paragraph<'_> {
    let mut lines: Vec<Line> = Vec::new();
    push_field(&mut lines, "Name", &info.hardware.name);
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
    push_field(
        &mut lines,
        "API",
        &format!("{}.{}", info.api_major, info.api_minor),
    );
    push_field(&mut lines, "Boot", &format!("{:?}", info.boot_mode));
    push_field(&mut lines, "Flash", &info.flash.vendor);
    push_field(&mut lines, "Flash Model", &info.flash.model);
    push_field(
        &mut lines,
        "Flash Size",
        &format!("{} KiB", info.flash.size_bytes),
    );
    Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" device "))
}

fn storage_panel(storage: Option<&StorageInfo>) -> Paragraph<'_> {
    let mut lines: Vec<Line> = Vec::new();
    match storage {
        Some(s) => {
            push_field(&mut lines, "Path", &s.path);
            push_field(&mut lines, "Label", &s.label);
            push_field(&mut lines, "Type", &s.fs_type);
            push_field(&mut lines, "Free", &human_bytes(s.free_bytes));
            push_field(&mut lines, "Total", &human_bytes(s.total_bytes));
            let used = s.total_bytes.saturating_sub(s.free_bytes);
            push_field(&mut lines, "Used", &human_bytes(used));
            if let Some(pct) = used
                .checked_mul(100)
                .and_then(|n| n.checked_div(s.total_bytes))
            {
                lines.push(Line::from(Span::styled(
                    format!("  {pct}% used"),
                    Style::default().fg(Color::Yellow),
                )));
            }
        }
        None => {
            lines.push(Line::from(Span::styled(
                "  (storage info unavailable — try `r` to refresh)",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }
    Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" storage "))
}

fn push_field(lines: &mut Vec<Line<'_>>, key: &str, value: &str) {
    lines.push(Line::from(vec![
        Span::styled(format!("{key:<14} "), Style::default().fg(Color::DarkGray)),
        Span::raw(value.to_string()),
    ]));
}

/// Render a byte count using the same `B`/`KiB`/`MiB`/`GiB` ladder
/// the Storage screen uses — keeps the size column visually
/// consistent across screens.
fn human_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * 1024 * 1024;
    if bytes >= GIB {
        format!("{} GiB", bytes / GIB)
    } else if bytes >= MIB {
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
    use flipper_core::{BootMode, FlashInfo, HardwareInfo, RadioInfo};

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

    fn sample_storage() -> StorageInfo {
        StorageInfo {
            label: "FLIPPER".into(),
            free_bytes: 100 * 1024 * 1024,
            total_bytes: 500 * 1024 * 1024,
            fs_type: "FAT".into(),
            path: "/ext".into(),
        }
    }

    #[test]
    fn snapshot_settings_with_storage() {
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

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let settings = Settings::new();
        let info = sample_info();
        let storage = sample_storage();
        terminal
            .draw(|f| settings.render(f, Rect::new(0, 0, 80, 24), &info, Some(&storage)))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let text = collect_text(&buf, Rect::new(0, 0, 80, 24));

        // Header + both panel titles.
        assert!(
            text.contains("flipper-tui"),
            "missing 'flipper-tui':\n{text}"
        );
        assert!(
            text.contains("settings"),
            "missing 'settings' header:\n{text}"
        );
        assert!(
            text.contains("device"),
            "missing 'device' panel title:\n{text}"
        );
        assert!(
            text.contains("storage"),
            "missing 'storage' panel title:\n{text}"
        );
        // Device panel rows. Note: the Settings panel uses
        // `Name` as the first row key (not `Hardware` — that's
        // reserved for the Dashboard screen) followed by Revision,
        // Region, Lot, Serial, Firmware family, then API/Boot/
        // Flash family.
        assert!(
            text.contains("Name"),
            "missing 'Name' row in device panel:\n{text}"
        );
        assert!(
            text.contains("Revision"),
            "missing 'Revision' row in device panel:\n{text}"
        );
        assert!(
            text.contains("R3llow4n"),
            "missing 'R3llow4n' value in device panel:\n{text}"
        );
        assert!(text.contains("API"), "missing 'API' row:\n{text}");
        assert!(text.contains("87.1"), "missing API '87.1':\n{text}");
        // Storage panel rows (since Some is passed).
        assert!(text.contains("Label"), "missing 'Label' row:\n{text}");
        assert!(text.contains("FLIPPER"), "missing 'FLIPPER' label:\n{text}");
        assert!(text.contains("Total"), "missing 'Total' row:\n{text}");
        // Footer.
        assert!(
            text.contains("refresh"),
            "footer missing 'refresh':\n{text}"
        );
        assert!(text.contains("back"), "footer missing 'back':\n{text}");
        assert!(text.contains("quit"), "footer missing 'quit':\n{text}");
    }

    #[test]
    fn snapshot_settings_without_storage() {
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

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let settings = Settings::new();
        let info = sample_info();
        terminal
            .draw(|f| settings.render(f, Rect::new(0, 0, 80, 24), &info, None))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let text = collect_text(&buf, Rect::new(0, 0, 80, 24));

        // Storage panel falls back to this exact literal hint.
        assert!(
            text.contains("storage info unavailable"),
            "missing fallback hint 'storage info unavailable':\n{text}"
        );
        // Footer still present.
        assert!(
            text.contains("refresh"),
            "footer missing 'refresh' in fallback state:\n{text}"
        );
    }
    #[test]
    fn human_bytes_renders_known_units() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(2048), "2 KiB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5 MiB");
        assert_eq!(human_bytes(2 * 1024 * 1024 * 1024), "2 GiB");
    }
}
