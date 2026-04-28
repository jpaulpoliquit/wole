use crate::config::Config;
use crate::output::{CategoryResult, OutputMode};
use crate::scan_events::{ScanPathReporter, ScanProgressEvent};
use crate::theme::Theme;
use crate::utils;
use anyhow::{Context, Result};
use bytesize;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::Arc;

/// Application cache locations to scan
/// Each tuple is (name, path_from_localappdata_or_appdata)
const APP_CACHE_LOCATIONS: &[(&str, AppCacheLocation)] = &[
    (
        "Discord",
        AppCacheLocation::LocalAppDataNested(&["discord", "Cache"]),
    ),
    (
        "VS Code",
        AppCacheLocation::LocalAppDataNested(&["Code", "Cache"]),
    ),
    (
        "VS Code (User)",
        AppCacheLocation::LocalAppDataNested(&["Code", "User", "CachedData"]),
    ),
    (
        "Slack",
        AppCacheLocation::LocalAppDataNested(&["slack", "Cache"]),
    ),
    (
        "Steam",
        AppCacheLocation::LocalAppDataNested(&["Steam", "htmlcache"]),
    ),
    (
        "Zoom",
        AppCacheLocation::LocalAppDataNested(&["Zoom", "Cache"]),
    ),
    (
        "Teams",
        AppCacheLocation::LocalAppDataNested(&["Microsoft", "Teams", "Cache"]),
    ),
    (
        "Notion",
        AppCacheLocation::LocalAppDataNested(&["Notion", "Cache"]),
    ),
    (
        "Notion (Roaming Cache)",
        AppCacheLocation::AppDataNested(&["Notion", "Cache"]),
    ),
    (
        "Notion (Roaming Code Cache)",
        AppCacheLocation::AppDataNested(&["Notion", "Code Cache"]),
    ),
    (
        "Notion (Roaming GPUCache)",
        AppCacheLocation::AppDataNested(&["Notion", "GPUCache"]),
    ),
    (
        "Notion (Roaming DawnWebGPUCache)",
        AppCacheLocation::AppDataNested(&["Notion", "DawnWebGPUCache"]),
    ),
    (
        "Notion (Roaming DawnGraphiteCache)",
        AppCacheLocation::AppDataNested(&["Notion", "DawnGraphiteCache"]),
    ),
    (
        "Notion (Roaming Partitions)",
        AppCacheLocation::AppDataNested(&["Notion", "Partitions"]),
    ),
    (
        "Figma",
        AppCacheLocation::LocalAppDataNested(&["Figma", "Cache"]),
    ),
    (
        "Adobe",
        AppCacheLocation::LocalAppDataNested(&["Adobe", "Common"]),
    ),
    (
        "Adobe Acrobat",
        AppCacheLocation::LocalAppDataNested(&["Adobe", "Acrobat", "Cache"]),
    ),
    (
        "Dropbox",
        AppCacheLocation::LocalAppDataNested(&["Dropbox", "Cache"]),
    ),
    (
        "OneDrive",
        AppCacheLocation::LocalAppDataNested(&["Microsoft", "OneDrive", "Cache"]),
    ),
    (
        "GitHub Desktop",
        AppCacheLocation::LocalAppDataNested(&["GitHub Desktop", "Cache"]),
    ),
    (
        "Postman",
        AppCacheLocation::LocalAppDataNested(&["Postman", "Cache"]),
    ),
    (
        "Docker",
        AppCacheLocation::LocalAppDataNested(&["Docker", "Cache"]),
    ),
    (
        "DBeaver",
        AppCacheLocation::LocalAppDataNested(&["DBeaver", "Cache"]),
    ),
    (
        "JetBrains",
        AppCacheLocation::LocalAppDataNested(&["JetBrains", "Cache"]),
    ),
    (
        "IntelliJ IDEA",
        AppCacheLocation::LocalAppDataNested(&["JetBrains", "IntelliJIdea", "cache"]),
    ),
    (
        "PyCharm",
        AppCacheLocation::LocalAppDataNested(&["JetBrains", "PyCharm", "cache"]),
    ),
    (
        "WebStorm",
        AppCacheLocation::LocalAppDataNested(&["JetBrains", "WebStorm", "cache"]),
    ),
    (
        "Android Studio",
        AppCacheLocation::LocalAppDataNested(&["Google", "AndroidStudio", "cache"]),
    ),
    (
        "Unity",
        AppCacheLocation::LocalAppDataNested(&["Unity", "cache"]),
    ),
    (
        "Blender",
        AppCacheLocation::LocalAppDataNested(&["Blender Foundation", "Blender", "cache"]),
    ),
    (
        "OBS Studio",
        AppCacheLocation::LocalAppDataNested(&["obs-studio", "Cache"]),
    ),
    (
        "VLC",
        AppCacheLocation::LocalAppDataNested(&["vlc", "cache"]),
    ),
    (
        "WinRAR",
        AppCacheLocation::LocalAppDataNested(&["WinRAR", "Cache"]),
    ),
    (
        "7-Zip",
        AppCacheLocation::LocalAppDataNested(&["7-Zip", "Cache"]),
    ),
    // App updaters (download residue under %LOCALAPPDATA%)
    (
        "Obsidian updater",
        AppCacheLocation::LocalAppDataNested(&["obsidian-updater"]),
    ),
    (
        "Notion updater",
        AppCacheLocation::LocalAppDataNested(&["notion-updater"]),
    ),
    (
        "Notion Calendar updater",
        AppCacheLocation::LocalAppDataNested(&["notion-calendar-web-updater"]),
    ),
    (
        "Cron updater",
        AppCacheLocation::LocalAppDataNested(&["cron-updater"]),
    ),
    (
        "Cron web updater",
        AppCacheLocation::LocalAppDataNested(&["cron-web-updater"]),
    ),
    (
        "Cursor updater",
        AppCacheLocation::LocalAppDataNested(&["cursor-updater"]),
    ),
    (
        "Cursor Nightly updater",
        AppCacheLocation::LocalAppDataNested(&["cursor-nightly-updater"]),
    ),
    // Cursor / Cursor Nightly: cache and logs only (never globalStorage / state DB)
    (
        "Cursor (CachedExtensionVSIXs)",
        AppCacheLocation::AppDataNested(&["Cursor", "CachedExtensionVSIXs"]),
    ),
    (
        "Cursor (CachedData)",
        AppCacheLocation::AppDataNested(&["Cursor", "CachedData"]),
    ),
    (
        "Cursor (Cache)",
        AppCacheLocation::AppDataNested(&["Cursor", "Cache"]),
    ),
    (
        "Cursor (Code Cache)",
        AppCacheLocation::AppDataNested(&["Cursor", "Code Cache"]),
    ),
    (
        "Cursor (GPUCache)",
        AppCacheLocation::AppDataNested(&["Cursor", "GPUCache"]),
    ),
    (
        "Cursor (DawnWebGPUCache)",
        AppCacheLocation::AppDataNested(&["Cursor", "DawnWebGPUCache"]),
    ),
    (
        "Cursor (DawnGraphiteCache)",
        AppCacheLocation::AppDataNested(&["Cursor", "DawnGraphiteCache"]),
    ),
    (
        "Cursor (logs)",
        AppCacheLocation::AppDataNested(&["Cursor", "logs"]),
    ),
    (
        "Cursor (snapshots)",
        AppCacheLocation::AppDataNested(&["Cursor", "snapshots"]),
    ),
    (
        "Cursor Nightly (CachedExtensionVSIXs)",
        AppCacheLocation::AppDataNested(&["Cursor Nightly", "CachedExtensionVSIXs"]),
    ),
    (
        "Cursor Nightly (CachedData)",
        AppCacheLocation::AppDataNested(&["Cursor Nightly", "CachedData"]),
    ),
    (
        "Cursor Nightly (Cache)",
        AppCacheLocation::AppDataNested(&["Cursor Nightly", "Cache"]),
    ),
    (
        "Cursor Nightly (Code Cache)",
        AppCacheLocation::AppDataNested(&["Cursor Nightly", "Code Cache"]),
    ),
    (
        "Cursor Nightly (GPUCache)",
        AppCacheLocation::AppDataNested(&["Cursor Nightly", "GPUCache"]),
    ),
    (
        "Cursor Nightly (DawnWebGPUCache)",
        AppCacheLocation::AppDataNested(&["Cursor Nightly", "DawnWebGPUCache"]),
    ),
    (
        "Cursor Nightly (DawnGraphiteCache)",
        AppCacheLocation::AppDataNested(&["Cursor Nightly", "DawnGraphiteCache"]),
    ),
    (
        "Cursor Nightly (logs)",
        AppCacheLocation::AppDataNested(&["Cursor Nightly", "logs"]),
    ),
    (
        "Cursor Nightly (snapshots)",
        AppCacheLocation::AppDataNested(&["Cursor Nightly", "snapshots"]),
    ),
];

