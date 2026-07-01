# flipper-tui

> Terminal UI for the Flipper Zero — feature-parity with [qFlipper](https://github.com/flipperdevices/qFlipper), in your shell.

`flipper-tui` is a terminal-native counterpart to the official [qFlipper](https://github.com/flipperdevices/qFlipper)
desktop application, plus its bundled `qFlipper-cli`. If you ever SSH into a Pi kiosk,
maintain a Flipper from a headless box, or just live in `tmux`/`kitty`/`iTerm2` and
don't want to launch a Qt GUI, this is for you.

It ships two entry points:

- `flipper-tui` — interactive Textual UI: device picker, dashboard, firmware
  install, repair, screen stream + remote control, file manager, backup & restore.
- `flipper-tui-cli` — non-interactive CLI that mirrors `qFlipper-cli`'s surface
  (`info`, `ping`, `backup`, `restore`, `install`, `storage`, `update`, `repair`)
  for scripting and CI.

GPL-3.0, derived from qFlipper.

## Feature parity vs qFlipper

| qFlipper capability                          | flipper-tui status                                          |
|----------------------------------------------|-------------------------------------------------------------|
| Auto-detect Flipper on USB                   | ✅ Device picker with hotplug                              |
| Firmware update (one button)                 | ✅ Firmware screen with progress + log stream              |
| Repair (DFU)                                 | ⚠️  CLI surface stubbed — full libusb DFU ships in 1.1     |
| Stream LCD + remote control                  | ✅ Screen stream widget + on-screen D-pad palette          |
| Install `.dfu`                               | ✅ DFU install dialog + `flipper-tui-cli install`          |
| Backup internal storage                      | ✅ Tar.gz backup via the `backup` CLI + TUI wizard         |
| Restore internal storage                     | ✅ Same                                                    |
| Self-update                                  | ⚠️  Planned for 1.1                                        |
| CLI (`qFlipper-cli`)                         | ✅ `flipper-tui-cli`                                       |
| Translation framework / Qt translations      | ❌ English-only TUI                                        |
| GUI installer / AppImage / DMG / NSIS        | ❌ Use `pipx install flipper-tui` or `uv tool install`      |

## Install

```bash
pipx install flipper-tui       # isolated env, exposes both scripts
# or
pip install --user flipper-tui
# or, for development:
git clone https://github.com/ankurCES/flipper-tui
cd flipper-tui
pip install -e ".[test,dev]"
```

## Quickstart

```bash
flipper-tui-cli info          # print device info, autodetects /dev/ttyACM* or cu.usbmodem*
flipper-tui-cli ping          # liveness check
flipper-tui-cli update        # fetch + install latest release firmware
flipper-tui                   # launch the TUI
```

## TUI keymap

| Key            | Action                                |
|----------------|---------------------------------------|
| `q` / `Ctrl-C` | Quit (confirm if a job is running)   |
| `:`            | Command palette                       |
| `?`            | Help                                  |
| `1..9`         | Jump to numbered dashboard section   |
| `u`            | Update firmware                       |
| `f`            | Install from `.dfu`                   |
| `R`            | Repair (DFU flow)                     |
| `r`            | Refresh current screen                |
| Arrows / Enter | D-pad on the LCD stream panel        |
| `Esc`          | Cancel any modal                      |

## CLI surface

```
flipper-tui-cli info                                       # full device info
flipper-tui-cli ping                                       # liveness
flipper-tui-cli backup  <target_dir>                       # tar.gz internal storage
flipper-tui-cli restore <source>                           # restore from tar.gz
flipper-tui-cli install <file_or_dir>...                   # install to /ext
flipper-tui-cli storage list [/path]                       # list contents
flipper-tui-cli storage read <path>                        # print file to stdout
flipper-tui-cli update [--channel release|rc|dev]          # fetch + install
flipper-tui-cli repair                                     # launch interactive repair
flipper-tui-cli --device /dev/ttyACM0 ...                  # force a specific endpoint
flipper-tui-cli --version
flipper-tui-cli --help
```

## Configuration

Config is read from the platform user-data dir (macOS: `~/Library/Application
Support/flipper-tui/`; Linux: `$XDG_CONFIG_HOME/flipper-tui/` or
`~/.config/flipper-tui/`; Windows: `%APPDATA%/flipper-tui/`):

- `preferences.json` — update channel, log level, theme, keymap override.
- `cache/firmware/` — content-addressed firmware blobs (by sha256).

## Development

```bash
git clone https://github.com/ankurCES/flipper-tui
cd flipper-tui
pip install -e ".[test,dev]"
ruff check .
ruff format .
pytest                          # hermetic — MockTransport, no device needed
pytest -m live                  # requires FLIPPER_TUI_DEVICE=/dev/ttyACM0
```

CI runs lint + matrix-tested pytest on Ubuntu, macOS, and Windows with Python 3.12.

### Architecture

Three layers — each is fully testable in isolation:

```
transport/    pure async I/O. No domain knowledge. Provides MockTransport for tests.
flipper/      domain logic over a Transport. Knows about DeviceInfo, storage, firmware.
              Returns dataclasses, raises typed errors. No Textual.
tui/          the only layer that imports Textual. Screens subscribe to events
              emitted by the lower layers.
```

See `docs/superpowers/specs/2026-07-01-flipper-tui-design.md` for the design spec
and `docs/superpowers/plans/2026-07-01-flipper-tui-plan.md` for the implementation plan.

## License

GPL-3.0-or-later, derived from [qFlipper](https://github.com/flipperdevices/qFlipper) © Flipper Devices Inc.
