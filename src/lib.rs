//! Sweeper library crate
//! 
//! This crate provides both a CLI binary and a library API for programmatic use

pub mod cli;
pub mod scanner;
pub mod cleaner;
pub mod output;
pub mod categories;
pub mod project;
pub mod git;
pub mod size;
pub mod config;
pub mod progress;
pub mod utils;
pub mod analyzer;
pub mod theme;
pub mod tui;
pub mod history;
pub mod scan_events;
pub mod restore;
pub mod disk_usage;