enum AppCacheLocation {
    LocalAppDataNested(&'static [&'static str]),
    /// Paths under `%APPDATA%` (Roaming).
    AppDataNested(&'static [&'static str]),
}

fn resolve_app_cache_path(
    location: &AppCacheLocation,
    local_appdata: Option<&PathBuf>,
    appdata: Option<&PathBuf>,
) -> Option<PathBuf> {
    match location {
        AppCacheLocation::LocalAppDataNested(subpaths) => {
            let mut path = local_appdata?.clone();
            for subpath in *subpaths {
                path = path.join(subpath);
            }
            Some(path)
        }
        AppCacheLocation::AppDataNested(subpaths) => {
            let mut path = appdata?.clone();
            for subpath in *subpaths {
                path = path.join(subpath);
            }
            Some(path)
        }
    }
}

/// Common cache directory names used by applications (generic discovery; tests only).
#[cfg(test)]
const CACHE_DIR_NAMES: &[&str] = &["Cache", "cache", "Caches", ".cache", "Cache_Data"];

#[derive(Clone, Copy)]
#[cfg(test)]
enum AppCacheScanBase {
    LocalAppData,
    AppData,
}

/// Skip generic cache discovery for Perplexity / Comet install trees and the updater.
/// Comet disk caches are handled only in `browser` with a narrow allowlist; do not
/// delete `%LOCALAPPDATA%\\Perplexity\\Comet` or `%APPDATA%\\Perplexity` wholesale.
#[cfg(test)]
fn should_skip_branded_app_cache_dir(base: AppCacheScanBase, dir: &Path) -> bool {
    let Some(name) = dir.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    match base {
        AppCacheScanBase::AppData => name.eq_ignore_ascii_case("Perplexity"),
        AppCacheScanBase::LocalAppData => {
            name.eq_ignore_ascii_case("Perplexity")
                || name.eq_ignore_ascii_case("perplexity-updater")
                || name.eq_ignore_ascii_case("Comet")
        }
    }
}

