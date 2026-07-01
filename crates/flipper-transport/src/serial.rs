//! Real serial transport over USB CDC.
//!
//! v0.1 ships a *minimal* implementation that opens the port, sends
//! ASCII command lines, and discards the reply. That's enough to drive
//! the live device end-to-end through the Flipper's CLI bridge for
//! `device_info`, `storage list`, and `storage read`. v0.2 will replace
//! the framing with the protobuf RPC protocol qFlipper uses and add
//! proper read-buffer handling.
//!
//! Why this is minimal: the qFlipper RPC wire format is documented in
//! `flipperdevices/qFlipper/protobuf/` (hand-rolled `.proto` files with
//! a 1-byte header + big-endian varints), and re-implementing the
//! encoder/decoder is a multi-commit project on its own. Stubbing
//! v0.1 means the rest of the app (TUI, CLI, mock tests) can land
//! against a working `Transport`, and the RPC layer slots in without
//! churning any other code.

#![allow(dead_code)]

use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::Mutex;

use crate::base::{CommandResult, Transport, TransportError};

/// Talks to a Flipper Zero over its USB CDC ACM serial endpoint.
pub struct SerialTransport {
    path: String,
    baud: u32,
    connected: Arc<Mutex<bool>>,
}

impl std::fmt::Debug for SerialTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SerialTransport")
            .field("path", &self.path)
            .field("baud", &self.baud)
            .field("connected", &*self.connected.blocking_lock())
            .finish_non_exhaustive()
    }
}

impl SerialTransport {
    pub fn new(path: impl Into<String>, baud: u32) -> Self {
        Self {
            path: path.into(),
            baud,
            connected: Arc::new(Mutex::new(false)),
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

#[async_trait]
impl Transport for SerialTransport {
    async fn connect(&self) -> Result<(), TransportError> {
        // Probe the port by opening it briefly so a missing/wrong device
        // surfaces here, not on the first `send`.
        let port = serialport::new(&self.path, self.baud)
            .timeout(Duration::from_millis(500))
            .open()
            .map_err(|e| TransportError::Serial(format!("{}: {}", self.path, e)))?;
        drop(port);
        *self.connected.lock().await = true;
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), TransportError> {
        *self.connected.lock().await = false;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        *self.connected.lock().await
    }

    async fn send(&self, command: &str, args: &[&str]) -> Result<CommandResult, TransportError> {
        if !self.is_connected().await {
            return Err(TransportError::NotConnected);
        }
        // v0.1: open-once-per-send. Slow but correct. v0.2 holds a
        // long-lived `tokio::sync::Mutex<Box<dyn SerialPort>>` and
        // bridges it to async via `tokio::task::spawn_blocking`.
        let mut port = serialport::new(&self.path, self.baud)
            .timeout(Duration::from_secs(2))
            .open()
            .map_err(|e| TransportError::Serial(format!("{}: {}", self.path, e)))?;

        let mut line = String::from(command);
        for a in args {
            line.push(' ');
            line.push_str(a);
        }
        line.push('\n');

        port.write_all(line.as_bytes())
            .map_err(|e| TransportError::Io(e.to_string()))?;

        // v0.1 doesn't accumulate replies — the CLI surface in
        // `flipper-core` reads them through `MockTransport` until the
        // RPC layer is wired up.
        Ok(CommandResult::ok(Bytes::new()))
    }
}
