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
3. Resolve the install ref to a commit SHA via `git ls-remote ${REPO} HEAD`
   (or use the SHA / branch the user passed via `FLIPPER_TUI_REF`).
4. Run `cargo install --git ${REPO} --rev <sha> --locked`.
5. If cargo install transiently fails with `revspec '…' not found` (which
   happens when the install runs within seconds of a fresh push and the
   CDN hasn't propagated yet), re-resolve the SHA and retry once.
6. Verify both `flipper-tui` and `flipper-tui-cli` are on `$PATH`.
7. Print a `--version` smoke test for each.

### Why pin to a SHA

`cargo install --git … --rev <branch>` can race the remote's CDN right
after `git push` and fail with `revspec '…' not found`. Pinning to a
specific commit SHA removes that race entirely — SHAs always resolve.

### Overrides

| Env var              | Default                                   | Purpose                              |
|----------------------|-------------------------------------------|--------------------------------------|
| `FLIPPER_TUI_REF`    | (resolves to `${REPO}`'s `HEAD` SHA)      | branch, tag, or commit SHA to install|
| `FLIPPER_TUI_REPO`   | `https://github.com/ankurCES/flipper-tui` | source git repo                      |
| `FLIPPER_TUI_NO_RUSTUP` | `0`                                     | set to `1` to refuse the rustup boot |

Pin to a known-good SHA:

```bash
FLIPPER_TUI_REF=8983161 \
  curl -fsSL https://raw.githubusercontent.com/ankurCES/flipper-tui/main/scripts/install.sh | bash
```

## `install.ps1`

Equivalent for Windows + PowerShell.

```powershell
irm https://raw.githubusercontent.com/ankurCES/flipper-tui/main/scripts/install.ps1 | iex
```

Same env-var overrides as above (`$env:FLIPPER_TUI_REF`,
`$env:FLIPPER_TUI_REPO`).

To force a specific SHA on Windows:

```powershell
$env:FLIPPER_TUI_REF = '8983161'; irm https://raw.githubusercontent.com/ankurCES/flipper-tui/main/scripts/install.ps1 | iex
```

## After install

```bash
flipper-tui-cli ping      # no-device smoke test
flipper-tui-cli --help    # full CLI surface
flipper-tui               # launch the TUI (needs a Flipper on USB)
```

## Troubleshooting

- **`error: revspec '…' not found`** — almost always a transient CDN race.
  The installer retries once automatically. If it still fails, re-run
  after a few seconds, or force a SHA via `FLIPPER_TUI_REF=<sha>`.
- **No Flipper detected** — the CLI prints a clear error. Plug a Flipper
  in (or pass `--device <path>`). The TUI's Devices screen has `r` to
  rescan.
