//! Boot-banner / firmware-version parser.
//!
//! The Momentum firmware's ASCII CLI bridge auto-emits a boot banner
//! on connect that contains a `Firmware version:` line of the form
//!
//! ```text
//! Firmware version: mntm-012 mntm-012 (e1784e74 built on 31-12-2025)
//! ```
//!
//! v0.1 of `flipper-tui` parses this banner for the dashboard's
//! "Firmware" panel. v0.2 will switch to the protobuf RPC `device_info`
//! verb for richer fields (radio flags, flash metadata, etc).

use flipper_transport::Transport;

use crate::exceptions::FlipperError;

/// Fields extracted from the `Firmware version:` line of the boot banner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Info {
    /// Full version string as printed, e.g. `mntm-012 mntm-012`.
    pub firmware_version: String,
    /// Branch, e.g. `mntm-012`.
    pub firmware_branch: String,
    /// Short commit SHA, e.g. `e1784e74`.
    pub firmware_commit: String,
    /// Build date, e.g. `31-12-2025`.
    pub firmware_build_date: String,
}

impl Info {
    /// Parse the boot banner's `Firmware version:` line.
    ///
    /// Tolerates surrounding text (the banner is the full 1 KiB+ CLI
    /// bridge startup dump). Returns `Parse` if the line is missing
    /// or doesn't match the expected shape.
    pub fn parse(banner: &str) -> Result<Self, FlipperError> {
        let line = banner
            .lines()
            .map(str::trim)
            .find(|l| l.starts_with("Firmware version:"))
            .ok_or_else(|| FlipperError::Parse("boot banner missing `Firmware version:`".into()))?;
        // Strip the `Firmware version:` prefix.
        let rest = line.strip_prefix("Firmware version:").unwrap_or("").trim();
        // Shape: `<branch> <commit-ish> (<sha> built on <date>)`
        // Real example: `mntm-012 mntm-012 (e1784e74 built on 31-12-2025)`
        let (head, tail) = rest
            .split_once('(')
            .ok_or_else(|| FlipperError::Parse("firmware version missing `(`".into()))?;
        let tail = tail
            .strip_suffix(')')
            .ok_or_else(|| FlipperError::Parse("firmware version missing `)`".into()))?;
        let mut head_parts = head.split_whitespace();
        let branch = head_parts
            .next()
            .ok_or_else(|| FlipperError::Parse("firmware version missing branch".into()))?
            .to_string();
        // Skip the human-readable second token (e.g. `mntm-012`) — the
        // SHA + date live inside the parens.
        let mut sha_date = tail.split_whitespace();
        let commit = sha_date
            .next()
            .ok_or_else(|| FlipperError::Parse("firmware version missing sha".into()))?
            .to_string();
        // `built on <date>` — grab the date after the literal `on`.
        let date = tail
            .split_whitespace()
            .skip_while(|w| *w != "on")
            .nth(1)
            .ok_or_else(|| FlipperError::Parse("firmware version missing build date".into()))?
            .to_string();
        Ok(Self {
            firmware_version: format!("{branch} {commit}"),
            firmware_branch: branch,
            firmware_commit: commit,
            firmware_build_date: date,
        })
    }
}

/// Send a one-shot probe and parse the resulting boot banner for the
/// firmware version. The Momentum firmware's ASCII CLI bridge emits
/// the full boot banner (firmware version, hardware name, etc.) on
/// the very first command after connect — but only once per USB
/// session, so subsequent commands don't re-emit it. We prefer
/// `Transport::boot_banner()` when the transport has stashed it
/// (real serial drains the banner during `connect` because the
/// bridge delays it past any reasonable idle gap), and fall back to
/// `send("device_info", &[])` otherwise.
pub async fn info<T: Transport + ?Sized>(tx: &T) -> Result<Info, FlipperError> {
    if let Some(banner) = tx.boot_banner().await {
        let text = std::str::from_utf8(&banner)
            .map_err(|e| FlipperError::Parse(format!("banner not utf-8: {e}")))?;
        return Info::parse(text);
    }
    let r = tx.send("device_info", &[]).await?;
    let text = std::str::from_utf8(&r.response)
        .map_err(|e| FlipperError::Parse(format!("banner not utf-8: {e}")))?;
    Info::parse(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
\x1b[38;2;255;130;0m\n\
              _.-------.._                    -,\n\
\r\n\
\x1b[97mWelcome to Flipper Zero Command Line Interface!\n\
\r\n\
\x1b[0m\n\
Firmware version: mntm-012 mntm-012 (e1784e74 built on 31-12-2025)\n\
\r\n\
>: help\n";

    #[test]
    fn parse_extracts_firmware_version_from_banner() {
        let info = Info::parse(SAMPLE).expect("parse");
        assert_eq!(info.firmware_branch, "mntm-012");
        assert_eq!(info.firmware_commit, "e1784e74");
        assert_eq!(info.firmware_build_date, "31-12-2025");
        assert_eq!(info.firmware_version, "mntm-012 e1784e74");
    }

    #[test]
    fn parse_errors_without_firmware_line() {
        let err = Info::parse("just some text\n").unwrap_err();
        assert!(matches!(err, FlipperError::Parse(_)));
    }

    #[test]
    fn parse_errors_on_malformed_firmware_line() {
        // No parentheses — parser rejects.
        let err = Info::parse("Firmware version: mntm-012 mntm-012\n").unwrap_err();
        assert!(matches!(err, FlipperError::Parse(_)));
    }
}
