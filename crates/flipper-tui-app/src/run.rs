//! TUI event loop — owns the local screen state, drives the active
//! screen's render, and dispatches key events.

use std::error::Error;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use flipper_core::DeviceInfo;
use ratatui::backend::Backend;
use ratatui::widgets::ListState;
use ratatui::Terminal;

use crate::screens::{Dashboard, Devices, Help};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Devices,
    Dashboard,
    Help,
}

struct State {
    screen: Screen,
    info: DeviceInfo,
    devices: Vec<String>,
    list: ListState,
}

impl State {
    fn new(info: DeviceInfo) -> Self {
        Self {
            screen: Screen::Devices,
            info,
            devices: Vec::new(),
            list: ListState::default(),
        }
    }

    fn render<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), Box<dyn Error>> {
        terminal.draw(|f| match self.screen {
            Screen::Devices => {
                Devices::new().render(f, f.area(), &self.devices, &mut self.list);
            }
            Screen::Dashboard => {
                Dashboard::new().render(f, f.area(), &self.info);
            }
            Screen::Help => {
                Help::new().render(f, f.area());
            }
        })?;
        Ok(())
    }

    fn on_key(&mut self, key: KeyEvent) {
        match (key.code, self.screen) {
            (KeyCode::Char('q'), _) => std::process::exit(0),
            (KeyCode::Char('?') | KeyCode::Esc, Screen::Help) => {
                self.screen = Screen::Dashboard;
            }
            (KeyCode::Char('?'), Screen::Dashboard | Screen::Devices) => {
                self.screen = Screen::Help;
            }
            (KeyCode::Esc, Screen::Dashboard) => self.screen = Screen::Devices,
            (KeyCode::Enter, Screen::Devices) => self.screen = Screen::Dashboard,
            (KeyCode::Char('r'), Screen::Devices) => {
                self.devices = flipper_transport::detect_devices()
                    .into_iter()
                    .map(|d| d.path)
                    .collect();
            }
            _ => {}
        }
    }
}

/// Run the TUI event loop until the user quits (`q`) or `Esc`s out.
pub async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    info: DeviceInfo,
) -> Result<(), Box<dyn Error>> {
    let mut state = State::new(info);
    loop {
        state.render(terminal)?;
        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    state.on_key(k);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_starts_on_devices() {
        let info = flipper_core::DeviceInfo {
            hardware: flipper_core::HardwareInfo {
                name: "f7".into(),
                revision: "R3llow4n".into(),
                region: "US".into(),
                lot: "2024-Q3-19".into(),
            },
            firmware_branch: "mntm-012".into(),
            firmware_version: "Momentum v1.4.4 OCT 2024".into(),
            firmware_build: "4106".into(),
            firmware_target: "f7".into(),
            radio: flipper_core::RadioInfo {
                ble_mac: "AA:BB:CC:DD:EE:FF".into(),
                subghz: true,
                nfc: true,
                ir: true,
            },
            flash: flipper_core::FlashInfo {
                vendor: "Winbond".into(),
                model: "W25Q128".into(),
                size_bytes: 16384,
            },
            api_major: 87,
            api_minor: 1,
            boot_mode: flipper_core::BootMode::Normal,
            serial: "flip_R3llow4n1".into(),
        };
        let s = State::new(info);
        assert_eq!(s.screen, Screen::Devices);
    }
}
