# flipper-tui — Design Spec (v1)

> Companion to the implementation plan. Authoritative for *what* we're building;
> the plan covers *how* and the bite-sized tasks.

## 1. Vision

A terminal-native counterpart to [qFlipper](https://github.com/flipperdevices/qFlipper)
that users run in a real shell — SSH, tmux, headless servers, CI runners — and that
exposes the same core workflows: discover your Flipper, update or repair firmware,
stream its screen, control it remotely, install a `.dfu`, back up / restore data,
and run the non-interactive operations as a CLI.

`flipper-tui` is intentionally NOT a Qt port. It's a re-thinking of the same
device-management surface in TUI primitives: panes, modals, command palettes,
keybindings, an embedded screen stream, and a non-interactive `flipper-tui-cli`
subcommand surface that mirrors qFlipper's `cli/` for the operations that make sense.

## 2. Audience & Use Cases

- **Headless / SSH users** who already maintain a Flipper Zero but can't (or
  don't want to) run a Qt GUI.
- **CI / automation authors** who need scripted firmware install, backup,
  restore, and storage operations as part of provisioning.
- **Developers inside a Terminal-driven workflow** (`tmux`, `kitty`, Terminal.app,
  iTerm2) who want the qFlipper feature set with vim-grade keybindings.
- **Operators on remote machines** (Raspberry Pi kiosks, jump hosts, pod
  containers with `usbdevice` / privileged passthrough) who only have a serial
  terminal.

## 3. Scope (v1)

### 3.1 In scope (parity with qFlipper)

| qFlipper capability                                | flipper-tui equivalent                                  |
|---------------------------------------------------|---------------------------------------------------------|
| Device registry / auto-discovery on USB insert    | ✅ Device Picker screen (`DevicesScreen`) with hotplug   |
| Firmware update (one-click)                       | ✅ Firmware screen with progress bar + log stream        |
| Repair (force-recovery over DFU)                  | ✅ Repair screen (DFU path) — mockable, real path stubbed |
| Stream the LCD + remote control                   | ✅ Screen Stream widget (16×→scaled ASCII/half-block) + input chord palette |
| Install `.dfu`                                    | ✅ DFU install dialog (CLI subcommand + TUI affordance) |
| Backup internal storage                           | ✅ Backup wizard → tar.gz to path                        |
| Restore internal storage                          | ✅ Restore wizard from tar.gz                            |
| Self-update                                       | ✅ In-app update check + auto-restart (deferred to v1.1) |
| CLI mode                                          | ✅ `flipper-tui-cli` subcommand (mirrors qFlipper-cli)   |
| Application auto-update (qFlipper Updater)        | ❌ Out of v1 (mature qFlipper feature; not core)         |
| Multi-language UI / Qt translations               | ❌ Out of v1 (TUI: English only)                         |

### 3.2 Out of scope (v1, documented in README)

- DFU firmware flashing over USB (libusb + DFU protocol — significant C/C++ work
  in qFlipper; v1 ships a CLI surface + clear `TODO` referencing the
  upstream `dfu/` library).
- Bluetooth support (qFlipper is wired-USB only; matches v1 scope).
- GUI installer, NSIS, AppImage, .dmg packaging (TUI ships a `pipx install`
  / `uv tool install` path).

## 4. Tech Stack

| Concern         | Choice           | Why                                                                                 |
|-----------------|------------------|-------------------------------------------------------------------------------------|
| Language        | **Python 3.12**  | Mature, runs anywhere qFlipper does (mac/linux/win with pyusb on win), rich TUI libs |
| TUI framework   | **Textual**      | Mouse + keyboard, async, modal screens, CSS-like styling, headless test harness — closest TUI analog of Qt's QML |
| Serial transport| **pyserial**     | Cross-platform, same upstream as pyFlipper's main path                              |
| Protobuf        | **protobuf ≥ 5** | qFlipper's protobuf-based RPC is the wire format                                     |
| DFU transport   | **pyusb + libusb1** | Cross-platform libusb wrapper for the DFU path                                   |
| Packaging       | **Hatchling**    | Modern pyproject.toml, no setup.py shim                                              |
| Tests           | **pytest + pytest-asyncio + syrupy** | Textual has a first-party `App.run_test()` harness                              |
| Lint / format   | **ruff**         | Single tool for both                                                               |
| CI              | **GitHub Actions** matrix: ubuntu-latest, macos-latest, windows-latest, Python 3.12 | Reproduces qFlipper's platform matrix |

## 5. Architecture

```
flipper-tui/
├── src/flipper_tui/
│   ├── __init__.py
│   ├── __main__.py          # `python -m flipper_tui`
│   ├── app.py               # Textual App root, global keybindings
│   ├── config.py            # preferences.json + CLI arg parsing
│   ├── cli.py               # `flipper-tui-cli` argparse surface
│   │
│   ├── transport/           # device I/O — pure async, NO Textual imports
│   │   ├── base.py          # Transport ABC, Progress / Log / Result events
│   │   ├── serial.py        # pyserial implementation (CLI protocol)
│   │   ├── dfu.py           # pyusb/libusb implementation (DFU protocol)
│   │   ├── registry.py      # USB hot-plug watcher (pyusb + poll fallback)
│   │   └── mock.py          # in-memory transport for tests
│   │
│   ├── flipper/             # domain logic over the transport
│   │   ├── device.py        # Device model (info, version, region, hardware)
│   │   ├── rpc/             # generated protobuf stubs from qFlipper's .proto
│   │   ├── firmware.py      # update / repair flows over transport
│   │   ├── storage.py       # ls / stat / read / write
│   │   ├── screen.py        # LCD stream subscription + screen_frame codec
│   │   ├── input.py         # InputEvent synthesis (press / release / chord)
│   │   ├── backup.py        # tar.gz serialize + restore
│   │   └── updates.py       # remote manifest fetch + diff + download
│   │
│   ├── tui/                 # Textual screens, widgets, keybindings
│   │   ├── screens/
│   │   │   ├── devices.py       # Device Picker
│   │   │   ├── dashboard.py     # Main dashboard
│   │   │   ├── firmware.py      # Firmware update / repair
│   │   │   ├── screen.py        # LCD stream + remote control
│   │   │   ├── storage.py       # Two-pane file manager
│   │   │   ├── backup.py        # Backup / Restore wizard
│   │   │   ├── settings.py      # Preferences
│   │   │   └── log.py           # Modal log viewer (full screen)
│   │   ├── widgets/
│   │   │   ├── flipper_screen.py # Frame buffer widget (half-block / braille)
│   │   │   ├── progress.py       # Logged progress bar
│   │   │   └── key_palette.py    # On-screen D-pad / button hint
│   │   ├── keymap.py            # Central keybinding table
│   │   └── styles/app.tcss      # Textual CSS
│   │
│   └── resources/
│       ├── brand.txt             # ASCII logo
│       └── data/                 # bundled region info, default prefs
│
├── tests/                    # mirrors src/, transport mock keeps them hermetic
├── docs/
│   ├── superpowers/
│   │   ├── specs/2026-06-26-flipper-tui-design.md  ← this file
│   │   └── plans/2026-06-26-flipper-tui-plan.md
│   └── screenshots/          # recorded Textual snapshots for the README
├── pyproject.toml
├── README.md
├── LICENSE                   # GPL-3.0
├── .github/workflows/ci.yml
└── .gitignore
```

### 5.1 Layering rules

- `transport/*` knows nothing of Textual; emits `Progress` / `LogLine` / `Result`
  events via an `asyncio.Queue` and `dataclass` payloads. Testable in pure
  pytest with `transport.mock.MockSerial`.
- `flipper/*` knows nothing of Textual either; takes a `Transport`, returns
  dataclasses / raises typed exceptions. 100% testable without a TTY.
- `tui/*` is the only layer that imports `textual`. Subscribes to transport
  events, renders widgets. Screens are independently runnable via
  `App.run_test()` for snapshot tests.

### 5.2 Wire formats

- **Normal mode**: 230400 8N1 USB-CDC serial. CLI commands, line-oriented.
- **DFU mode**: USB DFU class device. Different VID/PID, no serial.

### 5.3 CLI subcommands (mirrors qFlipper-cli)

```
flipper-tui-cli backup   <target_dir>
flipper-tui-cli restore  <source_dir>
flipper-tui-cli erase                            # factory reset
flipper-tui-cli firmware <firmware_file.dfu>
flipper-tui-cli install <file_or_dir>...         # copy file(s) to /ext
flipper-tui-cli storage list [/path]
flipper-tui-cli storage read <path>
flipper-tui-cli storage write <local> <remote>
flipper-tui-cli info                             # full device info
flipper-tui-cli ping                             # liveness check
flipper-tui-cli screen-stream                     # dump frames as PNG to stdout
flipper-tui-cli send-input <event>                # press|release <button>
flipper-tui-cli update-channel <release|rc|dev>
flipper-tui-cli self-update
flipper-tui-cli --version
flipper-tui-cli --help
```

## 6. UX / keybindings

The single most important contract — committed in v1, evolved later:

| Context     | Key           | Action                                |
|-------------|---------------|---------------------------------------|
| Global      | `q`, `Ctrl-C` | Quit (with confirm if a job is running) |
| Global      | `:`           | Command palette (fuzzy, like VS Code) |
| Global      | `?`           | Help screen                           |
| Devices     | `Enter`       | Connect / open dashboard              |
| Devices     | `r`           | Refresh                               |
| Dashboard   | `1..9`        | Jump to numbered section              |
| Firmware    | `u`           | Update                                |
| Firmware    | `f`           | Install from .dfu                     |
| Firmware    | `R`           | Repair (DFU)                          |
| Screen      | Arrows, Enter | D-pad, OK                            |
| Screen      | `b`           | Back                                 |
| Screen      | `Space`       | Hold (sends press, release on release key) |
| Storage     | `h/l`         | Toggle focus pane                     |
| Storage     | `n/p`         | Next / previous entry                 |
| Storage     | `Space`       | Multi-select                         |
| Any modal   | `Esc`         | Cancel                               |
| Any modal   | `Enter`       | Confirm                              |

Default keymap lives in `tui/keymap.py`; full rebind via `~/.config/flipper-tui/keymap.toml`.

## 7. Configuration & state

- macOS: `~/Library/Application Support/flipper-tui/`
- Linux: `~/.config/flipper-tui/`  (XDG)
- Windows: `%APPDATA%/flipper-tui/`

Files:
- `preferences.json` (update channel, log level, theme, keymap override)
- `last_device.json` (serial number / path for auto-reconnect)
- `cache/firmware/` (downloaded firmwares, content-addressed by sha256)

## 8. Testing strategy

- **Unit (pytest, hermetic)**: `transport/`, `flipper/` — drives `MockTransport`.
- **Component (Textual `App.run_test`)**: every screen under `tui/screens/`
  rendered with deterministic transports, snapshot via syrupy.
- **Live (manual, opt-in, gated by `--live` and `FLIPPER_TUI_DEVICE`)**:
  guarded by env var, skipped in CI. Documents how to run.
- **CI**: `ruff check`, `ruff format --check`, `pytest` with `--strict-markers`.
  Matrix: ubuntu-latest / macos-latest / windows-latest × Python 3.12.
  Live-tests job runs only on `main` + manual `workflow_dispatch`.

## 9. Distribution

- **PyPI**: `pip install flipper-tui` → exposes `flipper-tui` and `flipper-tui-cli`.
- **pipx / uv tool install**: recommended (isolated env, ships CLI).
- **Homebrew**: out of v1 (filed as a follow-up).
- No GUI installer needed.

## 10. Non-goals (v1)

- BLE / NFC / RFID operation (qFlipper doesn't expose these over USB either).
- Multi-device concurrent management (one TUI session = one device).
- Plugin system (qFlipper's `plugins/` is protobuf-based; deferred to v1.1).
- Translation framework.
- GUI installer / native packaging.

## 11. Open questions to resolve before plan execution

1. DFU path — ship stub-with-CLI-surface, or implement enough of the libusb
   DFU protocol to do a real repair? **Decision:** stub + CLI surface; real
   libusb DFU is a v1.1 task. The stub raises `NotImplementedError` with a
   pointer to the upstream `qFlipper/dfu/` source.
2. Protobuf stubs — check in `protoc`-generated files, or generate at
   install time? **Decision:** commit generated stubs (matches qFlipper's
   `plugins/` convention; reproducible).
3. Region-bundling — qFlipper ships a `regions.json`. **Decision:** copy from
   upstream at install time via `flipper_tui/resources/data/regions.json`
   pinned to a qFlipper release tag.
4. Bundle a real firmware blob in tests? **Decision:** no — use synthetic
   bytes with deterministic sha256; live-firmware paths in `--live` mode only.
