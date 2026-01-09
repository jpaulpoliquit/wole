use clap::{Parser, Subcommand, ArgAction};
use std::path::PathBuf;
use anyhow::Context;

use crate::scanner;
use crate::output::{self, OutputMode};
use crate::cleaner;
use crate::size;
use crate::config::Config;
use crate::theme::Theme;
use crate::restore;
use crate::history;

#[derive(Parser)]
#[command(name = "sweeper")]
#[command(version)]
#[command(about = "Reclaim disk space on Windows by cleaning unused files")]
#[command(long_about = "Sweeper is a developer-focused CLI tool that safely identifies and removes \
    unused files to free up disk space.\n\n\
    Examples:\n  \
    sweeper scan --all              # Scan all categories\n  \
    sweeper scan --cache --temp     # Scan specific categories\n  \
    sweeper clean --all -y          # Clean all categories without confirmation\n  \
    sweeper scan --large --min-size 500MB  # Find files over 500MB")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    /// Increase output verbosity (-v, -vv for more)
    #[arg(short = 'v', long, action = ArgAction::Count, global = true)]
    pub verbose: u8,
    
    /// Suppress all output except errors
    #[arg(short = 'q', long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Find cleanable files (dry-run, safe to run anytime)
    #[command(visible_alias = "s")]
    Scan {
        /// Enable all scan categories
        #[arg(short = 'a', long)]
        all: bool,
        
        /// Scan package manager cache directories (npm, pip, nuget, etc.)
        #[arg(long)]
        cache: bool,
        
        /// Scan application cache directories (Discord, VS Code, Slack, etc.)
        #[arg(long)]
        app_cache: bool,
        
        /// Scan temporary files (system temp folders)
        #[arg(long)]
        temp: bool,
        
        /// Scan Recycle Bin contents
        #[arg(long)]
        trash: bool,
        
        /// Scan build artifacts from inactive projects (node_modules, target, etc.)
        #[arg(long)]
        build: bool,
        
        /// Scan Downloads folder for old files
        #[arg(long)]
        downloads: bool,
        
        /// Scan for files over size threshold
        #[arg(long)]
        large: bool,
        
        /// Scan for files not accessed in N days
        #[arg(long)]
        old: bool,
        
        /// Root path to scan (default: home directory)
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,
        
        /// Output results as JSON for scripting
        #[arg(long)]
        json: bool,
        
        /// Project inactivity threshold in days [default: 14]
        #[arg(long, default_value = "14", value_name = "DAYS")]
        project_age: u64,
        
        /// Minimum file age in days for --downloads and --old [default: 30]
        #[arg(long, default_value = "30", value_name = "DAYS")]
        min_age: u64,
        
        /// Minimum file size for --large (e.g., 100MB, 1GB) [default: 100MB]
        #[arg(long, default_value = "100MB", value_name = "SIZE")]
        min_size: String,
        
        /// Exclude paths matching pattern (repeatable)
        #[arg(long, value_name = "PATTERN")]
        exclude: Vec<String>,
    },
    
    /// Delete files found by scan (with confirmation)
    #[command(visible_alias = "c")]
    Clean {
        /// Enable all clean categories
        #[arg(short = 'a', long)]
        all: bool,
        
        /// Clean package manager cache directories (npm, pip, nuget, etc.)
        #[arg(long)]
        cache: bool,
        
        /// Clean application cache directories (Discord, VS Code, Slack, etc.)
        #[arg(long)]
        app_cache: bool,
        
        /// Clean temporary files (system temp folders)
        #[arg(long)]
        temp: bool,
        
        /// Empty Recycle Bin
        #[arg(long)]
        trash: bool,
        
        /// Clean build artifacts from inactive projects (node_modules, target, etc.)
        #[arg(long)]
        build: bool,
        
        /// Clean old files in Downloads folder
        #[arg(long)]
        downloads: bool,
        
        /// Clean files over size threshold
        #[arg(long)]
        large: bool,
        
        /// Clean files not accessed in N days
        #[arg(long)]
        old: bool,
        
        /// Clean browser caches (Chrome, Edge, Firefox)
        #[arg(long)]
        browser: bool,
        
        /// Clean Windows system caches (thumbnails, updates, icons)
        #[arg(long)]
        system: bool,
        
        /// Clean empty folders
        #[arg(long)]
        empty: bool,
        
        /// Clean duplicate files (keeps one copy)
        #[arg(long)]
        duplicates: bool,
        
        /// Root path to scan (default: home directory)
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,
        
        /// Output results as JSON for scripting
        #[arg(long)]
        json: bool,
        
        /// Skip confirmation prompt (use with caution!)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        
        /// Project inactivity threshold in days [default: 14]
        #[arg(long, default_value = "14", value_name = "DAYS")]
        project_age: u64,
        
        /// Minimum file age in days for --downloads and --old [default: 30]
        #[arg(long, default_value = "30", value_name = "DAYS")]
        min_age: u64,
        
        /// Minimum file size for --large (e.g., 100MB, 1GB) [default: 100MB]
        #[arg(long, default_value = "100MB", value_name = "SIZE")]
        min_size: String,
        
        /// Exclude paths matching pattern (repeatable)
        #[arg(long, value_name = "PATTERN")]
        exclude: Vec<String>,
        
        /// Permanently delete (bypass Recycle Bin)
        #[arg(long)]
        permanent: bool,
        
        /// Preview only, don't delete
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Show detailed analysis with file lists
    #[command(visible_alias = "a")]
    Analyze {
        /// Enable disk insights mode (analyze folder sizes, default when no category flags)
        #[arg(long)]
        disk: bool,
        
        /// Launch interactive TUI for disk insights
        #[arg(short = 'i', long)]
        interactive: bool,
        
        /// Maximum depth to scan [default: 3]
        #[arg(long, default_value = "3", value_name = "DEPTH")]
        depth: u8,
        
        /// Show top N folders [default: 10]
        #[arg(long, value_name = "N")]
        top: Option<usize>,
        
        /// Sort order: size, name, or files [default: size]
        #[arg(long, value_name = "SORT")]
        sort: Option<String>,
        
        /// Enable all scan categories (legacy cleanable file analysis)
        #[arg(short = 'a', long)]
        all: bool,
        
        /// Scan package manager cache directories (npm, pip, nuget, etc.)
        #[arg(long)]
        cache: bool,
        
        /// Scan application cache directories (Discord, VS Code, Slack, etc.)
        #[arg(long)]
        app_cache: bool,
        
        /// Scan temporary files (system temp folders)
        #[arg(long)]
        temp: bool,
        
        /// Scan Recycle Bin contents
        #[arg(long)]
        trash: bool,
        
        /// Scan build artifacts from inactive projects (node_modules, target, etc.)
        #[arg(long)]
        build: bool,
        
        /// Scan Downloads folder for old files
        #[arg(long)]
        downloads: bool,
        
        /// Scan for files over size threshold
        #[arg(long)]
        large: bool,
        
        /// Scan for files not accessed in N days
        #[arg(long)]
        old: bool,
        
        /// Scan browser caches (Chrome, Edge, Firefox)
        #[arg(long)]
        browser: bool,
        
        /// Scan Windows system caches (thumbnails, updates, icons)
        #[arg(long)]
        system: bool,
        
        /// Scan for empty folders
        #[arg(long)]
        empty: bool,
        
        /// Scan for duplicate files
        #[arg(long)]
        duplicates: bool,
        
        /// Root path to scan (default: user profile)
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,
        
        /// Project inactivity threshold in days [default: 14]
        #[arg(long, default_value = "14", value_name = "DAYS")]
        project_age: u64,
        
        /// Minimum file age in days for --downloads and --old [default: 30]
        #[arg(long, default_value = "30", value_name = "DAYS")]
        min_age: u64,
        
        /// Minimum file size for --large (e.g., 100MB, 1GB) [default: 100MB]
        #[arg(long, default_value = "100MB", value_name = "SIZE")]
        min_size: String,
        
        /// Exclude paths matching pattern (repeatable)
        #[arg(long, value_name = "PATTERN")]
        exclude: Vec<String>,
    },
    
    /// View or modify configuration
    Config {
        /// Show current configuration
        #[arg(long)]
        show: bool,
        
        /// Reset to defaults
        #[arg(long)]
        reset: bool,
        
        /// Open config file in editor
        #[arg(long)]
        edit: bool,
    },
    
    /// Restore files from the last deletion session
    #[command(visible_alias = "r")]
    Restore {
        /// Restore from the last deletion session
        #[arg(long)]
        last: bool,
        
        /// Restore a specific file path
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,
        
        /// Restore from a specific log file
        #[arg(long, value_name = "LOG_FILE")]
        from: Option<PathBuf>,
    },
}

