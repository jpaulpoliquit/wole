use crate::categories;
use crate::output::ScanResults;
use anyhow::Result;
use std::io::{self, Write};

pub fn clean_all(results: &ScanResults, skip_confirm: bool) -> Result<()> {
    let total_items = results.cache.items
        + results.temp.items
        + results.trash.items
        + results.build.items;
    
    if total_items == 0 {
        println!("Nothing to clean.");
        return Ok(());
    }
    
    if !skip_confirm {
        print!("Delete {} items? [y/N]: ", total_items);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }
    
    // Clean cache
    if results.cache.items > 0 {
        for path in &results.cache.paths {
            if let Err(e) = categories::cache::clean(path) {
                eprintln!("Warning: Failed to clean {}: {}", path.display(), e);
            }
        }
    }
    
    // Clean temp
    if results.temp.items > 0 {
        for path in &results.temp.paths {
            if let Err(e) = categories::temp::clean(path) {
                eprintln!("Warning: Failed to clean {}: {}", path.display(), e);
            }
        }
    }
    
    // Clean trash
    if results.trash.items > 0 {
        if let Err(e) = categories::trash::clean() {
            eprintln!("Warning: Failed to empty Recycle Bin: {}", e);
        }
    }
    
    // Clean build artifacts
    if results.build.items > 0 {
        for path in &results.build.paths {
            if let Err(e) = categories::build::clean(path) {
                eprintln!("Warning: Failed to clean {}: {}", path.display(), e);
            }
        }
    }
    
    println!("Cleanup complete.");
    Ok(())
}
