//! Backup request + status.
//!
//! v0.1 stub: the Momentum firmware's ASCII CLI bridge does not
//! implement a `backup` verb — backups are a protobuf RPC operation
//! that qFlipper issues over the same serial endpoint. v0.1 of
//! `flipper-tui` issues a `backup <dest>` command so the TUI has a
//! typed shape to render, and returns `BackupStatus::Pending`. v0.2
//! will switch to the protobuf RPC channel and surface real
//! progress events.

use flipper_transport::Transport;

use crate::exceptions::FlipperError;

/// Where a backup is in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupState {
    /// The TUI asked the device to start a backup; the firmware has
    /// not yet confirmed completion (v0.2 will surface a real
    /// progress event).
    Pending,
    /// The firmware reported the backup finished cleanly.
    Complete,
    /// The firmware rejected the backup or the transport failed.
    Failed,
}

/// Snapshot of a backup request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupStatus {
    pub state: BackupState,
    pub destination: String,
}

/// Request a backup to `dest`. v0.1 returns `Pending` after issuing
/// the command; v0.2 will poll the RPC channel for the real status.
pub async fn request_backup<T: Transport + ?Sized>(
    tx: &T,
    dest: &str,
) -> Result<BackupStatus, FlipperError> {
    let r = tx.send("backup", &[dest]).await?;
    // The ASCII bridge replies with `ok\n` (or `ok` followed by the
    // `> ` prompt) for verbs it doesn't implement but accepts —
    // that's the closest v0.1 signal we have for "the firmware
    // didn't reject this outright". Match a leading `ok` token so a
    // reply like `nok: out of space` isn't mistaken for success.
    let text = std::str::from_utf8(&r.response).unwrap_or("");
    let state = if text
        .split_whitespace()
        .next()
        .is_some_and(|tok| tok == "ok")
    {
        BackupState::Pending
    } else {
        BackupState::Failed
    };
    Ok(BackupStatus {
        state,
        destination: dest.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use flipper_transport::mock::MockTransport;

    #[tokio::test]
    async fn request_backup_returns_pending_on_ok() {
        let tx = MockTransport::new();
        tx.on("backup", |_args| {
            flipper_transport::CommandResult::ok(b"ok".to_vec())
        });
        tx.connect().await.unwrap();
        let s = request_backup(&tx, "/ext/backups").await.unwrap();
        assert_eq!(s.state, BackupState::Pending);
        assert_eq!(s.destination, "/ext/backups");
    }

    #[tokio::test]
    async fn request_backup_returns_failed_when_device_rejects() {
        let tx = MockTransport::new();
        tx.on("backup", |_args| {
            flipper_transport::CommandResult::ok(b"nok: out of space".to_vec())
        });
        tx.connect().await.unwrap();
        let s = request_backup(&tx, "/ext/backups").await.unwrap();
        assert_eq!(s.state, BackupState::Failed);
    }

    #[tokio::test]
    async fn request_backup_surfaces_transport_error() {
        let tx = MockTransport::new();
        // No handler registered -> MockUnhandled -> TransportError ->
        // FlipperError::Transport.
        tx.connect().await.unwrap();
        let err = request_backup(&tx, "/ext/backups").await.unwrap_err();
        assert!(matches!(err, FlipperError::Transport(_)));
    }
}
