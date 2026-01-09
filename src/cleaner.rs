use crate::categories;
use crate::history::DeletionLog;
use crate::output::{OutputMode, ScanResults};
use crate::progress;
use crate::theme::Theme;
use crate::utils;
use anyhow::{Context, Result};
use std::io::{self, Write};
use std::path::Path;

/// Check if a file is locked by another process (Windows-specific)
///
/// Attempts to open the file with write access. If it fails with
/// ERROR_SHARING_VIOLATION (32), the file is locked.
#[cfg(windows)]
fn is_file_locked(path: &Path) -> bool {
    use std::fs::OpenOptions;

    if !path.is_file() {
        return false;
    }

    match OpenOptions::new().read(true).write(true).open(path) {
        Ok(_) => false,
        Err(e) if e.raw_os_error() == Some(32) => true, // ERROR_SHARING_VIOLATION
        Err(_) => false,
    }
}

#[cfg(not(windows))]
fn is_file_locked(_path: &Path) -> bool {
    // On Unix, file locking works differently (advisory locks)
    // We don't check for locks here as files can still be deleted
    false
}

/// Clean all categories based on scan results
///
/// Handles confirmation prompts, error tracking, and provides progress feedback
pub fn clean_all(
    results: &ScanResults,
    skip_confirm: bool,
    mode: OutputMode,
    permanent: bool,
    dry_run: bool,
) -> Result<()> {
    let total_items = results.cache.items
        + results.app_cache.items
        + results.temp.items
        + results.trash.items
        + results.build.items
        + results.downloads.items
        + results.large.items
        + results.old.items
        + results.browser.items
        + results.system.items
        + results.empty.items
        + results.duplicates.items;
    let total_bytes = results.cache.size_bytes
        + results.app_cache.size_bytes
        + results.temp.size_bytes
        + results.trash.size_bytes
        + results.build.size_bytes
        + results.downloads.size_bytes
        + results.large.size_bytes
        + results.old.size_bytes
        + results.browser.size_bytes
        + results.system.size_bytes
        + results.empty.size_bytes
        + results.duplicates.size_bytes;

    if total_items == 0 {
        if mode != OutputMode::Quiet {
            println!("{}", Theme::success("Nothing to clean."));
        }
        return Ok(());
    }

    if dry_run && mode != OutputMode::Quiet {
        println!(
            "{}",
            Theme::warning_msg("DRY RUN MODE - No files will be deleted")
        );
        println!();
    }

    if permanent && mode != OutputMode::Quiet {
        println!(
            "{}",
            Theme::error("PERMANENT DELETE MODE - Files will bypass Recycle Bin")
        );
    }

    if !skip_confirm && !dry_run {
        print!(
            "Delete {} items ({})? [y/N]: ",
            Theme::value(&total_items.to_string()),
            Theme::warning(&bytesize::to_string(total_bytes, true))
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("{}", Theme::muted("Cancelled."));
            return Ok(());
        }
    }

    // Create progress bar with ETA
    let progress = if mode != OutputMode::Quiet {
        Some(progress::create_progress_bar_with_eta(
            total_items as u64,
            "Cleaning...",
        ))
    } else {
        None
    };

    // Create deletion log for audit trail (not used in dry run)
    let mut history = if !dry_run {
        Some(DeletionLog::new())
    } else {
        None
    };

    let mut cleaned = 0u64;
    let mut cleaned_bytes = 0u64;
    let mut errors = 0;

    // Clean cache
    if results.cache.items > 0 {
        if let Some(ref pb) = progress {
            pb.set_message("Cleaning cache...");
        }
        for path in &results.cache.paths {
            let size = utils::safe_metadata(path).map(|m| m.len()).unwrap_or(0);
            if dry_run {
                cleaned += 1;
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
            } else {
                match clean_path(path, permanent) {
                    Ok(()) => {
                        cleaned += 1;
                        if let Some(ref pb) = progress {
                            pb.inc(1);
                        }
                        if let Some(ref mut log) = history {
                            log.log_success(path, size, "cache", permanent);
                        }
                    }
                    Err(e) => {
                        errors += 1;
                        if let Some(ref mut log) = history {
                            log.log_failure(path, size, "cache", permanent, &e.to_string());
                        }
                        if mode != OutputMode::Quiet {
                            eprintln!(
                                "[WARNING] Failed to clean {}: {}",
                                Theme::secondary(&path.display().to_string()),
                                Theme::error(&e.to_string())
                            );
                        }
                    }
                }
            }
        }
        cleaned_bytes += results.cache.size_bytes;
    }

    // Clean application cache
    if results.app_cache.items > 0 {
        if let Some(ref pb) = progress {
            pb.set_message("Cleaning application cache...");
        }
        for path in &results.app_cache.paths {
            let size = utils::safe_metadata(path).map(|m| m.len()).unwrap_or(0);
            if dry_run {
                cleaned += 1;
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
            } else {
                match clean_path(path, permanent) {
                    Ok(()) => {
                        cleaned += 1;
                        if let Some(ref pb) = progress {
                            pb.inc(1);
                        }
                        if let Some(ref mut log) = history {
                            log.log_success(path, size, "app_cache", permanent);
                        }
                    }
                    Err(e) => {
                        errors += 1;
                        if let Some(ref mut log) = history {
                            log.log_failure(path, size, "app_cache", permanent, &e.to_string());
                        }
                        if mode != OutputMode::Quiet {
                            eprintln!(
                                "[WARNING] Failed to clean {}: {}",
                                Theme::secondary(&path.display().to_string()),
                                Theme::error(&e.to_string())
                            );
                        }
                    }
                }
            }
        }
        cleaned_bytes += results.app_cache.size_bytes;
    }

    // Clean temp
    if results.temp.items > 0 {
        if let Some(ref pb) = progress {
            pb.set_message("Cleaning temp files...");
        }
        for path in &results.temp.paths {
            let size = utils::safe_metadata(path).map(|m| m.len()).unwrap_or(0);
            if dry_run {
                cleaned += 1;
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
            } else {
                match clean_path(path, permanent) {
                    Ok(()) => {
                        cleaned += 1;
                        if let Some(ref pb) = progress {
                            pb.inc(1);
                        }
                        if let Some(ref mut log) = history {
                            log.log_success(path, size, "temp", permanent);
                        }
                    }
                    Err(e) => {
                        errors += 1;
                        if let Some(ref mut log) = history {
                            log.log_failure(path, size, "temp", permanent, &e.to_string());
                        }
                        if mode != OutputMode::Quiet {
                            eprintln!(
                                "[WARNING] Failed to clean {}: {}",
                                Theme::secondary(&path.display().to_string()),
                                Theme::error(&e.to_string())
                            );
                        }
                    }
                }
            }
        }
        cleaned_bytes += results.temp.size_bytes;
    }

    // Clean trash
    if results.trash.items > 0 {
        if let Some(ref pb) = progress {
            pb.set_message("Emptying Recycle Bin...");
        }
        if dry_run {
            cleaned += results.trash.items as u64;
            if let Some(ref pb) = progress {
                pb.inc(results.trash.items as u64);
            }
            cleaned_bytes += results.trash.size_bytes;
        } else {
            match categories::trash::clean() {
                Ok(()) => {
                    cleaned += results.trash.items as u64;
                    if let Some(ref pb) = progress {
                        pb.inc(results.trash.items as u64);
                    }
                    cleaned_bytes += results.trash.size_bytes;
                    if let Some(ref mut log) = history {
                        log.log_success(
                            Path::new("Recycle Bin"),
                            results.trash.size_bytes,
                            "trash",
                            true,
                        );
                    }
                }
                Err(e) => {
                    errors += 1;
                    if let Some(ref mut log) = history {
                        log.log_failure(
                            Path::new("Recycle Bin"),
                            results.trash.size_bytes,
                            "trash",
                            true,
                            &e.to_string(),
                        );
                    }
                    if mode != OutputMode::Quiet {
                        eprintln!(
                            "[WARNING] Failed to empty Recycle Bin: {}",
                            Theme::error(&e.to_string())
                        );
                    }
                }
            }
        }
    }

    // Clean build artifacts
    if results.build.items > 0 {
        if let Some(ref pb) = progress {
            pb.set_message("Cleaning build artifacts...");
        }
        for path in &results.build.paths {
            let size = if path.is_dir() {
                utils::calculate_dir_size(path)
            } else {
                utils::safe_metadata(path).map(|m| m.len()).unwrap_or(0)
            };
            if dry_run {
                cleaned += 1;
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
            } else {
                match clean_path(path, permanent) {
                    Ok(()) => {
                        cleaned += 1;
                        if let Some(ref pb) = progress {
                            pb.inc(1);
                        }
                        if let Some(ref mut log) = history {
                            log.log_success(path, size, "build", permanent);
                        }
                    }
                    Err(e) => {
                        errors += 1;
                        if let Some(ref mut log) = history {
                            log.log_failure(path, size, "build", permanent, &e.to_string());
                        }
                        if mode != OutputMode::Quiet {
                            eprintln!(
                                "[WARNING] Failed to clean {}: {}",
                                Theme::secondary(&path.display().to_string()),
                                Theme::error(&e.to_string())
                            );
                        }
                    }
                }
            }
        }
        cleaned_bytes += results.build.size_bytes;
    }

    // Clean downloads
    if results.downloads.items > 0 {
        if let Some(ref pb) = progress {
            pb.set_message("Cleaning old downloads...");
        }
        for path in &results.downloads.paths {
            let size = utils::safe_metadata(path).map(|m| m.len()).unwrap_or(0);
            if dry_run {
                cleaned += 1;
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
            } else {
                match clean_path(path, permanent) {
                    Ok(()) => {
                        cleaned += 1;
                        if let Some(ref pb) = progress {
                            pb.inc(1);
                        }
                        if let Some(ref mut log) = history {
                            log.log_success(path, size, "downloads", permanent);
                        }
                    }
                    Err(e) => {
                        errors += 1;
                        if let Some(ref mut log) = history {
                            log.log_failure(path, size, "downloads", permanent, &e.to_string());
                        }
                        if mode != OutputMode::Quiet {
                            eprintln!(
                                "[WARNING] Failed to clean {}: {}",
                                Theme::secondary(&path.display().to_string()),
                                Theme::error(&e.to_string())
                            );
                        }
                    }
                }
            }
        }
        cleaned_bytes += results.downloads.size_bytes;
    }

    // Clean large files
    if results.large.items > 0 {
        if let Some(ref pb) = progress {
            pb.set_message("Cleaning large files...");
        }
        for path in &results.large.paths {
            let size = utils::safe_metadata(path).map(|m| m.len()).unwrap_or(0);
            if dry_run {
                cleaned += 1;
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
            } else {
                match clean_path(path, permanent) {
                    Ok(()) => {
                        cleaned += 1;
                        if let Some(ref pb) = progress {
                            pb.inc(1);
                        }
                        if let Some(ref mut log) = history {
                            log.log_success(path, size, "large", permanent);
                        }
                    }
                    Err(e) => {
                        errors += 1;
                        if let Some(ref mut log) = history {
                            log.log_failure(path, size, "large", permanent, &e.to_string());
                        }
                        if mode != OutputMode::Quiet {
                            eprintln!(
                                "[WARNING] Failed to clean {}: {}",
                                Theme::secondary(&path.display().to_string()),
                                Theme::error(&e.to_string())
                            );
                        }
                    }
                }
            }
        }
        cleaned_bytes += results.large.size_bytes;
    }

    // Clean old files
    if results.old.items > 0 {
        if let Some(ref pb) = progress {
            pb.set_message("Cleaning old files...");
        }
        for path in &results.old.paths {
            let size = utils::safe_metadata(path).map(|m| m.len()).unwrap_or(0);
            if dry_run {
                cleaned += 1;
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
            } else {
                match clean_path(path, permanent) {
                    Ok(()) => {
                        cleaned += 1;
                        if let Some(ref pb) = progress {
                            pb.inc(1);
                        }
                        if let Some(ref mut log) = history {
                            log.log_success(path, size, "old", permanent);
                        }
                    }
                    Err(e) => {
                        errors += 1;
                        if let Some(ref mut log) = history {
                            log.log_failure(path, size, "old", permanent, &e.to_string());
                        }
                        if mode != OutputMode::Quiet {
                            eprintln!(
                                "[WARNING] Failed to clean {}: {}",
                                Theme::secondary(&path.display().to_string()),
                                Theme::error(&e.to_string())
                            );
                        }
                    }
                }
            }
        }
        cleaned_bytes += results.old.size_bytes;
    }

    // Clean browser caches
    if results.browser.items > 0 {
        if let Some(ref pb) = progress {
            pb.set_message("Cleaning browser caches...");
        }
        for path in &results.browser.paths {
            let size = if path.is_dir() {
                utils::calculate_dir_size(path)
            } else {
                utils::safe_metadata(path).map(|m| m.len()).unwrap_or(0)
            };
            if dry_run {
                cleaned += 1;
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
            } else {
                match categories::browser::clean(path) {
                    Ok(()) => {
                        cleaned += 1;
                        if let Some(ref pb) = progress {
                            pb.inc(1);
                        }
                        if let Some(ref mut log) = history {
                            log.log_success(path, size, "browser", permanent);
                        }
                    }
                    Err(e) => {
                        errors += 1;
                        if let Some(ref mut log) = history {
                            log.log_failure(path, size, "browser", permanent, &e.to_string());
                        }
                        if mode != OutputMode::Quiet {
                            eprintln!(
                                "[WARNING] Failed to clean {}: {}",
                                Theme::secondary(&path.display().to_string()),
                                Theme::error(&e.to_string())
                            );
                        }
                    }
                }
            }
        }
        cleaned_bytes += results.browser.size_bytes;
    }

    // Clean system caches
    if results.system.items > 0 {
        if let Some(ref pb) = progress {
            pb.set_message("Cleaning system caches...");
        }
        for path in &results.system.paths {
            let size = if path.is_dir() {
                utils::calculate_dir_size(path)
            } else {
                utils::safe_metadata(path).map(|m| m.len()).unwrap_or(0)
            };
            if dry_run {
                cleaned += 1;
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
            } else {
                match categories::system::clean(path) {
                    Ok(()) => {
                        cleaned += 1;
                        if let Some(ref pb) = progress {
                            pb.inc(1);
                        }
                        if let Some(ref mut log) = history {
                            log.log_success(path, size, "system", permanent);
                        }
                    }
                    Err(e) => {
                        errors += 1;
                        if let Some(ref mut log) = history {
                            log.log_failure(path, size, "system", permanent, &e.to_string());
                        }
                        if mode != OutputMode::Quiet {
                            eprintln!(
                                "[WARNING] Failed to clean {}: {}",
                                Theme::secondary(&path.display().to_string()),
                                Theme::error(&e.to_string())
                            );
                        }
                    }
                }
            }
        }
        cleaned_bytes += results.system.size_bytes;
    }

    // Clean empty folders
    if results.empty.items > 0 {
        if let Some(ref pb) = progress {
            pb.set_message("Cleaning empty folders...");
        }
        for path in &results.empty.paths {
            if dry_run {
                cleaned += 1;
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
            } else {
                match categories::empty::clean(path) {
                    Ok(()) => {
                        cleaned += 1;
                        if let Some(ref pb) = progress {
                            pb.inc(1);
                        }
                        if let Some(ref mut log) = history {
                            log.log_success(path, 0, "empty", permanent);
                        }
                    }
                    Err(e) => {
                        errors += 1;
                        if let Some(ref mut log) = history {
                            log.log_failure(path, 0, "empty", permanent, &e.to_string());
                        }
                        if mode != OutputMode::Quiet {
                            eprintln!(
                                "[WARNING] Failed to clean {}: {}",
                                Theme::secondary(&path.display().to_string()),
                                Theme::error(&e.to_string())
                            );
                        }
                    }
                }
            }
        }
        cleaned_bytes += results.empty.size_bytes;
    }

    // Clean duplicate files
    if results.duplicates.items > 0 {
        if let Some(ref pb) = progress {
            pb.set_message("Cleaning duplicate files...");
        }
        for path in &results.duplicates.paths {
            let size = utils::safe_metadata(path).map(|m| m.len()).unwrap_or(0);
            if dry_run {
                cleaned += 1;
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
            } else {
                match clean_path(path, permanent) {
                    Ok(()) => {
                        cleaned += 1;
                        if let Some(ref pb) = progress {
                            pb.inc(1);
                        }
                        if let Some(ref mut log) = history {
                            log.log_success(path, size, "duplicates", permanent);
                        }
                    }
                    Err(e) => {
                        errors += 1;
                        if let Some(ref mut log) = history {
                            log.log_failure(path, size, "duplicates", permanent, &e.to_string());
                        }
                        if mode != OutputMode::Quiet {
                            eprintln!(
                                "[WARNING] Failed to clean {}: {}",
                                Theme::secondary(&path.display().to_string()),
                                Theme::error(&e.to_string())
                            );
                        }
                    }
                }
            }
        }
        cleaned_bytes += results.duplicates.size_bytes;
    }

    // Finish progress bar
    if let Some(pb) = progress {
        pb.finish_and_clear();
    }

    // Save history log (if not dry run)
    let log_path = if let Some(log) = history {
        match log.save() {
            Ok(path) => Some(path),
            Err(e) => {
                if mode != OutputMode::Quiet {
                    eprintln!("[WARNING] Failed to save deletion log: {}", e);
                }
                None
            }
        }
    } else {
        None
    };

    // Print summary
    if mode != OutputMode::Quiet {
        println!();
        if dry_run {
            println!(
                "[DRY RUN] Complete: {} items would be cleaned ({}), {} errors",
                Theme::value(&cleaned.to_string()),
                Theme::size(&bytesize::to_string(cleaned_bytes, true)),
                Theme::error(&errors.to_string())
            );
        } else if errors > 0 {
            println!(
                "[WARNING] Cleanup complete: {} items cleaned ({}), {} errors",
                Theme::success(&cleaned.to_string()),
                Theme::success(&bytesize::to_string(cleaned_bytes, true)),
                Theme::error(&errors.to_string())
            );
        } else {
            println!(
                "[OK] Cleanup complete: {} items cleaned, {} freed!",
                Theme::success(&cleaned.to_string()),
                Theme::success(&bytesize::to_string(cleaned_bytes, true))
            );
        }

        // Print log path if saved
        if let Some(path) = log_path {
            println!(
                "{}",
                Theme::muted(&format!("Deletion log saved to: {}", path.display()))
            );
        }
    }

    Ok(())
}

