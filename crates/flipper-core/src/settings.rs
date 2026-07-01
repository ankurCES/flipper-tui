//! Device-side settings snapshot.
//!
//! qFlipper surfaces the SD-card and device settings in its Home tab
//! (Firmware / Build Date / SD Card / Databases / Hardware / Radio FW
//! columns). The Momentum firmware's ASCII CLI bridge does not expose
//! a dedicated `settings` verb, but `storage info <path>` is the
//! closest v0.1 equivalent: it returns a key/value dump describing the
//! underlying storage volume (label, free bytes, total bytes, type).
//!
//! v0.1 of `flipper-tui` reads only — flipping any setting is a
//! destructive op on the Flipper's eMMC/SD metadata and lives on the
//! pyflipper safety list, so it stays out of scope until v0.2's
//! confirmation gates land.

use flipper_transport::Transport;

use crate::exceptions::FlipperError;

/// Snapshot of the SD card / `/ext` volume. Field names mirror what
/// the Flipper CLI prints under `storage info` so the parser can
/// stay line-oriented (one `key: value` pair per line).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StorageInfo {
    /// Volume label (`Label:`), empty string if the firmware omits it.
    pub label: String,
    /// Free bytes (`Free:`), `0` if unknown.
    pub free_bytes: u64,
    /// Total bytes (`Total:`), `0` if unknown.
    pub total_bytes: u64,
    /// Volume type (`Type:`, e.g. `FAT`, `LFS`, `LFS2`). Empty if unknown.
    pub fs_type: String,
    /// `Storage:` / `Path:` line — the path the firmware was queried on.
    pub path: String,
}

/// Parse the multi-line output of `storage info <path>`. Tolerant of
/// missing fields (returns `Default::default()` for absent keys) —
/// missing key=value pairs don't fail the whole snapshot.
pub fn parse_storage_info(payload: &str, default_path: &str) -> StorageInfo {
    let mut info = StorageInfo {
        path: default_path.to_string(),
        ..Default::default()
    };
    for line in payload.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "Label" => info.label = value.to_string(),
            "Type" => info.fs_type = value.to_string(),
            "Storage" | "Path" => info.path = value.to_string(),
            // Sizes may arrive as `1234k`, `1234 kB`, or plain `1234`.
            "Free" => info.free_bytes = parse_size(value),
            "Total" => info.total_bytes = parse_size(value),
            _ => {}
        }
    }
    info
}

/// Fetch a fresh `StorageInfo` snapshot from the device. Returns
/// `Err(Transport)` if the bridge rejects `storage info` outright;
/// returns `Ok(default)` if the bridge returns an empty payload
/// (the Momentum CLI occasionally does this on cold-start — the
/// TUI falls back to the cached `DeviceInfo` in that case).
pub async fn storage_info<T: Transport + ?Sized>(
    tx: &T,
    path: &str,
) -> Result<StorageInfo, FlipperError> {
    let result = tx.send("storage info", &[path]).await?;
    let text = std::str::from_utf8(&result.response)
        .map_err(|e| FlipperError::Parse(format!("storage info not utf-8: {e}")))?;
    Ok(parse_storage_info(text, path))
}

fn parse_size(raw: &str) -> u64 {
    // Strip a trailing unit suffix (`k` / `K` / `kB` / `KB`) and
    // multiply by the implied scale. The Flipper CLI prints volumes
    // in kB so we default to that scale.
    let trimmed = raw.trim();
    let (digits, scale) = if let Some(rest) = trimmed
        .strip_suffix('k')
        .or_else(|| trimmed.strip_suffix('K'))
    {
        (rest.trim(), 1024u64)
    } else if let Some(rest) = trimmed
        .strip_suffix("kB")
        .or_else(|| trimmed.strip_suffix("KB"))
        .or_else(|| trimmed.strip_suffix(" KiB"))
    {
        (rest.trim(), 1024u64)
    } else if let Some(rest) = trimmed
        .strip_suffix("MiB")
        .or_else(|| trimmed.strip_suffix("MB"))
    {
        (rest.trim(), 1024u64 * 1024)
    } else {
        (trimmed, 1u64)
    };
    digits.parse::<u64>().unwrap_or(0).saturating_mul(scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flipper_transport::{CommandResult, MockTransport};

    #[test]
    fn parse_storage_info_extracts_label_and_sizes() {
        let payload = "\
Storage: /ext\n\
Label:   FLIPPER\n\
Type:    FAT\n\
Free:    1234567k\n\
Total:   7890MiB\n";
        let info = parse_storage_info(payload, "/ext");
        assert_eq!(info.label, "FLIPPER");
        assert_eq!(info.fs_type, "FAT");
        assert_eq!(info.path, "/ext");
        // 1234567 * 1024 = 1264177408
        assert_eq!(info.free_bytes, 1_234_567 * 1024);
        // 7890 * 1024 * 1024
        assert_eq!(info.total_bytes, 7890 * 1_024 * 1_024);
    }

    #[test]
    fn parse_storage_info_defaults_missing_fields() {
        let info = parse_storage_info("Path: /int\n", "/int");
        assert_eq!(info.path, "/int");
        assert_eq!(info.label, "");
        assert_eq!(info.fs_type, "");
        assert_eq!(info.free_bytes, 0);
        assert_eq!(info.total_bytes, 0);
    }

    #[test]
    fn parse_storage_info_handles_plain_byte_counts() {
        // No unit suffix — treat as bytes.
        let info = parse_storage_info("Free: 4096\nTotal: 8192\n", "/ext");
        assert_eq!(info.free_bytes, 4096);
        assert_eq!(info.total_bytes, 8192);
    }

    #[tokio::test]
    async fn storage_info_round_trips_through_mock_transport() {
        let tx = MockTransport::new();
        tx.on("storage info", |args| {
            let path = args.first().copied().unwrap_or("/ext");
            let payload =
                format!("Storage: {path}\nLabel: MOCK\nType: LFS2\nFree: 100k\nTotal: 200k\n");
            CommandResult::ok(payload.into_bytes())
        });
        tx.connect().await.unwrap();
        let info = storage_info(&tx, "/ext").await.unwrap();
        assert_eq!(info.path, "/ext");
        assert_eq!(info.label, "MOCK");
        assert_eq!(info.fs_type, "LFS2");
        assert_eq!(info.free_bytes, 100 * 1024);
        assert_eq!(info.total_bytes, 200 * 1024);
    }
}
