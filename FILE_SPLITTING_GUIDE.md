# File Splitting Guide for AI Agents

## Overview
This guide provides a systematic, verifiable, and accurate approach for splitting large Rust source files into smaller, maintainable modules. The process ensures code correctness, preserves functionality, and maintains project structure.

## Prerequisites
- Understanding of Rust module system (`mod`, `pub mod`, `use`)
- Knowledge of the project's architecture and dependencies
- Access to the full codebase for dependency analysis
- Ability to run `cargo build` and `cargo test` for verification

## Step-by-Step Process

### Phase 1: Analysis & Planning

#### Step 1.1: Identify Split Candidates
**Action:**
- Analyze file sizes (target: files > 800 lines)
- Identify logical boundaries (functions, structs, enums, traits grouped by responsibility)
- Document dependencies between code sections

**Verification:**
- List all files > 800 lines: `find src -name "*.rs" -exec wc -l {} + | sort -rn | head -10`
- Create a dependency graph showing imports/exports
- Identify natural boundaries (e.g., command handlers, category-specific logic)

**Output:**
- List of files to split with line counts
- Proposed module structure for each file
- Dependency map

#### Step 1.2: Identify Logical Boundaries
**Action:**
For each candidate file, identify:
1. **Command/Feature Groups**: Related functions handling similar operations
2. **Data Structures**: Types that can be moved to separate modules
3. **Helper Functions**: Utility functions that can be extracted
4. **Public API**: What must remain public vs. what can be private

**Verification:**
- Review function signatures and their relationships
- Check for circular dependencies (must be avoided)
- Identify shared types/utilities

**Example Boundaries:**
- `cli.rs`: Split by command (Scan, Clean, Analyze, Config, etc.)
- `cleaner.rs`: Split by deletion strategy (batch, individual, category-specific)
- `tui/mod.rs`: Split by screen/operation (scanning, cleaning, restoring)

#### Step 1.3: Create Splitting Plan
**Action:**
For each file, create a detailed plan:
```
File: src/cli.rs (1814 lines)
├── Keep in cli.rs:
│   ├── Cli struct definition
│   ├── Commands enum
│   ├── ScanOptions struct
│   └── Cli::run() method (orchestration only)
├── Extract to cli/commands/:
│   ├── scan.rs - Scan command handler
│   ├── clean.rs - Clean command handler
│   ├── analyze.rs - Analyze command handler
│   ├── config.rs - Config command handler
│   ├── restore.rs - Restore command handler
│   ├── optimize.rs - Optimize command handler
│   ├── status.rs - Status command handler
│   └── startup.rs - Startup command handler
└── Extract to cli/helpers/:
    ├── menu.rs - Interactive menu display
    └── options.rs - Option parsing helpers
```

**Verification:**
- Plan reviewed for logical grouping
- No circular dependencies introduced
- Public API surface remains unchanged

### Phase 2: Implementation

#### Step 2.1: Create Module Structure
**Action:**
1. Create new directory structure (e.g., `src/cli/commands/`)
2. Create `mod.rs` files with proper `pub mod` declarations
3. Update parent module's `mod.rs` to include new modules

**Verification:**
```bash
# Structure should be created
ls -R src/cli/

# mod.rs should exist with proper declarations
cat src/cli/mod.rs
```

**Example:**
```rust
// src/cli/mod.rs
pub mod commands;
pub mod helpers;

pub use commands::*;
pub use helpers::*;

// Re-export main types
pub use crate::cli::Cli;
pub use crate::cli::Commands;
pub use crate::cli::ScanOptions;
```

#### Step 2.2: Extract Code Sections
**Action:**
For each planned extraction:
1. Copy code to new module file
2. Add necessary imports
3. Ensure `pub` visibility where needed
4. Update original file to use new module

**Rules:**
- **Preserve exact code**: Copy-paste, don't rewrite
- **Maintain visibility**: Keep `pub` where needed for external access
- **Add imports**: Include all necessary `use` statements
- **Keep tests**: Move `#[cfg(test)]` modules with their code

**Verification:**
- Code compiles: `cargo build --message-format=short 2>&1 | head -20`
- No new warnings introduced
- File sizes reduced appropriately

**Example Extraction:**
```rust
// Before: src/cli.rs (lines 663-802)
match command {
    Commands::Scan { ... } => {
        // 140 lines of scan logic
    }
    // ...
}

// After: src/cli/commands/scan.rs
use crate::cli::{Cli, Commands, ScanOptions};
use crate::scanner;
use crate::output;

pub fn handle_scan(
    options: ScanOptions,
    path: Option<PathBuf>,
    // ... other params
) -> anyhow::Result<()> {
    // 140 lines of scan logic (exact copy)
}

// After: src/cli.rs (updated)
match command {
    Commands::Scan { ... } => {
        commands::scan::handle_scan(...)?;
    }
    // ...
}
```

#### Step 2.3: Update Imports
**Action:**
1. Update all files importing the split module
2. Add new module paths to imports
3. Ensure no broken imports

