# Wole

A Windows-first cleanup tool that safely removes unused files, caches, and system junk to free up disk space.

[GitHub](https://github.com/jpaulpoliquit/wole)
[License: MIT](LICENSE)
[Rust](https://www.rust-lang.org/)

## Why Wole?

A comprehensive cleanup tool that handles caches, temp files, browser data, system files, duplicates, and more. The `--build` category is project-aware, only cleaning artifacts from inactive projects.

## Features

- General cleanup: Caches, temp files, browser data, system files, duplicates, empty folders
- Project-aware build cleanup: Only cleans artifacts from inactive projects (14+ days old)
- Git-aware: Skips projects with recent commits or uncommitted changes
- Windows-native: Handles NuGet, npm/yarn/pnpm caches, VS artifacts, browser caches
- Safe by default: Dry-run mode, Recycle Bin deletion, JSON output
- Interactive TUI: Run `wole` with no args
- Production-grade: Long path support, symlink protection, locked file detection

## Installation

**PowerShell:**

```powershell
irm https://raw.githubusercontent.com/jpaulpoliquit/wole/master/install.ps1 | iex
```

Or if the above doesn't work, try:

```powershell
irm https://raw.githubusercontent.com/jpaulpoliquit/wole/main/install.ps1 | iex
```

**Manual:** Download from [GitHub Releases](https://github.com/jpaulpoliquit/wole/releases) and add to PATH.

## Quick Start

```bash
wole                          # Launch interactive TUI (recommended!)
wole scan --all              # Preview what would be cleaned
wole clean --cache --temp    # Clean caches and temp files
wole clean --trash -y        # Empty Recycle Bin
```

## Commands & Output

### Scan Command

The `scan` command analyzes your system and finds files that can be safely cleaned. It's completely safe—no files are deleted, only identified. Use this to preview what cleanup would free up.

**Basic scan:**

```bash
$ wole scan --all
```

```
╔════════════════════════════════════════════════════════════╗
║                    Wole Scan Results                       ║
╠════════════════════════════════════════════════════════════╣

Category         Items      Size         Status
────────────────────────────────────────────────────────────
Package cache        45    2.3 GB    [OK] Safe to clean
Temp                128    456 MB     [OK] Safe to clean
Trash                23    89 MB      [OK] Safe to clean
Build                12    1.2 GB    [OK] Inactive projects
Browser              67    234 MB     [OK] Safe to clean
System               34    567 MB     [OK] Safe to clean
Large                 8    2.1 GB   [!] Review suggested
Old                  45    890 MB   [!] Review suggested
────────────────────────────────────────────────────────────
Total              362    8.1 GB         Reclaimable

Run wole clean --all to remove these files.
```

**Verbose mode** shows sample file paths for each category:

```bash
$ wole scan --all -v
```

```
Package cache        45    2.3 GB    [OK] Safe to clean
  C:\Users\user\AppData\Local\npm-cache
  C:\Users\user\AppData\Local\yarn\cache
  C:\Users\user\.cargo\registry\cache
  ... and 42 more

Temp                128    456 MB     [OK] Safe to clean
  C:\Users\user\AppData\Local\Temp\~DF123.tmp
  C:\Windows\Temp\installer.log
  ... and 126 more
```

**JSON output** for scripting and automation:

```bash
$ wole scan --all --json
```

```json
{
  "version": "1.0",
  "timestamp": "2024-01-15T10:30:00Z",
  "categories": {
    "cache": {
      "items": 45,
      "size_bytes": 2469606195,
      "size_human": "2.3 GB",
      "paths": [
        "C:\\Users\\user\\AppData\\Local\\npm-cache",
        "C:\\Users\\user\\AppData\\Local\\yarn\\cache",
        ...
      ]
    },
    "temp": {
      "items": 128,
      "size_bytes": 478150656,
      "size_human": "456 MB",
      "paths": [...]
    },
    ...
  },
  "summary": {
    "total_items": 362,
    "total_bytes": 8700000000,
    "total_human": "8.1 GB"
  }
}
```

### Clean Command

The `clean` command actually deletes the files found by scan. By default, files are moved to the Recycle Bin (recoverable). Use `--permanent` to bypass the Recycle Bin. Always requires confirmation unless you use `-y`.

**Clean with confirmation:**

```bash
$ wole clean --cache --temp
```

```
Scanning for cleanable files...
Found 173 files (2.8 GB) to clean.

⚠️  About to delete 173 files (2.8 GB)
Files will be moved to Recycle Bin (recoverable).

Continue? [y/N]: y

Cleaning...
✓ Package cache: 45 files (2.3 GB)
✓ Temp: 128 files (456 MB)
✓ Cleanup complete! Freed 2.8 GB
```

**Clean with auto-confirm:**

```bash
$ wole clean --trash -y
```

```
Scanning for cleanable files...
Found 23 files (89 MB) in Recycle Bin.

Cleaning...
✓ Trash: 23 files (89 MB)
✓ Cleanup complete! Freed 89 MB
```

**Dry-run mode** (preview only, no deletion):

```bash
$ wole clean --all --dry-run
```

```
[DRY RUN] Would delete 362 files (8.1 GB)
[DRY RUN] Package cache: 45 files (2.3 GB)
[DRY RUN] Temp: 128 files (456 MB)
...
[DRY RUN] No files were actually deleted.
```

### Analyze Command

The `analyze` command provides a detailed breakdown of disk usage, showing which categories take up the most space. Useful for understanding where your disk space is being used.

```bash
$ wole analyze
```

```
Scan Results

Category                  Files         Size
───────────────────────────────────────────────
Trash                        23        89 MB
Large Files                   8       2.1 GB
System Cache                 34       567 MB
Build Artifacts              12       1.2 GB
Old Downloads                45       890 MB
Temp Files                  128       456 MB
Package Cache                45       2.3 GB
Browser Cache                67       234 MB
───────────────────────────────────────────────
Total                      362       8.1 GB
```

**Interactive disk insights:**

```bash
$ wole analyze --interactive
```

Launches an interactive TUI showing folder sizes, largest files, and disk usage breakdown with visual progress bars.

### Config Command

View or edit your Wole configuration settings.

**View current config:**

```bash
$ wole config --show
```

```
Configuration file: C:\Users\user\AppData\Roaming\wole\config.toml

[thresholds]
project_age_days = 14
min_age_days = 30
min_size_mb = 100

[exclusions]
patterns = []
```

**Edit config:**

```bash
$ wole config --edit
```

Opens the config file in your default editor.

## Interactive TUI (Terminal User Interface)

**The best way to use Wole!** Simply run `wole` with no arguments to launch the interactive terminal interface. The TUI provides a full-screen, keyboard-driven experience for scanning, cleaning, and managing your disk space.

### Dashboard Screen

The main entry point when you launch `wole`. Here you can:

- **Select actions**: Scan, Clean, Analyze, Restore, or Config
- **Choose categories**: Toggle which file categories to scan (cache, temp, trash, build, etc.)
- **Navigate**: Use arrow keys to move, Space to toggle, Enter to confirm

```
┌─────────────────────────────────────────────────────────┐
│                                                         │
│    ██╗    ██╗ ██████╗ ██╗     ███████╗                  │
│    ██║    ██║██╔═══██╗██║     ██╔════╝                  │
│    ██║ █╗ ██║██║   ██║██║     █████╗                    │
│    ██║███╗██║██║   ██║██║     ██╔══╝                    │
│    ╚███╔███╔╝╚██████╔╝███████╗███████╗                  │
│     ╚══╝╚══╝  ╚═════╝ ╚══════╝╚══════╝                  │
│                                                         │
│    Windows-first cleanup tool                           │
│                                                         │
├─────────────────────────────────────────────────────────┤
│ ACTIONS                                                 │
│                                                         │
│ > Scan                                                  │
│    Find cleanable files (safe, dry-run)                 │
│   Clean                                                 │
│    Delete selected files                                │
│   Analyze                                               │
│    Explore disk usage (folder sizes)                    │
│   Restore                                               │
│    Restore files from last deletion                     │
│   Config                                                │
│    View or modify settings                              │
│                                                         │
├─────────────────────────────────────────────────────────┤
│ Select categories to scan:                              │
│                                                         │
│ [X] Package cache                                       │
│ [X] Temp                                                │
│ [X] Trash                                               │
│ [X] Build                                               │
│ [ ] Browser                                             │
│ [ ] System                                              │
│                                                         │
└─────────────────────────────────────────────────────────┘
[↑↓] Navigate  [Space] Toggle  [Enter] Confirm  [Esc] Exit
```

### Scanning Screen

Shows real-time progress as Wole scans your system. Displays:

- Current category being scanned
- Current file path being processed
- Progress bars for each category
- Running totals of files found and space reclaimable

```
┌─────────────────────────────────────────────────────────┐
│                    Wole - Scanning                      │
├─────────────────────────────────────────────────────────┤
│                                                         │
│ Scanning: Package cache                                 │
│ Current: C:\Users\user\AppData\Local\npm-cache          │
│                                                         │
│ Package cache    ████████████████░░░░  85%              │
│ Temp             ████████████████████ 100%              │
│ Trash            ████████████████████ 100%              │
│ Build            ████████░░░░░░░░░░░░  40%              │
│                                                         │
│ Files found: 156                                        │
│ Space reclaimable: 3.2 GB                               │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Results Screen

After scanning completes, view all found files organized by category:

- **Category summary table**: Shows items and sizes per category
- **File tree**: Hierarchical view of files grouped by category
- **Selection controls**: Toggle categories/files for cleaning
- **Disk space info**: Shows current free space and fun comparisons

```
┌─────────────────────────────────────────────────────────┐
│                    Wole - Results                       │
├─────────────────────────────────────────────────────────┤
│                                                         │
│ Found 362 files (8.1 GB) reclaimable                   │
│ Free space: 45.2 GB                                    │
│                                                        │
│ ┌──────────────┬─────────┬──────────┐                  │
│ │ Category     │ Items   │ Size     │                  │
│ ├──────────────┼─────────┼──────────┤                  │
│ │ Package cache│   45    │  2.3 GB  │                  │
│ │ Temp         │  128    │  456 MB  │                  │
│ │ Trash        │   23    │   89 MB  │                  │
│ │ Build        │   12    │  1.2 GB  │                  │
│ │ Browser      │   67    │  234 MB  │                  │
│ │ System       │   34    │  567 MB  │                  │
│ │ Large        │    8    │  2.1 GB  │                  │
│ │ Old          │   45    │  890 MB  │                  │
│ ├──────────────┼─────────┼──────────┤                  │
│ │ Total        │  362    │  8.1 GB  │                  │
│ └──────────────┴─────────┴──────────┘                  │
│                                                        │
│ File Tree:                                             │
│ > [X] Package cache (45 items, 2.3 GB)                 │
│   [X] Temp (128 items, 456 MB)                         │
│   [X] Trash (23 items, 89 MB)                          │
│   [ ] Large (8 items, 2.1 GB)                          │
│                                                        │
└────────────────────────────────────────────────────────┘
[↑↓] Navigate  [Space] Select  [Enter] Expand  [C] Clean
```

### Confirm Screen

Before deletion, review exactly what will be removed:

- **Warning section**: Shows deletion summary with fun comparisons
- **Category breakdown**: Left panel with summary table
- **File list**: Right panel with expandable file tree
- **Deletion options**: Choose Recycle Bin (recoverable) or Permanent

```
┌─────────────────────────────────────────────────────────┐
│                    Wole - Confirm                       │
├─────────────────────────────────────────────────────────┤
│                                                         │
│ ⚠️  About to delete 362 files (8.1 GB)                 │
│ That's like ~2 AAA game installs worth of space!        │
│ Files will be moved to Recycle Bin (recoverable)        │
│                                                         │
│ ┌──────────────┬─────────┬──────────┐                   │
│ │ Category     │ Items   │ Size     │                   │
│ ├──────────────┼─────────┼──────────┤                   │
│ │ Package cache│   45    │  2.3 GB  │                   │
│ │ Temp         │  128    │  456 MB  │                   │
│ │ ...          │   ...   │   ...    │                   │
│ ├──────────────┼─────────┼──────────┤                   │
│ │ Total        │  362    │  8.1 GB  │                   │
│ └──────────────┴─────────┴──────────┘                   │
│                                                         │
│ File List:                                              │
│ > [X] Package cache                                     │
│     C:\Users\...\npm-cache\package.tgz                  │
│     C:\Users\...\yarn\cache\package.zip                 │
│   [X] Temp                                              │
│     C:\Windows\Temp\installer.log                       │
│                                                         │
│ [Y] Delete (to Recycle Bin)                             │
│ [N] Cancel                                              │
│ [P] Permanent Delete                                    │
│                                                         │
└─────────────────────────────────────────────────────────┘
[Y] Delete  [N] Cancel  [P] Permanent  [Esc] Back
```

### Cleaning Screen

Real-time progress as files are deleted:

- Current category being cleaned
- Current file being deleted
- Progress indicator
- Running count of files cleaned and space freed

```
┌─────────────────────────────────────────────────────────┐
│                    Wole - Cleaning                      │
├─────────────────────────────────────────────────────────┤
│                                                         │
│ Cleaning: Package cache                                 │
│ Current: C:\Users\...\npm-cache\package.tgz             │
│                                                         │
│ Progress: ████████████████░░░░  85%                     │
│                                                         │
│ Files cleaned: 156 / 362                                │
│ Space freed: 3.2 GB / 8.1 GB                            │
│ Errors: 0                                               │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Success Screen

Celebration screen after cleanup completes:

- **Success message**: Confirmation of completion
- **Statistics**: Files cleaned, categories processed, space freed
- **Free space**: Shows current available disk space
- **Fun comparison**: Relates freed space to relatable items

```
┌─────────────────────────────────────────────────────────┐
│                    Wole - Success                       │
├─────────────────────────────────────────────────────────┤
│                                                         │
│   ✓ CLEANUP COMPLETE!                                  │
│                                                        │
│   Space freed: 8.1 GB │ Free space now: 53.3 GB        │
│                                                        │
│ ┌─────────────────────────────────────────────────┐    │
│ │ SUMMARY                                         │    │
│ │                                                 │    │
│ │   Files cleaned:       362                      │    │
│ │   Categories cleaned:  8                        │    │
│ │   Space freed:         8.1 GB                   │    │
│ │   Free space now:      53.3 GB                  │    │
│ │   Errors:              0                        │    │
│ │                                                 │    │
│ │   That's like ~2 AAA game installs worth of     │    │
│ │   space!                                        │    │
│ └─────────────────────────────────────────────────┘    │
│                                                        │
│   Press any key to return to dashboard...              │
│                                                        │
└────────────────────────────────────────────────────────┘
```

### Disk Insights Screen

Interactive exploration of disk usage:

- **Folder tree**: Navigate through directory structure
- **Size visualization**: Progress bars showing relative sizes
- **Largest files**: List of biggest files in current directory
- **Sort options**: Sort by size, name, or file count

```
┌─────────────────────────────────────────────────────────┐
│                    Wole - Disk Insights                 │
├─────────────────────────────────────────────────────────┤
│                                                         │
│ Disk Insights: C:\Users\user                            │
│ Total: 120 GB │ 15,234 files                            │
│                                                         │
│ #  ████████████████████  100.0%  120 GB                 │
│    C:\Users\user                                        │
│                                                         │
│ 1  ████████████████░░░░   85.2%   102 GB  Projects      │
│ 2  ████░░░░░░░░░░░░░░░░   18.5%    22 GB  Downloads     │
│ 3  ██░░░░░░░░░░░░░░░░░░    8.3%    10 GB  Documents     │
│                                                         │
│ Largest Files:                                          │
│   45 GB  C:\Users\user\Projects\game\assets.bin         │
│   12 GB  C:\Users\user\Downloads\movie.mkv              │
│                                                         │
└─────────────────────────────────────────────────────────┘
[↑↓] Navigate  [Enter] Open  [Esc] Back  [S] Sort
```

### Restore Screen

Restore files from your last deletion session:

- **Deletion log**: Shows files deleted in last session
- **Restore options**: Select individual files or restore all
- **Status indicators**: Shows which files were restored successfully

```
┌─────────────────────────────────────────────────────────┐
│                    Wole - Restore                       │
├─────────────────────────────────────────────────────────┤
│                                                         │
│ Restore files from last deletion session                │
│                                                         │
│ [X] C:\Users\user\Documents\file1.txt (2.3 MB)          │
│ [X] C:\Users\user\Downloads\file2.zip (45 MB)           │
│ [ ] C:\Users\user\Temp\file3.tmp (128 KB)               │
│                                                         │
│ Selected: 2 files (47.3 MB)                             │
│                                                         │
│ [R] Restore Selected                                    │
│ [A] Restore All                                         │
│ [Esc] Back                                              │
│                                                         │
└─────────────────────────────────────────────────────────┘
[R] Restore  [A] Restore All  [Space] Toggle  [Esc] Back
```

### Keyboard Shortcuts

All screens support consistent keyboard navigation:

- **Arrow Keys (↑↓)**: Navigate lists and menus
- **Space**: Toggle selections
- **Enter**: Confirm/activate selected item
- **Tab**: Switch between panels (where applicable)
- **Esc**: Go back or exit
- **Y/N**: Quick yes/no responses
- **C**: Quick access to Clean action
- **S**: Sort options (on Disk Insights)

The TUI adapts to your terminal size and provides helpful hints at the bottom of each screen.

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

Config file: `%APPDATA%\wole\config.toml`

```toml
[thresholds]
project_age_days = 14
min_age_days = 30
min_size_mb = 100

[exclusions]
patterns = ["**/important-project/**"]
```

```bash
wole config --show    # View config
wole config --edit    # Edit config
```

## Building from Source

**Prerequisites:** Rust, Visual Studio Build Tools

```powershell
.\build\build.ps1              # Debug
.\build\build.ps1 -Release     # Release
```

Or: `cargo build --release`

**Output:** `target\release\wole.exe`

## Troubleshooting

- **File locked:** File is open in another app. Will be skipped automatically.
- **Long paths:** Handled automatically. Update if issues persist.
- **Symlinks:** Automatically skipped (expected behavior).
- **TUI not working:** Use PowerShell/Windows Terminal, or CLI mode: `wole scan --all`
- **No items found:** Check project activity with `--project-age 0` or file ages with `--min-age 0`

For build issues, see [build/BUILD_TROUBLESHOOTING.md](build/BUILD_TROUBLESHOOTING.md)

## License

[MI](LICENSE)  