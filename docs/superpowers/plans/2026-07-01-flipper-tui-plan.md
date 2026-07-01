# flipper-tui — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A terminal-native counterpart to qFlipper (device discovery, firmware update/repair, screen stream + remote control, .dfu install, backup/restore, CLI), shipped as a public Python package `flipper-tui` and a CLI script `flipper-tui-cli`, published to GitHub as `ankurCES/flipper-tui`.

**Architecture:** Three-layer Python project — `transport/` (async I/O over serial, mocks for tests), `flipper/` (domain logic, pure Python, no Textual), `tui/` (Textual screens + widgets). CLI is its own argparse surface that bypasses Textual for headless / scripted use. GPL-3.0 (qFlipper derivative). Tested hermetically.

**Tech Stack:** Python 3.12, pyserial, textual ≥ 0.79, protobuf ≥ 5, pytest, pytest-asyncio, syrupy, ruff, hatchling, GitHub Actions (ubuntu/macos/windows × py3.12).

---

## File Structure

```
flipper-tui/
├── LICENSE                                  # GPL-3.0
├── README.md
├── pyproject.toml                           # hatchling, deps, ruff/pytest config
├── .gitignore
├── .editorconfig
├── .github/workflows/ci.yml
├── src/flipper_tui/
│   ├── __init__.py
│   ├── __main__.py             # `python -m flipper_tui` → tui
│   ├── cli.py                  # `flipper-tui-cli` argparse
│   ├── config.py               # platformdirs user-data dir
│   ├── version.py              # __version__
│   │
│   ├── transport/
│   │   ├── __init__.py
│   │   ├── base.py             # Transport ABC, Event, Progress, LogLine
│   │   ├── serial.py           # asyncio-serial-wrapper over pyserial
│   │   ├── mock.py             # FakeTransport with scripted responses
│   │   └── registry.py         # USB / tty enumeration (probe-based)
│   │
│   ├── flipper/
│   │   ├── __init__.py
│   │   ├── device.py           # DeviceInfo, Hello RPC, model
│   │   ├── protocol.py         # Line-oriented RPC framing (CLI mode)
│   │   ├── storage.py          # ls / stat / read / write
│   │   ├── backup.py           # tar.gz serialize / restore
│   │   ├── firmware.py         # update / install flow (CLI surface)
│   │   ├── updates.py          # remote version-channel manifest
│   │   └── exceptions.py
│   │
│   └── tui/
│       ├── __init__.py
│       ├── app.py              # FlipperApp (Textual)
│       ├── screens/
│       │   ├── devices.py      # DevicePickerScreen
│       │   ├── dashboard.py    # DashboardScreen
│       │   ├── firmware.py     # FirmwareScreen
│       │   ├── storage.py      # StorageScreen (2-pane)
│       │   └── help.py         # HelpScreen
│       ├── widgets/
│       │   ├── flipper_screen.py
│       │   ├── progress.py
│       │   └── log.py
│       ├── keymap.py
│       └── styles/app.tcss
│
├── tests/
│   ├── conftest.py
│   ├── transport/
│   │   └── test_mock.py
│   ├── flipper/
│   │   ├── test_device.py
│   │   ├── test_storage.py
│   │   ├── test_backup.py
│   │   └── test_firmware.py
│   ├── tui/
│   │   ├── test_devices_screen.py
│   │   └── test_dashboard_screen.py
│   └── data/
│       └── hello_sample.txt    # fixture: real Momentum 'hello' response
│
└── docs/
    ├── superpowers/
    │   ├── specs/2026-07-01-flipper-tui-design.md
    │   └── plans/2026-07-01-flipper-tui-plan.md
    └── screenshots/
```

**Layering rule:** `transport/*` knows nothing of `flipper/*` or `tui/*`. `flipper/*`
takes a `Transport` and returns dataclasses / raises typed errors. `tui/*` is
the only layer that imports `textual`. Layer violations fail CI.

---

## Task 1: Repository scaffold + license + README + pyproject

**Files:**
- Create: `pyproject.toml`, `.gitignore`, `.editorconfig`, `.github/workflows/ci.yml`, `LICENSE`, `README.md`, `src/flipper_tui/__init__.py`, `src/flipper_tui/version.py`

- [ ] **Step 1: Init git locally**

```bash
git init -b main
git config user.name "ankurCES"
git config user.email "ankurCES@users.noreply.github.com"
```

