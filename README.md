# Sweep

A Windows-first cleanup tool that safely removes unused files, caches, and system junk to free up disk space.

[GitHub](https://github.com/jpaulpoliquit/sweeper)
[License: MIT](LICENSE)
[Rust](https://www.rust-lang.org/)

## Why Sweep?

A comprehensive cleanup tool that handles caches, temp files, browser data, system files, duplicates, and more. The `--build` category is project-aware, only cleaning artifacts from inactive projects.

## Features

- General cleanup: Caches, temp files, browser data, system files, duplicates, empty folders
- Project-aware build cleanup: Only cleans artifacts from inactive projects (14+ days old)
- Git-aware: Skips projects with recent commits or uncommitted changes
- Windows-native: Handles NuGet, npm/yarn/pnpm caches, VS artifacts, browser caches
- Safe by default: Dry-run mode, Recycle Bin deletion, JSON output
- Interactive TUI: Run `sweeper` with no args
- Production-grade: Long path support, symlink protection, locked file detection

## Installation

**PowerShell:**

```powershell
irm https://raw.githubusercontent.com/jpaulpoliquit/sweeper/main/install.ps1 | iex
```

**Manual:** Download from [GitHub Releases](https://github.com/jpaulpoliquit/sweeper/releases) and add to PATH.

## Quick Start

```bash
sweeper                          # Launch interactive TUI
sweeper scan --all              # Preview what would be cleaned
sweeper clean --cache --temp    # Clean caches and temp files
sweeper clean --trash -y        # Empty Recycle Bin
```

## Commands


| Command   | Description                                            |
| --------- | ------------------------------------------------------ |
| `scan`    | Find cleanable files (safe, dry-run by default)        |
| `clean`   | Delete files (requires confirmation, use `-y` to skip) |
| `analyze` | Detailed breakdown with file lists                     |
| `config`  | View/edit configuration                                |


## Categories


| Flag           | Description                                                                         |
| -------------- | ----------------------------------------------------------------------------------- |
| `--cache`      | Package manager caches (npm/yarn/pnpm, NuGet, Cargo, pip)                           |
| `--temp`       | Windows temp files older than 1 day                                                 |
| `--trash`      | Recycle Bin contents                                                                |
| `--build`      | Build artifacts from inactive projects (`node_modules`, `target/`, `bin/obj`, etc.) |
| `--browser`    | Browser caches (Chrome, Edge, Firefox, Brave, etc.)                                 |
| `--system`     | Windows system caches (thumbnails, updates, icons)                                  |
| `--downloads`  | Old files in Downloads (30+ days)                                                   |
| `--large`      | Large files (100MB+)                                                                |
| `--old`        | Files not accessed in 30+ days                                                      |
| `--empty`      | Empty folders                                                                       |
| `--duplicates` | Duplicate files                                                                     |


**Note:** Only `--build` is project-aware. Other categories clean files system-wide.

## Options

**Common:**

- `--all` - Enable all categories
- `--exclude <PATTERN>` - Exclude paths (repeatable)
- `--json` - JSON output for scripting
- `-v`, `-vv` - Verbose output
- `-q` - Quiet mode

**Scan:**

- `--project-age <DAYS>` - Project inactivity threshold for `--build` (default: 14)
- `--min-age <DAYS>` - Minimum file age for `--downloads` and `--old` (default: 30)
- `--min-size <SIZE>` - Minimum file size for `--large` (default: 100MB)

**Clean:**

- `-y`, `--yes` - Skip confirmation
- `--permanent` - Bypass Recycle Bin
- `--dry-run` - Preview only

## Configuration

Config file: `%APPDATA%\sweeper\config.toml`

```toml
[thresholds]
project_age_days = 14
min_age_days = 30
min_size_mb = 100

[exclusions]
patterns = ["**/important-project/**"]
```

```bash
sweeper config --show    # View config
sweeper config --edit    # Edit config
```

## Building from Source

**Prerequisites:** Rust, Visual Studio Build Tools

```powershell
.\build\build.ps1              # Debug
.\build\build.ps1 -Release     # Release
```

Or: `cargo build --release`

**Output:** `target\release\sweeper.exe`

## TUI Screens

### Confirm Screen (Clean Action)

The Confirm screen appears when you're ready to delete selected files. It's organized into the following sections:

**UI Structure (top to bottom):**

1. **Logo & Tagline** - Application branding
2. **Warning Section** - Shows deletion summary:
  - Item count and total size
  - Fun comparison (e.g., "That's like ~2 4K movies!")
  - Default deletion method notice
3. **Items Area** (split horizontally):
  - **Left Panel: Summary Table** - Category breakdown showing:
    - Category name
    - Number of items per category
    - Total size per category
    - Grand total at bottom
  - **Right Panel: File List** - Hierarchical list of files to delete:
    - Grouped by category (expandable/collapsible)
    - Shows file paths and sizes
    - Categories can be expanded to see individual files
4. **Actions Section** - Deletion method options:
  - `[Y] Delete (to Recycle Bin)` - Standard deletion (recoverable)
  - `[N] Cancel` - Return to Results screen
  - `[P] Permanent Delete` - Bypass Recycle Bin (cannot be undone!)
5. **Shortcuts Bar** - Available keyboard shortcuts

**Available Actions:**

- `**Y**` - Execute deletion to Recycle Bin (recoverable)
- `**N**` - Cancel and return to Results screen
- `**P**` - Execute permanent deletion (bypasses Recycle Bin, cannot be undone)
- `**Esc**` - Cancel and return to Results screen

**Navigation:**

- **↑/↓ Arrow Keys** - Navigate through the file list (categories and individual items)
- **Space** - Toggle selection of the current item or category (select/deselect for deletion)
- **Enter** - Toggle category expansion/collapse when cursor is on a category header
- **Tab** - Not available on this screen (only file list navigation)

**Selection:**

- Items show `[X]` when selected, `[ ]` when not selected
- Categories show `[X]` when all items selected, `[-]` when partially selected, `[ ]` when none selected
- Use **Space** on an item to toggle its selection
- Use **Space** on a category header to toggle all items in that category
- Deselected items disappear from the list (you can go back to Results screen to reselect)
- Deletion is disabled when no items are selected

## Troubleshooting

- **File locked:** File is open in another app. Will be skipped automatically.
- **Long paths:** Handled automatically. Update if issues persist.
- **Symlinks:** Automatically skipped (expected behavior).
- **TUI not working:** Use PowerShell/Windows Terminal, or CLI mode: `sweeper scan --all`
- **No items found:** Check project activity with `--project-age 0` or file ages with `--min-age 0`

For build issues, see [build/BUILD_TROUBLESHOOTING.md](build/BUILD_TROUBLESHOOTING.md)

## License

[MIT](LICENSE)