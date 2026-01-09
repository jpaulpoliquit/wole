//! Deletion history logging for audit trails
//!
//! Provides logging of all deletion operations for:
//! - Audit trails
//! - Undo information (path records)
//! - Statistics tracking

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Record of a single deletion operation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeletionRecord {
    /// When the deletion occurred
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,
    /// Path that was deleted
    pub path: String,
    /// Size in bytes before deletion
    pub size_bytes: u64,
    /// Category (cache, temp, build, etc.)
    pub category: String,
    /// Whether it was permanently deleted (vs moved to trash)
    pub permanent: bool,
    /// Whether the deletion succeeded
    pub success: bool,
    /// Error message if deletion failed
    pub error: Option<String>,
}

impl DeletionRecord {
    /// Create a new successful deletion record
    pub fn success(path: &Path, size_bytes: u64, category: &str, permanent: bool) -> Self {
        Self {
            timestamp: Utc::now(),
            path: path.display().to_string(),
            size_bytes,
            category: category.to_string(),
            permanent,
            success: true,
            error: None,
        }
    }

    /// Create a new failed deletion record
    pub fn failure(path: &Path, size_bytes: u64, category: &str, permanent: bool, error: &str) -> Self {
        Self {
            timestamp: Utc::now(),
            path: path.display().to_string(),
            size_bytes,
            category: category.to_string(),
            permanent,
            success: false,
            error: Some(error.to_string()),
        }
    }
}

/// Log of all deletions in a session
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DeletionLog {
    /// When this session started
    #[serde(with = "chrono::serde::ts_seconds")]
    pub session_start: DateTime<Utc>,
    /// All deletion records
    pub records: Vec<DeletionRecord>,
    /// Total bytes successfully cleaned
    pub total_bytes_cleaned: u64,
    /// Total items attempted
    pub total_items: usize,
    /// Number of errors
    pub errors: usize,
}

impl DeletionLog {
    /// Create a new deletion log for the current session
    pub fn new() -> Self {
        Self {
            session_start: Utc::now(),
            records: Vec::new(),
            total_bytes_cleaned: 0,
            total_items: 0,
            errors: 0,
        }
    }

    /// Add a deletion record to the log
    pub fn add_record(&mut self, record: DeletionRecord) {
        self.total_items += 1;
        if record.success {
            self.total_bytes_cleaned += record.size_bytes;
        } else {
            self.errors += 1;
        }
        self.records.push(record);
    }

    /// Add a successful deletion
    pub fn log_success(&mut self, path: &Path, size_bytes: u64, category: &str, permanent: bool) {
        self.add_record(DeletionRecord::success(path, size_bytes, category, permanent));
    }

    /// Add a failed deletion
    pub fn log_failure(&mut self, path: &Path, size_bytes: u64, category: &str, permanent: bool, error: &str) {
        self.add_record(DeletionRecord::failure(path, size_bytes, category, permanent, error));
    }

    /// Save the log to the history directory
    ///
    /// Returns the path to the saved log file
    pub fn save(&self) -> Result<PathBuf> {
        let history_dir = get_history_dir()?;
        
        // Create filename with timestamp
        let filename = format!(
            "cleanup_{}.json",
            self.session_start.format("%Y%m%d_%H%M%S")
        );
        let log_path = history_dir.join(filename);
        
        // Serialize and write
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize deletion log")?;
        
        fs::write(&log_path, json)
            .with_context(|| format!("Failed to write deletion log to {}", log_path.display()))?;
        
        Ok(log_path)
    }

    /// Get the summary message for this log
    pub fn summary(&self) -> String {
        format!(
            "{} items cleaned ({} bytes), {} errors",
            self.total_items - self.errors,
            self.total_bytes_cleaned,
            self.errors
        )
    }
}

/// Get the history directory path
///
/// Creates the directory if it doesn't exist
/// Location: %LOCALAPPDATA%\sweeper\history\ (Windows)
///           ~/.local/share/sweeper/history/ (Linux/macOS)
pub fn get_history_dir() -> Result<PathBuf> {
    let base_dir = if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                // Fallback to %USERPROFILE%\AppData\Local
                std::env::var("USERPROFILE")
                    .map(|p| PathBuf::from(p).join("AppData").join("Local"))
                    .unwrap_or_else(|_| PathBuf::from("."))
            })
    } else {
        // Unix: ~/.local/share
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".local").join("share"))
            .unwrap_or_else(|_| PathBuf::from("."))
    };
    
    let history_dir = base_dir.join("sweeper").join("history");
    
    // Create directory if it doesn't exist
    if !history_dir.exists() {
        fs::create_dir_all(&history_dir)
            .with_context(|| format!("Failed to create history directory: {}", history_dir.display()))?;
    }
    
    Ok(history_dir)
}

