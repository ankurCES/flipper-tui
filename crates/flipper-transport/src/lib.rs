//! Flipper transport layer.
//!
//! Provides the [`Transport`] trait plus two implementations:
//!
//! - [`SerialTransport`] — talks to a real Flipper Zero over its USB CDC
//!   serial endpoint using `serialport-rs` + `tokio`.
//! - [`MockTransport`] — in-memory, programmable, used by every test and
//!   the TUI's "no device" mode.
//!
//! Both expose the same `async` command surface so domain code in
//! `flipper-core` is agnostic to the wire.

#![forbid(unsafe_code)]

pub mod base;
pub mod mock;
pub mod registry;
pub mod serial;

pub use base::{CommandResult, Transport, TransportError};
pub use mock::MockTransport;
pub use registry::{detect_devices, DeviceEndpoint};
pub use serial::SerialTransport;
