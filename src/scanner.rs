use crate::categories;
use crate::cli::ScanOptions;
use crate::config::Config;
use crate::output::{OutputMode, ScanResults};
use crate::progress;
use anyhow::Result;
use colored::*;
use std::path::Path;

/// Scan all requested categories and return aggregated results
/// 
/// Handles errors gracefully - if one category fails, others continue scanning
/// Filters out paths matching exclusion patterns from config
pub fn scan_all(path: &Path, options: ScanOptions, mode: OutputMode, config: &Config) -> Result<ScanResults> {
    let mut results = ScanResults::default();
    
    // Count enabled categories for progress
    let total_categories = [
        options.cache, options.temp, options.trash, options.build,
        options.downloads, options.large, options.old
    ].iter().filter(|&&x| x).count();
    
    // Create spinner for visual feedback (unless quiet mode)
    let spinner = if mode != OutputMode::Quiet && total_categories > 0 {
        Some(progress::create_spinner("Starting scan..."))
    } else {
        None
    };
    
    let mut scanned = 0;
    
    if options.cache {
        if let Some(ref sp) = spinner {
            sp.set_message(format!("Scanning cache directories... ({}/{})", scanned + 1, total_categories));
        }
        match categories::cache::scan(path) {
            Ok(cache_result) => results.cache = cache_result,
            Err(e) => {
                if mode != OutputMode::Quiet {
                    eprintln!("{} Cache scan failed: {}", "Warning:".yellow(), e);
                }
            }
        }
        scanned += 1;
    }
    
    if options.temp {
        if let Some(ref sp) = spinner {
            sp.set_message(format!("Scanning temp files... ({}/{})", scanned + 1, total_categories));
        }
        match categories::temp::scan(path) {
            Ok(temp_result) => results.temp = temp_result,
            Err(e) => {
                if mode != OutputMode::Quiet {
                    eprintln!("{} Temp scan failed: {}", "Warning:".yellow(), e);
                }
            }
        }
        scanned += 1;
    }
    
    if options.trash {
        if let Some(ref sp) = spinner {
            sp.set_message(format!("Scanning Recycle Bin... ({}/{})", scanned + 1, total_categories));
        }
        match categories::trash::scan() {
            Ok(trash_result) => results.trash = trash_result,
            Err(e) => {
                if mode != OutputMode::Quiet {
                    eprintln!("{} Trash scan failed: {}", "Warning:".yellow(), e);
                }
            }
        }
        scanned += 1;
    }
    
    if options.build {
        if let Some(ref sp) = spinner {
            sp.set_message(format!("Scanning build artifacts... ({}/{})", scanned + 1, total_categories));
        }
        match categories::build::scan(path, options.project_age_days) {
            Ok(build_result) => results.build = build_result,
            Err(e) => {
                if mode != OutputMode::Quiet {
                    eprintln!("{} Build scan failed: {}", "Warning:".yellow(), e);
                }
            }
        }
        scanned += 1;
    }
    
    if options.downloads {
        if let Some(ref sp) = spinner {
            sp.set_message(format!("Scanning Downloads folder... ({}/{})", scanned + 1, total_categories));
        }
        match categories::downloads::scan(path, options.min_age_days) {
            Ok(downloads_result) => results.downloads = downloads_result,
            Err(e) => {
                if mode != OutputMode::Quiet {
                    eprintln!("{} Downloads scan failed: {}", "Warning:".yellow(), e);
                }
            }
        }
        scanned += 1;
    }
    
    if options.large {
        if let Some(ref sp) = spinner {
            sp.set_message(format!("Scanning for large files... ({}/{})", scanned + 1, total_categories));
        }
        match categories::large::scan(path, options.min_size_bytes) {
            Ok(large_result) => results.large = large_result,
            Err(e) => {
                if mode != OutputMode::Quiet {
                    eprintln!("{} Large files scan failed: {}", "Warning:".yellow(), e);
                }
            }
        }
        scanned += 1;
    }
    
    if options.old {
        if let Some(ref sp) = spinner {
            sp.set_message(format!("Scanning for old files... ({}/{})", scanned + 1, total_categories));
        }
        match categories::old::scan(path, options.min_age_days) {
            Ok(old_result) => results.old = old_result,
            Err(e) => {
                if mode != OutputMode::Quiet {
                    eprintln!("{} Old files scan failed: {}", "Warning:".yellow(), e);
                }
            }
        }
    }
    
    // Clear spinner
    if let Some(sp) = spinner {
        progress::finish_and_clear(&sp);
    }
    
    // Filter out excluded paths
    filter_exclusions(&mut results, config);
    
    Ok(results)
}

/// Filter out paths matching exclusion patterns
fn filter_exclusions(results: &mut ScanResults, config: &Config) {
    let filter_paths = |paths: &mut Vec<std::path::PathBuf>| {
        paths.retain(|path| !config.is_excluded(path));
    };
    
    filter_paths(&mut results.cache.paths);
    filter_paths(&mut results.temp.paths);
    filter_paths(&mut results.trash.paths);
    filter_paths(&mut results.build.paths);
    filter_paths(&mut results.downloads.paths);
    filter_paths(&mut results.large.paths);
    filter_paths(&mut results.old.paths);
    
    // Recalculate item counts and sizes after filtering
    results.cache.items = results.cache.paths.len();
    results.cache.size_bytes = calculate_total_size(&results.cache.paths);
    
    results.temp.items = results.temp.paths.len();
    results.temp.size_bytes = calculate_total_size(&results.temp.paths);
    
    results.build.items = results.build.paths.len();
    results.build.size_bytes = calculate_total_size(&results.build.paths);
    
    results.downloads.items = results.downloads.paths.len();
    results.downloads.size_bytes = calculate_total_size(&results.downloads.paths);
    
    results.large.items = results.large.paths.len();
    results.large.size_bytes = calculate_total_size(&results.large.paths);
    
    results.old.items = results.old.paths.len();
    results.old.size_bytes = calculate_total_size(&results.old.paths);
}

/// Calculate total size of paths
fn calculate_total_size(paths: &[std::path::PathBuf]) -> u64 {
    paths.iter()
        .filter_map(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .sum()
}
