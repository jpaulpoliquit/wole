use crate::config::Config;
use crate::output::{CategoryResult, OutputMode};
use crate::scan_events::ScanProgressEvent;
use crate::theme::Theme;
use crate::utils;
use anyhow::{Context, Result};
use bytesize;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

#[cfg(windows)]
use winreg::enums::*;
#[cfg(windows)]
use winreg::RegKey;

/// Information about an installed application
#[derive(Debug, Clone)]
struct InstalledApp {
    display_name: String,
    install_location: PathBuf,
    publisher: Option<String>,
    estimated_size: Option<u64>,
}

/// Check if an application should be excluded (only truly system-critical apps)
fn should_exclude_app(app: &InstalledApp) -> bool {
    // Only exclude apps in critical Windows system directories
    // This allows Microsoft Store apps and regular Microsoft applications
    let install_str = app.install_location.to_string_lossy().to_lowercase();
    
    // Exclude only core Windows system directories
    if install_str.contains("\\windows\\system32\\")
        || install_str.contains("\\windows\\syswow64\\")
        || install_str.contains("\\windows\\winsxs\\")
        || install_str.contains("\\windows\\servicing\\")
    {
        return true;
    }

    // Exclude Windows Update and Windows Defender (critical security components)
    if let Some(ref publisher) = app.publisher {
        if publisher.contains("Microsoft Corporation") || publisher.contains("Microsoft") {
            let name_lower = app.display_name.to_lowercase();
            // Only exclude critical Windows security/update components
            if name_lower.contains("windows defender")
                || name_lower.contains("windows security")
                || name_lower == "windows update"
                || name_lower.starts_with("windows update ")
            {
                return true;
            }
        }
    }

    false
}

