use crate::git;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Node,
    Rust,
    DotNet,
    Python,
    Java,
}

/// Detect project type by looking for marker files
pub fn detect_project_type(path: &Path) -> Option<ProjectType> {
    // Check for Node.js
    if path.join("package.json").exists() {
        return Some(ProjectType::Node);
    }
    
    // Check for Rust
    if path.join("Cargo.toml").exists() {
        return Some(ProjectType::Rust);
    }
    
    // Check for .NET by globbing
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".csproj") || name_str.ends_with(".sln") {
                return Some(ProjectType::DotNet);
            }
        }
    }
    
    // Check for Python
    if path.join("pyproject.toml").exists() || path.join("requirements.txt").exists() {
        return Some(ProjectType::Python);
    }
    
    // Check for Java
    if path.join("build.gradle").exists() || path.join("pom.xml").exists() {
        return Some(ProjectType::Java);
    }
    
    None
}

/// Get the marker file path for a project type
fn get_marker_file(path: &Path, project_type: ProjectType) -> Option<PathBuf> {
    match project_type {
        ProjectType::Node => Some(path.join("package.json")),
        ProjectType::Rust => Some(path.join("Cargo.toml")),
        ProjectType::DotNet => {
            // Try to find any .csproj or .sln file
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.ends_with(".csproj") || name_str.ends_with(".sln") {
                        return Some(entry.path());
                    }
                }
            }
            None
        }
        ProjectType::Python => {
            if path.join("pyproject.toml").exists() {
                Some(path.join("pyproject.toml"))
            } else if path.join("requirements.txt").exists() {
                Some(path.join("requirements.txt"))
            } else {
                None
            }
        }
        ProjectType::Java => {
            if path.join("build.gradle").exists() {
                Some(path.join("build.gradle"))
            } else if path.join("pom.xml").exists() {
                Some(path.join("pom.xml"))
            } else {
                None
            }
        }
    }
}

/// Check if a project is active (recently modified or has uncommitted changes)
pub fn is_project_active(path: &Path, age_days: u64) -> Result<bool> {
    // First check if there's a git repo
    if let Some(git_root) = git::find_git_root(path) {
        // Check if repo is dirty
        if let Ok(true) = git::is_dirty(&git_root) {
            return Ok(true); // Active - has uncommitted changes
        }
        
        // Check last commit date
        if let Ok(Some(last_commit)) = git::last_commit_date(&git_root) {
            let cutoff = Utc::now() - Duration::days(age_days as i64);
            if last_commit > cutoff {
                return Ok(true); // Active - recent commit
            }
        }
    }
    
    // Fallback: check project file modification time
    if let Some(project_type) = detect_project_type(path) {
        if let Some(marker_file) = get_marker_file(path, project_type) {
            if let Ok(metadata) = std::fs::metadata(&marker_file) {
                if let Ok(modified) = metadata.modified() {
                    let modified_dt: DateTime<Utc> = modified.into();
                    let cutoff = Utc::now() - Duration::days(age_days as i64);
                    if modified_dt > cutoff {
                        return Ok(true); // Active - recent file modification
                    }
                }
            }
        }
    }
    
    Ok(false) // Inactive
}

/// Find all project roots in a directory tree
pub fn find_project_roots(root: &Path) -> Vec<PathBuf> {
    let mut projects = Vec::new();
    let mut seen = std::collections::HashSet::new();
    
    for entry in WalkDir::new(root)
        .max_depth(10) // Limit depth to avoid excessive scanning
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        
        // Skip if we've already seen a parent of this path
        let mut is_subproject = false;
        for existing in &projects {
            if path.starts_with(existing) && path != existing {
                is_subproject = true;
                break;
            }
        }
        if is_subproject {
            continue;
        }
        
        // Check if this is a project root
        if let Some(_project_type) = detect_project_type(path) {
            // Make sure we haven't already added this project
            if !seen.contains(path) {
                projects.push(path.to_path_buf());
                seen.insert(path.to_path_buf());
            }
        }
    }
    
    projects
}