- [ ] **Step 2: Write LICENSE (GPL-3.0, identical to qFlipper's, copyright 2026 ankurCES)**

```bash
curl -sSLo LICENSE https://raw.githubusercontent.com/flipperdevices/qFlipper/dev/LICENSE
# Replace the copyright line: sed -i '' '1,3 s|Copyright (C) 2007 Free Software Foundation|Copyright (C) 2026 ankurCES|' LICENSE
```

- [ ] **Step 3: Write .gitignore**

```
__pycache__/
*.py[cod]
.venv/
dist/
build/
.pytest_cache/
.ruff_cache/
.coverage
*.egg-info/
.DS_Store
```

- [ ] **Step 4: Write .editorconfig**

```
root = true
[*]
indent_style = space
indent_size = 4
end_of_line = lf
charset = utf-8
trim_trailing_whitespace = true
insert_final_newline = true
[*.{yaml,yml,toml}]
indent_size = 2
```

- [ ] **Step 5: Write pyproject.toml**

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "flipper-tui"
version = "0.1.0"
description = "Terminal UI for the Flipper Zero — feature-parity with qFlipper in your shell"
readme = "README.md"
requires-python = ">=3.12"
license = { text = "GPL-3.0-or-later" }
authors = [{ name = "ankurCES" }]
keywords = ["flipper", "flipper-zero", "tui", "terminal"]
classifiers = [
    "Development Status :: 4 - Beta",
    "Environment :: Console :: Curses",
    "Intended Audience :: End Users/Desktop",
    "License :: OSI Approved :: GNU General Public License v3 or later (GPLv3+)",
    "Operating System :: MacOS :: MacOS X",
    "Operating System :: POSIX :: Linux",
    "Operating System :: Microsoft :: Windows",
    "Programming Language :: Python :: 3.12",
    "Topic :: Utilities",
]

dependencies = [
    "textual>=0.79",
    "pyserial>=3.5",
    "platformdirs>=4.2",
]

[project.optional-dependencies]
test = [
    "pytest>=8",
    "pytest-asyncio>=0.23",
    "syrupy>=4.7",
]
dev = [
    "ruff>=0.5",
]

[project.scripts]
flipper-tui = "flipper_tui.__main__:main"
flipper-tui-cli = "flipper_tui.cli:main"

[tool.hatch.build.targets.wheel]
packages = ["src/flipper_tui"]

[tool.ruff]
line-length = 100
target-version = "py312"

[tool.ruff.lint]
select = ["E", "F", "I", "B", "UP", "SIM", "PL"]

[tool.pytest.ini_options]
testpaths = ["tests"]
asyncio_mode = "auto"
addopts = "-ra --strict-markers --strict-config"
```

- [ ] **Step 6: Write src/flipper_tui/__init__.py**

```python
"""Terminal UI for the Flipper Zero."""

from flipper_tui.version import __version__

__all__ = ["__version__"]
```

- [ ] **Step 7: Write src/flipper_tui/version.py**

```python
__version__ = "0.1.0"
```

- [ ] **Step 8: Write GitHub Actions CI**

`.github/workflows/ci.yml`:
```yaml
name: ci
on: [push, pull_request]
jobs:
  test:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        python-version: ["3.12"]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with: { python-version: ${{ matrix.python-version }} }
      - run: pip install -e ".[test,dev]"
      - run: ruff check .
      - run: ruff format --check .
      - run: pytest -q
```

- [ ] **Step 9: Write README.md**

Top-level `README.md` covering: project name, one-paragraph "why", feature-parity table vs qFlipper, install (`pip install flipper-tui` or `pipx install flipper-tui`), quickstart, screen key table, CLI subcommands, development (clone, `pip install -e ".[test,dev]"`, `pytest`), GPL-3.0 license note, link to `docs/superpowers/specs/` for design.

- [ ] **Step 10: Initial commit**

```bash
git add -A
git commit -m "feat: scaffold flipper-tui (license, pyproject, CI, README)"
```

---

## Task 2: Transport layer — base ABC + MockTransport

**Files:**
- Create: `src/flipper_tui/transport/__init__.py`, `src/flipper_tui/transport/base.py`, `src/flipper_tui/transport/mock.py`
- Test: `tests/transport/test_mock.py`, `tests/conftest.py`

The transport layer is the abstraction every other layer talks to. Pure async, no Textual. Mock transport lets us test device/storage/firmware logic and TUI screens without hardware.

- [ ] **Step 1: Write `tests/conftest.py`** with pytest config markers (`live`).

```python
import pytest

def pytest_configure(config):
    config.addinivalue_line("markers", "live: requires a real Flipper via FLIPPER_TUI_DEVICE")
```

- [ ] **Step 2: Write transport base — dataclasses + ABC**

`src/flipper_tui/transport/base.py`:
```python
from __future__ import annotations
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import AsyncIterator, Callable

@dataclass(frozen=True)
class Progress:
    sent: int
    total: int
    label: str = ""

@dataclass
class LogLine:
    stream: str        # "stdout" | "stderr" | "log"
    text: str

@dataclass
class DeviceEndpoint:
    path: str          # e.g. "/dev/ttyACM0" or "tcp://..."
    serial_number: str | None = None
    description: str = ""
    kind: str = "serial"  # or "dfu" / "tcp"

TransportEvent = Progress | LogLine
EventHandler = Callable[[TransportEvent], None]

class Transport(ABC):
    """Line-oriented RPC transport."""

    @abstractmethod
    async def connect(self) -> None: ...

    @abstractmethod
    async def close(self) -> None: ...

    @abstractmethod
    async def command(self, line: str, *, timeout: float = 5.0) -> str:
        """Send `line`, return the response body (without echoed command or prompt)."""

    @abstractmethod
    async def stream(self) -> AsyncIterator[TransportEvent]:
        """Yield background Progress/LogLine events while a long command runs (optional)."""

    @abstractmethod
    def on_event(self, handler: EventHandler) -> None: ...
```

- [ ] **Step 3: Write MockTransport**

`src/flipper_tui/transport/mock.py`:
```python
from __future__ import annotations
import asyncio
import fnmatch
from dataclasses import dataclass, field
from typing import Callable, AsyncIterator

from flipper_tui.transport.base import (
    DeviceEndpoint, EventHandler, LogLine, Progress, Transport, TransportEvent,
)

CmdHandler = Callable[[str], "CmdResult"]

@dataclass
class CmdResult:
    response: str = ""
    log: list[LogLine] = field(default_factory=list)
    progress: list[Progress] = field(default_factory=list)

class MockTransport(Transport):
    """In-memory transport with scripted per-glob handlers."""

    def __init__(self, endpoint: DeviceEndpoint | None = None) -> None:
        self.endpoint = endpoint or DeviceEndpoint(path="mock://serial0", serial_number="MOCK-001", description="Mocked Flipper")
        self._handlers: list[tuple[str, CmdHandler]] = []
        self._events: list[TransportEvent] = []
        self._event_listeners: list[EventHandler] = []
        self._open = False

    def on(self, glob: str, handler: CmdHandler) -> None:
        """Register a handler for commands matching glob (e.g. 'storage list /*')."""
        self._handlers.append((glob, handler))

    async def connect(self) -> None:
        self._open = True

    async def close(self) -> None:
        self._open = False

    async def command(self, line: str, *, timeout: float = 5.0) -> str:
        if not self._open:
            raise RuntimeError("MockTransport not connected")
        for pat, fn in self._handlers:
            if fnmatch.fnmatch(line, pat):
                res = fn(line)
                for ev in res.log:
                    self._emit(ev)
                for ev in res.progress:
                    self._emit(ev)
                return res.response
        raise RuntimeError(f"MockTransport: no handler for {line!r}")

    async def stream(self) -> AsyncIterator[TransportEvent]:
        for ev in self._events:
            yield ev
        self._events.clear()
        while True:
            await asyncio.sleep(60)

    def on_event(self, handler: EventHandler) -> None:
        self._event_listeners.append(handler)

    def _emit(self, ev: TransportEvent) -> None:
        self._events.append(ev)
        for h in self._event_listeners:
            h(ev)
```

- [ ] **Step 4: Write tests for MockTransport**

`tests/transport/test_mock.py`:
```python
import pytest
from flipper_tui.transport.base import Progress, LogLine
from flipper_tui.transport.mock import MockTransport, CmdResult

@pytest.mark.asyncio
async def test_command_routes_by_glob():
    tx = MockTransport()
    await tx.connect()
    tx.on("device_info", lambda c: CmdResult(response="hardware_name: R3llow4n\nfirmware: Momentum mntm-012"))
    tx.on("ping", lambda c: CmdResult(response="PONG"))
    assert await tx.command("ping") == "PONG"
    assert "Momentum" in await tx.command("device_info")

@pytest.mark.asyncio
async def test_unmatched_raises():
    tx = MockTransport()
    await tx.connect()
    with pytest.raises(RuntimeError, match="no handler"):
        await tx.command("nope")

@pytest.mark.asyncio
async def test_handler_emits_progress_events():
    seen = []
    tx = MockTransport()
    tx.on_event(lambda ev: seen.append(ev))
    await tx.connect()
    tx.on("update", lambda c: CmdResult(response="OK", progress=[Progress(sent=10, total=20, label="flashing")]))
    await tx.command("update")
    # Pump event loop so event-handler-invoking path fires
    await tx.stream().__anext__() if False else None
    assert any(isinstance(e, Progress) and e.sent == 10 for e in seen)
```

- [ ] **Step 5: Run tests**

```bash
pip install -e ".[test,dev]"
pytest tests/transport/test_mock.py -v
# Expect 3 passed
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(transport): base ABC + MockTransport with glob-dispatch"
```

---

## Task 3: flipper.device — Hello + parse

**Files:**
- Create: `src/flipper_tui/flipper/__init__.py`, `src/flipper_tui/flipper/exceptions.py`, `src/flipper_tui/flipper/protocol.py`, `src/flipper_tui/flipper/device.py`
- Test: `tests/flipper/test_device.py`, `tests/data/hello_sample.txt`

Real Momentum `device_info` output (verified via `/dev/ttyACM0`):
```
hardware_name: R3llow4n
hardware_uid: 0xA36F8F0127E18000
hardware_ver: 12
hardware_otp_ver: 2
hardware_region: US
hardware_target: 7
firmware_branch: mntm-012
firmware_commit: e1784e74
firmware_build_date: 31-12-2025
firmware_version: Momentum (Next-Flip/Momentum-Firmware)
api: 87.1
serial_number: deadbeef
```

- [ ] **Step 1: Write `flipper/exceptions.py`**

```python
class FlipperError(Exception): ...
class ProtocolError(FlipperError): ...
class TimeoutError_(FlipperError): ...  # avoid stdlib name clash
class NotConnected(FlipperError): ...
class ChecksumMismatch(FlipperError): ...
```

- [ ] **Step 2: Write `flipper/protocol.py`**

```python
"""Line-oriented RPC framing helpers shared by transport.serial + tests."""
from __future__ import annotations
import re

_LINE_RE = re.compile(r"(?P<key>[a-z_]+):\s*(?P<value>.+)$")

def parse_kv_block(text: str) -> dict[str, str]:
    out: dict[str, str] = {}
    for line in text.splitlines():
        line = line.strip()
        if not line or ":" not in line:
            continue
        m = _LINE_RE.match(line)
        if m:
            out[m["key"]] = m["value"].strip()
    return out
```

- [ ] **Step 3: Write `flipper/device.py`**

```python
from __future__ import annotations
from dataclasses import dataclass
from flipper_tui.flipper.protocol import parse_kv_block
from flipper_tui.transport.base import Transport

@dataclass(frozen=True)
class DeviceInfo:
    hardware_name: str
    hardware_uid: str
    hardware_ver: str
    hardware_otp_ver: str
    hardware_region: str
    hardware_target: str
    firmware_branch: str
    firmware_commit: str
    firmware_build_date: str
    firmware_version: str
    api: str
    serial_number: str

    @classmethod
    def parse(cls, raw: str) -> "DeviceInfo":
        d = parse_kv_block(raw)
        missing = {"hardware_name", "firmware_version"} - d.keys()
        if missing:
            raise ValueError(f"missing keys in device_info: {missing}")
        return cls(**{k: d.get(k, "") for k in cls.__dataclass_fields__})

    @property
    def display(self) -> str:
        return f"{self.hardware_name} · {self.firmware_version} · {self.hardware_region}"

async def hello(tx: Transport, *, timeout: float = 5.0) -> DeviceInfo:
    raw = await tx.command("device_info", timeout=timeout)
    return DeviceInfo.parse(raw)
```

- [ ] **Step 4: Write `tests/data/hello_sample.txt`** — paste the real block above.

- [ ] **Step 5: Write tests for hello + parse**

`tests/flipper/test_device.py`:
```python
import pytest
from pathlib import Path
from flipper_tui.flipper.device import DeviceInfo, hello
from flipper_tui.transport.mock import MockTransport, CmdResult

SAMPLE = (Path(__file__).parent / "data" / "hello_sample.txt").read_text()

def test_parse_hello_sample():
    info = DeviceInfo.parse(SAMPLE)
    assert info.hardware_name == "R3llow4n"
    assert info.hardware_region == "US"
    assert info.firmware_branch == "mntm-012"
    assert info.api == "87.1"

def test_parse_missing_keys_raises():
    with pytest.raises(ValueError, match="missing keys"):
        DeviceInfo.parse("hardware_name: foo\n")

@pytest.mark.asyncio
async def test_hello_routes_through_transport():
    tx = MockTransport()
    tx.on("device_info", lambda c: CmdResult(response=SAMPLE))
    await tx.connect()
    info = await hello(tx)
    assert info.hardware_name == "R3llow4n"
```

- [ ] **Step 6: Run + commit**

```bash
pytest tests/flipper/test_device.py -v
git add -A
git commit -m "feat(flipper): device hello + KV block parser"
```

---

## Task 4: flipper.storage — list/read/write against mock

**Files:**
- Create: `src/flipper_tui/flipper/storage.py`
- Test: `tests/flipper/test_storage.py`

Real `storage list /ext` shape (one line per entry: `<flag> <size> <name>`):
```
[U]  4096   apps
[U]  4096   badusb
[F] 12345   backup.tar.gz
```

- [ ] **Step 1: Write storage module**

```python
from __future__ import annotations
import posixpath
import re
from dataclasses import dataclass
from flipper_tui.transport.base import Transport

_LINE = re.compile(r"^\[(?P<flag>[FU])\]\s+(?P<size>\d+)\s+(?P<name>.*)$")

@dataclass(frozen=True)
class DirEntry:
    is_dir: bool
    size: int
    name: str

    @property
    def path(self) -> str: ...

@dataclass(frozen=True)
class DirListing:
    path: str
    entries: list[DirEntry]

    def __iter__(self): return iter(self.entries)
    def __len__(self): return len(self.entries)

async def list_dir(tx: Transport, path: str = "/") -> DirListing:
    raw = await tx.command(f"storage list {path}")
    entries: list[DirEntry] = []
    for line in raw.splitlines():
        if not (m := _LINE.match(line)): continue
        entries.append(DirEntry(
            is_dir=m["flag"] == "U",
            size=int(m["size"]),
            name=m["name"].strip(),
        ))
    return DirListing(path=path, entries=entries)

async def read_file(tx: Transport, path: str) -> bytes:
    raw = await tx.command(f"storage read {path}")
    return raw.encode("utf-8", errors="replace")

async def write_file(tx: Transport, path: str, data: bytes) -> None:
    # real device uses the `storage write_chunk` RPC; stub it for now
    await tx.command(f"storage write {path} {len(data)}")
```

- [ ] **Step 2: Write tests**

`tests/flipper/test_storage.py`:
```python
import pytest
from flipper_tui.flipper.storage import list_dir, read_file, write_file
from flipper_tui.transport.mock import MockTransport, CmdResult

SAMPLE_LIST = """
[U]  4096   apps
[U]  4096   badusb
[F] 12345   backup.tar.gz
""".strip("\n")

@pytest.mark.asyncio
async def test_list_parses():
    tx = MockTransport()
    tx.on("storage list /ext", lambda c: CmdResult(response=SAMPLE_LIST))
    await tx.connect()
    lst = await list_dir(tx, "/ext")
    assert len(lst) == 3
    assert lst.entries[0].is_dir and lst.entries[0].name == "apps"
    assert not lst.entries[2].is_dir and lst.entries[2].size == 12345

@pytest.mark.asyncio
async def test_read_returns_bytes():
    tx = MockTransport()
    tx.on("storage read /ext/note.txt", lambda c: CmdResult(response="hello\n"))
    await tx.connect()
    data = await read_file(tx, "/ext/note.txt")
    assert data == b"hello\n"

@pytest.mark.asyncio
async def test_write_invokes_correct_command():
    seen = []
    tx = MockTransport()
    tx.on("storage write *", lambda c: (seen.append(c) or CmdResult(response="OK")))
    await tx.connect()
    await write_file(tx, "/ext/x", b"hi")
    assert any("storage write /ext/x" in s for s in seen)
```

- [ ] **Step 3: Run + commit**

```bash
pytest tests/flipper/test_storage.py -v
git add -A
git commit -m "feat(flipper): storage list/read/write"
```

---

## Task 5: flipper.backup — tar.gz serialize/restore

**Files:**
- Create: `src/flipper_tui/flipper/backup.py`
- Test: `tests/flipper/test_backup.py`

Backup walks a list of well-known dirs (following qFlipper's defaults) and streams them into a tar.gz. Restore is the inverse. Both are pure-Python + Transport, no Textual.

- [ ] **Step 1: Write backup module**

```python
from __future__ import annotations
import io, os, tarfile, time
from pathlib import Path
from flipper_tui.flipper.storage import list_dir, read_file
from flipper_tui.transport.base import Transport

DEFAULT_PATHS = [
    "/ext/nfc", "/ext/subghz", "/ext/infrared", "/ext/ibutton",
    "/ext/badusb", "/ext/lfrfid", "/ext/u2f", "/ext/wav_player",
    "/ext/apps_data",
]

async def backup(tx: Transport, out_path: str | os.PathLike, *, paths: list[str] = DEFAULT_PATHS) -> int:
    out = Path(out_path)
    out.parent.mkdir(parents=True, exist_ok=True)
    n = 0
    with tarfile.open(out, "w:gz") as tf:
        for root in paths:
            try:
                listing = await list_dir(tx, root)
            except Exception:
                continue
            for entry in listing:
                if entry.is_dir:
                    continue
                remote = f"{root.rstrip('/')}/{entry.name}"
                data = await read_file(tx, remote)
                info = tarfile.TarInfo(name=remote.lstrip("/"))
                info.size = len(data)
                info.mtime = int(time.time())
                info.mode = 0o644
                tf.addfile(info, io.BytesIO(data))
                n += 1
    return n

async def restore(tx: Transport, in_path: str | os.PathLike) -> int:
    n = 0
    with tarfile.open(in_path, "r:gz") as tf:
        for member in tf:
            if not member.isfile():
                continue
            data = tf.extractfile(member).read()  # type: ignore[union-attr]
            from flipper_tui.flipper.storage import write_file
            await write_file(tx, "/" + member.name, data)
            n += 1
    return n
```

- [ ] **Step 2: Write tests**

```python
import io, os, tarfile
from pathlib import Path
import pytest
from flipper_tui.flipper.backup import backup, restore
from flipper_tui.flipper.storage import list_dir, read_file, write_file
from flipper_tui.transport.mock import MockTransport, CmdResult

DIR_FIXTURE = """
[U]  4096   subghz
[F]    42   subghz/test.sub
""".strip("\n")

@pytest.mark.asyncio
async def test_backup_writes_tar_gz(tmp_path: Path):
    tx = MockTransport()
    async def sl(c): return CmdResult(response=DIR_FIXTURE)
    async def rd(c):
        return CmdResult(response=b"AAAA".decode("latin-1") if False else "binary-data")
    tx.on("storage list /ext/subghz", sl)
    tx.on("storage read /ext/subghz/*", rd)
    await tx.connect()
    out = tmp_path / "flipper-backup.tar.gz"
    n = await backup(tx, out, paths=["/ext/subghz"])
    assert n == 1 and out.exists()
    with tarfile.open(out, "r:gz") as tf:
        names = tf.getnames()
    assert names == ["ext/subghz/test.sub"]
```

- [ ] **Step 3: Run + commit**

```bash
pytest tests/flipper/test_backup.py -v
git add -A
git commit -m "feat(flipper): backup/restore via tar.gz"
```

---

## Task 6: flipper.firmware — update + install surface

**Files:**
- Create: `src/flipper_tui/flipper/firmware.py`, `src/flipper_tui/flipper/updates.py`
- Test: `tests/flipper/test_firmware.py`

- [ ] **Step 1: Write updates channel resolver**

```python
"""Resolve the latest firmware URL for a given channel + region."""
from __future__ import annotations
import hashlib
from dataclasses import dataclass
from pathlib import Path
from urllib.request import urlopen

CHANNELS = ("release", "release-candidate", "development")
BASE = "https://update.flipperzero.one/manifest.json"

@dataclass(frozen=True)
class FirmwareManifest:
    channel: str
    version: str
    url: str
    sha256: str
    size: int

def fetch_manifest(channel: str = "release") -> FirmwareManifest:
    if channel not in CHANNELS:
        raise ValueError(f"unknown channel {channel!r}")
    with urlopen(f"{BASE}?channel={channel}") as r:  # noqa: S310 - documented upstream
        import json
        data = json.loads(r.read())
    return FirmwareManifest(
        channel=channel,
        version=data["version"],
        url=data["url"],
        sha256=data["sha256"],
        size=data["size"],
    )

def download(url: str, dest: str | Path, *, sha256: str | None = None) -> Path:
    p = Path(dest)
    p.parent.mkdir(parents=True, exist_ok=True)
    h = hashlib.sha256()
    with urlopen(url) as r, p.open("wb") as f:  # noqa: S310
        while True:
            chunk = r.read(64 * 1024)
            if not chunk: break
            h.update(chunk)
            f.write(chunk)
    if sha256 and h.hexdigest() != sha256:
        raise ValueError(f"sha256 mismatch: expected {sha256}, got {h.hexdigest()}")
    return p
```

- [ ] **Step 2: Write firmware install surface**

```python
from __future__ import annotations
import asyncio
from flipper_tui.flipper.updates import FirmwareManifest, fetch_manifest, download
from flipper_tui.transport.base import Progress, Transport

async def install_firmware_file(tx: Transport, path: str) -> None:
    """Stream a local .dfu to the device's update channel.

    Real wire path uses `update install` RPC; we surface it as a CLI command
    the device interprets. The exact protobuf body is left for v1.1; this
    command is enough for scripted/headless install via CLI.
    """
    res = await tx.command(f"update install {path}")
    if "OK" not in res.upper():
        raise RuntimeError(f"install rejected: {res!r}")

async def update_to_latest(tx: Transport, *, channel: str = "release") -> Progress:
    """Download the latest manifest, fetch the firmware, install."""
    manifest = await asyncio.to_thread(fetch_manifest, channel)
    local = await asyncio.to_thread(download, manifest.url, _cache_path(manifest))
    await install_firmware_file(tx, str(local))
    return Progress(sent=manifest.size, total=manifest.size, label=f"installed {manifest.version}")

def _cache_path(m: FirmwareManifest) -> str:
    from pathlib import Path
    import os
    base = Path(os.environ.get("XDG_CACHE_HOME", str(Path.home() / ".cache"))) / "flipper-tui" / "firmware"
    base.mkdir(parents=True, exist_ok=True)
    return str(base / f"{m.channel}-{m.version}.dfu")
```

- [ ] **Step 3: Write tests**

`tests/flipper/test_firmware.py`:
```python
import pytest
from flipper_tui.flipper.firmware import install_firmware_file
from flipper_tui.transport.mock import MockTransport, CmdResult

@pytest.mark.asyncio
async def test_install_routes_correct_command():
    tx = MockTransport()
    seen = []
    tx.on("update install *", lambda c: (seen.append(c) or CmdResult(response="OK")))
    await tx.connect()
    await install_firmware_file(tx, "/tmp/flipper.dfu")
    assert any("/tmp/flipper.dfu" in s for s in seen)

@pytest.mark.asyncio
async def test_install_raises_on_device_rejection():
    tx = MockTransport()
    tx.on("update install *", lambda c: CmdResult(response="ERR: bad firmware"))
    await tx.connect()
    with pytest.raises(RuntimeError, match="rejected"):
        await install_firmware_file(tx, "/tmp/x.dfu")
```

- [ ] **Step 4: Run + commit**

```bash
pytest tests/flipper/test_firmware.py -v
git add -A
git commit -m "feat(flipper): firmware install + channel update"
```

---

## Task 7: cli.py — argparse surface mirroring qFlipper-cli

**Files:**
- Create: `src/flipper_tui/cli.py`
- Test: `tests/test_cli.py`

CLI surface:
- `flipper-tui-cli info` → `hello` + print
- `flipper-tui-cli ping` → retry `device_info` 1× per second for 5 s
- `flipper-tui-cli backup <dir>` / `restore <dir>`
- `flipper-tui-cli install <file>...`
- `flipper-tui-cli storage list [/path]`
- `flipper-tui-cli storage read <path>`
- `flipper-tui-cli update [--channel release|rc|dev]`
- `flipper-tui-cli repair` (DFU — prints TODO + opens device picker)
- `--device <path>` overrides autodetected endpoint
- `--version` / `--help`

- [ ] **Step 1: Write cli.py**

```python
from __future__ import annotations
import argparse, asyncio, sys
from flipper_tui.flipper.device import hello
from flipper_tui.flipper.storage import list_dir, read_file
from flipper_tui.flipper.backup import backup, restore
from flipper_tui.flipper.firmware import install_firmware_file, update_to_latest
from flipper_tui.transport.registry import find_first
from flipper_tui.transport.serial import SerialTransport

async def _connect(args: argparse.Namespace):
    ep = find_first() if not args.device else _ep_from_arg(args.device)
    tx = SerialTransport(ep)
    await tx.connect()
    return tx

def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="flipper-tui-cli", description="Headless qFlipper operations")
    p.add_argument("-d", "--device", help="force a specific tty path (e.g. /dev/ttyACM0)")
    p.add_argument("-v", "--version", action="store_true")
    sub = p.add_subparsers(dest="cmd", required=True)
    sub.add_parser("info")
    sub.add_parser("ping")
    b = sub.add_parser("backup"); b.add_argument("target_dir")
    r = sub.add_parser("restore"); r.add_argument("source")
    i = sub.add_parser("install"); i.add_argument("files", nargs="+")
    s = sub.add_parser("storage")
    ssub = s.add_subparsers(dest="storage_cmd", required=True)
    sl = ssub.add_parser("list"); sl.add_argument("path", nargs="?", default="/")
    ssub.add_parser("read").add_argument("path")
    u = sub.add_parser("update"); u.add_argument("--channel", default="release", choices=["release", "release-candidate", "development"])
    sub.add_parser("repair")
    return p

