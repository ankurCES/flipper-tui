//! `flipper-tui` — interactive TUI binary.
//!
//! On launch:
//!   1. Enumerate Flipper endpoints via `flipper_transport::detect_devices`.
//!   2. Open a `SerialTransport` to the first detected endpoint, fall back
//!      to `MockTransport` for offline demos.
//!   3. Send `device_info` to populate the dashboard.
//!   4. Hand off to `flipper_tui_app::run` which owns the event loop.

use std::error::Error;
use std::io::{stdout, Stdout};

use clap::Parser;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use flipper_transport::{MockTransport, SerialTransport, Transport};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tracing_subscriber::EnvFilter;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Interactive TUI for the Flipper Zero — at-par with qFlipper.
///
/// On launch:
///   1. Enumerate Flipper endpoints via `flipper_transport::detect_devices`.
///   2. Open a `SerialTransport` to the first detected endpoint, fall back
///      to `MockTransport` for offline demos.
///   3. Send `device_info` to populate the dashboard.
///   4. Hand off to `flipper_tui_app::run` which owns the event loop.
#[derive(Debug, Parser)]
#[command(name = "flipper-tui", version = VERSION, about = "Terminal UI for the Flipper Zero", long_about = None)]
struct Cli {
    /// Specific serial port to use (e.g. `/dev/tty.usbmodemflip_R3llow4n1`).
    /// If omitted, the first detected endpoint is used.
    #[arg(long)]
    device: Option<String>,

    /// Baud rate for the serial transport. Default: 115200.
    #[arg(long, default_value_t = 115_200)]
    baud: u32,
}

type Term = Terminal<CrosstermBackend<Stdout>>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal, &cli).await;
    teardown_terminal(&mut terminal).ok();
    result
}

async fn run(terminal: &mut Term, cli: &Cli) -> Result<(), Box<dyn Error>> {
    let endpoints = flipper_transport::detect_devices();
    let chosen: Option<flipper_transport::DeviceEndpoint> = if let Some(path) = cli.device.as_ref()
    {
        endpoints.into_iter().find(|d| &d.path == path).or_else(|| {
            // User-specified path not in the detected list — assume
            // it's a Flipper and use the canonical STMicro VID with
            // the normal-mode PID. The serial layer will surface a
            // real error if it can't open.
            Some(flipper_transport::DeviceEndpoint {
                path: path.clone(),
                vid: 0x0483,
                pid: 0x5740,
                label: None,
            })
        })
    } else {
        endpoints.into_iter().next()
    };

    let mut tx: Box<dyn Transport> = if let Some(dev) = chosen {
        let t = SerialTransport::new(dev.path.clone(), cli.baud);
        t.connect()
            .await
            .map_err(|e| format!("connect serial: {e}"))?;
        Box::new(t)
    } else {
        // Offline mode — seed the dashboard from a fixed fixture so
        // demos and CI runs still render something useful.
        let t = MockTransport::new();
        t.on("device_info", |_| {
            flipper_transport::CommandResult::ok(MOMENTUM_FIXTURE.as_bytes().to_vec())
        });
        t.on("storage list", |args| {
            // Echo the requested path back inside the listing so the
            // mock device behavior matches what the CLI bridge returns.
            let path = args.first().copied().unwrap_or("/ext");
            let listing = format!(
                "        [D] ext         4096\n\
                 \x20       [F] Manifest       24\n\
                 \x20       [F] {path}\n"
            );
            flipper_transport::CommandResult::ok(listing.into_bytes())
        });
        t.connect().await?;
        Box::new(t)
    };

    let info = flipper_core::hello(&*tx)
        .await
        .map_err(|e| format!("device_info: {e}"))?;

    flipper_tui_app::run(terminal, info, tx.as_mut()).await
}

const MOMENTUM_FIXTURE: &str = "\
hardware_name: f7\n\
hardware_revision: R3llow4n\n\
hardware_region: US\n\
hardware_lot: 2024-Q3-19\n\
firmware_branch: mntm-012\n\
firmware_version: Momentum v1.4.4 OCT 2024\n\
firmware_build: 4106\n\
firmware_target: f7\n\
radio_ble_mac: AA:BB:CC:DD:EE:FF\n\
radio_subghz: true\n\
radio_nfc: true\n\
radio_ir: true\n\
flash_vendor: Winbond\n\
flash_model: W25Q128\n\
flash_size: 16384 kB\n\
api_major: 87\n\
api_minor: 1\n\
boot_mode: Normal\n\
serial_number: flip_R3llow4n1\n";

fn setup_terminal() -> Result<Term, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    Ok(Terminal::new(backend)?)
}

fn teardown_terminal(term: &mut Term) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    term.show_cursor()?;
    Ok(())
}
