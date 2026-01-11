# AI Agent Prompt: Safely Split Large Rust Files - One File, One Feature

## Your Task
Split large Rust source files (>800 lines) into smaller modules following the **"one file, one feature"** principle. Each file should own and handle exactly one distinct feature/component, making the codebase more maintainable and easier to navigate.

## Core Principles
1. **One File, One Feature**: Each file owns exactly one feature/component/responsibility
2. **Self-Contained Ownership**: A file contains everything needed for its feature (structs, functions, tests)
3. **Preserve Exact Behavior**: Copy code exactly - do NOT refactor during splitting
4. **Maintain Public API**: Use re-exports so external code continues to work
5. **Verify Continuously**: Build and test after each change

## Step-by-Step Execution

### STEP 1: Identify Target Files
```bash
# Find large files
find src -name "*.rs" -exec wc -l {} + | sort -rn | head -10
```
**Action**: List files > 800 lines with their line counts. These are your targets.

### STEP 2: Identify Features (One Feature Per File)
For each target file, identify **distinct features** - each feature becomes its own file.

**What is a "feature"?**
A feature is a cohesive unit of functionality that:
- Has a single, clear responsibility (does ONE thing)
- Can be understood independently
- Owns its data structures and operations
- Has clear boundaries (inputs/outputs)
- Can be tested in isolation

**Feature Identification Guidelines:**
Ask yourself:
- "What does this code do?" → If multiple answers, it's multiple features
- "Can I describe this in one sentence?" → If yes, it's likely one feature
- "Would I look for this code by this name?" → That's your feature name

**Examples of features (from your codebase):**
- `ScanCommand` - handles "wole scan" command (one complete feature)
- `CleanCommand` - handles "wole clean" command (one complete feature)
- `BatchDeletion` - handles batch file deletion operations (one feature)
- `PathPrecheck` - validates paths before deletion (one feature)
- `ScanOperation` - TUI scanning operation with progress (one feature)
- `InteractiveMenu` - displays interactive help menu (one feature)

**Bad examples (multiple features mixed):**
- ❌ `commands.rs` - contains multiple command handlers (should be separate files)
- ❌ `deletion.rs` - contains batch, single, and category deletion (should be 3 files)
- ❌ `operations.rs` - contains scan, clean, restore operations (should be 3 files)

**Good examples (one feature per file):**
- ✅ `scan_command.rs` - owns scan command feature
- ✅ `batch_deletion.rs` - owns batch deletion feature
- ✅ `scan_operation.rs` - owns TUI scan operation feature

**Action**: For each target file, list distinct features:
```
File: src/cli.rs (1814 lines)
Features identified:
1. ScanCommand - handles "wole scan" (lines 663-802)
2. CleanCommand - handles "wole clean" (lines 803-956)
3. AnalyzeCommand - handles "wole analyze" (lines 957-1201)
4. ConfigCommand - handles "wole config" (lines 1203-1374)
5. RestoreCommand - handles "wole restore" (lines 1376-1482)
6. OptimizeCommand - handles "wole optimize" (lines 1546-1621)
7. StatusCommand - handles "wole status" (lines 1660-1697)
8. StartupCommand - handles "wole startup" (lines 1699-1787)
9. CliOrchestrator - routes commands (Cli struct, Commands enum, Cli::run)
10. InteractiveMenu - displays help menu (lines 524-643)

Each feature → one file
```

### STEP 3: Create One-File-Per-Feature Structure
**Action**: Create a file for each identified feature. Use descriptive names that match the feature.

**Naming Convention**: `{FeatureName}.rs` (e.g., `scan_command.rs`, `batch_deletion.rs`)

```bash
# Example: For CLI commands, each command is one feature = one file
mkdir -p src/cli/commands
```