/// List all history log files
pub fn list_logs() -> Result<Vec<PathBuf>> {
    let history_dir = get_history_dir()?;
    
    let mut logs: Vec<PathBuf> = fs::read_dir(&history_dir)
        .with_context(|| format!("Failed to read history directory: {}", history_dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .map(|ext| ext == "json")
                .unwrap_or(false)
        })
        .collect();
    
    // Sort by filename (which includes timestamp) in reverse order (newest first)
    logs.sort();
    logs.reverse();
    
    Ok(logs)
}

/// Load a deletion log from a file
pub fn load_log(path: &Path) -> Result<DeletionLog> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read log file: {}", path.display()))?;
    
    let log: DeletionLog = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse log file: {}", path.display()))?;
    
    Ok(log)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::path::Path;

    #[test]
    fn test_deletion_record_success() {
        let record = DeletionRecord::success(
            Path::new("/test/file.txt"),
            1024,
            "cache",
            false,
        );
        
        assert!(record.success);
        assert!(record.error.is_none());
        assert_eq!(record.size_bytes, 1024);
        assert_eq!(record.category, "cache");
        assert!(!record.permanent);
    }

    #[test]
    fn test_deletion_record_failure() {
        let record = DeletionRecord::failure(
            Path::new("/test/locked.txt"),
            2048,
            "temp",
            true,
            "File is locked",
        );
        
        assert!(!record.success);
        assert_eq!(record.error, Some("File is locked".to_string()));
        assert!(record.permanent);
    }

    #[test]
    fn test_deletion_log_new() {
        let log = DeletionLog::new();
        
        assert_eq!(log.records.len(), 0);
        assert_eq!(log.total_bytes_cleaned, 0);
        assert_eq!(log.total_items, 0);
        assert_eq!(log.errors, 0);
    }

    #[test]
    fn test_deletion_log_add_success() {
        let mut log = DeletionLog::new();
        log.log_success(Path::new("/test/file.txt"), 1024, "cache", false);
        
        assert_eq!(log.records.len(), 1);
        assert_eq!(log.total_bytes_cleaned, 1024);
        assert_eq!(log.total_items, 1);
        assert_eq!(log.errors, 0);
    }

    #[test]
    fn test_deletion_log_add_failure() {
        let mut log = DeletionLog::new();
        log.log_failure(Path::new("/test/file.txt"), 1024, "cache", false, "Permission denied");
        
        assert_eq!(log.records.len(), 1);
        assert_eq!(log.total_bytes_cleaned, 0);
        assert_eq!(log.total_items, 1);
        assert_eq!(log.errors, 1);
    }

    #[test]
    fn test_deletion_log_mixed() {
        let mut log = DeletionLog::new();
        log.log_success(Path::new("/test/file1.txt"), 1000, "cache", false);
        log.log_success(Path::new("/test/file2.txt"), 2000, "temp", false);
        log.log_failure(Path::new("/test/locked.txt"), 500, "cache", false, "Locked");
        
        assert_eq!(log.records.len(), 3);
        assert_eq!(log.total_bytes_cleaned, 3000);
        assert_eq!(log.total_items, 3);
        assert_eq!(log.errors, 1);
    }

    #[test]
    fn test_get_history_dir() {
        // This test just verifies the function works without panicking
        let result = get_history_dir();
        assert!(result.is_ok());
        
        let dir = result.unwrap();
        // The directory should exist after calling get_history_dir
        assert!(dir.exists());
    }

    #[test]
    fn test_deletion_log_summary() {
        let mut log = DeletionLog::new();
        log.log_success(Path::new("/test/file1.txt"), 1000, "cache", false);
        log.log_failure(Path::new("/test/file2.txt"), 500, "temp", false, "Error");
        
        let summary = log.summary();
        assert!(summary.contains("1 items cleaned"));
        assert!(summary.contains("1000 bytes"));
        assert!(summary.contains("1 errors"));
    }
}
