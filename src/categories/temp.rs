use crate::output::CategoryResult;
use anyhow::Result;
use chrono::{Duration, Utc};
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn scan(_root: &Path) -> Result<CategoryResult> {
    let mut result = CategoryResult::default();
    let mut paths = Vec::new();
    
    let cutoff = Utc::now() - Duration::days(1);
    
    // %TEMP% directory
    if let Ok(temp_dir) = env::var("TEMP") {
        scan_temp_dir(&PathBuf::from(&temp_dir), &cutoff, &mut result, &mut paths)?;
    }
    
    // %LOCALAPPDATA%\Temp
    if let Ok(local_appdata) = env::var("LOCALAPPDATA") {
        let local_temp = PathBuf::from(&local_appdata).join("Temp");
        if local_temp.exists() {
            scan_temp_dir(&local_temp, &cutoff, &mut result, &mut paths)?;
        }
    }
    
    result.paths = paths;
    Ok(result)
}

fn scan_temp_dir(
    temp_path: &Path,
    cutoff: &chrono::DateTime<Utc>,
    result: &mut CategoryResult,
    paths: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in WalkDir::new(temp_path)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                if let Ok(modified) = metadata.modified() {
                    let modified_dt: chrono::DateTime<Utc> = modified.into();
                    if modified_dt < *cutoff {
                        result.items += 1;
                        result.size_bytes += metadata.len();
                        paths.push(entry.path().to_path_buf());
                    }
                }
            }
        }
    }
    
    Ok(())
}

pub fn clean(path: &Path) -> Result<()> {
    trash::delete(path)?;
    Ok(())
}