**Create feature files** (one per feature):
```rust
// src/cli/commands/scan_command.rs
//! Scan command handler
//!
//! This module owns and handles the "wole scan" command feature.

use crate::cli::ScanOptions;
use crate::scanner;
use crate::output::{self, OutputMode};
// ... other imports

/// Handle the scan command - this file owns this entire feature
pub fn handle_scan(
    // ... parameters
) -> anyhow::Result<()> {
    // All scan command logic here - this file owns it
}
```

**Create `src/cli/mod.rs`** (orchestrator only):
```rust
//! CLI command parsing and execution
//!
//! This module orchestrates commands. Each command is handled by its own feature file.

pub mod commands;

// Re-export main types to maintain API compatibility
pub use crate::cli::Cli;
pub use crate::cli::Commands;
pub use crate::cli::ScanOptions;
```

### STEP 4: Extract Each Feature to Its Own File
**For each feature, create a dedicated file that owns that feature:**

1. **Create feature file** with descriptive name: `{feature_name}.rs`
2. **Copy ALL feature code** exactly (preserve whitespace, comments, formatting)
3. **Include feature's data structures** - if a struct/enum belongs to this feature, move it here
4. **Add necessary imports** at the top
5. **Mark as `pub`** if it needs to be public
6. **Move tests** with their code (`#[cfg(test)]` modules) - tests belong to the feature

**Example - One Feature Per File**:
```rust
// src/cli/commands/scan_command.rs
//! Scan command feature
//!
//! This file owns and handles the entire "wole scan" command feature.
//! All scan-related logic, helpers, and tests live here.

use crate::cli::ScanOptions;
use crate::scanner;
use crate::output::{self, OutputMode};
use crate::config::Config;
use crate::size;
use anyhow::Result;
use std::path::PathBuf;

/// Handle the scan command - this file owns this entire feature
pub fn handle_scan(
    all: bool,
    cache: bool,
    // ... all parameters from Commands::Scan
    path: Option<PathBuf>,
    json: bool,
    project_age: u64,
    min_age: u64,
    min_size: String,
    exclude: Vec<String>,
    output_mode: OutputMode,
) -> Result<()> {
    // EXACT COPY of all scan command code from cli.rs
    // This file owns ALL scan command logic
    // Do NOT modify logic, only extract
}

// If there are scan-specific helpers, they belong here too
fn build_scan_options(...) -> ScanOptions {
    // Helper functions for scan feature
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scan_basic() {
        // Tests for scan feature - owned by this file
    }
}
```

7. **Update orchestrator file** to call feature:
```rust
// src/cli.rs (updated - orchestrator only)
use crate::cli::commands;

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            Commands::Scan { all, cache, ... } => {
                // Delegate to feature file - it owns this feature
                commands::scan_command::handle_scan(
                    all, cache, /* ... */, output_mode
                )?;
            }
            Commands::Clean { ... } => {
                // Delegate to clean feature file
                commands::clean_command::handle_clean(...)?;
            }
            // ... other commands delegate to their feature files
        }
    }
}
```

### STEP 5: Update Module Declarations (One Feature Per Declaration)
**In parent `mod.rs`** - declare each feature file:
```rust
// src/cli/commands/mod.rs
//! Command feature handlers
//!
//! Each file owns one command feature.

pub mod scan_command;    // One feature = one file
pub mod clean_command;   // One feature = one file
pub mod analyze_command; // One feature = one file
pub mod config_command;  // One feature = one file
pub mod restore_command; // One feature = one file
pub mod optimize_command;// One feature = one file
pub mod status_command;  // One feature = one file
pub mod startup_command; // One feature = one file

// Re-export for convenience (optional, maintains API compatibility)
pub use scan_command::handle_scan;
pub use clean_command::handle_clean;
// ... etc
```

**In root module** (if needed):
```rust
// src/lib.rs or src/cli.rs
pub mod cli;
pub use cli::*; // Re-exports maintain API compatibility
```

**Key Principle**: Each `pub mod` declaration = one feature file = one distinct responsibility.

### STEP 6: Verify After Each Extraction
**Run these commands after EACH file extraction**:

