//! Progress events emitted during scanning (used by TUI)

use std::path::PathBuf;

/// Real-time progress updates during scanning.
#[derive(Debug, Clone)]
pub enum ScanProgressEvent {
    /// A category scan has started.
    CategoryStarted {
        category: String,
        total_units: Option<u64>,
        current_path: Option<PathBuf>,
    },

    /// Incremental progress within a category scan.
    CategoryProgress {
        category: String,
        completed_units: u64,
        total_units: Option<u64>,
        current_path: Option<PathBuf>,
    },

    /// A category scan has finished.
    CategoryFinished {
        category: String,
        items: usize,
        size_bytes: u64,
    },
}

