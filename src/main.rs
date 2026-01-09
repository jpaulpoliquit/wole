mod cli;
mod scanner;
mod cleaner;
mod output;
mod categories;
mod project;
mod git;
mod size;
mod config;
mod progress;

use anyhow::Result;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run()
}
