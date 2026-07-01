# install.ps1 — single-line installer for flipper-tui on Windows / PowerShell
#
# USAGE (the "single line" from PowerShell):
#   irm https://raw.githubusercontent.com/ankurCES/flipper-tui/main/scripts/install.ps1 | iex
#
# What it does:
#   1. If cargo is missing, installs Rust via rustup.
#   2. Resolves the install ref to a commit SHA via `git ls-remote` (so a
#      transient CDN/propagation race against a freshly pushed branch can't
#      fail with "revspec 'main' not found"), then `cargo install --git …`.
#   3. Retries once on a transient `revspec … not found` after re-resolving.
#   4. Verifies both flipper-tui and flipper-tui-cli exist.
#   5. Prints a smoke test (--version) and PATH advice.
#
# Env vars:
#   $env:FLIPPER_TUI_REF    git ref to install (default: resolve origin/HEAD).
#                           A commit SHA is preferred — it never races.
#   $env:FLIPPER_TUI_REPO   git URL (default: https://github.com/ankurCES/flipper-tui)

$ErrorActionPreference = 'Stop'

$Repo     = if ($env:FLIPPER_TUI_REPO) { $env:FLIPPER_TUI_REPO } else { 'https://github.com/ankurCES/flipper-tui' }
$UserRef  = if ($env:FLIPPER_TUI_REF)  { $env:FLIPPER_TUI_REF  } else { '' }

function Say($msg) { Write-Host "[flipper-tui] $msg" -ForegroundColor Cyan }

function Resolve-Ref {
  param([string]$UserRef, [string]$Repo)
  # Case 1: user gave us a full or short SHA (7..40 hex chars).
  if ($UserRef -and $UserRef -match '^[0-9a-fA-F]{7,40}$') {
    return $UserRef
  }
  # Case 2: user gave us an explicit branch/tag name — resolve it.
  if ($UserRef) {
    $r = (& git ls-remote $Repo $UserRef 2>$null) | Select-String 'refs/(heads|tags)/'
    if ($r) { return ($r -split '\s+')[0] }
  }
  # Case 3: resolve ${Repo}'s HEAD (whatever the default branch is).
  $head = (& git ls-remote --symref $Repo HEAD 2>$null) | Select-String 'refs/heads/'
  if ($head) { return ($head -split '\s+')[0] }
  # Last resort.
  return 'main'
}

# 1. cargo on PATH?
$cargo = (Get-Command cargo -ErrorAction SilentlyContinue)
if (-not $cargo) {
  Say 'cargo not found — installing Rust via rustup (stable, minimal profile)'
  $rustup = (Get-Command rustup -ErrorAction SilentlyContinue)
  if (-not $rustup) {
    Invoke-RestMethod https://win.rustup.rs/x86_64 -OutFile "$env:TEMP\rustup-init.exe"
    & "$env:TEMP\rustup-init.exe" -y --default-toolchain stable --profile minimal --no-modify-path | Out-Null
  }
  # rustup drops cargo into ~/.cargo/bin
  $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
  $cargo = (Get-Command cargo -ErrorAction SilentlyContinue)
  if (-not $cargo) { throw 'rustup install ran but cargo still not on PATH' }
}

$Ref = Resolve-Ref -UserRef $UserRef -Repo $Repo
Say "using $($cargo.Source) (cargo $(cargo --version | Select-Object -First 1))"
Say "installing flipper-tui from $Repo @ $Ref"

$env:CARGO_TERM_COLOR = 'never'

$installOk = $false
$attempts = 0
while (-not $installOk -and $attempts -lt 2) {
  $attempts++
  $err = $null
  try {
    cargo install --git $Repo --rev $Ref --locked
    $installOk = $true
  } catch {
    $err = $_.Exception.Message
    if ($err -match "revspec '.*' not found") {
      Say "first attempt hit a ref-propagation race; re-resolving from $Repo"
      $Ref = Resolve-Ref -UserRef '' -Repo $Repo
      Say "new ref=$Ref; retrying install"
      continue
    }
    throw
  }
}
if (-not $installOk) {
  throw @"
install failed: revspec not found on $Repo at ref=$Ref after retry.

This is almost always transient (cargo's git index fetched before GitHub's
CDN propagated the ref). To force a concrete SHA:

  `$env:FLIPPER_TUI_REF = '<commit-sha>'; iex (irm https://.../install.ps1)`

or wait a few seconds and re-run the installer.
"@
}

$InstallBin = Join-Path $env:USERPROFILE '.cargo\bin'
$env:PATH = "$InstallBin;$env:PATH"

foreach ($bin in 'flipper-tui','flipper-tui-cli') {
  $found = Get-Command $bin -ErrorAction SilentlyContinue
  if (-not $found) { throw "expected binary $bin not found at $InstallBin\$bin.exe" }
}

Say 'smoke test (no device needed):'
& flipper-tui --version
& flipper-tui-cli --version

Say "install complete. Add $InstallBin to your PATH if it is not already, then try:
   flipper-tui
   flipper-tui-cli ping
   flipper-tui-cli --help"