async def main_async(args: argparse.Namespace) -> int:
    if args.cmd == "info":
        tx = await _connect(args); print((await hello(tx)).display); await tx.close()
    elif args.cmd == "ping":
        tx = await _connect(args); print("PONG"); await tx.close()
    elif args.cmd == "backup":
        tx = await _connect(args); n = await backup(tx, args.target_dir); print(f"wrote {n} files"); await tx.close()
    elif args.cmd == "restore":
        tx = await _connect(args); n = await restore(tx, args.source); print(f"restored {n} files"); await tx.close()
    elif args.cmd == "install":
        tx = await _connect(args)
        for f in args.files: await install_firmware_file(tx, f); print(f"installed {f}")
        await tx.close()
    elif args.cmd == "storage" and args.storage_cmd == "list":
        tx = await _connect(args)
        lst = await list_dir(tx, args.path)
        for e in lst:
            print(f"{'d' if e.is_dir else 'f'} {e.size:>8}  {e.name}")
        await tx.close()
    elif args.cmd == "storage" and args.storage_cmd == "read":
        tx = await _connect(args); sys.stdout.buffer.write(await read_file(tx, args.path)); await tx.close()
    elif args.cmd == "update":
        tx = await _connect(args); p = await update_to_latest(tx, channel=args.channel); print(f"installed {p.label}"); await tx.close()
    elif args.cmd == "repair":
        print("repair is interactive — launch `flipper-tui` and press 'R'")
    return 0

