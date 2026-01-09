use anyhow::Result;
use sweeper::cli::Cli;
use std::env;

fn main() -> Result<()> {
    // Check if no arguments provided (only program name)
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        // Launch TUI instead of text menu
        sweeper::tui::run(None)?;
        return Ok(());
    }
    
    let cli = Cli::parse();
    cli.run()
}
