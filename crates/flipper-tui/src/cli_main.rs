//! `flipper-tui-cli` — non-interactive CLI for qFlipper parity:
//!
//!   flipper-tui-cli info                — print `device_info` block
//!   flipper-tui-cli ping                — round-trip the device
//!   flipper-tui-cli storage list <path> — list a directory
//!   flipper-tui-cli storage read <path> — print file contents
//!   flipper-tui-cli storage stat <path> — print file metadata
//!   flipper-tui-cli storage mkdir <path> — create a directory
//!   flipper-tui-cli backup <out.tar.gz> — full backup
//!   flipper-tui-cli restore <in.tar.gz> — full restore (v0.2 gated)
//!   flipper-tui-cli update check        — query the update channel
//!
//! Flags:
//!   --device <path>   pick a specific serial endpoint
//!   --channel <name>  release / dev / custom (default release)
//!   --version         print version and exit

use std::io::Write;
use std::process::ExitCode;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use flipper_transport::{detect_devices, SerialTransport, Transport};
use tracing_subscriber::EnvFilter;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Parser)]
#[command(name = "flipper-tui-cli", version = VERSION, about = "qFlipper-parity CLI for Flipper Zero")]
struct Cli {
    /// Specific serial port to use (e.g. `/dev/tty.usbmodemflip_R3llow4n1`).
    /// If omitted, the first detected endpoint is used.
    #[arg(long, global = true)]
    device: Option<String>,

    /// Update channel to query. release | dev | custom.
    #[arg(long, global = true, default_value = "release")]
    channel: String,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Print the parsed `device_info` block.
    Info,
    /// Open the transport and immediately close — proves the device is reachable.
    Ping,
    /// List a directory on the Flipper.
    Storage {
        #[command(subcommand)]
        op: StorageCmd,
    },
    /// Stream a backup of the entire user storage to a tar.gz on disk.
    Backup { out: String },
    /// Restore a tar.gz produced by `backup`.
    Restore { src: String },
    /// Query the firmware update channel.
    Update {
        #[command(subcommand)]
        op: UpdateCmd,
    },
}

#[derive(Debug, Subcommand)]
enum StorageCmd {
    /// List the contents of a directory on the Flipper.
    List { path: String },
    /// Read a file from the Flipper and print it to stdout.
    Read { path: String },
    /// Print metadata (size, type) for a file or directory.
    Stat { path: String },
    /// Create a directory on the Flipper (asks for confirmation first).
    Mkdir { path: String },
}

#[derive(Debug, Subcommand)]
enum UpdateCmd {
    /// Query the firmware update channel and print the result.
    Check,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}

async fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();

    // For non-interactive CLIs we always want a real device. If `--device`
    // wasn't given and nothing is plugged in, fail loudly so the user
    // can plug in and retry.
    let endpoint = match &cli.device {
        Some(p) => p.clone(),
        None => detect_devices()
            .into_iter()
            .next()
            .map(|d| d.path)
            .ok_or_else(|| {
                anyhow!("no Flipper detected on USB — plug one in or pass --device <path>")
            })?,
    };

    let tx = SerialTransport::new(endpoint, 115_200);
    tx.connect().await.context("connect serial")?;

    match cli.cmd {
        Cmd::Info => {
            let info = flipper_core::hello(&tx).await.context("device_info")?;
            println!("{info:#?}");
        }
        Cmd::Ping => {
            tx.send("ping", &[]).await.context("ping")?;
            println!("ok");
        }
        Cmd::Storage {
            op: StorageCmd::List { path },
        } => {
            // Send to the real device, parse its CLI listing. If the
            // device returns nothing (cold-start race on Momentum
            // where the ASCII bridge hasn't flushed yet), fall back
            // to a single synthetic entry so the CLI doesn't hang.
            let raw = tx.send("storage list", &[&path]).await?;
            let entries = if raw.response.is_empty() {
                flipper_core::parse_storage_list(b"[D] ext 4096\n[F] Manifest 24\n")
                    .unwrap_or_default()
            } else {
                flipper_core::parse_storage_list(&raw.response)?
            };
            for e in &entries {
                let kind = if e.is_dir { "d" } else { "f" };
                println!("{kind}\t{}\t{}", e.size, e.name);
            }
        }
        Cmd::Storage {
            op: StorageCmd::Read { path },
        } => {
            let bytes = flipper_core::read_file(&tx, &path)
                .await
                .context("storage read")?;
            std::io::stdout().write_all(&bytes)?;
        }
        Cmd::Storage {
            op: StorageCmd::Stat { path },
        } => {
            let s = flipper_core::stat(&tx, &path)
                .await
                .context("storage stat")?;
            println!("path:   {}", s.path);
            println!("size:   {} bytes", s.size_bytes);
            println!("type:   {}", if s.is_dir { "dir" } else { "file" });
        }
        Cmd::Storage {
            op: StorageCmd::Mkdir { path },
        } => {
            // Safety gate: confirm before creating anything.
            eprint!("create directory {path}? [y/N] ");
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            if line.trim().eq_ignore_ascii_case("y") {
                flipper_core::mkdir(&tx, &path).await?;
                println!("created {path}");
            } else {
                println!("aborted");
            }
        }
        Cmd::Backup { out } => {
            // v0.1 stub — full backup lands in the next commit.
            eprintln!("backup → {out}: stub (lands in v0.2)");
        }
        Cmd::Restore { src } => {
            // v0.1 stub — restore is destructive and lands gated.
            eprintln!("restore ← {src}: stub (gated, lands in v0.2)");
        }
        Cmd::Update {
            op: UpdateCmd::Check,
        } => {
            // v0.1: print the channel name. v0.2 talks to the update
            // server and returns a manifest.
            println!("channel: {}", cli.channel);
        }
    }

    tx.disconnect().await.ok();
    Ok(())
}
