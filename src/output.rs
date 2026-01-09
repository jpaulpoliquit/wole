use serde::Serialize;
use std::path::PathBuf;

#[derive(Default, Debug, Clone)]
pub struct ScanResults {
    pub cache: CategoryResult,
    pub temp: CategoryResult,
    pub trash: CategoryResult,
    pub build: CategoryResult,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CategoryResult {
    pub items: usize,
    pub size_bytes: u64,
    pub paths: Vec<PathBuf>,
}

impl CategoryResult {
    pub fn size_human(&self) -> String {
        bytesize::to_string(self.size_bytes, true)
    }
}

#[derive(Serialize)]
struct JsonResults {
    version: String,
    timestamp: String,
    categories: JsonCategories,
    summary: JsonSummary,
}

#[derive(Serialize)]
struct JsonCategories {
    cache: JsonCategory,
    temp: JsonCategory,
    trash: JsonCategory,
    build: JsonCategory,
}

#[derive(Serialize)]
struct JsonCategory {
    items: usize,
    size_bytes: u64,
    size_human: String,
    paths: Vec<String>,
}

#[derive(Serialize)]
struct JsonSummary {
    total_items: usize,
    total_bytes: u64,
    total_human: String,
}

pub fn print_human(results: &ScanResults) {
    println!("sweeper Scan Results");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("{:<15} {:>8} {:>12} {:>20}", "Category", "Items", "Size", "Status");
    println!("{}", "─".repeat(60));
    
    if results.cache.items > 0 {
        println!(
            "{:<15} {:>8} {:>12} {:>20}",
            "Cache",
            results.cache.items,
            results.cache.size_human(),
            "✓ Safe to clean"
        );
    }
    
    if results.temp.items > 0 {
        println!(
            "{:<15} {:>8} {:>12} {:>20}",
            "Temp",
            results.temp.items,
            results.temp.size_human(),
            "✓ Safe to clean"
        );
    }
    
    if results.trash.items > 0 {
        println!(
            "{:<15} {:>8} {:>12} {:>20}",
            "Trash",
            results.trash.items,
            results.trash.size_human(),
            "✓ Safe to clean"
        );
    }
    
    if results.build.items > 0 {
        println!(
            "{:<15} {:>8} {:>12} {:>20}",
            "Build",
            results.build.items,
            results.build.size_human(),
            "✓ Inactive projects"
        );
    }
    
    let total_items = results.cache.items
        + results.temp.items
        + results.trash.items
        + results.build.items;
    let total_bytes = results.cache.size_bytes
        + results.temp.size_bytes
        + results.trash.size_bytes
        + results.build.size_bytes;
    
    println!("{}", "─".repeat(60));
    println!(
        "{:<15} {:>8} {:>12} {:>20}",
        "Total",
        total_items,
        bytesize::to_string(total_bytes, true),
        "Reclaimable"
    );
    println!();
    println!("Run 'sweeper clean' to remove these files.");
}

pub fn print_json(results: &ScanResults) -> anyhow::Result<()> {
    let json_results = JsonResults {
        version: "1.0".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        categories: JsonCategories {
            cache: JsonCategory {
                items: results.cache.items,
                size_bytes: results.cache.size_bytes,
                size_human: results.cache.size_human(),
                paths: results.cache.paths.iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
            temp: JsonCategory {
                items: results.temp.items,
                size_bytes: results.temp.size_bytes,
                size_human: results.temp.size_human(),
                paths: results.temp.paths.iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
            trash: JsonCategory {
                items: results.trash.items,
                size_bytes: results.trash.size_bytes,
                size_human: results.trash.size_human(),
                paths: results.trash.paths.iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
            build: JsonCategory {
                items: results.build.items,
                size_bytes: results.build.size_bytes,
                size_human: results.build.size_human(),
                paths: results.build.paths.iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
        },
        summary: JsonSummary {
            total_items: results.cache.items
                + results.temp.items
                + results.trash.items
                + results.build.items,
            total_bytes: results.cache.size_bytes
                + results.temp.size_bytes
                + results.trash.size_bytes
                + results.build.size_bytes,
            total_human: bytesize::to_string(
                results.cache.size_bytes
                    + results.temp.size_bytes
                    + results.trash.size_bytes
                    + results.build.size_bytes,
                true,
            ),
        },
    };
    
    println!("{}", serde_json::to_string_pretty(&json_results)?);
    Ok(())
}