**Verification:**
```bash
# Check for import errors
cargo build 2>&1 | grep -i "unresolved\|cannot find\|not found"

# Verify all imports resolve
cargo check --message-format=short
```

**Common Import Patterns:**
```rust
// Before
use crate::cli::ScanOptions;

// After (if ScanOptions moved to cli::helpers)
use crate::cli::helpers::ScanOptions;
// OR if re-exported in cli/mod.rs
use crate::cli::ScanOptions; // Still works if re-exported
```

#### Step 2.4: Preserve Public API
**Action:**
1. Identify all public items (`pub fn`, `pub struct`, `pub enum`)
2. Ensure they remain accessible via original paths
3. Use re-exports in `mod.rs` to maintain compatibility

**Verification:**
```bash
# Check public API surface
cargo doc --no-deps --open  # Review public items

# Test that external code still compiles
cargo build --all-targets
```

**Re-export Pattern:**
```rust
// src/cli/mod.rs
pub mod commands;
pub mod helpers;

// Re-export to maintain API compatibility
pub use commands::handle_scan;
pub use helpers::ScanOptions;
```

### Phase 3: Verification

#### Step 3.1: Compilation Check
**Action:**
```bash
cargo clean
cargo build
```

**Verification Criteria:**
- ✅ Build succeeds without errors
- ✅ No new warnings (or only expected deprecation warnings)
- ✅ All targets build (lib, bin, tests, examples if any)

**If errors occur:**
1. Check import paths
2. Verify module declarations in `mod.rs`
3. Ensure `pub` visibility is correct
4. Check for circular dependencies

#### Step 3.2: Test Suite
**Action:**
```bash
cargo test
```

**Verification Criteria:**
- ✅ All existing tests pass
- ✅ No test compilation errors
- ✅ Test coverage maintained (tests moved with their code)

**If tests fail:**
1. Check test imports
2. Verify test modules are in correct files
3. Ensure test helpers are accessible

#### Step 3.3: Functional Verification
**Action:**
Run key functionality manually:
```bash
# For CLI tool
cargo run -- --help
cargo run -- scan --all
cargo run -- clean --all --dry-run

# For library (if applicable)
cargo test --lib
```

**Verification Criteria:**
- ✅ Help text displays correctly
- ✅ Commands execute without errors
- ✅ Output format unchanged
- ✅ No regressions in behavior

#### Step 3.4: Code Quality Checks
**Action:**
```bash
cargo fmt --check
cargo clippy -- -D warnings
```

**Verification Criteria:**
- ✅ Code is properly formatted
- ✅ No clippy warnings introduced
- ✅ Code style consistent with project

#### Step 3.5: Dependency Verification
**Action:**
1. Verify no circular dependencies created
2. Check module dependency graph
3. Ensure logical separation maintained

**Verification:**
```bash
# Check for circular dependencies (manual review)
# Look for mod declarations that create cycles

# Verify module structure
tree src/ -I target
```

**Dependency Rules:**
- ✅ Parent modules can depend on children
- ✅ Sibling modules can depend on each other (carefully)
- ❌ Children should NOT depend on parent (creates cycles)
- ❌ Avoid deep dependency chains (>3 levels)

### Phase 4: Documentation & Cleanup

#### Step 4.1: Update Module Documentation
**Action:**
Add module-level documentation:
```rust
//! Command handlers for CLI operations
//!
//! This module contains handlers for each CLI command, extracted from
//! the main `cli.rs` file for better maintainability.

pub mod scan;
pub mod clean;
// ...
```

**Verification:**
- All new modules have `//!` module docs
- Public functions have `///` doc comments
- Examples in docs still work

#### Step 4.2: Update Project Documentation
**Action:**
Update `AGENTS.md` or similar with new structure:
```markdown
## Project Structure & Module Organization
- `src/cli/`: CLI command parsing and execution
  - `commands/`: Individual command handlers
  - `helpers/`: Shared CLI utilities
```

#### Step 4.3: Final Verification
**Action:**
Complete end-to-end check:
```bash
# Full build
cargo clean && cargo build --release

# Full test suite
cargo test --all-features

# Lint check
cargo clippy --all-targets --all-features -- -D warnings

# Format check
cargo fmt --check
```

**Success Criteria:**
- ✅ All checks pass
- ✅ File sizes reduced (target: < 500 lines per file)
- ✅ No functionality lost
- ✅ Code is more maintainable

## Specific File Splitting Strategies

### Strategy 1: Split by Command/Feature (CLI)
**Target:** `src/cli.rs` (1814 lines)

**Approach:**
1. Extract each `Commands` variant handler to `cli/commands/{command}.rs`
2. Keep `Cli` struct and `Commands` enum in `cli.rs`
3. Keep `Cli::run()` as thin orchestrator

