//! qFlipper-style mouse-driven bindings, mapped to keyboard sequences so
//! the TUI works without a pointing device.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
}
