"""Command-line entry point: ``python -m flipper_tui`` -> launch the TUI app."""

from __future__ import annotations

from flipper_tui.tui.app import run


def main() -> None:
    """Launch the Textual TUI."""
    run()


if __name__ == "__main__":
    main()
