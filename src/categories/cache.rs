use crate::output::CategoryResult;
use anyhow::Result;
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn scan(_root: &Path) -> Result<CategoryResult> {
    let mut result = CategoryResult::default();
    let mut paths = Vec::new();
    
    // npm cache
    if let Ok(npm_cache) = env::var("LOCALAPPDATA") {
        let npm_path = PathBuf::from(&npm_cache).join("npm-cache");
        if npm_path.exists() {
            let size = calculate_size(&npm_path)?;
            result.items += 1;
            result.size_bytes += size;
            paths.push(npm_path);
        }
    }
    
    // pip cache
    if let Ok(local_appdata) = env::var("LOCALAPPDATA") {
        let pip_cache = PathBuf::from(&local_appdata).join("pip").join("cache");
        if pip_cache.exists() {
            let size = calculate_size(&pip_cache)?;
            result.items += 1;
            result.size_bytes += size;
            paths.push(pip_cache);
        }
    }
    
    // yarn cache
    if let Ok(local_appdata) = env::var("LOCALAPPDATA") {
        let yarn_cache = PathBuf::from(&local_appdata).join("Yarn").join("Cache");
        if yarn_cache.exists() {
            let size = calculate_size(&yarn_cache)?;
            result.items += 1;
            result.size_bytes += size;
            paths.push(yarn_cache);
        }
    }
    
    result.paths = paths;
    Ok(result)
}

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

pub fn clean(path: &Path) -> Result<()> {
    trash::delete(path)?;
    Ok(())
}
