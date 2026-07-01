//! Transport trait + shared types.

use async_trait::async_trait;
use bytes::Bytes;
use thiserror::Error;

/// The result of a single RPC command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    /// Raw payload bytes returned by the device.
    pub response: Bytes,
    /// Optional textual status the Flipper may attach to a response.
    pub status: Option<String>,
}

impl CommandResult {
    pub fn ok(response: impl Into<Bytes>) -> Self {
        Self {
            response: response.into(),
            status: None,
        }
    }

    #[must_use]
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }
}

/// Transport-layer failures.
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("transport not connected")]
    NotConnected,
    #[error("i/o error: {0}")]
    Io(String),
    #[error("serial error: {0}")]
    Serial(String),
    #[error("command rejected by device: {0}")]
    Rejected(String),
    #[error("mock transport: no handler registered for command {0}")]
    MockUnhandled(String),
    #[error("invalid frame: {0}")]
    InvalidFrame(String),
}

/// A command channel to a Flipper Zero (real or mocked).
#[async_trait]
pub trait Transport: Send + Sync {
    /// Open the underlying channel. Idempotent.
    async fn connect(&self) -> Result<(), TransportError>;

    /// Close the underlying channel. Idempotent.
    async fn disconnect(&self) -> Result<(), TransportError>;

    /// Whether `connect` has succeeded and the channel is usable.
    async fn is_connected(&self) -> bool;

    /// Send a single command and await its reply.
    async fn send(&self, command: &str, args: &[&str]) -> Result<CommandResult, TransportError>;

    /// Lightweight liveness check. Default impl sends an empty line and
    /// returns Ok iff the device produced any bytes. Override for
    /// transports that have a cheaper way to detect liveness.
    async fn ping(&self) -> Result<(), TransportError> {
        let r = self.send("", &[]).await?;
        if r.response.is_empty() {
            Err(TransportError::Io("ping: empty reply".into()))
        } else {
            Ok(())
        }
    }

    /// Boot banner captured during `connect`, if any. Real serial
    /// transports drain the firmware's boot banner into a buffer so
    /// `info()` and the dashboard can read it after the first command
    /// has consumed the banner from the wire. Mocks return `None`.
    async fn boot_banner(&self) -> Option<Bytes> {
        None
    }
}