/// Scan for app-specific cache directories via name heuristics.
///
/// **Not used by default scans** — application cache is curated allowlist only.
/// Kept for unit tests and possible future opt-in tooling.
#[cfg(test)]
fn scan_app_caches(
    base_path: &Path,
    base_kind: AppCacheScanBase,
    known_paths: &mut HashSet<PathBuf>,
    config: &Config,
) -> Vec<PathBuf> {
    let mut app_cache_paths = Vec::new();

    if !base_path.exists() {
        return app_cache_paths;
    }

    // Read the base directory (e.g., LOCALAPPDATA or APPDATA)
    let entries = match utils::safe_read_dir(base_path) {
        Ok(entries) => entries,
        Err(_) => return app_cache_paths,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let app_dir = entry.path();

        // Skip if not a directory
        if !app_dir.is_dir() {
            continue;
        }

        if should_skip_branded_app_cache_dir(base_kind, &app_dir) {
            continue;
        }

        // Check for cache directories directly in the app directory
        for cache_name in CACHE_DIR_NAMES {
            let cache_path = app_dir.join(cache_name);
            if cache_path.exists()
                && cache_path.is_dir()
                && !known_paths.contains(&cache_path)
                && !config.is_excluded(&cache_path)
            {
                // Defer size calculation to the caller for parallelism
                known_paths.insert(cache_path.clone());
                app_cache_paths.push(cache_path);
            }
        }

        // Also check nested app directories (e.g., CompanyName\AppName\Cache)
        if let Ok(nested_entries) = utils::safe_read_dir(&app_dir) {
            for nested_entry in nested_entries.filter_map(|e| e.ok()) {
                let nested_dir = nested_entry.path();
                if !nested_dir.is_dir() {
                    continue;
                }

                if should_skip_branded_app_cache_dir(base_kind, &nested_dir) {
                    continue;
                }

                // Check for cache directories in nested app directories
                for cache_name in CACHE_DIR_NAMES {
                    let cache_path = nested_dir.join(cache_name);
                    if cache_path.exists()
                        && cache_path.is_dir()
                        && !known_paths.contains(&cache_path)
                        && !config.is_excluded(&cache_path)
                    {
                        // Defer size calculation to the caller for parallelism
                        known_paths.insert(cache_path.clone());
                        app_cache_paths.push(cache_path);
                    }
                }
            }
        }
    }

    app_cache_paths
}

