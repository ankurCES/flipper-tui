//! Low-level RPC helpers.
//!
//! v0.1 implements the ASCII bridge that the Flipper's CLI exposes
//! (`device_info`, `storage list`, `storage read`, etc.). v0.2 will
//! replace the framing with the protobuf RPC protocol qFlipper uses.

use bytes::Bytes;
use flipper_transport::Transport;

use crate::device::DeviceInfo;
use crate::exceptions::FlipperError;
use crate::storage::FileStat;

/// One entry from `storage list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageEntry {
    /// Always-present.
    pub name: String,
    /// `true` if the entry is a directory (`-` flag in the Flipper CLI).
    pub is_dir: bool,
    /// Size in bytes for files, `0` for directories.
    pub size: u64,
}

fn flipper_line_to_entry(line: &str) -> Option<StorageEntry> {
    // The Flipper CLI prints lines like:
    //   "        [D] ext         4096"
    //   "        [F] Manifest       24"
    // We don't care about column alignment — we just match the leading
    // flag, name, and trailing size.
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (flag, rest) = if let Some(rest) = trimmed.strip_prefix("[D]") {
        (true, rest.trim_start())
    } else if let Some(rest) = trimmed.strip_prefix("[F]") {
        (false, rest.trim_start())
    } else {
        return None;
    };
    // Last whitespace-separated token is the size (or `-`).
    let mut parts = rest.split_whitespace();
    let name = parts.next()?.to_string();
    let last = parts.next_back().unwrap_or("0");
    let size = last.parse::<u64>().unwrap_or(0);
    Some(StorageEntry {
        name,
        is_dir: flag,
        size,
    })
}

/// Parse the multi-line output of `storage list <path>`.
pub fn parse_storage_list(payload: &[u8]) -> Result<Vec<StorageEntry>, FlipperError> {
    let text = std::str::from_utf8(payload)
        .map_err(|e| FlipperError::Parse(format!("storage list not utf-8: {e}")))?;
    Ok(text.lines().filter_map(flipper_line_to_entry).collect())
}

/// Send `device_info` and parse the response into a [`DeviceInfo`].
pub async fn hello<T: Transport + ?Sized>(tx: &T) -> Result<DeviceInfo, FlipperError> {
    let result = tx.send("device_info", &[]).await?;
    let text = std::str::from_utf8(&result.response)
        .map_err(|e| FlipperError::Parse(format!("device_info not utf-8: {e}")))?;
    DeviceInfo::parse(text)
}

/// Read a file and return the bytes. Caller is responsible for picking a
/// reasonable max size — the Flipper will happily stream multi-MB files.
pub async fn read_file<T: Transport + ?Sized>(tx: &T, path: &str) -> Result<Bytes, FlipperError> {
    let result = tx.send("storage read", &[path]).await?;
    Ok(result.response)
}

/// Stat a file or directory. Returns `(size_bytes, is_dir)`.
pub async fn stat<T: Transport + ?Sized>(tx: &T, path: &str) -> Result<FileStat, FlipperError> {
    crate::storage::stat_file(tx, path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
        [D] ext         4096\n\
        [F] Manifest       24\n\
        [F] nfc          128\n";

    #[test]
    fn parse_storage_list_extracts_dirs_and_files() {
        let entries = parse_storage_list(SAMPLE.as_bytes()).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].name, "ext");
        assert_eq!(entries[0].size, 4096);
        assert!(!entries[1].is_dir);
        assert_eq!(entries[1].name, "Manifest");
        assert_eq!(entries[1].size, 24);
    }

    #[test]
    fn parse_storage_list_ignores_blank_lines() {
        let entries = parse_storage_list(b"\n\n[D] ext  0\n").unwrap();
        assert_eq!(entries.len(), 1);
    }
}
