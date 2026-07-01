//! Firmware update state — v0.1 scaffold.
//!
//! The Momentum firmware's ASCII CLI bridge does NOT implement a
//! `firmware update` verb — firmware updates are a protobuf RPC
//! operation that qFlipper issues over the same serial endpoint.
//! v0.1 of `flipper-tui` therefore surfaces only the *installed*
//! firmware metadata (branch + version, parsed from the boot banner
//! in `flipper_core::info`) and a placeholder update state. Any
//! actual update / restore / repair flow is on the pyflipper safety
//! list (destructive, can wipe user data) and is gated behind
//! explicit user confirmation when it lands in v0.2.
//!
//! This module's job is to define the typed shape so the TUI screen
//! has something to render today and v0.2 can swap the protobuf RPC
//! in without touching the screen code.

use flipper_transport::Transport;

use crate::exceptions::FlipperError;
use crate::info::Info;

/// Where the firmware-upgrade flow currently stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateState {
    /// The TUI hasn't asked yet — the screen shows "Press c to check".
    Unknown,
    /// The CLI bridge doesn't speak protobuf RPC for updates. The TUI
    /// surfaces this so the user knows the screen is scaffolded but
    /// the live firmware check isn't wired up.
    NotSupported,
    /// A check is in flight (v0.2 — unused in v0.1).
    Checking,
    /// The user is up to date.
    NoUpdates,
    /// An update is available. v0.2 will populate `target_version`
    /// from the qFlipper update channel / latest-firmware API.
    UpdateAvailable {
        /// Branch the available update belongs to (`release`, `dev`,
        /// `rc`, custom Momentum channel, etc.).
        branch: String,
        /// Target version string for display in the Updates panel.
        target_version: String,
    },
    /// The transport returned an error or the protobuf RPC signaled
    /// a failure. `message` is the human-readable explanation.
    Error(String),
}

/// Snapshot the Updates screen renders. Combines the cached
/// `DeviceInfo` (so the user always sees what's installed) with a
/// `state` field that v0.2 will populate from the RPC channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateStatus {
    /// Currently-installed firmware metadata (from `flipper_core::info`).
    pub installed: Info,
    /// Current update-check state.
    pub state: UpdateState,
}

impl UpdateStatus {
    /// Build an `UpdateStatus` from the cached firmware version and
    /// a state. Use [`UpdateStatus::unsupported`] for the v0.1
    /// default — the ASCII CLI bridge doesn't speak update RPC.
    pub fn new(installed: Info, state: UpdateState) -> Self {
        Self { installed, state }
    }

    /// Convenience: build the v0.1 default. The screen renders this
    /// until v0.2 wires the protobuf RPC.
    pub fn unsupported(installed: Info) -> Self {
        Self::new(installed, UpdateState::NotSupported)
    }
}

/// One-shot update check. v0.1 issues a `firmware update check` to
/// the bridge purely so the screen has a transport round-trip to
/// call when the user presses `c`. v0.2 will replace this with the
/// protobuf RPC and surface a real [`UpdateState`].
pub async fn check<T: Transport + ?Sized>(tx: &T) -> Result<UpdateState, FlipperError> {
    let _ = tx.send("firmware update check", &[]).await?;
    // The ASCII CLI bridge does not respond to `firmware update`
    // with anything the v0.1 parser understands. v0.2 swaps in the
    // protobuf RPC and populates `UpdateState::UpdateAvailable`
    // from the channel response.
    Ok(UpdateState::NotSupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_info() -> Info {
        Info {
            firmware_version: "mntm-012 e1784e74".into(),
            firmware_branch: "mntm-012".into(),
            firmware_commit: "e1784e74".into(),
            firmware_build_date: "31-12-2025".into(),
        }
    }

    #[test]
    fn unsupported_status_holds_installed_metadata() {
        let status = UpdateStatus::unsupported(sample_info());
        assert_eq!(status.installed.firmware_branch, "mntm-012");
        assert_eq!(status.installed.firmware_commit, "e1784e74");
        assert_eq!(status.state, UpdateState::NotSupported);
    }

    #[test]
    fn new_status_with_explicit_state() {
        let info = sample_info();
        let status = UpdateStatus::new(info.clone(), UpdateState::NoUpdates);
        assert_eq!(status.state, UpdateState::NoUpdates);
        assert_eq!(status.installed, info);
    }

    #[test]
    fn update_available_carries_branch_and_version() {
        let info = sample_info();
        let status = UpdateStatus::new(
            info,
            UpdateState::UpdateAvailable {
                branch: "release".into(),
                target_version: "Momentum v1.5.0 JAN 2026".into(),
            },
        );
        match status.state {
            UpdateState::UpdateAvailable {
                branch,
                target_version,
            } => {
                assert_eq!(branch, "release");
                assert_eq!(target_version, "Momentum v1.5.0 JAN 2026");
            }
            other => panic!("expected UpdateAvailable, got {other:?}"),
        }
    }

    #[test]
    fn error_state_carries_message() {
        let info = sample_info();
        let status = UpdateStatus::new(info, UpdateState::Error("network down".into()));
        match status.state {
            UpdateState::Error(msg) => assert_eq!(msg, "network down"),
            other => panic!("expected Error, got {other:?}"),
        }
    }
}