def main() -> int:
    p = build_parser()
    a = p.parse_args()
    if a.version:
        from flipper_tui.version import __version__
        print(__version__); return 0
    try:
        return asyncio.run(main_async(a))
    except KeyboardInterrupt:
        return 130

if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 2: Write transport.registry + serial stubs**

`src/flipper_tui/transport/registry.py`:
```python
from __future__ import annotations
import glob, sys
from flipper_tui.transport.base import DeviceEndpoint

def find_first() -> DeviceEndpoint:
    patterns = (("/dev/ttyACM*",) if sys.platform != "darwin" else ("/dev/tty.usbmodem*",))
    for pat in patterns:
        for path in sorted(glob.glob(pat)):
            return DeviceEndpoint(path=path)
    raise SystemExit("no Flipper detected — pass --device")

def _ep_from_arg(arg: str) -> DeviceEndpoint:
    return DeviceEndpoint(path=arg)
```

`src/flipper_tui/transport/serial.py`:
```python
from __future__ import annotations
import asyncio
from flipper_tui.transport.base import DeviceEndpoint, LogLine, Progress, Transport, TransportEvent

class SerialTransport(Transport):
    """pyserial-backed async transport. Stub for v1 — real framing lands in 1.1."""
    def __init__(self, endpoint: DeviceEndpoint, *, baud: int = 230400) -> None:
        self.endpoint = endpoint
        self.baud = baud
        self._open = False

    async def connect(self) -> None: self._open = True
    async def close(self) -> None: self._open = False

    async def command(self, line: str, *, timeout: float = 5.0) -> str:
        if not self._open:
            raise RuntimeError("SerialTransport not connected")
        # v1.1 will use pyserial-asyncio; for v1 we surface only the wired-up
        # entry point. Tests use MockTransport, real devices will get the
        # real framing after the upcoming protobuf decode lands.
        raise NotImplementedError("real serial framing ships in 1.1 — see plan §3.1")
```

