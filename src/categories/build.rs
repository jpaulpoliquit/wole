use crate::config::{CategoryConfig, Config};
use crate::output::CategoryResult;
use crate::project;
use crate::utils;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// Default build artifact directories to detect
const DEFAULT_BUILD_ARTIFACTS: &[&str] = &[
    "node_modules",
    "target",
    "bin",
    "obj",
    "dist",
    "build",
    ".next",
    ".nuxt",
    ".output",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".venv",
    "venv",
    ".gradle",
    ".parcel-cache",
    ".turbo",
    ".angular",
    ".svelte-kit",
    "coverage",
    ".nyc_output",
];

/// Get the list of build artifacts, merging defaults with custom artifacts from config
fn get_build_artifacts(config: Option<&CategoryConfig>) -> Vec<String> {
    let mut artifacts: Vec<String> = DEFAULT_BUILD_ARTIFACTS
        .iter()
        .map(|s| s.to_string())
        .collect();

    if let Some(cfg) = config {
        // Add custom artifacts, avoiding duplicates
        for custom in &cfg.custom_artifacts {
            if !artifacts.contains(custom) {
                artifacts.push(custom.clone());
            }
        }
    }

    artifacts
}

/// Project artifact information
#[derive(Debug, Clone)]
pub struct ProjectArtifact {
    pub project_path: PathBuf,
    pub project_name: String,
    pub artifact_path: PathBuf,
    pub artifact_type: String,
    pub size_bytes: u64,
    pub is_active: bool,
}

/// Scan for build artifacts grouped by project
///
/// Only returns build artifacts from inactive projects (projects that haven't been
/// accessed for at least `project_age_days`). This prevents deletion of build
/// artifacts from projects currently being worked on.
pub fn scan(
    root: &Path,
    project_age_days: u64,
    config: Option<&CategoryConfig>,
    global_config: &Config,
) -> Result<CategoryResult> {
    let mut result = CategoryResult::default();

    // Get the list of artifacts to scan (defaults + custom from config)
    let artifacts_to_scan = get_build_artifacts(config);

    // Check if root itself is a project - if so, only scan that
    let all_project_roots = if crate::project::detect_project_type(root).is_some() {
        vec![root.to_path_buf()]
    } else {
        // Walk to find projects (with exclusion filtering)
        project::find_project_roots(root, global_config)
    };

    // Filter to only inactive projects (safety feature: don't delete from active projects)
    let inactive_project_roots: Vec<PathBuf> = all_project_roots
        .par_iter()
        .filter_map(|project_root| {
            // Check if project is inactive (not recently modified)
            match project::is_project_active(project_root, project_age_days) {
                Ok(false) => Some(project_root.clone()), // Inactive - include it
                Ok(true) => None,                        // Active - skip it
                Err(_) => None, // Error checking activity - skip for safety
            }
        })
        .collect();

    // Collect all artifact paths from inactive projects only (fast check for existence)
    let all_artifact_paths: Vec<PathBuf> = inactive_project_roots
        .par_iter()
        .flat_map(|project_root| find_build_artifacts(project_root, &artifacts_to_scan))
        .filter(|p| p.exists())
        .collect();

    // Calculate sizes sequentially per artifact to avoid disk thrashing
    // Individually, calculate_dir_size is still parallel
    let mut artifacts_with_sizes: Vec<(PathBuf, u64)> = all_artifact_paths
        .iter()
        .map(|path| {
            let size = utils::calculate_dir_size(path);
            (path.clone(), size)
        })
        .filter(|(_, size)| *size > 0)
        .collect();

    // Sort by size descending (biggest first)
    artifacts_with_sizes.par_sort_by(|a, b| b.1.cmp(&a.1));

    // Build result
    for (path, size) in artifacts_with_sizes {
        result.items += 1;
        result.size_bytes += size;
        result.paths.push(path);
    }

    Ok(result)
}

/// Find build artifact directories in a project
fn find_build_artifacts(project_path: &Path, artifacts_to_scan: &[String]) -> Vec<PathBuf> {
    let mut artifacts = Vec::new();

    for artifact_name in artifacts_to_scan {
        let artifact_path = project_path.join(artifact_name);
        if artifact_path.exists() && artifact_path.is_dir() {
            artifacts.push(artifact_path);
        }
    }

    artifacts
}

/// Clean (delete) a build artifact directory by moving it to the Recycle Bin
pub fn clean(path: &Path) -> Result<()> {
    trash::delete(path)
        .with_context(|| format!("Failed to delete build artifact: {}", path.display()))?;
    Ok(())
}
