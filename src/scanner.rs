use crate::categories;
use crate::cli::ScanOptions;
use crate::output::ScanResults;
use anyhow::Result;
use std::path::Path;

pub fn scan_all(path: &Path, options: ScanOptions) -> Result<ScanResults> {
    let mut results = ScanResults::default();
    
    if options.cache {
        results.cache = categories::cache::scan(path)?;
    }
    
    if options.temp {
        results.temp = categories::temp::scan(path)?;
    }
    
    if options.trash {
        results.trash = categories::trash::scan()?;
    }
    
    if options.build {
        results.build = categories::build::scan(path, options.project_age_days)?;
    }
    
    Ok(results)
}