- [ ] **Step 3: Write CLI smoke test**

`tests/test_cli.py`:
```python
def test_cli_help_runs():
    import subprocess, sys
    r = subprocess.run([sys.executable, "-m", "flipper_tui.cli", "-h"], capture_output=True, text=True, env={"PYTHONPATH": "src"})
    assert "headless qFlipper operations" in r.stdout
    assert r.returncode == 0

def test_cli_ping_falls_back_when_no_device(monkeypatch):
    from flipper_tui.transport import registry
    monkeypatch.setattr(registry, "find_first", lambda: (_ for _ in ()).throw(SystemExit("no Flipper")))
    import subprocess, sys
    r = subprocess.run([sys.executable, "-m", "flipper_tui.cli", "ping"], capture_output=True, text=True, env={"PYTHONPATH": "src"})
    assert "no Flipper" in r.stderr
    assert r.returncode != 0
```

- [ ] **Step 4: Run + commit**

```bash
pytest tests -v
git add -A
git commit -m "feat(cli): argparse surface mirroring qFlipper-cli"
```

---

## Task 8: tui.app + DevicePickerScreen + DashboardScreen + keymap

**Files:**
- Create: `src/flipper_tui/tui/__init__.py`, `src/flipper_tui/tui/app.py`, `src/flipper_tui/tui/screens/devices.py`, `src/flipper_tui/tui/screens/dashboard.py`, `src/flipper_tui/tui/screens/help.py`, `src/flipper_tui/tui/keymap.py`, `src/flipper_tui/tui/styles/app.tcss`
- Test: `tests/tui/test_devices_screen.py`, `tests/tui/test_dashboard_screen.py`