#[cfg(windows)]
/// Read installed applications from Windows registry
fn read_registry_apps() -> Result<Vec<InstalledApp>> {
    let mut apps = Vec::new();

    // Registry paths to scan
    let registry_paths = vec![
        (
            RegKey::predef(HKEY_LOCAL_MACHINE),
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
        (
            RegKey::predef(HKEY_CURRENT_USER),
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
        (
            RegKey::predef(HKEY_LOCAL_MACHINE),
            "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
    ];

    for (hive, path) in registry_paths {
        if let Ok(key) = hive.open_subkey(path) {
            // Enumerate all subkeys (each represents an installed app)
            for subkey_name in key.enum_keys().map(|x| x.unwrap()) {
                if let Ok(subkey) = key.open_subkey(&subkey_name) {
                    // Read DisplayName
                    let display_name = subkey
                        .get_value::<String, _>("DisplayName")
                        .unwrap_or_default();

                    if display_name.is_empty() {
                        continue;
                    }

                    // Read InstallLocation
                    let install_location_str = subkey
                        .get_value::<String, _>("InstallLocation")
                        .unwrap_or_default();

                    if install_location_str.is_empty() {
                        continue;
                    }

                    let install_location = PathBuf::from(install_location_str);

                    // Verify the directory exists
                    if !install_location.exists() || !install_location.is_dir() {
                        continue;
                    }

                    // Read Publisher
                    let publisher = subkey
                        .get_value::<String, _>("Publisher")
                        .ok()
                        .filter(|s| !s.is_empty());

                    // Read EstimatedSize (in bytes, stored as DWORD)
                    let estimated_size = subkey
                        .get_value::<u32, _>("EstimatedSize")
                        .ok()
                        .map(|size| size as u64 * 1024); // Convert KB to bytes

                    let app = InstalledApp {
                        display_name,
                        install_location,
                        publisher,
                        estimated_size,
                    };

                    // Filter out system-critical apps
                    if !should_exclude_app(&app) {
                        apps.push(app);
                    }
                }
            }
        }
    }

    Ok(apps)
}

#[cfg(not(windows))]
/// Stub for non-Windows platforms
fn read_registry_apps() -> Result<Vec<InstalledApp>> {
    Ok(Vec::new())
}

/// Scan for installed applications
pub fn scan(_root: &Path, config: &Config, output_mode: OutputMode) -> Result<CategoryResult> {
    let mut result = CategoryResult::default();

    #[cfg(windows)]
    {
        if output_mode != OutputMode::Quiet {
            println!(
                "  {} Scanning installed applications...",
                Theme::muted("→")
            );
        }

        let apps = read_registry_apps()?;

        let mut apps_with_sizes: Vec<(PathBuf, u64)> = Vec::new();

        for app in apps {
            if config.is_excluded(&app.install_location) {
                continue;
            }

            // Calculate size: use EstimatedSize from registry if available, otherwise walk directory
            let size = if let Some(est_size) = app.estimated_size {
                est_size
            } else {
                utils::calculate_dir_size(&app.install_location)
            };

            if size > 0 {
                apps_with_sizes.push((app.install_location.clone(), size));
                if output_mode != OutputMode::Quiet {
                    println!(
                        "    {} Found {} ({})",
                        Theme::muted("•"),
                        app.display_name,
                        Theme::size(&bytesize::to_string(size, true))
                    );
                }
            }
        }

        // Sort by size descending
        apps_with_sizes.sort_by(|a, b| b.1.cmp(&a.1));

        // Store results
        result.paths = apps_with_sizes.iter().map(|(p, _)| p.clone()).collect();
        result.size_bytes = apps_with_sizes.iter().map(|(_, size)| *size).sum();
        result.items = apps_with_sizes.len();

        if output_mode != OutputMode::Quiet && !apps_with_sizes.is_empty() {
            println!(
                "  {} Found {} installed applications:",
                Theme::muted("→"),
                apps_with_sizes.len()
            );
            let show_count = match output_mode {
                OutputMode::VeryVerbose => apps_with_sizes.len(),
                OutputMode::Verbose => apps_with_sizes.len(),
                OutputMode::Normal => apps_with_sizes.len().min(10),
                OutputMode::Quiet => 0,
            };

            for (i, (path, size)) in apps_with_sizes.iter().take(show_count).enumerate() {
                let size_str = bytesize::to_string(*size, true);
                println!(
                    "      {} {} ({})",
                    Theme::muted("→"),
                    path.display(),
                    Theme::size(&size_str)
                );

                if i == 9 && output_mode == OutputMode::Normal && apps_with_sizes.len() > 10 {
                    println!(
                        "      {} ... and {} more (use -v to see all)",
                        Theme::muted("→"),
                        apps_with_sizes.len() - 10
                    );
                    break;
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        if output_mode != OutputMode::Quiet {
            println!(
                "  {} Applications scanning is only available on Windows",
                Theme::muted("→")
            );
        }
    }

    Ok(result)
}

/// Scan with real-time progress events (for TUI)
pub fn scan_with_progress(_root: &Path, tx: &Sender<ScanProgressEvent>) -> Result<CategoryResult> {
    const CATEGORY: &str = "Installed Applications";
    let mut result = CategoryResult::default();

    #[cfg(windows)]
    {
        let apps = read_registry_apps()?;

        let total = apps.len() as u64;

        let _ = tx.send(ScanProgressEvent::CategoryStarted {
            category: CATEGORY.to_string(),
            total_units: Some(total),
            current_path: None,
        });

        let mut apps_with_sizes: Vec<(PathBuf, u64)> = Vec::new();

        for (idx, app) in apps.iter().enumerate() {
            // Calculate size
            let size = if let Some(est_size) = app.estimated_size {
                est_size
            } else {
                utils::calculate_dir_size(&app.install_location)
            };

            if size > 0 {
                apps_with_sizes.push((app.install_location.clone(), size));
            }

            let completed = (idx + 1) as u64;
            let _ = tx.send(ScanProgressEvent::CategoryProgress {
                category: CATEGORY.to_string(),
                completed_units: completed,
                total_units: Some(total),
                current_path: Some(app.install_location.clone()),
            });
        }

        // Sort by size descending
        apps_with_sizes.sort_by(|a, b| b.1.cmp(&a.1));

        // Build final result
        for (path, size) in apps_with_sizes {
            result.items += 1;
            result.size_bytes += size;
            result.paths.push(path);
        }

        let _ = tx.send(ScanProgressEvent::CategoryFinished {
            category: CATEGORY.to_string(),
            items: result.items,
            size_bytes: result.size_bytes,
        });
    }

    #[cfg(not(windows))]
    {
        let _ = tx.send(ScanProgressEvent::CategoryStarted {
            category: CATEGORY.to_string(),
            total_units: Some(0),
            current_path: None,
        });

        let _ = tx.send(ScanProgressEvent::CategoryFinished {
            category: CATEGORY.to_string(),
            items: 0,
            size_bytes: 0,
        });
    }

    Ok(result)
}

/// Clean (delete) an installed application directory by moving it to the Recycle Bin
pub fn clean(path: &Path) -> Result<()> {
    trash::delete(path).with_context(|| {
        format!(
            "Failed to delete installed application directory: {}",
            path.display()
        )
    })?;
    Ok(())
}
