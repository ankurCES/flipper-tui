//! Ratatui-based TUI screens for the Flipper.
//!
//! Mirrors the qFlipper webapp's nav tree 1:1:
//!
//! - Devices (picker)
//! - Dashboard (overview)
//! - Storage (list/read/write/mkdir/rename/remove)
//! - Apps
//! - NFC
//! - Sub-GHz
//! - IR
//! - GPIO
//! - `BadUSB`
//! - Settings
//! - Updates
//! - Help
//!
//! v0.1 lands Devices + Dashboard + Storage + Settings + Updates + Help.
//! NFC / Sub-GHz / IR / GPIO / `BadUSB` follow in v0.2 with their
//! dedicated screens + qFlipper-style safety gates.

#![forbid(unsafe_code)]

pub mod keymap;
pub mod run;
pub mod screens;

pub use keymap::{Binding, ClickType, Keymap};
pub use run::{run, Screen};