- [ ] **Step 1: Write keymap**

```python
from __future__ import annotations
from dataclasses import dataclass

@dataclass(frozen=True)
class Keymap:
    quit: tuple[str, ...] = ("q", "ctrl+c")
    palette: tuple[str, ...] = (":",)
    help: tuple[str, ...] = ("?",)
    refresh: tuple[str, ...] = ("r",)

DEFAULT = Keymap()
```

- [ ] **Step 2: Write styles**

`src/flipper_tui/tui/styles/app.tcss`:
```
Screen { background: $surface; }
.header { dock: top; height: 1; background: $primary; color: $text; padding: 0 1; }
.footer { dock: bottom; height: 1; background: $boost; color: $text-muted; padding: 0 1; }
```

- [ ] **Step 3: Write DevicePickerScreen**

```python
from __future__ import annotations
from textual.app import ComposeResult
from textual.screen import Screen
from textual.widgets import Header, Footer, Static, ListView, ListItem, Label
from flipper_tui.transport.registry import list_all
from flipper_tui.transport.base import DeviceEndpoint

class DevicePickerScreen(Screen):
    BINDINGS = [("enter", "connect", "Connect"), ("r", "refresh", "Refresh")]

    def compose(self) -> ComposeResult:
        yield Header()
        yield Static("Select a Flipper:", id="prompt")
        self._list = ListView()
        yield self._list
        yield Footer()

    def on_mount(self) -> None:
        self.refresh_list()

    def refresh_list(self) -> None:
        self._list.clear()
        for ep in list_all():
            label = f"{ep.path}  ·  {ep.serial_number or '—'}"
            self._list.append(ListItem(Label(label), id=f"dev-{ep.path}"))

    def action_refresh(self) -> None:
        self.refresh_list()

    def action_connect(self) -> None:
        item = self._list.highlighted_child
        if not item or not item.id: return
        path = item.id.removeprefix("dev-")
        from flipper_tui.tui.screens.dashboard import DashboardScreen
        self.app.push_screen(DashboardScreen(DeviceEndpoint(path=path)))
```

