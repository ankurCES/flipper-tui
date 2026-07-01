//! TUI event loop — owns the local screen state, drives the active
//! screen's render, and dispatches key events.

use std::error::Error;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use flipper_core::{parse_storage_list, DeviceInfo, StorageEntry};
use flipper_transport::Transport;
use ratatui::backend::Backend;
use ratatui::widgets::ListState;
use ratatui::Terminal;

use crate::screens::{Apps, Dashboard, Devices, Help, Storage, StorageLocation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Devices,
    Dashboard,
    Storage,
    Apps,
    Help,
}

struct State {
    screen: Screen,
    info: DeviceInfo,
    devices: Vec<String>,
    list: ListState,
    /// Where the Storage browser is currently pointed.
    storage_location: StorageLocation,
    /// Last `parse_storage_list` result for `storage_location`.
    storage_entries: Vec<StorageEntry>,
    /// True when the user pressed `r` and we need to re-fetch the
    /// current storage location. Drained at the top of the loop.
    storage_dirty: bool,
    /// Where the Apps browser is currently pointed. Independent
    /// from `storage_location` so navigating Storage doesn't lose
    /// the user's spot in Apps (and vice versa).
    apps_location: StorageLocation,
    apps_entries: Vec<StorageEntry>,
    apps_dirty: bool,
}

impl State {
    fn new(info: DeviceInfo) -> Self {
        Self {
            screen: Screen::Devices,
            info,
            devices: Vec::new(),
            list: ListState::default(),
            storage_location: StorageLocation::root(),
            storage_entries: Vec::new(),
            storage_dirty: false,
            apps_location: Apps::root(),
            apps_entries: Vec::new(),
            apps_dirty: false,
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
            Screen::Storage => {
                Storage::new().render(
                    f,
                    f.area(),
                    &self.storage_location,
                    &self.storage_entries,
                    &mut self.list,
                );
            }
            Screen::Apps => {
                Apps::new().render(
                    f,
                    f.area(),
                    &self.apps_location,
                    &self.apps_entries,
                    &mut self.list,
                );
            }
            Screen::Help => {
                Help::new().render(f, f.area());
            }
        })?;
        Ok(())
    }

    async fn refresh_storage<T: Transport + ?Sized>(
        &mut self,
        tx: &T,
    ) -> Result<(), Box<dyn Error>> {
        let result = tx
            .send("storage list", &[self.storage_location.path.as_str()])
            .await?;
        self.storage_entries = parse_storage_list(&result.response).unwrap_or_default();
        Ok(())
    }

    async fn refresh_apps<T: Transport + ?Sized>(&mut self, tx: &T) -> Result<(), Box<dyn Error>> {
        let result = tx
            .send("storage list", &[self.apps_location.path.as_str()])
            .await?;
        self.apps_entries = parse_storage_list(&result.response).unwrap_or_default();
        Ok(())
    }

    fn on_key<T: Transport + ?Sized>(&mut self, key: KeyEvent, tx: &mut T) {
        match (key.code, self.screen) {
            (KeyCode::Char('q'), _) => std::process::exit(0),
            (KeyCode::Char('?') | KeyCode::Esc, Screen::Help) => {
                self.screen = Screen::Dashboard;
            }
            (KeyCode::Char('?'), _) => {
                // `?` opens Help from any screen (Devices, Dashboard,
                // Storage, Apps). Mirrors qFlipper's global `?` binding.
                self.screen = Screen::Help;
            }
            (KeyCode::Esc, Screen::Dashboard) => self.screen = Screen::Devices,
            (KeyCode::Esc, Screen::Storage) => {
                // Pop one dir component if we can, otherwise fall back
                // to the Dashboard (qFlipper's FileManager "back"
                // gesture takes you to the previous screen at root).
                if let Some(parent) = self.storage_location.ascend() {
                    self.storage_location = parent;
                    self.storage_dirty = true;
                } else {
                    self.screen = Screen::Dashboard;
                }
            }
            (KeyCode::Esc, Screen::Apps) => {
                if let Some(parent) = self.apps_location.ascend() {
                    self.apps_location = parent;
                    self.apps_dirty = true;
                } else {
                    self.screen = Screen::Dashboard;
                }
            }
            (KeyCode::Enter, Screen::Devices) => self.screen = Screen::Dashboard,
            (KeyCode::Char('s'), Screen::Dashboard) => {
                self.screen = Screen::Storage;
                self.storage_location = StorageLocation::root();
                self.storage_dirty = true;
            }
            (KeyCode::Char('a'), Screen::Dashboard) => {
                self.screen = Screen::Apps;
                self.apps_location = Apps::root();
                self.apps_dirty = true;
            }
            (KeyCode::Enter, Screen::Storage) => {
                if let Some(idx) = self.list.selected() {
                    if let Some(entry) = self.storage_entries.get(idx) {
                        if entry.is_dir {
                            self.storage_location = self.storage_location.descend(&entry.name);
                            self.storage_dirty = true;
                        }
                        // Files: v0.1 is display-only, do nothing.
                    }
                }
            }
            (KeyCode::Enter, Screen::Apps) => {
                if let Some(idx) = self.list.selected() {
                    if let Some(entry) = self.apps_entries.get(idx) {
                        if entry.is_dir {
                            self.apps_location = self.apps_location.descend(&entry.name);
                            self.apps_dirty = true;
                        }
                        // Files: v0.1 is display-only, do nothing.
                    }
                }
            }
            (KeyCode::Char('r'), Screen::Devices) => {
                self.devices = flipper_transport::detect_devices()
                    .into_iter()
                    .map(|d| d.path)
                    .collect();
            }
            (KeyCode::Char('r'), Screen::Storage) => {
                self.storage_dirty = true;
            }
            (KeyCode::Char('r'), Screen::Apps) => {
                self.apps_dirty = true;
            }
            _ => {}
        }
        // Drain dirty flags after each key event so the next loop
        // iteration re-fetches without bouncing back into this arm.
        // Best-effort: a transport error here just means the listing
        // will be empty until the next refresh.
        if self.storage_dirty {
            self.storage_dirty = false;
            let _ = futures::executor::block_on(self.refresh_storage(tx));
        }
        if self.apps_dirty {
            self.apps_dirty = false;
            let _ = futures::executor::block_on(self.refresh_apps(tx));
        }
    }
}

/// Run the TUI event loop until the user quits (`q`) or `Esc`s out.
pub async fn run<B: Backend, T: Transport + ?Sized>(
    terminal: &mut Terminal<B>,
    info: DeviceInfo,
    tx: &mut T,
) -> Result<(), Box<dyn Error>> {
    let mut state = State::new(info);
    loop {
        state.render(terminal)?;
        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    state.on_key(k, tx);
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
            api_minor: 0,
            boot_mode: flipper_core::BootMode::Normal,
            serial: "flip_R3llow4n1".into(),
        };
        let s = State::new(info);
        assert_eq!(s.screen, Screen::Devices);
        assert_eq!(s.storage_location.path, "/ext");
        assert_eq!(s.apps_location.path, "/ext/apps");
        assert!(s.storage_entries.is_empty());
        assert!(s.apps_entries.is_empty());
        assert!(!s.storage_dirty);
        assert!(!s.apps_dirty);
    }
}
