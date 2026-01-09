use crate::output::CategoryResult;
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Scan Downloads folder for old files
/// 
/// Filters files older than the specified age threshold (default 30 days)
pub fn scan(_root: &Path, min_age_days: u64) -> Result<CategoryResult> {
    let mut result = CategoryResult::default();
    let mut paths = Vec::new();
    
    let cutoff = Utc::now() - Duration::days(min_age_days as i64);
    
    // Get Downloads folder
    let downloads_path = if let Ok(user_profile) = env::var("USERPROFILE") {
        PathBuf::from(&user_profile).join("Downloads")
    } else {
        return Ok(result); // Can't find Downloads folder
    };
    
    if !downloads_path.exists() {
        return Ok(result); // Downloads folder doesn't exist
    }
    
    // Scan Downloads directory
    for entry in WalkDir::new(&downloads_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                if let Ok(modified) = metadata.modified() {
                    let modified_dt: chrono::DateTime<Utc> = modified.into();
                    if modified_dt < cutoff {
                        result.items += 1;
                        result.size_bytes += metadata.len();
                        paths.push(entry.path().to_path_buf());
                    }
                }
            }
        }
    }
    
    result.paths = paths;
    Ok(result)
}

/// Clean (delete) a file from Downloads by moving it to the Recycle Bin
pub fn clean(path: &Path) -> Result<()> {
    trash::delete(path)
        .with_context(|| format!("Failed to delete file: {}", path.display()))?;
    Ok(())
}
