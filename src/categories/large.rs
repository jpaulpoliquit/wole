use crate::output::CategoryResult;
use crate::project;
use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// System directories to skip during large file scanning
const SYSTEM_DIRS: &[&str] = &[
    "Windows",
    "Program Files",
    "Program Files (x86)",
    "ProgramData",
    "$Recycle.Bin",
    "System Volume Information",
    ".git",
];

/// Scan for large files in user directories
/// 
/// Scans Downloads, Documents, and Desktop folders for files larger than
/// the specified threshold. Skips system directories and active project directories.
pub fn scan(_root: &Path, min_size_bytes: u64) -> Result<CategoryResult> {
    let mut result = CategoryResult::default();
    let mut paths = Vec::new();
    
    // Get user directories to scan
    let user_dirs = get_user_directories()?;
    
    for dir in user_dirs {
        scan_directory(&dir, min_size_bytes, &mut result, &mut paths)?;
    }
    
    result.paths = paths;
    Ok(result)
}

/// Get user directories to scan (Downloads, Documents, Desktop)
fn get_user_directories() -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    
    if let Ok(user_profile) = env::var("USERPROFILE") {
        let profile_path = PathBuf::from(&user_profile);
        dirs.push(profile_path.join("Downloads"));
        dirs.push(profile_path.join("Documents"));
        dirs.push(profile_path.join("Desktop"));
    }
    
    Ok(dirs)
}

/// Scan a directory for large files, skipping system dirs and active projects
fn scan_directory(
    dir: &Path,
    min_size_bytes: u64,
    result: &mut CategoryResult,
    paths: &mut Vec<PathBuf>,
) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    
    for entry in WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        
        // Skip system directories
        if is_system_directory(path) {
            continue;
        }
        
        // Skip active project directories
        if let Some(project_root) = crate::git::find_git_root(path) {
            if let Ok(true) = project::is_project_active(&project_root, 14) {
                continue; // Skip active projects
            }
        }
        
        // Check if it's a file and meets size threshold
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                // Skip hidden files
                if is_hidden_file(path) {
                    continue;
                }
                
                if metadata.len() >= min_size_bytes {
                    result.items += 1;
                    result.size_bytes += metadata.len();
                    paths.push(path.to_path_buf());
                }
            }
        }
    }
    
    Ok(())
}

/// Check if a path is in a system directory
fn is_system_directory(path: &Path) -> bool {
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            let name_str = name.to_string_lossy();
            if SYSTEM_DIRS.iter().any(|&sys_dir| name_str.eq_ignore_ascii_case(sys_dir)) {
                return true;
            }
        }
    }
    false
}

/// Check if a file is hidden (starts with dot or has hidden attribute)
fn is_hidden_file(path: &Path) -> bool {
    // Check if filename starts with dot
    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            return true;
        }
    }
    
    // On Windows, check hidden attribute
    if let Ok(_metadata) = std::fs::metadata(path) {
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            let attrs = metadata.file_attributes();
            // FILE_ATTRIBUTE_HIDDEN = 0x2
            if attrs & 0x2 != 0 {
                return true;
            }
        }
    }
    
    false
}

/// Clean (delete) a large file by moving it to the Recycle Bin
pub fn clean(path: &Path) -> Result<()> {
    trash::delete(path)
        .with_context(|| format!("Failed to delete large file: {}", path.display()))?;
    Ok(())
}
