use crate::config::Config;
use crate::output::CategoryResult;
use crate::utils;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Chromium-based browsers: display name and path under %LOCALAPPDATA% up to and including `User Data`.
///
/// **Perplexity Comet** is not scanned as a full Chromium profile tree: only narrow
/// cache folders under `Perplexity\\Comet\\User Data` are collected by
/// `collect_comet_narrow_caches`. We never target `Comet\\Application`, the whole `Comet`
/// install directory, or Roaming `Perplexity` (those are excluded from application-cache
/// heuristics in the `app_cache` module).
const CHROMIUM_USER_DATA_ROOTS: &[(&str, &[&str])] = &[
    ("Chrome", &["Google", "Chrome", "User Data"]),
    ("Chrome (Beta)", &["Google", "Chrome Beta", "User Data"]),
    ("Chrome (Dev)", &["Google", "Chrome Dev", "User Data"]),
    ("Edge", &["Microsoft", "Edge", "User Data"]),
    ("Edge (Beta)", &["Microsoft", "Edge Beta", "User Data"]),
    ("Edge (Dev)", &["Microsoft", "Edge Dev", "User Data"]),
    ("Brave", &["BraveSoftware", "Brave-Browser", "User Data"]),
    (
        "Brave (Beta)",
        &["BraveSoftware", "Brave-Browser-Beta", "User Data"],
    ),
    ("Arc", &["The Browser Company", "Arc", "User Data"]),
    ("Atlas", &["OpenAI", "Atlast", "User Data"]),
    ("Vivaldi", &["Vivaldi", "User Data"]),
    ("Chromium", &["Chromium", "User Data"]),
    ("Sidekick", &["Redundant", "Sidekick", "User Data"]),
    ("Yandex", &["Yandex", "YandexBrowser", "User Data"]),
    (
        "Avast Secure Browser",
        &["AVAST Software", "Browser", "User Data"],
    ),
    (
        "CCleaner Browser",
        &["CCleaner", "CCleaner Browser", "User Data"],
    ),
    ("Torch", &["Torch", "User Data"]),
    ("Epic", &["Epic Privacy Browser", "User Data"]),
];

/// Cache folder names relative to a Chromium profile directory (e.g. `Default`, `Profile 1`).
const CHROMIUM_PROFILE_CACHE_DIRS: &[&str] = &[
    "Cache",
    "Code Cache",
    "GPUCache",
    "GrShaderCache",
    "ShaderCache",
    "DawnCache",
    "DawnWebGPUCache",
    "DawnGraphiteCache",
    "component_crx_cache",
    "optimization_guide_model_store",
    "Safe Browsing",
    "Snapshots",
];

/// Paths relative to the Chromium `User Data` root (sibling of profile folders).
const CHROMIUM_USER_DATA_ROOT_CACHE_DIRS: &[&str] = &[
    "BrowserMetrics",
    "component_crx_cache",
    "optimization_guide_model_store",
    "Crashpad",
];

/// Opera-style single cache root under %LOCALAPPDATA% (not Chromium profile layout).
const OPERA_STYLE_CACHES: &[(&str, &[&str])] =
    &[("Opera", &["Opera Software", "Opera Stable", "Cache"])];

/// Comet (`%LOCALAPPDATA%\\Perplexity\\Comet\\User Data`): only these profile subfolders,
/// plus `Crashpad\\reports` under `User Data` — never `Application` or the `Comet` root.
const COMET_USER_DATA_SEGMENTS: &[&str] = &["Perplexity", "Comet", "User Data"];
const COMET_PROFILE_NARROW_CACHE_DIRS: &[&str] = &[
    "Cache",
    "Code Cache",
    "GPUCache",
    "GrShaderCache",
    "ShaderCache",
];

/// Narrow disk-cache paths for Perplexity Comet only (browser install stays intact).
fn collect_comet_narrow_caches(
    local_appdata: &Path,
    paths: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
    config: &Config,
) {
    let user_data_root = join_localappdata(local_appdata, COMET_USER_DATA_SEGMENTS);
    if !user_data_root.is_dir() {
        return;
    }

    let reports = user_data_root.join("Crashpad").join("reports");
    if reports.is_dir() && !config.is_excluded(&reports) && seen.insert(reports.clone()) {
        paths.push(reports);
    }

    let entries = match utils::safe_read_dir(&user_data_root) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !is_chromium_profile_dir(name) {
            continue;
        }
        for cache_name in COMET_PROFILE_NARROW_CACHE_DIRS {
            let cache_path = path.join(cache_name);
            if cache_path.is_dir()
                && !config.is_excluded(&cache_path)
                && seen.insert(cache_path.clone())
            {
                paths.push(cache_path);
            }
        }
    }
}

/// Returns true if `name` looks like a Chromium profile directory under `User Data`.
fn is_chromium_profile_dir(name: &str) -> bool {
    name == "Default"
        || name.starts_with("Profile ")
        || name == "Guest Profile"
        || name == "System Profile"
}