**Structure:**
```
src/cli/
├── mod.rs          # Cli struct, Commands enum, Cli::run()
├── commands/
│   ├── mod.rs      # Re-exports all command handlers
│   ├── scan.rs     # Scan command (~150 lines)
│   ├── clean.rs    # Clean command (~200 lines)
│   ├── analyze.rs  # Analyze command (~250 lines)
│   ├── config.rs   # Config command (~150 lines)
│   ├── restore.rs  # Restore command (~100 lines)
│   ├── optimize.rs # Optimize command (~80 lines)
│   ├── status.rs   # Status command (~50 lines)
│   └── startup.rs  # Startup command (~100 lines)
└── helpers/
    ├── mod.rs
    ├── menu.rs     # Interactive menu (~100 lines)
    └── options.rs  # Option parsing (~50 lines)
```

**Verification:**
- Each command handler is self-contained
- `Cli::run()` delegates to command handlers
- Public API unchanged (re-exports maintain compatibility)

### Strategy 2: Split by Operation Type (Cleaner)
**Target:** `src/cleaner.rs` (1315 lines)

**Approach:**
1. Extract batch deletion logic
2. Extract category-specific cleaning
3. Extract precheck/validation logic
4. Keep main `clean_all()` as orchestrator

**Structure:**
```
src/cleaner/
├── mod.rs           # Main clean_all(), public API
├── batch.rs         # Batch deletion (~300 lines)
├── categories.rs    # Category-specific cleaning (~400 lines)
├── precheck.rs      # Path validation/precheck (~150 lines)
└── single.rs        # Single file deletion (~100 lines)
```

**Verification:**
- Batch operations isolated
- Category logic separated
- Safety checks preserved

### Strategy 3: Split by Screen/Operation (TUI)
**Target:** `src/tui/mod.rs` (1873 lines)

**Approach:**
1. Extract screen-specific operations
2. Extract event handling
3. Keep main event loop in `mod.rs`

**Structure:**
```
src/tui/
├── mod.rs           # Main event loop (~400 lines)
├── operations/
│   ├── mod.rs
│   ├── scan.rs      # Scan operation (~500 lines)
│   ├── clean.rs     # Clean operation (~600 lines)
│   └── restore.rs   # Restore operation (~300 lines)
└── (existing screens/, widgets/, etc.)
```

**Verification:**
- Each operation is self-contained
- Event loop remains in mod.rs
- Screen rendering unchanged

## Common Pitfalls & Solutions

### Pitfall 1: Circular Dependencies
**Problem:** Module A imports B, B imports A
**Solution:**
- Move shared code to common module
- Use dependency inversion (traits/interfaces)
- Restructure to remove cycle

### Pitfall 2: Broken Imports
**Problem:** Code can't find moved items
**Solution:**
- Use re-exports in `mod.rs`
- Update all import paths
- Verify with `cargo check`

### Pitfall 3: Lost Functionality
**Problem:** Tests pass but behavior changed
**Solution:**
- Copy code exactly (don't refactor during split)
- Run full integration tests
- Manual verification of key features

### Pitfall 4: Visibility Issues
**Problem:** `pub` items not accessible
**Solution:**
- Check `pub` modifiers
- Use re-exports in parent `mod.rs`
- Verify with `cargo doc`

### Pitfall 5: Test Failures
**Problem:** Tests can't find moved code
**Solution:**
- Move `#[cfg(test)]` modules with code
- Update test imports
- Ensure test helpers accessible

## Verification Checklist

Before considering a split complete, verify:

- [ ] Code compiles: `cargo build`
- [ ] Tests pass: `cargo test`
- [ ] No new warnings: `cargo clippy`
- [ ] Formatted: `cargo fmt --check`
- [ ] Public API unchanged: `cargo doc`
- [ ] File sizes reduced: `wc -l src/**/*.rs`
- [ ] No circular dependencies: Manual review
- [ ] Documentation updated: Module docs added
- [ ] Functionality verified: Manual testing
- [ ] Integration works: End-to-end test

## Success Metrics

A successful split should achieve:

1. **Maintainability**: Files < 500 lines each
2. **Correctness**: All tests pass, no regressions
3. **Clarity**: Logical grouping, clear module boundaries
4. **Compatibility**: Public API unchanged
5. **Quality**: No new warnings, properly documented

## Example: Complete Split Workflow

```bash
# 1. Analyze
find src -name "*.rs" -exec wc -l {} + | sort -rn | head -5

# 2. Plan (document in this file or separate plan.md)

# 3. Create structure
mkdir -p src/cli/commands src/cli/helpers

# 4. Extract code (manual or tool-assisted)

# 5. Update mod.rs files

# 6. Verify
cargo clean && cargo build
cargo test
cargo clippy
cargo fmt --check

# 7. Test functionality
cargo run -- --help
cargo run -- scan --all

# 8. Document
# Update AGENTS.md with new structure
```

## Conclusion

This guide provides a systematic approach to safely splitting large Rust files. The key principles are:

1. **Plan First**: Understand dependencies and boundaries
2. **Extract Carefully**: Copy code exactly, don't refactor during split
3. **Verify Thoroughly**: Build, test, lint, and manually verify
4. **Maintain API**: Use re-exports to preserve compatibility
5. **Document Changes**: Update docs and module comments

Following this process ensures code correctness while improving maintainability.
