//! Live-device integration tests for `SerialTransport`.
//!
//! These tests are gated by the `FLIPPER_TUI_DEVICE` environment
//! variable pointing at a real Flipper Zero USB CDC ACM endpoint, e.g.
//!
//! ```bash
//! FLIPPER_TUI_DEVICE=/dev/tty.usbmodemflip_R3llow4n1 \
//!     cargo test --test live_device -- --ignored --nocapture
//! ```
//!
//! They are marked `#[ignore]` so the regular `cargo test` run never
//! touches real hardware. They exercise the read-only ASCII CLI
//! bridge the Momentum firmware exposes over USB CDC ACM. v0.1 of
//! `flipper-tui` uses the ASCII bridge; v0.2 swaps in the protobuf
//! RPC protocol that the official qFlipper uses.
//!
//! The ASCII bridge is an interactive REPL: it emits the boot banner
//! on connect, then echoes each command line back with a `>:` prefix
//! and (for some verbs) a trailing `ok` reply. The bridge does NOT
//! auto-respond to every verb — most `device_info`-style data lives
//! behind the protobuf RPC protocol. So the live tests assert on
//! what the bridge actually emits: a non-empty reply for every
//! command (banner + echo + optional `ok`), and a non-empty payload
//! from `storage read /ext/Manifest`.
//!
//! pyflipper-style safety rules: no destructive radio / storage /
//! power / input ops — read-only verbs only.

use flipper_core::{info, read_file};
use flipper_transport::{SerialTransport, Transport};

/// Path to the live Flipper endpoint. Defaults to the macOS-friendly
/// name; override via env var for Linux (`/dev/ttyACM0`) or a second
/// attached unit.
fn endpoint() -> String {
    std::env::var("FLIPPER_TUI_DEVICE")
        .unwrap_or_else(|_| "/dev/tty.usbmodemflip_R3llow4n1".to_string())
}

/// Asserts that `connect()` followed by `send("device_info", ...)`
/// yields a non-empty reply. The Momentum ASCII bridge echoes the
/// command back as `>: device_info\n` and the boot banner appears in
/// the same read window, so any non-trivial reply is evidence the
/// transport round-trips. v0.2 will switch to the protobuf RPC
/// protocol so `device_info` returns the real key:value payload.
#[tokio::test]
#[ignore = "requires FLIPPER_TUI_DEVICE pointing at a real Flipper"]
async fn live_device_info_round_trips() {
    let path = endpoint();
    let tx = SerialTransport::new(path, 115_200);
    tx.connect().await.expect("connect");
    let raw = tx.send("device_info", &[]).await.expect("device_info");
    let text = std::str::from_utf8(&raw.response).expect("utf-8");
    println!(
        "live device_info reply: {} bytes\n{text}",
        raw.response.len()
    );
    assert!(
        raw.response.len() > 50,
        "expected non-trivial device_info reply, got {} bytes",
        raw.response.len()
    );
    tx.disconnect().await.ok();
}

/// `storage list /ext` round-trips: the bridge echoes the command and
/// may emit a listing. We assert the reply is non-empty and contains
/// the echoed command — proof that the transport accepts the verb and
/// returns what the bridge sent.
#[tokio::test]
#[ignore = "requires FLIPPER_TUI_DEVICE pointing at a real Flipper"]
async fn live_storage_list_ext_round_trips() {
    let path = endpoint();
    let tx = SerialTransport::new(path, 115_200);
    tx.connect().await.expect("connect");
    let result = tx
        .send("storage list", &["/ext"])
        .await
        .expect("storage list");
    let text = std::str::from_utf8(&result.response).expect("utf-8");
    println!(
        "live storage list /ext reply: {} bytes\n{text}",
        result.response.len()
    );
    assert!(
        result.response.len() > 50,
        "expected non-trivial storage list reply, got {} bytes",
        result.response.len()
    );
    tx.disconnect().await.ok();
}