fn join_localappdata(local_appdata: &Path, segments: &[&str]) -> PathBuf {
    let mut p = local_appdata.to_path_buf();
    for s in segments {
        p.push(s);
    }
    p
}

/// Collect Chromium-family cache paths under one `User Data` root. Deduplicates paths.
fn collect_chromium_family_caches(
    local_appdata: &Path,
    user_data_segments: &[&str],
    paths: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
    config: &Config,
) {
    let user_data_root = join_localappdata(local_appdata, user_data_segments);
    if !user_data_root.is_dir() {
        return;
    }

    // Root-level buckets (e.g. BrowserMetrics, Crashpad/reports)
    for rel in CHROMIUM_USER_DATA_ROOT_CACHE_DIRS {
        let candidate = user_data_root.join(rel);
        if rel == &"Crashpad" {
            let reports = candidate.join("reports");
            if reports.is_dir() && !config.is_excluded(&reports) && seen.insert(reports.clone()) {
                paths.push(reports);
            }
        } else if candidate.is_dir()
            && !config.is_excluded(&candidate)
            && seen.insert(candidate.clone())
        {
            paths.push(candidate);
        }
    }

    // Per-profile cache directories
    let entries = match utils::safe_read_dir(&user_data_root) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !is_chromium_profile_dir(name) {
            continue;
        }
        for cache_name in CHROMIUM_PROFILE_CACHE_DIRS {
            let cache_path = path.join(cache_name);
            if cache_path.is_dir()
                && !config.is_excluded(&cache_path)
                && seen.insert(cache_path.clone())
            {
                paths.push(cache_path);
            }
        }
    }
}

/// Scan for browser cache directories
///
/// Checks well-known Windows cache locations for Chromium-family browsers (disk caches
/// only, not cookies/history databases), Opera, and Firefox.
pub fn scan(_root: &Path, config: &Config) -> Result<CategoryResult> {
    let mut result = CategoryResult::default();
    let mut paths = Vec::new();
    let mut seen = HashSet::new();

    let local_appdata = env::var("LOCALAPPDATA").ok().map(PathBuf::from);

    if let Some(ref local_appdata_path) = local_appdata {
        for (_name, segments) in CHROMIUM_USER_DATA_ROOTS {
            collect_chromium_family_caches(
                local_appdata_path,
                segments,
                &mut paths,
                &mut seen,
                config,
            );
        }

        for (_name, segments) in OPERA_STYLE_CACHES {
            let cache_path = join_localappdata(local_appdata_path, segments);
            if cache_path.exists()
                && !config.is_excluded(&cache_path)
                && seen.insert(cache_path.clone())
            {
                paths.push(cache_path);
            }
        }

        collect_comet_narrow_caches(local_appdata_path, &mut paths, &mut seen, config);
    }

    // Scan Firefox profiles (need to glob for profile directories)
    if let Some(ref local_appdata_path) = local_appdata {
        let firefox_profiles = local_appdata_path
            .join("Mozilla")
            .join("Firefox")
            .join("Profiles");
        if firefox_profiles.exists() {
            for entry in WalkDir::new(&firefox_profiles)
                .max_depth(2)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_dir()
                    && path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.contains(".default"))
                        .unwrap_or(false)
                {
                    let cache2_path = path.join("cache2");
                    if cache2_path.exists()
                        && !config.is_excluded(&cache2_path)
                        && seen.insert(cache2_path.clone())
                    {
                        paths.push(cache2_path);
                    }
                }
            }
        }
    }

    let mut paths_with_sizes: Vec<(PathBuf, u64)> = paths
        .into_iter()
        .filter(|p| p.exists())
        .map(|p| {
            let size = utils::calculate_dir_size(&p);
            (p, size)
        })
        .filter(|(_, size)| *size > 0)
        .collect();
    paths_with_sizes.sort_by(|a, b| b.1.cmp(&a.1));

    for (path, size) in paths_with_sizes.iter() {
        result.items += 1;
        result.size_bytes += size;
        result.paths.push(path.clone());
    }

    Ok(result)
}

/// Clean (delete) a browser cache directory by moving it to the Recycle Bin
pub fn clean(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    crate::trash_ops::delete(path)
        .with_context(|| format!("Failed to delete browser cache: {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chromium_profile_dir_detection() {
        assert!(is_chromium_profile_dir("Default"));
        assert!(is_chromium_profile_dir("Profile 1"));
        assert!(is_chromium_profile_dir("Guest Profile"));
        assert!(!is_chromium_profile_dir("Crashpad"));
        assert!(!is_chromium_profile_dir("Local State"));
    }

    #[test]
    fn join_localappdata_builds_path() {
        let base = Path::new(r"C:\Users\test\AppData\Local");
        let p = join_localappdata(base, &["Google", "Chrome", "User Data"]);
        assert!(p.ends_with("User Data"));
        assert!(p.to_string_lossy().contains("Chrome"));
    }

    #[test]
    fn comet_narrow_cache_allowlist_count() {
        assert_eq!(COMET_PROFILE_NARROW_CACHE_DIRS.len(), 5);
    }
}
