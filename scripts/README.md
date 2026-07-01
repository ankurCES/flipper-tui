# scripts/

Helper scripts for installing and working on `flipper-tui`.

## `install.sh`

Single-line installer for macOS, Linux, WSL.

```bash
curl -fsSL https://raw.githubusercontent.com/ankurCES/flipper-tui/main/scripts/install.sh | bash
```

The script will, in order:

1. Refuse to run if `curl` is missing.
2. Install Rust via `rustup` if `cargo` isn't on `$PATH` (override with
   `FLIPPER_TUI_NO_RUSTUP=1` to skip and demand a manual Rust install).
3. `cargo install --git https://github.com/ankurCES/flipper-tui --locked`.
4. Verify both `flipper-tui` and `flipper-tui-cli` are on `$PATH`.
5. Print a `--version` smoke test for each.

Override the install ref:

```bash
FLIPPER_TUI_REF=feat/install-sh curl -fsSL https://raw.githubusercontent.com/ankurCES/flipper-tui/main/scripts/install.sh | bash
```

## `install.ps1`

Equivalent for Windows + PowerShell.

```powershell
irm https://raw.githubusercontent.com/ankurCES/flipper-tui/main/scripts/install.ps1 | iex
```

Same env-var overrides as above (`$env:FLIPPER_TUI_REF`,
`$env:FLIPPER_TUI_REPO`).

## After install

```bash
flipper-tui-cli ping      # no-device smoke test
flipper-tui-cli --help    # full CLI surface
flipper-tui               # launch the TUI (needs a Flipper on USB)
```