impl Cli {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
    
    /// Show interactive menu when no command is provided
    pub fn show_interactive_menu() {
        println!();
        println!("{}", Theme::header("Sweeper - Reclaim Disk Space on Windows"));
        println!("{}", Theme::divider_bold(60));
        println!();
        println!("{}", Theme::primary("Available Commands:"));
        println!();
        println!("  {}  {}  {}", 
            Theme::command("scan"), 
            Theme::muted("or"), 
            Theme::command("s"),
        );
        println!("     {} Find cleanable files (safe, dry-run)", Theme::muted("→"));
        println!();
        println!("  {}  {}  {}", 
            Theme::command("clean"), 
            Theme::muted("or"), 
            Theme::command("c"),
        );
        println!("     {} Delete files found by scan", Theme::muted("→"));
        println!();
        println!("  {}  {}  {}", 
            Theme::command("analyze"), 
            Theme::muted("or"), 
            Theme::command("a"),
        );
        println!("     {} Show detailed analysis with file lists", Theme::muted("→"));
        println!();
        println!("  {}", Theme::command("config"));
        println!("     {} View or modify configuration", Theme::muted("→"));
        println!();
        println!("  {}  {}  {}", 
            Theme::command("restore"), 
            Theme::muted("or"), 
            Theme::command("r"),
        );
        println!("     {} Restore files from last deletion", Theme::muted("→"));
        println!();
        println!("{}", Theme::divider(60));
        println!();
        println!("{}", Theme::primary("Quick Examples:"));
        println!();
        println!("  {} Scan all categories", Theme::command("sweeper scan --all"));
        println!("  {} Scan specific categories", Theme::command("sweeper scan --cache --temp"));
        println!("  {} Clean all files", Theme::command("sweeper clean --all -y"));
        println!("  {} Find large files", Theme::command("sweeper scan --large --min-size 500MB"));
        println!("  {} Restore last deletion", Theme::command("sweeper restore --last"));
        println!();
        println!("{}", Theme::muted("Tip: Use --help with any command for detailed options"));
        println!();
    }
    
