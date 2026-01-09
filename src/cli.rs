use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::scanner;
use crate::output;
use crate::cleaner;

#[derive(Parser)]
#[command(name = "sweeper")]
#[command(about = "Clean your Windows machine without fear")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Find cleanable files (dry-run)
    Scan {
        /// Scan cache directories
        #[arg(long)]
        cache: bool,
        
        /// Scan temp files
        #[arg(long)]
        temp: bool,
        
        /// Scan Recycle Bin
        #[arg(long)]
        trash: bool,
        
        /// Scan build artifacts from inactive projects
        #[arg(long)]
        build: bool,
        
        /// Scan path (default: home directory)
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,
        
        /// Output as JSON
        #[arg(long)]
        json: bool,
        
        /// Project age threshold in days (for --build)
        #[arg(long, default_value = "14")]
        project_age: u64,
    },
    
    /// Delete files (with confirmation)
    Clean {
        /// Clean cache directories
        #[arg(long)]
        cache: bool,
        
        /// Clean temp files
        #[arg(long)]
        temp: bool,
        
        /// Clean Recycle Bin
        #[arg(long)]
        trash: bool,
        
        /// Clean build artifacts from inactive projects
        #[arg(long)]
        build: bool,
        
        /// Scan path (default: home directory)
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,
        
        /// Output as JSON
        #[arg(long)]
        json: bool,
        
        /// Skip confirmation prompt
        #[arg(short = 'y')]
        yes: bool,
        
        /// Project age threshold in days (for --build)
        #[arg(long, default_value = "14")]
        project_age: u64,
    },
}

impl Cli {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
    
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            Commands::Scan { cache, temp, trash, build, path, json, project_age } => {
                let scan_path = path.unwrap_or_else(|| {
                    directories::UserDirs::new()
                        .expect("Failed to get user directory")
                        .home_dir()
                        .to_path_buf()
                });
                
                let results = scanner::scan_all(
                    &scan_path,
                    ScanOptions {
                        cache,
                        temp,
                        trash,
                        build,
                        project_age_days: project_age,
                    },
                )?;
                
                if json {
                    output::print_json(&results)?;
                } else {
                    output::print_human(&results);
                }
                
                Ok(())
            }
            Commands::Clean { cache, temp, trash, build, path, json, yes, project_age } => {
                let scan_path = path.unwrap_or_else(|| {
                    directories::UserDirs::new()
                        .expect("Failed to get user directory")
                        .home_dir()
                        .to_path_buf()
                });
                
                let results = scanner::scan_all(
                    &scan_path,
                    ScanOptions {
                        cache,
                        temp,
                        trash,
                        build,
                        project_age_days: project_age,
                    },
                )?;
                
                if json {
                    output::print_json(&results)?;
                } else {
                    output::print_human(&results);
                }
                
                cleaner::clean_all(&results, yes)?;
                
                Ok(())
            }
        }
    }
}

pub struct ScanOptions {
    pub cache: bool,
    pub temp: bool,
    pub trash: bool,
    pub build: bool,
    pub project_age_days: u64,
}