```bash
# 1. Compile check
cargo build 2>&1 | head -30

# 2. If errors, check:
# - Import paths correct?
# - Module declared in mod.rs?
# - pub visibility correct?
# - Circular dependencies?

# 3. Once compiling:
cargo test

# 4. Quality checks
cargo fmt --check
cargo clippy -- -D warnings 2>&1 | head -20
```

**If errors occur:**
- **Import errors**: Check `use` statements, verify module paths
- **"cannot find"**: Ensure module declared in parent `mod.rs`
- **Visibility errors**: Add `pub` where needed or use re-exports
- **Circular deps**: Restructure to remove cycle

### STEP 7: Preserve Public API
**Critical**: External code must continue to work.

**Strategy**: Use re-exports in `mod.rs`:
```rust
// src/cli/mod.rs
pub mod commands;

// Re-export to maintain compatibility
pub use commands::scan::handle_scan as handle_scan_command;
// OR keep original path working:
pub use crate::cli::commands::scan;
```

**Verify API**:
```bash
# Check what's public
cargo doc --no-deps --open

# Test external usage still works
cargo build --all-targets
```

### STEP 8: Move Tests
**Action**: Move `#[cfg(test)]` modules with their code:
```rust
// src/cli/commands/scan.rs
// ... main code ...

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scan_basic() {
        // Tests moved here with the code
    }
}
```

**Verify**: `cargo test` should find all tests.

### STEP 9: Final Verification
**After ALL extractions complete**:

```bash
# Full clean build
cargo clean
cargo build --release

# Full test suite
cargo test --all-features

# Lint check
cargo clippy --all-targets --all-features -- -D warnings

# Format check
cargo fmt --check

# Verify file sizes reduced
find src -name "*.rs" -exec wc -l {} + | sort -rn | head -10
```

**Success Criteria**:
- ✅ All files < 500 lines (target achieved)
- ✅ **One file = one feature** (clear ownership boundaries)
- ✅ File names clearly indicate their feature
- ✅ All tests pass
- ✅ No new warnings
- ✅ Code compiles
- ✅ Functionality verified manually
- ✅ Each feature is self-contained

### STEP 10: Update Documentation
**Action**: Add module documentation:
```rust
//! Command handlers for CLI operations
//!
//! This module contains handlers for each CLI command.

pub mod scan;
pub mod clean;
```

**Update project docs** (e.g., `AGENTS.md`):
```markdown
- `src/cli/`: CLI command parsing and execution
  - `commands/`: Individual command handlers
```

## Specific File Strategies - One File, One Feature

### Strategy A: Split CLI by Command (One Command = One File)
**File**: `src/cli.rs` (1814 lines)

**Principle**: Each command is one feature → one file owns that feature.

**Plan**:
1. Keep `Cli` struct, `Commands` enum, `ScanOptions` in `cli.rs` (orchestrator only)
2. Extract each command handler to its own feature file: `cli/commands/{command}_command.rs`
3. Extract menu display to its own feature file: `cli/interactive_menu.rs`
4. Keep `Cli::run()` as thin orchestrator that delegates to feature files

**Structure** (One Feature Per File):
```
src/cli.rs (~150 lines - orchestrator only, delegates to features)
src/cli/
├── mod.rs
├── commands/
│   ├── mod.rs
│   ├── scan_command.rs      (~150 lines) - owns scan feature
│   ├── clean_command.rs     (~200 lines) - owns clean feature
│   ├── analyze_command.rs   (~250 lines) - owns analyze feature
│   ├── config_command.rs    (~150 lines) - owns config feature
│   ├── restore_command.rs   (~100 lines) - owns restore feature
│   ├── optimize_command.rs  (~80 lines)  - owns optimize feature
│   ├── status_command.rs    (~50 lines)   - owns status feature
│   └── startup_command.rs   (~100 lines)  - owns startup feature
└── interactive_menu.rs      (~100 lines)   - owns menu display feature
```

**Key**: Each file name clearly indicates the feature it owns. No shared files, no mixed responsibilities.