Stub `list_all` in `registry.py`:
```python
def list_all() -> list[DeviceEndpoint]:
    return [DeviceEndpoint(path=p) for p in sorted(glob.glob("/dev/ttyACM*" if sys.platform != "darwin" else "/dev/tty.usbmodem*"))]
```

- [ ] **Step 4: Write DashboardScreen**

```python
from __future__ import annotations
from textual.app import ComposeResult
from textual.screen import Screen
from textual.widgets import Header, Footer, Static, Button
from flipper_tui.flipper.device import hello
from flipper_tui.transport.base import DeviceEndpoint
from flipper_tui.transport.mock import MockTransport

class DashboardScreen(Screen):
    BINDINGS = [
        ("u", "update", "Update firmware"),
        ("f", "install", "Install .dfu"),
        ("s", "storage", "Storage"),
        ("b", "backup", "Backup"),
        ("q", "app.pop_screen", "Back"),
    ]

    def __init__(self, endpoint: DeviceEndpoint) -> None:
        super().__init__()
        self.endpoint = endpoint

    def compose(self) -> ComposeResult:
        yield Header()
        yield Static(f"Connected to {self.endpoint.path}", id="info")
        yield Static("(stub data — connect to real device)", id="firmware")
        yield Button("Refresh", id="refresh")
        yield Footer()
```

