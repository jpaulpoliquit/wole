use crate::output::CategoryResult;
use crate::project;
use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Build artifact directories to detect
const BUILD_ARTIFACTS: &[&str] = &[
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
];

/// Scan for build artifacts in inactive projects
pub fn scan(root: &Path, project_age_days: u64) -> Result<CategoryResult> {
    let mut result = CategoryResult::default();
    let mut paths = Vec::new();
    
    // Find all project roots
    let project_roots = project::find_project_roots(root);
    
    // Filter to inactive projects only
    for project_root in project_roots {
        match project::is_project_active(&project_root, project_age_days) {
            Ok(false) => {
                // Project is inactive - find its build artifacts
                let artifacts = find_build_artifacts(&project_root);
                for artifact_path in artifacts {
                    if artifact_path.exists() {
                        let size = calculate_size(&artifact_path)?;
                        result.items += 1;
                        result.size_bytes += size;
                        paths.push(artifact_path);
                    }
                }
            }
            Ok(true) => {
                // Project is active - skip it
            }
            Err(e) => {
                // Error checking activity - skip with warning
                eprintln!("Warning: Could not check activity for {}: {}", project_root.display(), e);
            }
        }
    }
    
    result.paths = paths;
    Ok(result)
}

/// Find build artifact directories in a project
fn find_build_artifacts(project_path: &Path) -> Vec<PathBuf> {
    let mut artifacts = Vec::new();
    
    for artifact_name in BUILD_ARTIFACTS {
        let artifact_path = project_path.join(artifact_name);
        if artifact_path.exists() {
            artifacts.push(artifact_path);
        }
    }
    
    artifacts
}

/// Calculate the total size of a directory
fn calculate_size(path: &Path) -> Result<u64> {
    let mut total = 0u64;
    
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                total += metadata.len();
            }
        }
    }
    
    Ok(total)
}

/// Clean (delete) a build artifact directory
pub fn clean(path: &Path) -> Result<()> {
    trash::delete(path)?;
    Ok(())
}
