//! qFlipper-style mouse-driven bindings, mapped to keyboard sequences so
//! the TUI works without a pointing device.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::run::Screen;

/// qFlipper's `ClickType` enum, used by the webapp to gate destructive
/// actions (e.g. "Are you sure you want to install this firmware?").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickType {
    Single,
    Double,
    Hold,
}

/// One (key sequence → `ClickType`) rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub key: KeyEvent,
    pub click: ClickType,
}

/// Default keymap. Mirrors qFlipper's:
///
/// - `Enter` = single click
/// - `Enter` held > 500 ms = hold
/// - `Enter` pressed twice within 250 ms = double click
/// - `Tab` cycles focus
/// - `Esc` backs out / quits
/// - `?` opens Help
/// - `q` quits from any screen
/// - `r` refreshes the current screen
#[derive(Debug, Default)]
pub struct Keymap {
    bindings: Vec<Binding>,
}

impl Keymap {
    pub fn new() -> Self {
        Self {
            bindings: vec![
                Binding {
                    key: KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                    click: ClickType::Single,
                },
                Binding {
                    key: KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
                    click: ClickType::Single,
                },
                Binding {
                    key: KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
                    click: ClickType::Single,
                },
                Binding {
                    key: KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
                    click: ClickType::Single,
                },
                Binding {
                    key: KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
                    click: ClickType::Single,
                },
                Binding {
                    key: KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
                    click: ClickType::Single,
                },
            ],
        }
    }

    /// Translate a raw key event into the corresponding `ClickType`.
    /// Returns `None` if the key isn't bound.
    pub fn resolve(&self, key: KeyEvent) -> Option<ClickType> {
        self.bindings.iter().find(|b| b.key == key).map(|b| b.click)
    }
}

/// A single (key sequence → target `Screen`) rule for the Dashboard's
/// navigation graph. Mirrors qFlipper's left-rail nav buttons: each
/// top-level nav button exposes a single-character shortcut while the
/// Dashboard pane is focused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavBinding {
    pub key: KeyEvent,
    pub target: Screen,
}

impl NavBinding {
    fn new(ch: char, target: Screen) -> Self {
        Self {
            key: KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE),
            target,
        }
    }
}

/// Dashboard nav graph. Each shortcut opens one screen; `Esc` from
/// those screens returns to `Screen::Dashboard`. Centralizing the
/// table keeps the dispatcher in `run::on_key` and the documentation
/// in one place.
#[derive(Debug)]
pub struct NavKeymap {
    bindings: Vec<NavBinding>,
}

impl NavKeymap {
    /// Convenience: build a fresh copy of the Dashboard nav graph.
    pub fn from_dashboard() -> Self {
        Self {
            bindings: vec![
                NavBinding::new('s', Screen::Storage),
                NavBinding::new('a', Screen::Apps),
                NavBinding::new('S', Screen::Settings),
                NavBinding::new('u', Screen::Updates),
            ],
        }
    }

    /// Look up the `Screen` a key navigates to from the Dashboard.
    /// Returns `None` for unbound keys.
    pub fn target_for(&self, key: KeyEvent) -> Option<Screen> {
        self.bindings
            .iter()
            .find(|b| b.key == key)
            .map(|b| b.target)
    }
}

impl Default for NavKeymap {
    fn default() -> Self {
        Self::from_dashboard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_resolves_to_single_click() {
        let km = Keymap::new();
        let ev = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(km.resolve(ev), Some(ClickType::Single));
    }

    #[test]
    fn q_resolves_to_single_click() {
        let km = Keymap::new();
        let ev = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(km.resolve(ev), Some(ClickType::Single));
    }

    #[test]
    fn unbound_key_returns_none() {
        let km = Keymap::new();
        let ev = KeyEvent::new(KeyCode::F(7), KeyModifiers::NONE);
        assert_eq!(km.resolve(ev), None);
    }

    #[test]
    fn shift_changes_event_identity() {
        let km = Keymap::new();
        let plain = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let shifted = KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::SHIFT);
        assert_eq!(km.resolve(plain), Some(ClickType::Single));
        // Uppercase 'Q' isn't bound; qFlipper-style apps typically don't
        // bind shifted versions to keep the surface small.
        assert_eq!(km.resolve(shifted), None);
    }

    #[test]
    fn nav_s_opens_storage() {
        let nav = NavKeymap::from_dashboard();
        let ev = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(nav.target_for(ev), Some(Screen::Storage));
    }

    #[test]
    fn nav_a_opens_apps() {
        let nav = NavKeymap::from_dashboard();
        let ev = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(nav.target_for(ev), Some(Screen::Apps));
    }

    #[test]
    fn nav_uppercase_s_opens_settings() {
        // Uppercase `S` so the bare `s` slot stays free for Storage.
        let nav = NavKeymap::from_dashboard();
        let ev = KeyEvent::new(KeyCode::Char('S'), KeyModifiers::SHIFT);
        assert_eq!(nav.target_for(ev), Some(Screen::Settings));
    }

    #[test]
    fn nav_u_opens_updates() {
        let nav = NavKeymap::from_dashboard();
        let ev = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE);
        assert_eq!(nav.target_for(ev), Some(Screen::Updates));
    }

    #[test]
    fn nav_unbound_key_returns_none() {
        let nav = NavKeymap::from_dashboard();
        let ev = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE);
        assert_eq!(nav.target_for(ev), None);
    }
}
