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
#   FLIPPER_TUI_REF       git ref to install (default: try origin/HEAD first,
#                          then fall back to "main"). A commit SHA always works.
#   FLIPPER_TUI_REPO      git URL                  (default: https://github.com/ankurCES/flipper-tui)
#   FLIPPER_TUI_NO_RUSTUP set to "1" to skip rustup install if cargo is missing
#                          (the script will then exit 1 with a clear message)
#
# Why we resolve the ref dynamically:
# When a branch is freshly pushed (or the install is run within seconds of a
# push), `cargo install --git … --rev main` can transiently fail with
# `revspec 'main' not found` because cargo's git index fetches from the remote
# before GitHub's CDN has fully propagated the new ref. To make the installer
# resilient, we resolve `${REPO}`'s `HEAD` (= whatever the default branch is)
# to a commit SHA via `git ls-remote`, then pass that SHA to `cargo install`.
# SHAs never race; branches can.

set -euo pipefail

REPO="${FLIPPER_TUI_REPO:-https://github.com/ankurCES/flipper-tui}"
USER_REF="${FLIPPER_TUI_REF:-}"

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

# 2a. Resolve the ref to install. Pinned to a SHA when possible so that
#     `cargo install --rev <sha>` is immune to CDN-propagation races against
#     a freshly pushed branch (which manifest as the original symptom:
#     `revspec 'main' not found`).
#     - If the user passed a SHA (40 hex chars), use it as-is.
#     - If the user passed a branch/tag name, resolve it to a SHA via
#       `git ls-remote <repo> <name>`.
#     - Otherwise, resolve `${REPO}`'s default branch (HEAD) to a SHA.
#     - As a last resort, retry with the literal "main".
resolve_ref() {
  local user_ref="$1" repo="$2"
  local resolved
  # Case 1: user gave us a full or short SHA.
  if [[ "${user_ref}" =~ ^[0-9a-fA-F]{7,40}$ ]]; then
    resolved="${user_ref}"
    # Verify it exists on the remote; warn but proceed if it doesn't.
    if ! git ls-remote --quiet "${repo}" "${resolved}" >/dev/null 2>&1; then
      log "WARN: ${resolved} not found on ${repo}; continuing anyway (cargo will give a clearer error)"
    fi
    printf '%s\n' "${resolved}"
    return 0
  fi
  # Case 2: user gave us an explicit ref name — resolve it.
  if [[ -n "${user_ref}" ]]; then
    resolved="$(git ls-remote "${repo}" "${user_ref}" 2>/dev/null | awk '!/^ref:/ && $1 ~ /^[0-9a-f]{40}$/ {print $1; exit}')"
    if [[ -n "${resolved}" ]]; then
      printf '%s\n' "${resolved}"
      return 0
    fi
  fi
  # Case 3: resolve ${REPO}'s HEAD (whatever its default branch is).
  # `git ls-remote --symref HEAD` emits two lines: a "ref: refs/heads/X
  # HEAD" symref and a "<sha> HEAD" line. We need the SHA, NOT the
  # symref's first field (which is the literal "ref:"), so skip lines
  # starting with "ref:" and grab the 40-char hex on the next line.
  resolved="$(git ls-remote --symref "${repo}" HEAD 2>/dev/null | awk '!/^ref:/ && $1 ~ /^[0-9a-f]{40}$/ {print $1; exit}')"
  if [[ -n "${resolved}" ]]; then
    printf '%s\n' "${resolved}"
    return 0
  fi
  # Last resort — let cargo try with a literal "main".
  printf 'main\n'
}
REF="$(resolve_ref "${USER_REF}" "${REPO}")"
log "installing flipper-tui from ${REPO} @ ${REF}"

# 3. Install. --locked pins to the committed Cargo.lock so versions are
#    reproducible. `cargo install --git` already runs a release build with
#    sensible defaults.
#
#    CARGO_TERM_COLOR=never keeps the build log readable when piped; the
#    curl|bash flow benefits from this.
#
#    On the (transient) `revspec '…' not found` race, ref-resolution to a
#    SHA means retrying cargo install won't help; re-resolve once more and
#    retry once if it does happen. If it still fails, surface the exact
#    workaround to the user.
install_cargo_release() {
  CARGO_TERM_COLOR=never cargo install \
    --git "${REPO}" \
    --rev "${REF}" \
    --locked \
    --root "${CARGO_INSTALL_ROOT:-$HOME/.cargo}"
}

if ! install_cargo_release; then
  err="$(cargo install --git "${REPO}" --rev "${REF}" --locked 2>&1 || true)"
  if grep -q "revspec '.*' not found" <<<"${err}"; then
    log "first attempt hit a ref-propagation race; re-resolving from ${REPO}"
    NEW_REF="$(resolve_ref "" "${REPO}")"
    if [[ "${NEW_REF}" != "${REF}" ]]; then
      REF="${NEW_REF}"
      log "new ref=${REF}; retrying install"
      CARGO_TERM_COLOR=never cargo install \
        --git "${REPO}" \
        --rev "${REF}" \
        --locked \
        --root "${CARGO_INSTALL_ROOT:-$HOME/.cargo}"
    else
      die "$(cat <<EOF
install failed: revspec not found on ${REPO} at ref=${REF}.

This is almost always transient (cargo's git index fetched before GitHub's
CDN propagated the ref). To force a concrete SHA:

  FLIPPER_TUI_REF=<commit-sha> $0

or wait a few seconds and re-run the installer.

Original error:
${err}
EOF
)"
    fi
  else
    die "cargo install failed: ${err}"
  fi
fi

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
#
#    On macOS, binaries installed to `~/.cargo/bin/` are not ad-hoc
#    signed, so the kernel's AMFI hook kills them at exec time with
#    `proc ...: load code signature error 2`. Re-sign them with the
#    ad-hoc identity (`-`) before the smoke test so `--version`
#    actually returns. Skip silently on non-macOS.
if [[ "$(uname -s)" == "Darwin" ]]; then
  log "ad-hoc codesigning installed binaries (macOS AMFI)"
  for bin in flipper-tui flipper-tui-cli; do
    codesign --sign - --force --deep "${INSTALL_BIN}/${bin}" >/dev/null 2>&1 || \
      log "WARNING: codesign failed for ${bin}; --version may not run"
  done
fi

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
