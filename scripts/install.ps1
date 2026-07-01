# install.ps1 — single-line installer for flipper-tui on Windows / PowerShell
#
# USAGE (the "single line" from PowerShell):
#   irm https://raw.githubusercontent.com/ankurCES/flipper-tui/main/scripts/install.ps1 | iex
#
# What it does:
#   1. If cargo is missing, installs Rust via rustup.
#   2. cargo install --git the flipper-tui repo.
#   3. Verifies both flipper-tui and flipper-tui-cli exist.
#   4. Prints a smoke test (--version) and PATH advice.
#
# Env vars:
#   $env:FLIPPER_TUI_REF   git ref (default: main)
#   $env:FLIPPER_TUI_REPO  git URL (default: https://github.com/ankurCES/flipper-tui)

$ErrorActionPreference = 'Stop'

$Repo = if ($env:FLIPPER_TUI_REPO) { $env:FLIPPER_TUI_REPO } else { 'https://github.com/ankurCES/flipper-tui' }
$Ref  = if ($env:FLIPPER_TUI_REF)  { $env:FLIPPER_TUI_REF  } else { 'main' }

function Say($msg) { Write-Host "[flipper-tui] $msg" -ForegroundColor Cyan }

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

Say "using $($cargo.Source) (cargo $(cargo --version | Select-Object -First 1))"
Say "installing flipper-tui from $Repo @ $Ref"

$env:CARGO_TERM_COLOR = 'never'
cargo install --git $Repo --rev $Ref --locked

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
