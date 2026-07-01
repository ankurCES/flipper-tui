#!/usr/bin/env bash
# install.sh — single-line installer for flipper-tui
#
# USAGE (the "single line"):
#   curl -fsSL https://raw.githubusercontent.com/ankurCES/flipper-tui/main/scripts/install.sh | bash
#
# What it does:
#   1. Ensures `curl` is available (refuses to self-bootstrap from nothing).
#   2. If `cargo` is missing, installs Rust via rustup (stable, no profile prompts).
#   3. Runs `cargo install --git https://github.com/ankurCES/flipper-tui --locked`.
#   4. Verifies `flipper-tui` and `flipper-tui-cli` are on $PATH.
#   5. Prints a smoke test (`--help` for both binaries) so you can confirm
#      the install worked.
#
# Flags (env vars, not positional, so the curl|bash one-liner stays clean):
#   FLIPPER_TUI_REF       git ref to install (default: main)
#   FLIPPER_TUI_REPO      git URL                  (default: https://github.com/ankurCES/flipper-tui)
#   FLIPPER_TUI_NO_RUSTUP set to "1" to skip rustup install if cargo is missing
#                          (the script will then exit 1 with a clear message)

set -euo pipefail

REPO="${FLIPPER_TUI_REPO:-https://github.com/ankurCES/flipper-tui}"
REF="${FLIPPER_TUI_REF:-main}"

log() { printf '\033[1;34m[flipper-tui]\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31m[flipper-tui]\033[0m %s\n' "$*" >&2; exit 1; }

# 1. Pre-flight: need curl (the script is fetched with curl, so assume it,
#    but be defensive about curl-as-a-binary-after-curl).
command -v curl >/dev/null 2>&1 || die "curl is required on PATH"

# 2. Ensure cargo / Rust.
if ! command -v cargo >/dev/null 2>&1; then
  if [[ "${FLIPPER_TUI_NO_RUSTUP:-0}" == "1" ]]; then
    die "cargo not found and FLIPPER_TUI_NO_RUSTUP=1; install Rust from https://rustup.rs and re-run"
  fi
  log "cargo not found — installing Rust via rustup (stable, minimal profile)"
  if ! command -v rustup >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal --no-modify-path
  fi
  # shellcheck source=/dev/null
  if [[ -f "${HOME}/.cargo/env" ]]; then
    . "${HOME}/.cargo/env"
  fi
  command -v cargo >/dev/null 2>&1 || die "rustup install ran but cargo still not on PATH"
fi

log "using $(cargo --version) at $(command -v cargo)"
log "installing flipper-tui from ${REPO} @ ${REF}"

# 3. Install. --locked pins to the committed Cargo.lock so versions are
#    reproducible. `cargo install --git` already runs a release build with
#    sensible defaults.
#
#    CARGO_TERM_COLOR=never keeps the build log readable when piped; the
#    curl|bash flow benefits from this.
CARGO_TERM_COLOR=never cargo install \
  --git "${REPO}" \
  --rev "${REF}" \
  --locked \
  --root "${CARGO_INSTALL_ROOT:-$HOME/.cargo}"

# 4. cargo install puts binaries in $CARGO_INSTALL_ROOT/bin (defaults to
#    ~/.cargo/bin). Make sure that's on PATH for the rest of this script
#    and warn loudly if it's not on the user's interactive PATH.
INSTALL_BIN="${CARGO_INSTALL_ROOT:-$HOME/.cargo}/bin"
export PATH="${INSTALL_BIN}:${PATH}"

for bin in flipper-tui flipper-tui-cli; do
  if ! command -v "${bin}" >/dev/null 2>&1; then
    die "expected binary ${bin} not found at ${INSTALL_BIN}/${bin}; check install output above"
  fi
done

# 5. Smoke test — `--help` is hermetic (doesn't touch the device).
log "smoke test: flipper-tui --version"
flipper-tui --version || true
log "smoke test: flipper-tui-cli --version"
flipper-tui-cli --version || true

# PATH hint
case ":${PATH}:" in
  *":${INSTALL_BIN}:"*) log "${INSTALL_BIN} is already on PATH" ;;
  *) log "NOTE: add ${INSTALL_BIN} to your shell rc to put flipper-tui on PATH interactively:
   echo 'export PATH=\"${INSTALL_BIN}:\$PATH\"' >> ~/.bashrc   # or ~/.zshenv" ;;
esac

log "install complete. try:
   flipper-tui                  # launch the TUI (needs a Flipper on USB)
   flipper-tui-cli ping         # no-device smoke test
   flipper-tui-cli --help       # full CLI surface"
