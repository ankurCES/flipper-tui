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

use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use flipper_transport::{MockTransport, SerialTransport, Transport};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tracing_subscriber::EnvFilter;

type Term = Terminal<CrosstermBackend<Stdout>>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal).await;
    teardown_terminal(&mut terminal).ok();
    result
}

async fn run(terminal: &mut Term) -> Result<(), Box<dyn Error>> {
    let endpoints = flipper_transport::detect_devices();
    let chosen = endpoints.into_iter().next();

    let info = if let Some(dev) = chosen {
        let tx = SerialTransport::new(dev.path.clone(), 115_200);
        tx.connect()
            .await
            .map_err(|e| format!("connect serial: {e}"))?;
        flipper_core::hello(&tx)
            .await
            .map_err(|e| format!("device_info: {e}"))?
    } else {
        // Offline mode — seed the dashboard from a fixed fixture so
        // demos and CI runs still render something useful.
        let tx = MockTransport::new();
        tx.on("device_info", |_| {
            flipper_transport::CommandResult::ok(MOMENTUM_FIXTURE.as_bytes().to_vec())
        });
        tx.connect().await?;
        flipper_core::hello(&tx).await?
    };

    flipper_tui_app::run(terminal, info).await
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
