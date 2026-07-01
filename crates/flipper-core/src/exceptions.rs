//! Typed Flipper protocol errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FlipperError {
    #[error("transport error: {0}")]
    Transport(#[from] flipper_transport::TransportError),
    #[error("protocol parse error: {0}")]
    Parse(String),
    #[error("device rejected request: {0}")]
    Rejected(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("device is not connected; call hello() first")]
    NotConnected,
    #[error("unsupported operation on this firmware: {0}")]
    Unsupported(&'static str),
}
