use crate::config::Config;
use crate::output::CategoryResult;
use crate::scan_events::ScanProgressEvent;
use crate::utils;
use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

/// Package manager cache locations to scan
/// Each tuple is (name, path_from_localappdata_or_userprofile)
const CACHE_LOCATIONS: &[(&str, CacheLocation)] = &[
    ("npm", CacheLocation::LocalAppData("npm-cache")),
    ("pip", CacheLocation::LocalAppDataNested(&["pip", "cache"])),
    (
        "yarn",
        CacheLocation::LocalAppDataNested(&["Yarn", "Cache"]),
    ),
    ("pnpm", CacheLocation::LocalAppData("pnpm-cache")),
    ("pnpm-store", CacheLocation::LocalAppData("pnpm-store")),
    (
        "NuGet",
        CacheLocation::LocalAppDataNested(&["NuGet", "v3-cache"]),
    ),
    (
        "Cargo",
        CacheLocation::UserProfileNested(&[".cargo", "registry"]),
    ),
    (
        "Go",
        CacheLocation::UserProfileNested(&["go", "pkg", "mod", "cache"]),
    ),
    (
        "Maven",
        CacheLocation::UserProfileNested(&[".m2", "repository"]),
    ),
    (
        "Gradle",
        CacheLocation::UserProfileNested(&[".gradle", "caches"]),
    ),
];

enum CacheLocation {
    LocalAppData(&'static str),
    LocalAppDataNested(&'static [&'static str]),
    UserProfileNested(&'static [&'static str]),
}

/// Scan for package manager cache directories
///
/// Checks well-known Windows cache locations for various package managers.
/// Uses shared calculate_dir_size for consistent size calculation.
pub fn scan(_root: &Path, config: &Config) -> Result<CategoryResult> {
    let mut result = CategoryResult::default();
    let mut candidates = Vec::new();

    let local_appdata = env::var("LOCALAPPDATA").ok().map(PathBuf::from);
    let userprofile = env::var("USERPROFILE").ok().map(PathBuf::from);

    // 1. Collect candidate paths
    for (_name, location) in CACHE_LOCATIONS {
        let cache_path = match location {
            CacheLocation::LocalAppData(subpath) => local_appdata.as_ref().map(|p| p.join(subpath)),
            CacheLocation::LocalAppDataNested(subpaths) => local_appdata.as_ref().map(|p| {
                let mut path = p.clone();
                for subpath in *subpaths {
                    path = path.join(subpath);
                }
                path
            }),
            CacheLocation::UserProfileNested(subpaths) => userprofile.as_ref().map(|p| {
                let mut path = p.clone();
                for subpath in *subpaths {
                    path = path.join(subpath);
                }
                path
            }),
        };

        if let Some(cache_path) = cache_path {
            if cache_path.exists() && !config.is_excluded(&cache_path) {
                candidates.push(cache_path);
            }
        }
    }

    // 2. Calculate sizes sequentially (one parallel walk at a time)
    let mut paths_with_sizes: Vec<(PathBuf, u64)> = candidates
        .iter()
        .map(|p| {
            let size = utils::calculate_dir_size(p);
            (p.clone(), size)
        })
        .filter(|(_, size)| *size > 0)
        .collect();

    // Sort by size descending
    paths_with_sizes.sort_by(|a, b| b.1.cmp(&a.1));

    for (path, size) in paths_with_sizes {
        result.items += 1;
        result.size_bytes += size;
        result.paths.push(path);
    }

    Ok(result)
}

/// Scan with real-time progress events (for TUI).
/// Scan with real-time progress events (for TUI).
pub fn scan_with_progress(_root: &Path, tx: &Sender<ScanProgressEvent>) -> Result<CategoryResult> {
    const CATEGORY: &str = "Package cache";
    let total = CACHE_LOCATIONS.len() as u64;

    let mut result = CategoryResult::default();
    let mut files_with_sizes: Vec<(PathBuf, u64)> = Vec::new();

    let local_appdata = env::var("LOCALAPPDATA").ok().map(PathBuf::from);
    let userprofile = env::var("USERPROFILE").ok().map(PathBuf::from);

    let _ = tx.send(ScanProgressEvent::CategoryStarted {
        category: CATEGORY.to_string(),
        total_units: Some(total),
        current_path: None,
    });

    // Scan known package manager caches
    for (idx, (_name, location)) in CACHE_LOCATIONS.iter().enumerate() {
        let cache_path = match location {
            CacheLocation::LocalAppData(subpath) => local_appdata.as_ref().map(|p| p.join(subpath)),
            CacheLocation::LocalAppDataNested(subpaths) => local_appdata.as_ref().map(|p| {
                let mut path = p.clone();
                for subpath in *subpaths {
                    path = path.join(subpath);
                }
                path
            }),
            CacheLocation::UserProfileNested(subpaths) => userprofile.as_ref().map(|p| {
                let mut path = p.clone();
                for subpath in *subpaths {
                    path = path.join(subpath);
                }
                path
            }),
        };

        if let Some(cache_path) = cache_path {
            if cache_path.exists() {
                let size = utils::calculate_dir_size(&cache_path);
                if size > 0 {
                    files_with_sizes.push((cache_path.clone(), size));
                }
            }

            let _ = tx.send(ScanProgressEvent::CategoryProgress {
                category: CATEGORY.to_string(),
                completed_units: (idx + 1) as u64,
                total_units: Some(total),
                current_path: Some(cache_path),
            });
        } else {
            let _ = tx.send(ScanProgressEvent::CategoryProgress {
                category: CATEGORY.to_string(),
                completed_units: (idx + 1) as u64,
                total_units: Some(total),
                current_path: None,
            });
        }
    }

    // Sort by size descending
    files_with_sizes.sort_by(|a, b| b.1.cmp(&a.1));

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

/// Clean (delete) a package cache directory by moving it to the Recycle Bin
pub fn clean(path: &Path) -> Result<()> {
    trash::delete(path).with_context(|| {
        format!(
            "Failed to delete package cache directory: {}",
            path.display()
        )
    })?;
    Ok(())
}