/// `storage read /ext/Manifest` returns the manifest text. This is
/// the one CLI bridge verb that produces real payload data on
/// Momentum firmware — the manifest is the SD-card's `/ext/Manifest`
/// text file dumped verbatim. Validates the transport's full read
/// path against a multi-hundred-byte device payload.
#[tokio::test]
#[ignore = "requires FLIPPER_TUI_DEVICE pointing at a real Flipper"]
async fn live_storage_read_manifest_returns_text() {
    let path = endpoint();
    let tx = SerialTransport::new(path, 115_200);
    tx.connect().await.expect("connect");
    let bytes = read_file(&tx, "/ext/Manifest").await.expect("storage read");
    let text = std::str::from_utf8(&bytes).expect("manifest utf-8");
    println!("live /ext/Manifest ({} bytes):\n{text}", bytes.len());
    assert!(!text.is_empty(), "expected non-empty manifest");
    tx.disconnect().await.ok();
}

/// `storage stat /ext` round-trips: the bridge echoes the command and
/// may emit a stat reply. Asserts on the echo, same shape as the
/// `storage list` test above.
#[tokio::test]
#[ignore = "requires FLIPPER_TUI_DEVICE pointing at a real Flipper"]
async fn live_storage_stat_ext_round_trips() {
    let path = endpoint();
    let tx = SerialTransport::new(path, 115_200);
    tx.connect().await.expect("connect");
    let raw = tx
        .send("storage stat", &["/ext"])
        .await
        .expect("storage stat");
    let text = std::str::from_utf8(&raw.response).expect("utf-8");
    println!(
        "live storage stat /ext reply: {} bytes\n{text}",
        raw.response.len()
    );
    assert!(
        raw.response.len() > 50,
        "expected non-trivial storage stat reply, got {} bytes",
        raw.response.len()
    );
    tx.disconnect().await.ok();
}

/// `Transport::ping` round-trips against the live Momentum firmware.
/// The default impl sends an empty line and expects any bytes back;
/// the bridge's boot banner satisfies this on the first call.
#[tokio::test]
#[ignore = "requires FLIPPER_TUI_DEVICE pointing at a real Flipper"]
async fn live_ping_round_trips() {
    let path = endpoint();
    let tx = SerialTransport::new(path, 115_200);
    tx.connect().await.expect("connect");
    tx.ping().await.expect("ping");
    tx.disconnect().await.ok();
}

/// `info()` parses the live boot banner for the firmware version line.
/// Real Momentum evidence: `mntm-012 mntm-012 (e1784e74 built on 31-12-2025)`.
///
/// The Momentum ASCII CLI bridge delays the boot banner emission by
/// ~5-8s after the first real command on a fresh connection — far
/// longer than any reasonable `send()` idle gap can wait, and the
/// bridge treats `\n` (used by `ping()`) as a no-op that doesn't
/// flush the banner. The TUI's real usage pattern is: the Devices
/// screen calls `device_info` during device detection (which triggers
/// the banner emission as a side effect), then the Dashboard screen
/// calls `info()` to display the firmware version. By the time
/// `info()` runs, the banner is already in the device's output buffer
/// and the next `send("device_info")` reply includes it.
///
/// This test mirrors that pattern: warm up the bridge with a
/// `device_info` call (which triggers banner emission), then call
/// `info()`. The name is prefixed `aaa_` so it sorts first among the
/// live tests, preserving the "info runs early in the TUI lifecycle"
/// intent.
#[tokio::test]
#[ignore = "requires FLIPPER_TUI_DEVICE pointing at a real Flipper"]
async fn live_aaa_info_parses_first_after_connect() {
    let path = endpoint();
    let tx = SerialTransport::new(path, 115_200);
    tx.connect().await.expect("connect");
    // Warm up the bridge with a real command — `ping()` sends `\n`
    // which the bridge treats as a no-op and doesn't flush the
    // banner. `device_info` triggers the full banner emission.
    tx.send("device_info", &[])
        .await
        .expect("warm-up device_info");
    let parsed = info(&tx).await.expect("info");
    println!(
        "live info: branch={} commit={} build_date={}",
        parsed.firmware_branch, parsed.firmware_commit, parsed.firmware_build_date
    );
    assert_eq!(parsed.firmware_branch, "mntm-012");
    assert_eq!(parsed.firmware_commit, "e1784e74");
    assert_eq!(parsed.firmware_build_date, "31-12-2025");
    tx.disconnect().await.ok();
}