/// Scan for application cache directories
///
/// Checks a curated allowlist of well-known Windows cache locations for various
/// applications (plus a few review-worthy state-adjacent paths such as Notion
/// `Partitions` and Cursor `snapshots`). Generic `%LOCALAPPDATA%` / `%APPDATA%`
/// cache-name discovery is disabled for normal scans.
///
/// Optimized to calculate directory sizes in parallel.
pub fn scan(_root: &Path, config: &Config, output_mode: OutputMode) -> Result<CategoryResult> {
    let mut result = CategoryResult::default();
    let mut known_paths = HashSet::new();
    let mut candidates = Vec::new();

    let local_appdata = env::var("LOCALAPPDATA").ok().map(PathBuf::from);
    let appdata = env::var("APPDATA").ok().map(PathBuf::from);

    if output_mode != OutputMode::Quiet {
        println!(
            "  {} Scanning application cache directories...",
            Theme::muted("→")
        );
    }

    // 1. Collect all candidate paths first (fast IO check)

    // Scan known application caches
    for (name, location) in APP_CACHE_LOCATIONS {
        let cache_path = resolve_app_cache_path(location, local_appdata.as_ref(), appdata.as_ref());

        if let Some(cache_path) = cache_path {
            if cache_path.exists() && !config.is_excluded(&cache_path) {
                known_paths.insert(cache_path.clone());
                candidates.push(cache_path);
                if output_mode != OutputMode::Quiet {
                    println!("    {} Found {} cache", Theme::muted("•"), name);
                }
            }
        }
    }

    // 2. Calculate sizes sequentially per folder, but folder size check is parallel
    // This is much Kinder to the disk than starting N parallel walks
    let mut paths_with_sizes: Vec<(PathBuf, u64)> = candidates
        .iter()
        .map(|path| {
            let size = utils::calculate_dir_size(path);
            (path.clone(), size)
        })
        .filter(|(_, size)| *size > 0)
        .collect();

    // Sort by size descending
    paths_with_sizes.sort_by_key(|b| std::cmp::Reverse(b.1));

    // Show found caches
    if output_mode != OutputMode::Quiet && !paths_with_sizes.is_empty() {
        println!(
            "  {} Found {} application caches:",
            Theme::muted("→"),
            paths_with_sizes.len()
        );
        let show_count = match output_mode {
            OutputMode::VeryVerbose => paths_with_sizes.len(),
            OutputMode::Verbose => paths_with_sizes.len(),
            OutputMode::Normal => paths_with_sizes.len().min(10),
            OutputMode::Quiet => 0,
        };

        for (i, (path, size)) in paths_with_sizes.iter().take(show_count).enumerate() {
            let size_str = bytesize::to_string(*size, false);
            println!(
                "      {} {} ({})",
                Theme::muted("→"),
                path.display(),
                Theme::size(&size_str)
            );

            if i == 9 && output_mode == OutputMode::Normal && paths_with_sizes.len() > 10 {
                println!(
                    "      {} ... and {} more (use -v to see all)",
                    Theme::muted("→"),
                    paths_with_sizes.len() - 10
                );
                break;
            }
        }
    }

    // Store paths
    result.paths = paths_with_sizes.iter().map(|(p, _)| p.clone()).collect();
    result.size_bytes = paths_with_sizes.iter().map(|(_, size)| *size).sum();
    result.items = paths_with_sizes.len();

    Ok(result)
}