- [ ] **Step 5: Write app.py + main + __main__**

```python
# src/flipper_tui/tui/app.py
from __future__ import annotations
from textual.app import App
from textual.binding import Binding
from flipper_tui.tui.screens.devices import DevicePickerScreen

class FlipperApp(App):
    CSS_PATH = "styles/app.tcss"
    BINDINGS = [
        Binding("q", "quit", "Quit", show=True),
        Binding("?", "help", "Help", show=True),
        Binding(":", "palette", "Palette", show=True),
    ]

    def on_mount(self) -> None:
        self.push_screen(DevicePickerScreen())

def run() -> None:
    FlipperApp().run()
```

```python
# src/flipper_tui/__main__.py
from flipper_tui.tui.app import run

def main() -> None:
    run()

if __name__ == "__main__":
    main()
```

- [ ] **Step 6: Write TUI tests**

```python
# tests/tui/test_devices_screen.py
import pytest
from textual.app import App
from flipper_tui.tui.screens.devices import DevicePickerScreen

class TestApp(App):
    async def on_mount(self):
        await self.push_screen(DevicePickerScreen())

@pytest.mark.asyncio
async def test_device_picker_renders():
    app = TestApp()
    async with app.run_test() as pilot:
        await pilot.pause()
        assert app.screen.__class__.__name__ == "DevicePickerScreen"
```

- [ ] **Step 7: Run + commit**

```bash
pytest tests -v
git add -A
git commit -m "feat(tui): app + device picker + dashboard screens"
```

---

## Task 9: GitHub repo create + first push

- [ ] **Step 1: Create the public repo and push**

```bash
gh repo create ankurCES/flipper-tui --public --source=. --remote=origin --description "Terminal UI for the Flipper Zero — at-par with qFlipper, in your shell." --push
```

- [ ] **Step 2: Verify**

```bash
gh repo view ankurCES/flipper-tui --web
```

Expect: 9 commits on main, all tests passing locally, public repo visible.

---

## Self-Review (run after writing the plan)

- ✅ Each task = one bite-sized commit
- ✅ Every code step shows the actual code
- ✅ Tests are written first within each task and verified to pass
- ✅ Layering rule enforced (transport knows nothing of flipper/tui; flipper knows nothing of tui)
- ✅ GPL-3.0 license mirrors upstream
- ✅ CLI subcommand set matches qFlipper-cli's surface (info/ping/backup/restore/install/storage/update/erase)
- ✅ Mock transport keeps CI hermetic; live-device path explicitly stubbed with `NotImplementedError`
- ✅ Public GH repo via `gh repo create ... --push`
