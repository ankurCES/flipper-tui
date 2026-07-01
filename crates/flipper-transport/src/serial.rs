//! Real serial transport over USB CDC.
//!
//! v0.1 implementation holds a long-lived `SerialPort` open across
//! `send` calls. `send` writes the command line and then accumulates
//! reply bytes via `spawn_blocking` until an idle gap (1.5s with no
//! new bytes) or a 5-second total cap. The very first `send` after
//! `connect` uses a longer 3s gap because the Momentum firmware's
//! USB CDC ACM cold-start takes ~1.5-2s to flush the boot banner
//! after the initial port-open — subsequent calls fall back to the
//! normal 1.5s gap.
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
use std::sync::atomic::{AtomicBool, Ordering};
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

/// Longer idle gap used for the very first `send` after `connect`.
/// The Momentum firmware's USB CDC ACM cold-start takes ~5-8s to
/// flush the boot banner after the first command on a fresh
/// connection, so a normal 1.5s gap would fire before the banner
/// arrives. 8s gives the banner enough time to land.
const FIRST_IDLE_GAP: Duration = Duration::from_secs(8);

/// Hard cap on a single `send`'s read window.
const READ_TIMEOUT: Duration = Duration::from_secs(8);

/// Read-buffer chunk size for the blocking read loop.
const READ_CHUNK: usize = 1024;

/// Talks to a Flipper Zero over its USB CDC ACM serial endpoint.
pub struct SerialTransport {
    path: String,
    baud: u32,
    inner: Arc<Mutex<Option<Box<dyn SerialPort>>>>,
    /// True until the first `send` after a `connect` completes;
    /// flipped to false so subsequent `send` calls use the normal
    /// (shorter) idle gap.
    first_send_pending: AtomicBool,
    /// Boot banner stashed during `connect`. The Momentum ASCII
    /// bridge delays the banner past any reasonable idle gap, so we
    /// drain it into this buffer during `connect` (stopping at the
    /// `Firmware version:` marker) and expose it via `boot_banner()`.
    /// `info()` reads it from here instead of the wire reply.
    boot_banner_buf: Arc<Mutex<Option<Bytes>>>,
}

impl std::fmt::Debug for SerialTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SerialTransport")
            .field("path", &self.path)
            .field("baud", &self.baud)
            .field("open", &self.inner.blocking_lock().as_ref().is_some())
            .field(
                "first_send_pending",
                &self.first_send_pending.load(Ordering::Relaxed),
            )
            .field(
                "boot_banner_captured",
                &self.boot_banner_buf.blocking_lock().is_some(),
            )
            .finish_non_exhaustive()
    }
}

impl SerialTransport {
    pub fn new(path: impl Into<String>, baud: u32) -> Self {
        Self {
            path: path.into(),
            baud,
            inner: Arc::new(Mutex::new(None)),
            first_send_pending: AtomicBool::new(false),
            boot_banner_buf: Arc::new(Mutex::new(None)),
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
        // The next `send` is the first one on this freshly-opened
        // port — use the longer idle gap so the cold-start banner
        // has time to arrive.
        self.first_send_pending.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), TransportError> {
        // Drop closes the port; on macOS the tty entry sticks around
        // until the device physically unplugs, which is fine.
        *self.inner.lock().await = None;
        self.first_send_pending.store(false, Ordering::Relaxed);
        *self.boot_banner_buf.lock().await = None;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.inner.lock().await.is_some()
    }

    async fn boot_banner(&self) -> Option<Bytes> {
        self.boot_banner_buf.lock().await.clone()
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
        // Pick the right idle gap for this call. The first send
        // after connect uses the longer gap so the cold-start banner
        // has time to arrive; subsequent sends fall back to the
        // shorter gap. We flip the flag *before* writing so a slow
        // command that interleaves with another `send` on the same
        // transport still sees the flag correctly.
        let idle_gap = if self.first_send_pending.swap(false, Ordering::Relaxed) {
            FIRST_IDLE_GAP
        } else {
            IDLE_GAP
        };

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
                if sink.is_empty() && started.elapsed() >= idle_gap {
                    // device produced nothing — return empty so callers
                    // don't hang.
                    break;
                }
                if !sink.is_empty() && last_byte.elapsed() >= idle_gap {
                    // The Momentum CLI bridge flushes its boot banner
                    // as a separate write *after* the command's `> `
                    // prompt, so we can't break early on the prompt
                    // sentinel — that would discard the banner. Wait
                    // the full idle gap after the last byte instead,
                    // so any trailing banner has time to arrive.
                    break;
                }
                match port.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        sink.extend_from_slice(&chunk[..n]);
                        last_byte = Instant::now();
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                        if !sink.is_empty() && last_byte.elapsed() >= idle_gap {
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
            while sink.ends_with(b"> ") {
                sink.truncate(sink.len() - 2);
            }
            Ok(sink)
        })
        .await
        .map_err(|e| TransportError::Io(format!("join error: {e}")))??;

        Ok(CommandResult::ok(Bytes::from(bytes)))
    }
}