/// Scan with real-time progress events (for TUI).
pub fn scan_with_progress(
    _root: &Path,
    config: &Config,
    tx: &Sender<ScanProgressEvent>,
) -> Result<CategoryResult> {
    const CATEGORY: &str = "Application Cache";
    let mut result = CategoryResult::default();
    let mut files_with_sizes: Vec<(PathBuf, u64)> = Vec::new();
    let mut known_paths = HashSet::new();

    let local_appdata = env::var("LOCALAPPDATA").ok().map(PathBuf::from);
    let appdata = env::var("APPDATA").ok().map(PathBuf::from);

    let total = APP_CACHE_LOCATIONS.len() as u64;
    #[allow(unused_assignments)]
    let mut completed = 0u64;

    let _ = tx.send(ScanProgressEvent::CategoryStarted {
        category: CATEGORY.to_string(),
        total_units: Some(total),
        current_path: None,
    });

    let reporter = Arc::new(ScanPathReporter::new(CATEGORY, tx.clone(), 10));
    let on_path = |path: &Path| reporter.emit_path(path);

    // Scan known application caches
    for (idx, (_name, location)) in APP_CACHE_LOCATIONS.iter().enumerate() {
        let cache_path = resolve_app_cache_path(location, local_appdata.as_ref(), appdata.as_ref());

        if let Some(cache_path) = cache_path {
            if cache_path.exists() && !config.is_excluded(&cache_path) {
                let size = utils::calculate_dir_size_with_progress(&cache_path, &on_path);
                if size > 0 {
                    known_paths.insert(cache_path.clone());
                    files_with_sizes.push((cache_path.clone(), size));
                }
            }

            completed = (idx + 1) as u64;
            let _ = tx.send(ScanProgressEvent::CategoryProgress {
                category: CATEGORY.to_string(),
                completed_units: completed,
                total_units: Some(total),
                current_path: Some(cache_path),
            });
        } else {
            completed = (idx + 1) as u64;
            let _ = tx.send(ScanProgressEvent::CategoryProgress {
                category: CATEGORY.to_string(),
                completed_units: completed,
                total_units: Some(total),
                current_path: None,
            });
        }
    }

    // Sort by size descending
    files_with_sizes.sort_by_key(|b| std::cmp::Reverse(b.1));

    // Build final result
    for (path, size) in files_with_sizes {
        result.items += 1;
        result.size_bytes += size;
        result.paths.push(path);
    }

    let _ = tx.send(ScanProgressEvent::CategoryFinished {
        category: CATEGORY.to_string(),
        items: result.items,
        size_bytes: result.size_bytes,
    });

    Ok(result)
}

/// True when any scanned path is state-adjacent and should be reviewed before clean
/// (Notion roaming `Partitions`, Cursor / Cursor Nightly `snapshots`).
pub fn scan_includes_review_worthy_paths(paths: &[PathBuf]) -> bool {
    paths.iter().any(|p| is_review_worthy_app_cache_path(p))
}

fn is_review_worthy_app_cache_path(path: &Path) -> bool {
    let Some(leaf) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };

    if leaf.eq_ignore_ascii_case("Partitions") {
        if let Some(parent) = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
        {
            return parent.eq_ignore_ascii_case("Notion");
        }
    }

    if leaf.eq_ignore_ascii_case("snapshots") {
        if let Some(parent) = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
        {
            return parent.eq_ignore_ascii_case("Cursor")
                || parent.eq_ignore_ascii_case("Cursor Nightly");
        }
    }

    false
}