/// Clean a single path, optionally permanently
///
/// Features:
/// - Checks for locked files before deletion (Windows)
/// - Uses long path support for paths > 260 characters
/// - Provides clear error messages
pub fn clean_path(path: &Path, permanent: bool) -> Result<()> {
    // Check if file is locked (Windows only)
    if path.is_file() && is_file_locked(path) {
        return Err(anyhow::anyhow!("File is locked by another process"));
    }

    if permanent {
        // Permanent delete - bypass Recycle Bin
        // Use safe_* functions for long path support
        if path.is_dir() {
            utils::safe_remove_dir_all(path).with_context(|| {
                format!("Failed to permanently delete directory: {}", path.display())
            })?;
        } else {
            utils::safe_remove_file(path).with_context(|| {
                format!("Failed to permanently delete file: {}", path.display())
            })?;
        }
    } else {
        // Move to Recycle Bin
        // Note: trash crate should handle long paths internally
        trash::delete(path).with_context(|| format!("Failed to delete: {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::ScanResults;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_clean_all_empty_results() {
        let results = ScanResults::default();

        // Should return Ok without doing anything
        // Use Quiet mode in tests to avoid spinner thread issues
        let result = clean_all(&results, true, OutputMode::Quiet, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_clean_all_dry_run() {
        let temp_dir = create_test_dir();
        let file = temp_dir.path().join("test.txt");
        fs::write(&file, "test content").unwrap();

        let mut results = ScanResults::default();
        results.cache.paths.push(file.clone());
        results.cache.items = 1;
        results.cache.size_bytes = 12;

        // Dry run should not delete the file
        // Use Quiet mode in tests to avoid spinner thread issues
        let result = clean_all(&results, true, OutputMode::Quiet, false, true);
        assert!(result.is_ok());
        assert!(file.exists()); // File should still exist
    }

    #[test]
    fn test_is_file_locked_regular_file() {
        let temp_dir = create_test_dir();
        let file = temp_dir.path().join("unlocked.txt");
        fs::write(&file, "test").unwrap();

        // File should not be locked
        assert!(!is_file_locked(&file));
    }

    #[test]
    fn test_is_file_locked_directory() {
        let temp_dir = create_test_dir();
        let dir = temp_dir.path().join("testdir");
        fs::create_dir(&dir).unwrap();

        // Directories are never "locked" in our check
        assert!(!is_file_locked(&dir));
    }

    #[test]
    fn test_is_file_locked_nonexistent() {
        let temp_dir = create_test_dir();
        let nonexistent = temp_dir.path().join("nonexistent.txt");

        // Non-existent files are not locked
        assert!(!is_file_locked(&nonexistent));
    }

    #[test]
    fn test_clean_path_nonexistent() {
        let temp_dir = create_test_dir();
        let nonexistent = temp_dir.path().join("nonexistent.txt");

        // Cleaning a non-existent file should fail
        let result = clean_path(&nonexistent, true);
        assert!(result.is_err());
    }
}
