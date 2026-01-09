use crate::theme::Theme;
use serde::Serialize;
use std::path::PathBuf;

/// Output verbosity mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Quiet,       // Only errors
    Normal,      // Standard output
    Verbose,     // More details
    VeryVerbose, // All details including file paths
}

#[derive(Default, Debug, Clone)]
pub struct ScanResults {
    pub cache: CategoryResult,
    pub app_cache: CategoryResult,
    pub temp: CategoryResult,
    pub trash: CategoryResult,
    pub build: CategoryResult,
    pub downloads: CategoryResult,
    pub large: CategoryResult,
    pub old: CategoryResult,
    pub browser: CategoryResult,
    pub system: CategoryResult,
    pub empty: CategoryResult,
    pub duplicates: CategoryResult,
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
    app_cache: JsonCategory,
    temp: JsonCategory,
    trash: JsonCategory,
    build: JsonCategory,
    downloads: JsonCategory,
    large: JsonCategory,
    old: JsonCategory,
    browser: JsonCategory,
    system: JsonCategory,
    empty: JsonCategory,
    duplicates: JsonCategory,
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

pub fn print_human(results: &ScanResults, mode: OutputMode) {
    if mode == OutputMode::Quiet {
        return;
    }

    println!();
    println!("{}", Theme::header("Sweeper Scan Results"));
    println!("{}", Theme::divider_bold(60));
    println!();
    println!(
        "{:<15} {:>8} {:>12} {:>20}",
        Theme::primary("Category"),
        Theme::primary("Items"),
        Theme::primary("Size"),
        Theme::primary("Status")
    );
    println!("{}", Theme::divider(60));

    let categories = [
        ("Package cache", &results.cache, "[OK] Safe to clean"),
        (
            "Application cache",
            &results.app_cache,
            "[OK] Safe to clean",
        ),
        ("Temp", &results.temp, "[OK] Safe to clean"),
        ("Trash", &results.trash, "[OK] Safe to clean"),
        ("Build", &results.build, "[OK] Inactive projects"),
        ("Downloads", &results.downloads, "[OK] Old files"),
        ("Large", &results.large, "[!] Review suggested"),
        ("Old", &results.old, "[!] Review suggested"),
        ("Browser", &results.browser, "[OK] Safe to clean"),
        ("System", &results.system, "[OK] Safe to clean"),
        ("Empty", &results.empty, "[OK] Safe to clean"),
        ("Duplicates", &results.duplicates, "[!] Review suggested"),
    ];

    for (name, result, status) in categories {
        if result.items > 0 {
            let status_colored = if status.starts_with("[OK]") {
                Theme::status_safe(status)
            } else {
                Theme::status_review(status)
            };
            println!(
                "{:<15} {:>8} {:>12} {:>20}",
                Theme::category(name),
                Theme::value(&result.items.to_string()),
                Theme::size(&result.size_human()),
                status_colored
            );

            // In verbose mode, show first few paths
            if mode == OutputMode::Verbose && !result.paths.is_empty() {
                let show_count = std::cmp::min(3, result.paths.len());
                for path in result.paths.iter().take(show_count) {
                    println!("  {}", Theme::muted(&path.display().to_string()));
                }
                if result.paths.len() > show_count {
                    println!(
                        "  {} ... and {} more",
                        Theme::muted(""),
                        Theme::muted(&(result.paths.len() - show_count).to_string())
                    );
                }
            }

            // In very verbose mode, show all paths
            if mode == OutputMode::VeryVerbose {
                for path in &result.paths {
                    println!("  {}", Theme::muted(&path.display().to_string()));
                }
            }
        }
    }

    let total_items = results.cache.items
        + results.app_cache.items
        + results.temp.items
        + results.trash.items
        + results.build.items
        + results.downloads.items
        + results.large.items
        + results.old.items
        + results.browser.items
        + results.system.items
        + results.empty.items
        + results.duplicates.items;
    let total_bytes = results.cache.size_bytes
        + results.app_cache.size_bytes
        + results.temp.size_bytes
        + results.trash.size_bytes
        + results.build.size_bytes
        + results.downloads.size_bytes
        + results.large.size_bytes
        + results.old.size_bytes
        + results.browser.size_bytes
        + results.system.size_bytes
        + results.empty.size_bytes
        + results.duplicates.size_bytes;

    println!("{}", Theme::divider(60));

    if total_items == 0 {
        println!(
            "{}",
            Theme::success("Your system is clean! No reclaimable space found.")
        );
    } else {
        println!(
            "{:<15} {:>8} {:>12} {:>20}",
            Theme::header("Total"),
            Theme::value(&total_items.to_string()),
            Theme::size(&bytesize::to_string(total_bytes, true)),
            Theme::success("Reclaimable")
        );
        println!();
        println!(
            "Run {} to remove these files.",
            Theme::command("sweeper clean --all")
        );
    }
    println!();
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
                paths: results
                    .cache
                    .paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
            app_cache: JsonCategory {
                items: results.app_cache.items,
                size_bytes: results.app_cache.size_bytes,
                size_human: results.app_cache.size_human(),
                paths: results
                    .app_cache
                    .paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
            temp: JsonCategory {
                items: results.temp.items,
                size_bytes: results.temp.size_bytes,
                size_human: results.temp.size_human(),
                paths: results
                    .temp
                    .paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
            trash: JsonCategory {
                items: results.trash.items,
                size_bytes: results.trash.size_bytes,
                size_human: results.trash.size_human(),
                paths: results
                    .trash
                    .paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
            build: JsonCategory {
                items: results.build.items,
                size_bytes: results.build.size_bytes,
                size_human: results.build.size_human(),
                paths: results
                    .build
                    .paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
            downloads: JsonCategory {
                items: results.downloads.items,
                size_bytes: results.downloads.size_bytes,
                size_human: results.downloads.size_human(),
                paths: results
                    .downloads
                    .paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
            large: JsonCategory {
                items: results.large.items,
                size_bytes: results.large.size_bytes,
                size_human: results.large.size_human(),
                paths: results
                    .large
                    .paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
            old: JsonCategory {
                items: results.old.items,
                size_bytes: results.old.size_bytes,
                size_human: results.old.size_human(),
                paths: results
                    .old
                    .paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
            browser: JsonCategory {
                items: results.browser.items,
                size_bytes: results.browser.size_bytes,
                size_human: results.browser.size_human(),
                paths: results
                    .browser
                    .paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
            system: JsonCategory {
                items: results.system.items,
                size_bytes: results.system.size_bytes,
                size_human: results.system.size_human(),
                paths: results
                    .system
                    .paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
            empty: JsonCategory {
                items: results.empty.items,
                size_bytes: results.empty.size_bytes,
                size_human: results.empty.size_human(),
                paths: results
                    .empty
                    .paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
            duplicates: JsonCategory {
                items: results.duplicates.items,
                size_bytes: results.duplicates.size_bytes,
                size_human: results.duplicates.size_human(),
                paths: results
                    .duplicates
                    .paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            },
        },
        summary: JsonSummary {
            total_items: results.cache.items
                + results.app_cache.items
                + results.temp.items
                + results.trash.items
                + results.build.items
                + results.downloads.items
                + results.large.items
                + results.old.items
                + results.browser.items
                + results.system.items
                + results.empty.items
                + results.duplicates.items,
            total_bytes: results.cache.size_bytes
                + results.app_cache.size_bytes
                + results.temp.size_bytes
                + results.trash.size_bytes
                + results.build.size_bytes
                + results.downloads.size_bytes
                + results.large.size_bytes
                + results.old.size_bytes
                + results.browser.size_bytes
                + results.system.size_bytes
                + results.empty.size_bytes
                + results.duplicates.size_bytes,
            total_human: bytesize::to_string(
                results.cache.size_bytes
                    + results.app_cache.size_bytes
                    + results.temp.size_bytes
                    + results.trash.size_bytes
                    + results.build.size_bytes
                    + results.downloads.size_bytes
                    + results.large.size_bytes
                    + results.old.size_bytes
                    + results.browser.size_bytes
                    + results.system.size_bytes
                    + results.empty.size_bytes
                    + results.duplicates.size_bytes,
                true,
            ),
        },
    };

    println!("{}", serde_json::to_string_pretty(&json_results)?);
    Ok(())
}

pub fn print_analyze(results: &ScanResults, mode: OutputMode) {
    if mode == OutputMode::Quiet {
        return;
    }

    println!();
    println!("Scan Results");
    println!();

    // Define categories with their display names
    let mut categories: Vec<(&str, &CategoryResult)> = vec![
        ("Trash", &results.trash),
        ("Large Files", &results.large),
        ("System Cache", &results.system),
        ("Build Artifacts", &results.build),
        ("Old Downloads", &results.downloads),
        ("Duplicates", &results.duplicates),
        ("Old Files", &results.old),
        ("Temp Files", &results.temp),
        ("Package Cache", &results.cache),
        ("Application Cache", &results.app_cache),
        ("Browser Cache", &results.browser),
        ("Empty Folders", &results.empty),
    ];

    // Filter out categories with no items and sort by size descending
    categories.retain(|(_, result)| result.items > 0);
    categories.sort_by(|a, b| b.1.size_bytes.cmp(&a.1.size_bytes));

    // Print table header
    println!("{:<25} {:>10} {:>12}", "Category", "Files", "Size");
    println!("{}", "─".repeat(47));

    // Print category rows
    for (name, result) in &categories {
        println!(
            "{:<25} {:>10} {:>12}",
            name,
            format_number(result.items as u64),
            result.size_human()
        );
    }

    // Calculate totals
    let total_items = results.cache.items
        + results.app_cache.items
        + results.temp.items
        + results.trash.items
        + results.build.items
        + results.downloads.items
        + results.large.items
        + results.old.items
        + results.browser.items
        + results.system.items
        + results.empty.items
        + results.duplicates.items;
    let total_bytes = results.cache.size_bytes
        + results.app_cache.size_bytes
        + results.temp.size_bytes
        + results.trash.size_bytes
        + results.build.size_bytes
        + results.downloads.size_bytes
        + results.large.size_bytes
        + results.old.size_bytes
        + results.browser.size_bytes
        + results.system.size_bytes
        + results.empty.size_bytes
        + results.duplicates.size_bytes;

    // Print separator and total
    println!("{}", "─".repeat(47));
    println!(
        "{:<25} {:>10} {:>12}",
        "Total",
        format_number(total_items as u64),
        bytesize::to_string(total_bytes, true)
    );
    println!();
}

/// Print disk insights in CLI format with progress bars
pub fn print_disk_insights(
    insights: &crate::disk_usage::DiskInsights,
    root_path: &std::path::Path,
    top_n: usize,
    _sort_by: crate::disk_usage::SortBy,
    mode: OutputMode,
) {
    if mode == OutputMode::Quiet {
        return;
    }

    use crate::disk_usage::get_top_folders;

    // Get top folders
    let top_folders = get_top_folders(&insights.root, top_n);

    println!();
    println!(
        "{}  {}  |  Total: {}  |  {} files",
        Theme::header("Disk Insights"),
        Theme::primary(&root_path.display().to_string()),
        Theme::size(&bytesize::to_string(insights.total_size, true)),
        Theme::value(&format_number(insights.total_files))
    );
    println!();

    // Show root with 100% bar
    let root_bar = render_progress_bar(100.0, 20);
    println!(
        "{}  {}  {}  {}",
        Theme::secondary("#"),
        root_bar,
        Theme::value("100.0%"),
        Theme::size(&bytesize::to_string(insights.total_size, true))
    );
    println!("   {}", Theme::muted(&root_path.display().to_string()));
    println!();

    // Show top folders
    for (i, folder) in top_folders.iter().enumerate() {
        let num = i + 1;
        let bar = render_progress_bar(folder.percentage, 20);
        let size_str = bytesize::to_string(folder.size, true);
        let files_str = format_number(folder.file_count);

        println!(
            "{}  {}  {}  {}  {}  {}",
            Theme::value(&num.to_string()),
            bar,
            Theme::value(&format!("{:.1}%", folder.percentage)),
            Theme::size(&size_str),
            Theme::category(&folder.name),
            Theme::muted(&format!("({} files)", files_str))
        );
    }

    // Show largest files if available
    if !insights.largest_files.is_empty() {
        println!();
        println!("{}", Theme::divider(60));
        println!();
        println!("{}", Theme::primary("Largest Files:"));
        for (file_path, size) in insights.largest_files.iter().take(5) {
            let relative = crate::utils::to_relative_path(file_path, root_path);
            println!(
                "  {}  {}",
                Theme::size(&bytesize::to_string(*size, true)),
                Theme::muted(&relative)
            );
        }
    }

    println!();
    if mode == OutputMode::Normal || mode == OutputMode::Verbose {
        println!(
            "Run {} to explore interactively.",
            Theme::command("sweeper analyze --interactive")
        );
    }
    println!();
}

/// Render a progress bar with filled and empty blocks
fn render_progress_bar(percentage: f64, width: usize) -> String {
    let filled = (percentage / 100.0 * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width.saturating_sub(filled);
    format!(
        "{}{}",
        Theme::size(&"█".repeat(filled)),
        Theme::muted(&"░".repeat(empty))
    )
}

/// Format number with commas
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