/// Clean (delete) an application cache directory by moving it to the Recycle Bin
pub fn clean(path: &Path) -> Result<()> {
    crate::trash_ops::delete(path).with_context(|| {
        format!(
            "Failed to delete application cache directory: {}",
            path.display()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static APP_CACHE_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn app_cache_allowlist_never_targets_cursor_state_or_settings() {
        let forbidden = [
            "globalStorage",
            "state.vscdb",
            "workspaceStorage",
            "User\\settings",
            "User/settings",
        ];
        for (label, loc) in APP_CACHE_LOCATIONS {
            let joined = match loc {
                AppCacheLocation::LocalAppDataNested(parts)
                | AppCacheLocation::AppDataNested(parts) => parts.join("/"),
            };
            for fragment in forbidden {
                assert!(
                    !joined.contains(fragment),
                    "label={label} segments={joined} must not contain {fragment}"
                );
            }
        }
    }

    #[test]
    fn resolve_appdata_nested_cursor_cache() {
        let local = PathBuf::from(r"C:\AppData\Local");
        let roaming = PathBuf::from(r"C:\AppData\Roaming");
        let loc = AppCacheLocation::AppDataNested(&["Cursor", "Cache"]);
        let p = resolve_app_cache_path(&loc, Some(&local), Some(&roaming)).unwrap();
        assert_eq!(p.file_name().and_then(|n| n.to_str()), Some("Cache"));
        assert_eq!(
            p.parent()
                .and_then(|parent| parent.file_name())
                .and_then(|n| n.to_str()),
            Some("Cursor")
        );
    }

    #[test]
    fn resolve_localappdata_nested() {
        let local = PathBuf::from(r"C:\AppData\Local");
        let roaming = PathBuf::from(r"C:\AppData\Roaming");
        let loc = AppCacheLocation::LocalAppDataNested(&["notion-updater"]);
        let p = resolve_app_cache_path(&loc, Some(&local), Some(&roaming)).unwrap();
        assert!(p.ends_with("notion-updater"));
    }

    #[test]
    fn generic_scan_skips_perplexity_comet_and_updater_roots() {
        assert!(!APP_CACHE_LOCATIONS
            .iter()
            .any(|(n, _)| n.to_ascii_lowercase().contains("perplexity-updater")));
        let roaming = PathBuf::from(r"C:\AppData\Roaming\Perplexity");
        assert!(should_skip_branded_app_cache_dir(
            AppCacheScanBase::AppData,
            &roaming
        ));
        let local_perp = PathBuf::from(r"C:\AppData\Local\Perplexity");
        assert!(should_skip_branded_app_cache_dir(
            AppCacheScanBase::LocalAppData,
            &local_perp
        ));
        let updater = PathBuf::from(r"C:\AppData\Local\perplexity-updater");
        assert!(should_skip_branded_app_cache_dir(
            AppCacheScanBase::LocalAppData,
            &updater
        ));
        let comet = PathBuf::from(r"C:\AppData\Local\Comet");
        assert!(should_skip_branded_app_cache_dir(
            AppCacheScanBase::LocalAppData,
            &comet
        ));
        let discord = PathBuf::from(r"C:\AppData\Local\discord");
        assert!(!should_skip_branded_app_cache_dir(
            AppCacheScanBase::LocalAppData,
            &discord
        ));
    }

    #[test]
    fn allowlist_excludes_telegram_tdata_and_spotify_storage() {
        for (_label, loc) in APP_CACHE_LOCATIONS {
            let joined = match loc {
                AppCacheLocation::LocalAppDataNested(parts)
                | AppCacheLocation::AppDataNested(parts) => parts.join("\\"),
            };
            let lower = joined.to_ascii_lowercase();
            assert!(
                !lower.contains("telegram desktop\\tdata"),
                "must not target Telegram tdata: {joined}"
            );
            assert!(
                !lower.contains("spotify\\storage"),
                "must not target Spotify Storage: {joined}"
            );
        }
    }

    #[test]
    fn allowlist_has_no_perplexity_comet_or_updater_labels() {
        for (label, _loc) in APP_CACHE_LOCATIONS {
            let l = label.to_ascii_lowercase();
            assert!(
                !l.contains("perplexity"),
                "unexpected explicit Perplexity target: {label}"
            );
            assert!(
                !l.contains("comet"),
                "unexpected explicit Comet target: {label}"
            );
            assert!(
                !l.contains("perplexity-updater"),
                "unexpected explicit perplexity-updater: {label}"
            );
        }
    }

    #[test]
    fn review_worthy_path_detection() {
        assert!(scan_includes_review_worthy_paths(&[PathBuf::from(
            r"C:\Users\x\AppData\Roaming\Notion\Partitions"
        )]));
        assert!(scan_includes_review_worthy_paths(&[PathBuf::from(
            r"C:\Users\x\AppData\Roaming\Cursor\snapshots"
        )]));
        assert!(scan_includes_review_worthy_paths(&[PathBuf::from(
            r"C:\Users\x\AppData\Roaming\Cursor Nightly\snapshots"
        )]));
        assert!(!scan_includes_review_worthy_paths(&[PathBuf::from(
            r"C:\Users\x\AppData\Roaming\Cursor\Cache"
        )]));
    }

    #[test]
    fn curated_scan_does_not_pick_up_generic_vendor_cache_dirs() {
        let _guard = APP_CACHE_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().expect("tempdir");
        let vendor_cache = tmp.path().join("WoleHeuristicVendor").join("Cache");
        std::fs::create_dir_all(&vendor_cache).expect("mkdir");
        std::fs::write(vendor_cache.join("blob"), b"x").expect("write");

        let roaming = tmp.path().join("Roaming");
        std::fs::create_dir_all(&roaming).expect("roaming");

        let prev_local = std::env::var("LOCALAPPDATA").ok();
        let prev_app = std::env::var("APPDATA").ok();
        std::env::set_var("LOCALAPPDATA", tmp.path());
        std::env::set_var("APPDATA", &roaming);

        let config = Config::default();
        let r = scan(Path::new(""), &config, OutputMode::Quiet).expect("scan");

        let hit = r
            .paths
            .iter()
            .any(|p| p.to_string_lossy().contains("WoleHeuristicVendor"));
        assert!(
            !hit,
            "curated scan must not add heuristic-only vendor cache paths"
        );

        match prev_local {
            Some(v) => std::env::set_var("LOCALAPPDATA", v),
            None => std::env::remove_var("LOCALAPPDATA"),
        }
        match prev_app {
            Some(v) => std::env::set_var("APPDATA", v),
            None => std::env::remove_var("APPDATA"),
        }
    }

    #[test]
    fn heuristic_scan_still_skips_branded_dirs_when_used_in_tests() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let local = tmp.path();
        let mut known = HashSet::new();
        let config = Config::default();

        let perp = local.join("Perplexity");
        std::fs::create_dir_all(perp.join("Cache")).expect("mkdir");
        let comet = local.join("Comet");
        std::fs::create_dir_all(comet.join("Cache")).expect("mkdir");
        let safe = local.join("SafeVendor");
        std::fs::create_dir_all(safe.join("Cache")).expect("mkdir");

        let found = scan_app_caches(local, AppCacheScanBase::LocalAppData, &mut known, &config);
        let safe_cache = Path::new("SafeVendor").join("Cache");
        assert!(
            found.iter().any(|p| p.ends_with(&safe_cache)),
            "expected SafeVendor Cache, got {found:?}"
        );
        assert!(!found
            .iter()
            .any(|p| p.to_string_lossy().contains("Perplexity")));
        assert!(!found.iter().any(|p| p.to_string_lossy().contains("Comet")));
    }
}
