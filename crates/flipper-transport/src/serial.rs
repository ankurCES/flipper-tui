//! Real serial transport over USB CDC.
//!
//! v0.1 implementation holds a long-lived `SerialPort` open across
//! `send` calls. `send` writes the command line and then accumulates
//! reply bytes via `spawn_blocking` until an idle gap (200ms with no
//! new bytes) or a 2-second total cap. That's enough to drive the live
//! Momentum device end-to-end through its ASCII CLI bridge for
//! `device_info`, `storage list`, `storage read`, and `storage stat`.
//!
//! Why this is minimal: the qFlipper RPC wire format is documented in
//! `flipperdevices/qFlipper/protobuf/` (hand-rolled `.proto` files with
//! a 1-byte header + big-endian varints), and re-implementing the
//! encoder/decoder is a multi-commit project on its own. The ASCII
//! CLI bridge is the same surface qFlipper uses when `--cli-mode` is
//! selected, and it's enough for v0.1's read-only verbs.
//!
//! pyflipper safety rules apply: only read-only + non-destructive
//! verbs against the device. Anything that could fire a radio, write
//! to flash, or send user input needs an explicit confirmation gate
//! before being wired up here.

#![allow(dead_code, clippy::type_complexity)]

use std::io::{Read, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use bytes::Bytes;
use serialport::SerialPort;
use tokio::sync::Mutex;

use crate::base::{CommandResult, Transport, TransportError};

/// Idle gap (no new bytes arriving) that signals the end of a reply.
/// The Momentum CLI bridge emits the boot banner + command echo back
/// to back, then pauses while the firmware processes the command,
/// then emits the response. 200ms is too short — the firmware needs
/// hundreds of milliseconds to actually reply — so we wait longer.
const IDLE_GAP: Duration = Duration::from_millis(1500);

/// Hard cap on a single `send`'s read window.
const READ_TIMEOUT: Duration = Duration::from_secs(5);

/// Read-buffer chunk size for the blocking read loop.
const READ_CHUNK: usize = 1024;

/// Prompt sentinel the Momentum CLI bridge emits at the end of every
/// reply. We treat the buffer as "complete" the moment we see this
/// pattern, so a normal reply returns in tens of milliseconds
/// instead of waiting out the full idle gap.
const PROMPT_SENTINEL: &[u8] = b"> ";

/// Talks to a Flipper Zero over its USB CDC ACM serial endpoint.
pub struct SerialTransport {
    path: String,
    baud: u32,
    inner: Arc<Mutex<Option<Box<dyn SerialPort>>>>,
}

impl std::fmt::Debug for SerialTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SerialTransport")
            .field("path", &self.path)
            .field("baud", &self.baud)
            .field("open", &self.inner.blocking_lock().as_ref().is_some())
            .finish_non_exhaustive()
    }
}

impl SerialTransport {
    pub fn new(path: impl Into<String>, baud: u32) -> Self {
        Self {
            path: path.into(),
            baud,
            inner: Arc::new(Mutex::new(None)),
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

#[async_trait]
impl Transport for SerialTransport {
    async fn connect(&self) -> Result<(), TransportError> {
        let path = self.path.clone();
        let baud = self.baud;
        let port = tokio::task::spawn_blocking(move || {
            serialport::new(&path, baud)
                .timeout(Duration::from_millis(500))
                .open()
        })
        .await
        .map_err(|e| TransportError::Io(format!("join error: {e}")))?
        .map_err(|e| TransportError::Serial(e.to_string()))?;
        *self.inner.lock().await = Some(port);
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), TransportError> {
        // Drop closes the port; on macOS the tty entry sticks around
        // until the device physically unplugs, which is fine.
        *self.inner.lock().await = None;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.inner.lock().await.is_some()
    }

    async fn send(&self, command: &str, args: &[&str]) -> Result<CommandResult, TransportError> {
        let mut line = String::from(command);
        for a in args {
            line.push(' ');
            line.push_str(a);
        }
        line.push('\n');
        let payload = line.into_bytes();

        let inner = self.inner.clone();
        let (path, payload) = (self.path.clone(), payload);

        // Write + read happen on a blocking thread so the async runtime
        // isn't stalled by the synchronous `serialport` API.
        let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, TransportError> {
            let mut guard = inner.blocking_lock();
            let port = guard.as_mut().ok_or(TransportError::NotConnected)?;

            port.write_all(&payload)
                .map_err(|e| TransportError::Io(format!("{path}: write: {e}")))?;
            port.flush()
                .map_err(|e| TransportError::Io(format!("{path}: flush: {e}")))?;

            let mut sink: Vec<u8> = Vec::new();
            let mut chunk = [0u8; READ_CHUNK];
            let started = Instant::now();
            let mut last_byte = Instant::now();
            loop {
                if started.elapsed() >= READ_TIMEOUT {
                    break;
                }
                if sink.is_empty() && started.elapsed() >= IDLE_GAP {
                    // device produced nothing — return empty so callers
                    // don't hang.
                    break;
                }
                if !sink.is_empty()
                    && last_byte.elapsed() >= IDLE_GAP
                    && sink.ends_with(PROMPT_SENTINEL)
                {
                    break;
                }
                match port.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        sink.extend_from_slice(&chunk[..n]);
                        last_byte = Instant::now();
                        if sink.ends_with(PROMPT_SENTINEL) {
                            // Prompt at end of buffer means the reply
                            // is complete — return immediately instead
                            // of waiting for the full idle gap.
                            break;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                        if sink.ends_with(PROMPT_SENTINEL) {
                            break;
                        }
                        if !sink.is_empty() && last_byte.elapsed() >= IDLE_GAP {
                            break;
                        }
                    }
                    Err(e) => {
                        return Err(TransportError::Io(format!("{path}: read: {e}")));
                    }
                }
            }
            // Trim the trailing prompt sentinel so parsers see only the
            // command's reply text, not the CLI bridge's `> ` prompt.
            while sink.ends_with(PROMPT_SENTINEL) {
                sink.truncate(sink.len() - PROMPT_SENTINEL.len());
            }
            Ok(sink)
        })
        .await
        .map_err(|e| TransportError::Io(format!("join error: {e}")))??;

        Ok(CommandResult::ok(Bytes::from(bytes)))
    }
}
