# flipper-tui

> Terminal UI for the Flipper Zero — feature-parity with [qFlipper](https://github.com/flipperdevices/qFlipper), in your shell.

`flipper-tui` is a terminal-native counterpart to the official [qFlipper](https://github.com/flipperdevices/qFlipper)
desktop application, plus its bundled `qFlipper-cli`. If you ever SSH into a Pi kiosk,
maintain a Flipper from a headless box, or just live in `tmux`/`kitty`/`iTerm2` and
don't want to launch a Qt GUI, this is for you.

It ships two entry points:

- `flipper-tui` — interactive Ratatui UI: device picker, dashboard, storage manager,
  update channel, settings, help. Mouse-free navigation via qFlipper-style bindings.
- `flipper-tui-cli` — non-interactive CLI that mirrors `qFlipper-cli`'s surface
  (`info`, `ping`, `storage`, `backup`, `restore`, `update`) for scripting and CI.

GPL-3.0-or-later, derived from qFlipper.

## Feature parity vs qFlipper

| qFlipper capability                | flipper-tui status                                       |
|------------------------------------|----------------------------------------------------------|
| Auto-detect Flipper on USB         | ✅ Device picker with `r` rescan                          |
| Device info / dashboard            | ✅ Two-column hardware + radio panel                     |
| Storage list / read / stat         | ✅ CLI + TUI navigation                                  |
| Storage mkdir / rename / remove    | ✅ CLI (TUI lands in 0.2)                                |
| Backup internal storage            | ⚠️  Stub in 0.1 — full tar.gz ships in 0.2               |
| Restore internal storage           | ⚠️  Stub + safety-gated prompt                           |
| Firmware update channel            | ⚠️  Channel query stubbed; installer lands in 0.2        |
| Repair (DFU)                       | ❌ Out of scope for 0.1; protobuf RPC lands in 0.2        |
| Stream LCD + remote control        | ❌ Out of scope for 0.1 (qFlipper uses protobuf frames)  |
| CLI (`qFlipper-cli`)               | ✅ `flipper-tui-cli`                                     |
| GUI installer / DMG / NSIS / AppImage | ❌ `cargo install flipper-tui` instead                  |

## Install

### One line (macOS, Linux, WSL)

```bash
curl -fsSL https://raw.githubusercontent.com/ankurCES/flipper-tui/main/scripts/install.sh | bash
```

Windows + PowerShell:

```powershell
irm https://raw.githubusercontent.com/ankurCES/flipper-tui/main/scripts/install.ps1 | iex
```

The script installs Rust via `rustup` if `cargo` isn't already on `$PATH`,
then `cargo install --git https://github.com/ankurCES/flipper-tui --locked`,
and verifies both `flipper-tui` and `flipper-tui-cli` are on `$PATH` with a
`--version` smoke test. See [`scripts/README.md`](scripts/README.md) for
override flags (`FLIPPER_TUI_REF`, `FLIPPER_TUI_REPO`, `FLIPPER_TUI_NO_RUSTUP`)
and the same details for Windows.

### Manual

If you'd rather drive `cargo` yourself:

```bash
cargo install --git https://github.com/ankurCES/flipper-tui --locked
# exposes both `flipper-tui` and `flipper-tui-cli` on $PATH
```

For development:

```bash
git clone https://github.com/ankurCES/flipper-tui
cd flipper-tui
cargo build --release
```

## Quickstart

```bash
flipper-tui-cli info                # print device info, autodetects cu.usbmodem* / ttyACM*
flipper-tui-cli ping                # liveness check
flipper-tui-cli storage list /ext   # list a directory on the device
flipper-tui-cli storage read /ext/Manifest
flipper-tui                         # launch the TUI
```

## TUI keymap

| Key         | Action                                |
|-------------|---------------------------------------|
| `q`         | Quit                                  |
| `?`         | Toggle Help                           |
| `Esc`       | Back / cancel                         |
| `Tab`       | Cycle focus                           |
| `Enter`     | Activate (qFlipper single-click)      |
| `r`         | Refresh / rescan devices              |
| `↑` / `↓`   | Move selection                        |

## CLI surface

```
flipper-tui-cli info                                       # full device info
flipper-tui-cli ping                                       # liveness
flipper-tui-cli storage list <path>                        # list contents
flipper-tui-cli storage read <path>                        # print file to stdout
flipper-tui-cli storage stat <path>                        # file/dir metadata
flipper-tui-cli storage mkdir <path>                       # confirm-then-create
flipper-tui-cli backup  <out.tar.gz>                       # full backup
flipper-tui-cli restore <in.tar.gz>                        # restore from tar.gz
flipper-tui-cli update check                               # query update channel
flipper-tui-cli --device /dev/tty.usbmodemflip_R3llow4n1   # force a specific endpoint
flipper-tui-cli --channel release|dev|custom               # update channel
flipper-tui-cli --version
flipper-tui-cli --help
```

## Development

```bash
git clone https://github.com/ankurCES/flipper-tui
cd flipper-tui
cargo build                              # compile everything
cargo test                               # hermetic — MockTransport, no device needed
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

CI runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test` on Ubuntu, macOS, and Windows with the stable toolchain. The matrix
is pinned by `dtolnay/rust-toolchain@stable`.

### Architecture

Three crates — each is fully testable in isolation:

```
crates/flipper-transport/    pure async I/O. No domain knowledge.
                             Provides SerialTransport (real device) and
                             MockTransport (tests, offline TUI runs).

crates/flipper-core/         domain logic over a Transport. Knows about
                             DeviceInfo, storage, backup. Returns typed
                             structs, raises typed errors. No Ratatui.

crates/flipper-tui-app/     the only crate that imports Ratatui.
                             Screens subscribe to events emitted by the
                             lower layers; keymap mirrors qFlipper's
                             ClickType → keyboard mapping.
```

See `docs/superpowers/specs/2026-06-26-flipper-tui-design.md` for the design spec
and `docs/superpowers/plans/2026-07-01-flipper-tui-plan.md` for the implementation plan.

## License

GPL-3.0-or-later, derived from [qFlipper](https://github.com/flipperdevices/qFlipper) © Flipper Devices Inc.