    pub fn run(self) -> anyhow::Result<()> {
        let output_mode = if self.quiet {
            OutputMode::Quiet
        } else if self.verbose >= 2 {
            OutputMode::VeryVerbose
        } else if self.verbose == 1 {
            OutputMode::Verbose
        } else {
            OutputMode::Normal
        };
        
        match self.command {
            Commands::Scan { all, cache, app_cache, temp, trash, build, downloads, large, old, path, json, project_age, min_age, min_size, exclude: _ } => {
                // --all enables all categories
                let (cache, app_cache, temp, trash, build, downloads, large, old, browser, system, empty, duplicates) = if all {
                    (true, true, true, true, true, true, true, true, true, true, true, true)
                } else if !cache && !app_cache && !temp && !trash && !build && !downloads && !large && !old {
                    // No categories specified - show help message
                    eprintln!("No categories specified. Use --all or specify categories like --cache, --app-cache, --temp, --build");
                    eprintln!("Run 'sweeper scan --help' for more information.");
                    return Ok(());
                } else {
                    // Scan command doesn't support browser, system, empty, duplicates
                    (cache, app_cache, temp, trash, build, downloads, large, old, false, false, false, false)
                };
                
                
                // Default to current directory to avoid stack overflow from OneDrive/UserDirs
                // CRITICAL FIX: Use simple env var instead of directories crate which may cause stack overflow
                let scan_path = path.unwrap_or_else(|| {
                    if let Ok(userprofile) = std::env::var("USERPROFILE") {
                        let base = PathBuf::from(&userprofile);
                        
                        // Try OneDrive\Documents first (common on Windows 11)
                        let onedrive_docs = base.join("OneDrive").join("Documents");
                        if onedrive_docs.exists() {
                            return onedrive_docs;
                        }
                        
                        // Try regular Documents
                        let docs = base.join("Documents");
                        if docs.exists() {
                            return docs;
                        }
                        
                        // Fallback to user profile root
                        base
                    } else {
                        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                    }
                });
                
                // Load config first
                let mut config = Config::load();
                
                // Apply CLI overrides to config
                config.apply_cli_overrides(
                    Some(project_age),
                    Some(min_age),
                    Some(size::parse_size(&min_size)
                        .map_err(|e| anyhow::anyhow!("Invalid size format '{}': {}", min_size, e))?
                        / (1024 * 1024)), // Convert bytes to MB for config
                );
                
                // Use config values (after CLI overrides) for scan options
                let min_size_bytes = config.thresholds.min_size_mb * 1024 * 1024;
                
                let results = scanner::scan_all(
                    &scan_path,
                    ScanOptions {
                        cache,
                        app_cache,
                        temp,
                        trash,
                        build,
                        downloads,
                        large,
                        old,
                        browser,
                        system,
                        empty,
                        duplicates,
                        project_age_days: config.thresholds.project_age_days,
                        min_age_days: config.thresholds.min_age_days,
                        min_size_bytes,
                    },
                    output_mode,
                    &config,
                )?;
                
                if json {
                    output::print_json(&results)?;
                } else {
                    output::print_human(&results, output_mode);
                }
                
                Ok(())
            }
            Commands::Clean { all, cache, app_cache, temp, trash, build, downloads, large, old, browser, system, empty, duplicates, path, json, yes, project_age, min_age, min_size, exclude, permanent, dry_run } => {
                // --all enables all categories
                let (cache, app_cache, temp, trash, build, downloads, large, old, browser, system, empty, duplicates) = if all {
                    (true, true, true, true, true, true, true, true, true, true, true, true)
                } else if !cache && !app_cache && !temp && !trash && !build && !downloads && !large && !old && !browser && !system && !empty && !duplicates {
                    // No categories specified - show help message
                    eprintln!("No categories specified. Use --all or specify categories like --cache, --app-cache, --temp, --build");
                    eprintln!("Run 'sweeper clean --help' for more information.");
                    return Ok(());
                } else {
                    (cache, app_cache, temp, trash, build, downloads, large, old, browser, system, empty, duplicates)
                };
                
                let scan_path = path.unwrap_or_else(|| {
                    directories::UserDirs::new()
                        .expect("Failed to get user directory")
                        .home_dir()
                        .to_path_buf()
                });
                
                // Load config first
                let mut config = Config::load();
                
                // Apply CLI overrides to config
                config.apply_cli_overrides(
                    Some(project_age),
                    Some(min_age),
                    Some(size::parse_size(&min_size)
                        .map_err(|e| anyhow::anyhow!("Invalid size format '{}': {}", min_size, e))?
                        / (1024 * 1024)), // Convert bytes to MB for config
                );
                
                // Merge CLI exclusions
                config.exclusions.patterns.extend(exclude.iter().cloned());
                
                // Use config values (after CLI overrides) for scan options
                let min_size_bytes = config.thresholds.min_size_mb * 1024 * 1024;
                
                let results = scanner::scan_all(
                    &scan_path,
                    ScanOptions {
                        cache,
                        app_cache,
                        temp,
                        trash,
                        build,
                        downloads,
                        large,
                        old,
                        browser,
                        system,
                        empty,
                        duplicates,
                        project_age_days: config.thresholds.project_age_days,
                        min_age_days: config.thresholds.min_age_days,
                        min_size_bytes,
                    },
                    output_mode,
                    &config,
                )?;
                
                if json {
                    output::print_json(&results)?;
                } else {
                    output::print_human(&results, output_mode);
                }
                
                cleaner::clean_all(&results, yes, output_mode, permanent, dry_run)?;
                
                Ok(())
            }
            Commands::Analyze { disk, interactive, depth, top, sort, all, cache, app_cache, temp, trash, build, downloads, large, old, browser, system, empty, duplicates, path, project_age, min_age, min_size, exclude } => {
                // Determine if we're in disk insights mode or legacy cleanable file mode
                let has_category_flags = cache || app_cache || temp || trash || build || downloads || large || old || browser || system || empty || duplicates || all;
                let disk_mode = disk || (!has_category_flags); // Default to disk mode if no category flags
                
                if disk_mode {
                    // Disk insights mode
                    use crate::disk_usage::{scan_directory, SortBy};
                    
                    // Determine scan path (default to user profile)
                    let scan_path = path.unwrap_or_else(|| {
                        if let Ok(userprofile) = std::env::var("USERPROFILE") {
                            PathBuf::from(&userprofile)
                        } else {
                            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                        }
                    });
                    
                    if !scan_path.exists() {
                        return Err(anyhow::anyhow!("Path does not exist: {}", scan_path.display()));
                    }
                    
                    // Parse sort option
                    let sort_by = match sort.as_deref() {
                        Some("name") => SortBy::Name,
                        Some("files") => SortBy::Files,
                        _ => SortBy::Size,
                    };
                    
                    // Scan directory
                    if output_mode != OutputMode::Quiet {
                        println!("Scanning {}...", scan_path.display());
                    }
                    
                    let insights = scan_directory(&scan_path, depth)?;
                    
                    if interactive {
                        // Launch TUI mode
                        use crate::tui;
                        let mut app_state = tui::state::AppState::new();
                        app_state.screen = tui::state::Screen::DiskInsights {
                            insights: insights.clone(),
                            current_path: scan_path.clone(),
                            cursor: 0,
                            sort_by,
                        };
                        tui::run(Some(app_state))?;
                    } else {
                        // CLI output mode
                        output::print_disk_insights(&insights, &scan_path, top.unwrap_or(10), sort_by, output_mode);
                    }
                    
                    Ok(())
                } else {
                    // Legacy cleanable file analysis mode
                    let (cache, app_cache, temp, trash, build, downloads, large, old, browser, system, empty, duplicates) = if all {
                        (true, true, true, true, true, true, true, true, true, true, true, true)
                    } else {
                        (cache, app_cache, temp, trash, build, downloads, large, old, browser, system, empty, duplicates)
                    };
                    
                    let scan_path = path.unwrap_or_else(|| {
                        directories::UserDirs::new()
                            .expect("Failed to get user directory")
                            .home_dir()
                            .to_path_buf()
                    });
                    
                    // Load config first
                    let mut config = Config::load();
                    
                    // Apply CLI overrides to config
                    config.apply_cli_overrides(
                        Some(project_age),
                        Some(min_age),
                        Some(size::parse_size(&min_size)
                            .map_err(|e| anyhow::anyhow!("Invalid size format '{}': {}", min_size, e))?
                            / (1024 * 1024)), // Convert bytes to MB for config
                    );
                    
                    // Merge CLI exclusions
                    config.exclusions.patterns.extend(exclude.iter().cloned());
                    
                    // Use config values (after CLI overrides) for scan options
                    let min_size_bytes = config.thresholds.min_size_mb * 1024 * 1024;
                    
                    let results = scanner::scan_all(
                        &scan_path,
                        ScanOptions {
                            cache,
                            app_cache,
                            temp,
                            trash,
                            build,
                            downloads,
                            large,
                            old,
                            browser,
                            system,
                            empty,
                            duplicates,
                            project_age_days: config.thresholds.project_age_days,
                            min_age_days: config.thresholds.min_age_days,
                            min_size_bytes,
                        },
                        output_mode,
                        &config,
                    )?;
                    
                    // Launch TUI if interactive mode requested
                    if interactive {
                        use crate::tui;
                        let mut app_state = tui::state::AppState::new();
                        app_state.scan_path = scan_path;
                        app_state.config = config;
                        // Store scan results and process them
                        app_state.scan_results = Some(results);
                        app_state.flatten_results();
                        app_state.screen = tui::state::Screen::Results;
                        tui::run(Some(app_state))?;
                    } else {
                        output::print_analyze(&results, output_mode);
                    }
                    
                    Ok(())
                }
            }
            Commands::Config { show, reset, edit } => {
                if show {
                    let config = Config::load_or_create();
                    println!("{}", Theme::header("Current Configuration"));
                    println!("{}", Theme::divider_bold(60));
                    println!();
                    println!("Thresholds:");
                    println!("  Project age: {} days", config.thresholds.project_age_days);
                    println!("  Min age: {} days", config.thresholds.min_age_days);
                    println!("  Min size: {} MB", config.thresholds.min_size_mb);
                    println!();
                    println!("Paths:");
                    if config.paths.scan_roots.is_empty() {
                        println!("  (none - using default)");
                    } else {
                        for path in &config.paths.scan_roots {
                            println!("  {}", path);
                        }
                    }
                    println!();
                    println!("Exclusions:");
                    if config.exclusions.patterns.is_empty() {
                        println!("  (none)");
                    } else {
                        for pattern in &config.exclusions.patterns {
                            println!("  {}", pattern);
                        }
                    }
                    println!();
                    println!("UI Settings:");
                    if let Some(ref path) = config.ui.default_scan_path {
                        println!("  Default scan path: {}", path);
                    } else {
                        println!("  Default scan path: (auto-detect)");
                    }
                    println!("  Output mode: {}", config.ui.output_mode);
                    println!("  Animations: {}", config.ui.animations);
                    println!("  Refresh rate: {} ms", config.ui.refresh_rate_ms);
                    println!();
                    println!("Safety Settings:");
                    println!("  Always confirm: {}", config.safety.always_confirm);
                    println!("  Default permanent: {}", config.safety.default_permanent);
                    println!("  Max no-confirm files: {}", config.safety.max_no_confirm);
                    println!("  Max no-confirm size: {} MB", config.safety.max_size_no_confirm_mb);
                    println!("  Skip locked files: {}", config.safety.skip_locked_files);
                    println!("  Dry run default: {}", config.safety.dry_run_default);
                    println!();
                    println!("Performance Settings:");
                    println!("  Scan threads: {} (0 = auto)", config.performance.scan_threads);
                    println!("  Batch size: {}", config.performance.batch_size);
                    println!("  Parallel scanning: {}", config.performance.parallel_scanning);
                    println!();
                    println!("History Settings:");
                    println!("  Enabled: {}", config.history.enabled);
                    println!("  Max entries: {} (0 = unlimited)", config.history.max_entries);
                    println!("  Max age: {} days (0 = forever)", config.history.max_age_days);
                    println!();
                    if let Ok(path) = Config::config_path() {
                        println!("Config file: {}", path.display());
                    }
                } else if reset {
                    let default_config = Config::default();
                    default_config.save()?;
                    println!("{} Configuration reset to defaults.", Theme::success("OK"));
                } else if edit {
                    if let Ok(path) = Config::config_path() {
                        // Create default config if it doesn't exist
                        if !path.exists() {
                            Config::default().save()?;
                        }
                        // Try to open in default editor
                        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "notepad".to_string());
                        std::process::Command::new(editor)
                            .arg(&path)
                            .status()
                            .map_err(|e| anyhow::anyhow!("Failed to open editor: {}", e))?;
                    } else {
                        return Err(anyhow::anyhow!("Failed to get config file path"));
                    }
                } else {
                    // Default: show config
                    let config = Config::load_or_create();
                    println!("{}", Theme::header("Current Configuration"));
                    println!("{}", Theme::divider_bold(60));
                    println!();
                    println!("Thresholds:");
                    println!("  Project age: {} days", config.thresholds.project_age_days);
                    println!("  Min age: {} days", config.thresholds.min_age_days);
                    println!("  Min size: {} MB", config.thresholds.min_size_mb);
                    println!();
                    println!("Paths:");
                    if config.paths.scan_roots.is_empty() {
                        println!("  (none - using default)");
                    } else {
                        for path in &config.paths.scan_roots {
                            println!("  {}", path);
                        }
                    }
                    println!();
                    println!("Exclusions:");
                    if config.exclusions.patterns.is_empty() {
                        println!("  (none)");
                    } else {
                        for pattern in &config.exclusions.patterns {
                            println!("  {}", pattern);
                        }
                    }
                    println!();
                    println!("UI Settings:");
                    if let Some(ref path) = config.ui.default_scan_path {
                        println!("  Default scan path: {}", path);
                    } else {
                        println!("  Default scan path: (auto-detect)");
                    }
                    println!("  Output mode: {}", config.ui.output_mode);
                    println!("  Animations: {}", config.ui.animations);
                    println!("  Refresh rate: {} ms", config.ui.refresh_rate_ms);
                    println!();
                    println!("Safety Settings:");
                    println!("  Always confirm: {}", config.safety.always_confirm);
                    println!("  Default permanent: {}", config.safety.default_permanent);
                    println!("  Max no-confirm files: {}", config.safety.max_no_confirm);
                    println!("  Max no-confirm size: {} MB", config.safety.max_size_no_confirm_mb);
                    println!("  Skip locked files: {}", config.safety.skip_locked_files);
                    println!("  Dry run default: {}", config.safety.dry_run_default);
                    println!();
                    println!("Performance Settings:");
                    println!("  Scan threads: {} (0 = auto)", config.performance.scan_threads);
                    println!("  Batch size: {}", config.performance.batch_size);
                    println!("  Parallel scanning: {}", config.performance.parallel_scanning);
                    println!();
                    println!("History Settings:");
                    println!("  Enabled: {}", config.history.enabled);
                    println!("  Max entries: {} (0 = unlimited)", config.history.max_entries);
                    println!("  Max age: {} days (0 = forever)", config.history.max_age_days);
                    println!();
                    if let Ok(path) = Config::config_path() {
                        println!("Config file: {}", path.display());
                    }
                }
                Ok(())
            }
            Commands::Restore { last, path, from } => {
                let output_mode = if self.quiet {
                    OutputMode::Quiet
                } else if self.verbose >= 2 {
                    OutputMode::VeryVerbose
                } else if self.verbose == 1 {
                    OutputMode::Verbose
                } else {
                    OutputMode::Normal
                };
                
                if last {
                    // Restore from last deletion session
                    match restore::restore_last(output_mode) {
                        Ok(result) => {
                            if output_mode != OutputMode::Quiet {
                                println!();
                                println!("{} {}", Theme::success("OK"), Theme::success(&result.summary()));
                            }
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("Failed to restore: {}", e));
                        }
                    }
                } else if let Some(ref restore_path) = path {
                    // Restore specific path
                    match restore::restore_path(restore_path, output_mode) {
                        Ok(result) => {
                            if output_mode != OutputMode::Quiet {
                                println!();
                                println!("{} {}", Theme::success("OK"), Theme::success(&result.summary()));
                            }
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("Failed to restore: {}", e));
                        }
                    }
                } else if let Some(ref log_path) = from {
                    // Restore from specific log file
                    let log = history::load_log(log_path)
                        .with_context(|| format!("Failed to load log file: {}", log_path.display()))?;
                    match restore::restore_from_log(&log, output_mode) {
                        Ok(result) => {
                            if output_mode != OutputMode::Quiet {
                                println!();
                                println!("{} {}", Theme::success("OK"), Theme::success(&result.summary()));
                            }
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("Failed to restore: {}", e));
                        }
                    }
                } else {
                    // Default: restore from last session
                    match restore::restore_last(output_mode) {
                        Ok(result) => {
                            if output_mode != OutputMode::Quiet {
                                println!();
                                println!("{} {}", Theme::success("OK"), Theme::success(&result.summary()));
                            }
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("Failed to restore: {}", e));
                        }
                    }
                }
                
                Ok(())
            }
        }
    }
}

pub struct ScanOptions {
    pub cache: bool,
    pub app_cache: bool,
    pub temp: bool,
    pub trash: bool,
    pub build: bool,
    pub downloads: bool,
    pub large: bool,
    pub old: bool,
    pub browser: bool,
    pub system: bool,
    pub empty: bool,
    pub duplicates: bool,
    pub project_age_days: u64,
    pub min_age_days: u64,
    pub min_size_bytes: u64,
}