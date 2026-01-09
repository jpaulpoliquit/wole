# Sweep

A Windows-first developer cleanup tool that safely removes build artifacts and caches from inactive projects.

## Why Sweep?

Existing tools (kondo, npkill, BleachBit) lack **project activity awareness**. Sweep only cleans artifacts from projects you're not actively working on.

## Features

- **Git-aware** — Skips projects with recent commits or uncommitted changes
- **Multi-language** — Node, Rust, .NET, Python, Java
- **Windows-native** — Handles NuGet, npm/yarn/pnpm caches, VS artifacts
- **Safe by default** — Dry-run mode, Recycle Bin deletion, JSON manifests

## Installation

### Windows (PowerShell)

```powershell
# Download and install from GitHub releases
irm https://raw.githubusercontent.com/jpaulpoliquit/sweeper/main/install.ps1 | iex
```

Or download `install.ps1` and run:
```powershell
.\install.ps1
```

### Windows (Batch)

```cmd
# Download install.bat and run
install.bat
```

### Windows (Git Bash / MINGW64)

```bash
# Download install.sh and run
./install.sh
```

### Manual Installation

1. Download the latest release from [GitHub Releases](https://github.com/jpaulpoliquit/sweeper/releases)
2. Extract `sweeper.exe` to a directory in your PATH (e.g., `%LOCALAPPDATA%\sweeper\bin`)
3. Add that directory to your PATH environment variable

## Quick Start

```bash
# Scan for reclaimable space
sweeper scan --build --cache --temp

# Preview what would be deleted
sweeper scan --build --cache

# Clean inactive projects (with confirmation)
sweeper clean --build --cache

# Clean without confirmation
sweeper clean --build --cache -y
```

## Categories

| Flag | Targets |
|------|---------|
| `--build` | Build artifacts from inactive projects (`node_modules`, `target/`, `bin/obj`, `dist/`, `__pycache__`, etc.) |
| `--cache` | npm/yarn/pnpm, NuGet, Cargo, pip caches |
| `--temp` | Windows temp directories (files older than 1 day) |
| `--trash` | Recycle Bin contents |

## Configuration

Config file: `%APPDATA%\sweeper\config.toml` (coming in Phase 3)

```toml
[activity]
active_threshold_days = 30
skip_if_uncommitted_changes = true
```

## License

[MIT](LICENSE)