### Strategy B: Split Cleaner by Feature (One Feature = One File)
**File**: `src/cleaner.rs` (1315 lines)

**Principle**: Each deletion operation type is one feature → one file owns it.

**Plan**:
1. Keep `clean_all()` in `cleaner.rs` as orchestrator (delegates to features)
2. Extract batch deletion feature → `cleaner/batch_deletion.rs` (owns batch deletion)
3. Extract category-specific cleaning → `cleaner/category_cleaning.rs` (owns category logic)
4. Extract path precheck feature → `cleaner/path_precheck.rs` (owns validation)
5. Extract single file deletion → `cleaner/single_deletion.rs` (owns single-file ops)

**Structure** (One Feature Per File):
```
src/cleaner.rs (~150 lines - orchestrator, delegates to features)
src/cleaner/
├── mod.rs
├── batch_deletion.rs      (~300 lines) - owns batch deletion feature
├── category_cleaning.rs   (~400 lines) - owns category-specific cleaning feature
├── path_precheck.rs       (~150 lines) - owns path validation feature
└── single_deletion.rs     (~100 lines) - owns single file deletion feature
```

**Key**: Each file owns one distinct deletion operation type. Clear boundaries, clear ownership.

### Strategy C: Split TUI by Operation Feature (One Operation = One File)
**File**: `src/tui/mod.rs` (1873 lines)

**Principle**: Each TUI operation is one feature → one file owns that operation.

**Plan**:
1. Keep main event loop in `tui/mod.rs` (orchestrator, delegates to operation features)
2. Extract scan operation feature → `tui/operations/scan_operation.rs` (owns scan UI logic)
3. Extract cleanup operation feature → `tui/operations/cleanup_operation.rs` (owns cleanup UI logic)
4. Extract restore operation feature → `tui/operations/restore_operation.rs` (owns restore UI logic)

**Structure** (One Feature Per File):
```
src/tui/mod.rs (~400 lines - event loop orchestrator, delegates to features)
src/tui/operations/
├── mod.rs
├── scan_operation.rs      (~500 lines) - owns scan operation feature
├── cleanup_operation.rs   (~600 lines) - owns cleanup operation feature
└── restore_operation.rs    (~300 lines) - owns restore operation feature
```

**Key**: Each operation is self-contained in its own file. Event loop delegates, features own their logic.

## Critical Rules - One File, One Feature

### DO:
✅ **One file = one feature** - each file owns exactly one distinct feature/responsibility
✅ **Self-contained features** - a feature file contains everything it needs (structs, functions, tests)
✅ Copy code exactly (preserve formatting, comments)
✅ Extract one feature at a time
✅ Build and test after each feature extraction
✅ Use descriptive file names that match the feature (`scan_command.rs`, not `scan.rs`)
✅ Use re-exports to maintain API compatibility
✅ Move tests with their feature code
✅ Add feature documentation explaining what the file owns

### DON'T:
❌ Put multiple features in one file
❌ Create "shared" or "common" files unless truly shared across many features
❌ Mix responsibilities in a single file
❌ Refactor code during extraction (extract first, refactor later)
❌ Change function signatures
❌ Remove comments or documentation
❌ Create circular dependencies
❌ Break public API
❌ Skip verification steps

### Feature Ownership Checklist:
When extracting a feature, ensure the file:
- [ ] Has a clear, single responsibility (one feature)
- [ ] Contains all code needed for that feature
- [ ] Has a descriptive name matching the feature
- [ ] Is self-contained (minimal external dependencies)
- [ ] Includes its own tests
- [ ] Has documentation explaining what it owns

## Error Resolution

### "unresolved import"
- Check `use` path matches new module location
- Verify module declared in parent `mod.rs`
- Check `pub` visibility

### "cannot find module"
- Ensure `mod.rs` exists in directory
- Verify `pub mod x;` declaration in parent
- Check file naming matches module name

### "private item in public interface"
- Add `pub` to item
- Or use re-export in `mod.rs`

