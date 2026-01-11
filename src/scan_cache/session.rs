//! Scan session management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Statistics for a scan session
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanStats {
    pub total_files: usize,
    pub new_files: usize,
    pub changed_files: usize,
    pub removed_files: usize,
}

/// Scan session record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSession {
    pub id: i64,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub scan_type: String, // "full" or "incremental"
    pub categories: Vec<String>,
    pub stats: ScanStats,
}

impl ScanSession {
    /// Create a new scan session
    pub fn new(scan_type: String, categories: Vec<String>) -> Self {
        Self {
            id: 0, // Will be set by database
            started_at: Utc::now(),
            finished_at: None,
            scan_type,
            categories,
            stats: ScanStats::default(),
        }
    }

    /// Mark session as finished
    pub fn finish(mut self, stats: ScanStats) -> Self {
        self.finished_at = Some(Utc::now());
        self.stats = stats;
        self
    }
}
