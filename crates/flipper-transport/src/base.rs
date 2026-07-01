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
}