### Circular dependency
- Move shared code to common module
- Restructure dependencies
- Use traits for abstraction

### Tests fail
- Check test imports updated
- Verify `#[cfg(test)]` modules moved
- Ensure test helpers accessible

## Verification Checklist

After completing a split, verify:

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] `cargo clippy` shows no new warnings
- [ ] `cargo fmt --check` passes
- [ ] File sizes reduced (check with `wc -l`)
- [ ] Public API unchanged (`cargo doc`)
- [ ] Manual functionality test passes
- [ ] No circular dependencies
- [ ] Documentation updated

## Success Metrics

- **One File, One Feature**: Each file owns exactly one distinct feature
- **Maintainability**: All files < 500 lines, clear ownership
- **Discoverability**: File names clearly indicate their feature
- **Correctness**: 100% tests passing
- **Compatibility**: Public API unchanged
- **Quality**: No new warnings, properly documented
- **Self-Containment**: Features are self-contained with minimal external deps

## Visual Example: Before and After

### Before (Monolithic File)
```
src/cli.rs (1814 lines)
├── Cli struct
├── Commands enum  
├── ScanOptions struct
├── handle_scan() - 140 lines
├── handle_clean() - 150 lines
├── handle_analyze() - 250 lines
├── handle_config() - 170 lines
├── handle_restore() - 100 lines
├── handle_optimize() - 80 lines
├── handle_status() - 40 lines
├── handle_startup() - 90 lines
└── show_menu() - 120 lines

Problem: One file owns 10+ features, hard to navigate
```

### After (One File, One Feature)
```
src/cli.rs (~150 lines - orchestrator only)
├── Cli struct (delegates to features)
├── Commands enum
├── ScanOptions struct
└── Cli::run() - thin orchestrator

src/cli/commands/
├── mod.rs
├── scan_command.rs      (~150 lines) - owns scan feature
├── clean_command.rs     (~200 lines) - owns clean feature
├── analyze_command.rs   (~250 lines) - owns analyze feature
├── config_command.rs    (~170 lines) - owns config feature
├── restore_command.rs   (~100 lines) - owns restore feature
├── optimize_command.rs  (~80 lines)  - owns optimize feature
├── status_command.rs   (~40 lines)  - owns status feature
└── startup_command.rs   (~90 lines)  - owns startup feature

src/cli/
└── interactive_menu.rs  (~120 lines) - owns menu display feature

Result: Each feature is self-contained, easy to find and modify
```

**Key Benefits:**
- ✅ Find scan code? → `scan_command.rs`
- ✅ Modify clean feature? → `clean_command.rs` (all code in one place)
- ✅ Add new command? → Create new `{command}_command.rs` file
- ✅ Test a feature? → Tests are in the feature file

## Execution Template

```bash
# 1. Identify targets
find src -name "*.rs" -exec wc -l {} + | sort -rn | head -5

# 2. For each target file:
#    a. Identify distinct features (one feature = one file)
#    b. Create feature files (one per feature)
#    c. Extract code to feature files (one feature at a time)
#    d. Verify after each extraction (cargo build && cargo test)
#    e. Update documentation

# 3. Final verification
cargo clean && cargo build --release
cargo test --all-features
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# 4. Verify file sizes and feature ownership
find src -name "*.rs" -exec wc -l {} + | sort -rn | head -10
# Each file should be < 500 lines and own one clear feature
```

## Remember - One File, One Feature Philosophy

**Your goal**: Split large files following the "one file, one feature" principle. Each file should own exactly one distinct feature/responsibility, making it easy to:
- Find code for a specific feature (look for `{feature_name}.rs`)
- Understand what a file does (it's in the name and docs)
- Modify a feature (all its code is in one place)
- Test a feature (tests are with the feature code)

**Key Principle**: Think like the C++ example - `ColorButton` has its own files, `FileTreeControl` has its own files. Each feature is self-contained and owns its functionality.

**Extract first, improve later**. Functionality must remain 100% identical. Focus on clear ownership boundaries, not code improvements.
