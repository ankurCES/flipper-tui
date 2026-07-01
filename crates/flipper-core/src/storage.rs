//! Storage operations against the Flipper's `/ext` filesystem via the
//! CLI bridge.

use bytes::Bytes;
use flipper_transport::Transport;

use crate::exceptions::FlipperError;

/// Optional flags for [`stat_file`].
#[derive(Debug, Clone, Copy, Default)]
pub struct StatFlags {
    pub follow_symlinks: bool,
}

/// Result of `stat_file`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStat {
    pub path: String,
    pub size_bytes: u64,
    pub is_dir: bool,
}

/// `storage stat <path>`.
pub async fn stat_file<T: Transport + ?Sized>(
    tx: &T,
    path: &str,
) -> Result<FileStat, FlipperError> {
    let result = tx.send("storage stat", &[path]).await?;
    let text = std::str::from_utf8(&result.response)
        .map_err(|e| FlipperError::Parse(format!("storage stat not utf-8: {e}")))?;
    Ok(parse_stat(text, path))
}

fn parse_stat(text: &str, default_path: &str) -> FileStat {
    let mut size: u64 = 0;
    let mut is_dir = false;
    for line in text.lines() {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        match k.trim() {
            "size" => size = v.trim().parse().unwrap_or(0),
            "type" => is_dir = v.trim().eq_ignore_ascii_case("dir"),
            _ => {}
        }
    }
    FileStat {
        path: default_path.to_string(),
        size_bytes: size,
        is_dir,
    }
}

/// `storage mkdir <path>`.
pub async fn mkdir<T: Transport + ?Sized>(tx: &T, path: &str) -> Result<(), FlipperError> {
    tx.send("storage mkdir", &[path]).await?;
    Ok(())
}

/// `storage read <path>` — full file contents.
pub async fn read_file<T: Transport + ?Sized>(tx: &T, path: &str) -> Result<Bytes, FlipperError> {
    let result = tx.send("storage read", &[path]).await?;
    Ok(result.response)
}

/// `storage write <path> <hex-data>` — v0.1 takes hex via the CLI.
/// v0.2 swaps this for the streaming RPC write verb.
pub async fn write_file<T: Transport + ?Sized>(
    tx: &T,
    path: &str,
    data: &[u8],
) -> Result<(), FlipperError> {
    let mut hex = String::with_capacity(data.len() * 2);
    for b in data {
        use std::fmt::Write as _;
        let _ = write!(hex, "{b:02x}");
    }
    tx.send("storage write", &[path, &hex]).await?;
    Ok(())
}

/// `storage remove <path>`.
pub async fn remove<T: Transport + ?Sized>(tx: &T, path: &str) -> Result<(), FlipperError> {
    tx.send("storage remove", &[path]).await?;
    Ok(())
}

/// `storage rename <from> <to>`.
pub async fn rename<T: Transport + ?Sized>(
    tx: &T,
    from: &str,
    to: &str,
) -> Result<(), FlipperError> {
    tx.send("storage rename", &[from, to]).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flipper_transport::{CommandResult, MockTransport};

    #[tokio::test]
    async fn stat_parses_dir() {
        let tx = MockTransport::new();
        tx.on("storage stat", |_args| {
            CommandResult::ok(b"type: dir\nsize: 0\n".to_vec())
        });
        tx.connect().await.unwrap();
        let s = stat_file(&tx, "/ext").await.unwrap();
        assert!(s.is_dir);
        assert_eq!(s.size_bytes, 0);
    }

    #[tokio::test]
    async fn mkdir_calls_transport() {
        let tx = MockTransport::new();
        tx.on("storage mkdir", |_| CommandResult::ok(b"".to_vec()));
        tx.connect().await.unwrap();
        mkdir(&tx, "/ext/notes").await.unwrap();
    }

    #[tokio::test]
    async fn read_file_returns_bytes() {
        let tx = MockTransport::new();
        tx.on("storage read", |_| CommandResult::ok(b"hello".to_vec()));
        tx.connect().await.unwrap();
        let b = read_file(&tx, "/ext/Manifest").await.unwrap();
        assert_eq!(&b[..], b"hello");
    }
}
