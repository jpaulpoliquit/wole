//! SQLite database operations for scan cache

use crate::scan_cache::signature::{FileSignature, FileStatus};
use crate::scan_cache::session::{ScanSession, ScanStats};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde_json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i32 = 1;

/// Scan cache database
pub struct ScanCache {
    db: Connection,
    current_scan_id: Option<i64>,
}

impl ScanCache {
    /// Open or create the scan cache database
    pub fn open() -> Result<Self> {
        let cache_dir = get_cache_dir()?;
        std::fs::create_dir_all(&cache_dir)
            .with_context(|| format!("Failed to create cache directory: {}", cache_dir.display()))?;

        let db_path = cache_dir.join("scan_cache.db");
        
        // Enable WAL mode for better concurrency and performance
        let mut db = Connection::open(&db_path)
            .with_context(|| format!("Failed to open database: {}", db_path.display()))?;
        
        // Enable WAL mode (Write-Ahead Logging) for better concurrency
        // This allows multiple readers while one writer is active
        db.pragma_update(None, "journal_mode", "WAL")
            .with_context(|| "Failed to enable WAL mode")?;
        
        // Set busy timeout to handle concurrent access gracefully
        db.busy_timeout(std::time::Duration::from_secs(30))
            .with_context(|| "Failed to set busy timeout")?;

        let mut cache = Self {
            db,
            current_scan_id: None,
        };

        // Initialize schema - if it fails due to corruption, try to recover
        if let Err(e) = cache.init_schema() {
            // If schema init fails, try to backup and recreate
            eprintln!("Warning: Failed to initialize cache schema: {}. Attempting recovery...", e);
            
            // Try to backup corrupted database
            let backup_path = db_path.with_extension("db.backup");
            let _ = std::fs::copy(&db_path, &backup_path);
            
            // Remove corrupted database and try again
            let _ = std::fs::remove_file(&db_path);
            
            // Retry opening
            let db = Connection::open(&db_path)
                .with_context(|| format!("Failed to recreate database: {}", db_path.display()))?;
            db.pragma_update(None, "journal_mode", "WAL")?;
            db.busy_timeout(std::time::Duration::from_secs(30))?;
            
            let mut cache = Self {
                db,
                current_scan_id: None,
            };
            cache.init_schema()
                .with_context(|| "Failed to initialize schema after recovery")?;
            return Ok(cache);
        }
        
        Ok(cache)
    }

    /// Initialize database schema
    fn init_schema(&mut self) -> Result<()> {
        // Check if schema_version table exists
        let version: i32 = self
            .db
            .query_row(
                "SELECT version FROM schema_version LIMIT 1",
                [],
                |row| row.get(0),
            )
            .or_else(|_| {
                // Table doesn't exist, create it and return version 0
                self.db.execute(
                    "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)",
                    [],
                )?;
                self.db.execute("INSERT INTO schema_version (version) VALUES (0)", [])?;
                Ok::<i32, rusqlite::Error>(0)
            })?;

        if version < SCHEMA_VERSION {
            self.migrate_schema(version)?;
        }

        Ok(())
    }

    /// Migrate schema to current version
    fn migrate_schema(&mut self, from_version: i32) -> Result<()> {
        // Use transaction to ensure atomic migration
        let tx = self.db.transaction()
            .with_context(|| "Failed to start migration transaction")?;

        if from_version == 0 {
            // Initial schema
            tx.execute(
                "CREATE TABLE IF NOT EXISTS file_records (
                    path TEXT PRIMARY KEY,
                    size INTEGER NOT NULL,
                    mtime_secs INTEGER NOT NULL,
                    mtime_nsecs INTEGER NOT NULL,
                    content_hash TEXT,
                    category TEXT NOT NULL,
                    last_scan_id INTEGER NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                )",
                [],
            )
            .with_context(|| "Failed to create file_records table")?;

            tx.execute(
                "CREATE TABLE IF NOT EXISTS scan_sessions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    started_at INTEGER NOT NULL,
                    finished_at INTEGER,
                    scan_type TEXT NOT NULL,
                    categories TEXT NOT NULL,
                    total_files INTEGER,
                    new_files INTEGER,
                    changed_files INTEGER,
                    removed_files INTEGER
                )",
                [],
            )
            .with_context(|| "Failed to create scan_sessions table")?;

            // Create indexes
            tx.execute(
                "CREATE INDEX IF NOT EXISTS idx_category ON file_records(category)",
                [],
            )
            .with_context(|| "Failed to create category index")?;
            tx.execute(
                "CREATE INDEX IF NOT EXISTS idx_scan_id ON file_records(last_scan_id)",
                [],
            )
            .with_context(|| "Failed to create scan_id index")?;
            tx.execute(
                "CREATE INDEX IF NOT EXISTS idx_size ON file_records(size)",
                [],
            )
            .with_context(|| "Failed to create size index")?;

            // Update schema version
            tx.execute("UPDATE schema_version SET version = ?1", [SCHEMA_VERSION])
                .with_context(|| "Failed to update schema version")?;
        }

        // Commit migration transaction
        tx.commit()
            .with_context(|| "Failed to commit migration transaction")?;

        Ok(())
    }

    /// Start a new scan session
    pub fn start_scan(&mut self, scan_type: &str, categories: &[&str]) -> Result<i64> {
        let started_at = Utc::now().timestamp();
        let categories_json = serde_json::to_string(categories)?;

        self.db.execute(
            "INSERT INTO scan_sessions (started_at, scan_type, categories) VALUES (?1, ?2, ?3)",
            params![started_at, scan_type, categories_json],
        )?;

        let scan_id = self.db.last_insert_rowid();
        self.current_scan_id = Some(scan_id);
        Ok(scan_id)
    }

    /// Check if a file needs rescanning
    pub fn check_file(&self, path: &Path) -> Result<FileStatus> {
        let path_str = normalize_path(path);

        let result: Option<(u64, i64, i64)> = self.db.query_row(
            "SELECT size, mtime_secs, mtime_nsecs FROM file_records WHERE path = ?1",
            [&path_str],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).ok();

        let Some((cached_size, mtime_secs, mtime_nsecs)) = result else {
            return Ok(FileStatus::New);
        };

        // Get current file metadata
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return Ok(FileStatus::Deleted),
        };

        let current_size = metadata.len();
        let current_mtime = metadata
            .modified()
            .with_context(|| format!("Failed to get mtime: {}", path.display()))?;

        let (current_secs, current_nsecs) = system_time_to_secs_nsecs(current_mtime);

        // Compare signatures
        if current_size != cached_size || current_secs != mtime_secs || current_nsecs != mtime_nsecs {
            Ok(FileStatus::Modified)
        } else {
            Ok(FileStatus::Unchanged)
        }
    }

    /// Batch check multiple files (more efficient)
    pub fn check_files_batch(&self, paths: &[PathBuf]) -> Result<HashMap<PathBuf, FileStatus>> {
        let mut result = HashMap::new();

        if paths.is_empty() {
            return Ok(result);
        }

        // Get all cached records for these paths (normalize for consistent lookup)
        let path_strs: Vec<String> = paths.iter().map(|p| normalize_path(p)).collect();
        let placeholders = path_strs.iter().map(|_| "?").collect::<Vec<_>>().join(",");

        let mut stmt = self.db.prepare(&format!(
            "SELECT path, size, mtime_secs, mtime_nsecs FROM file_records WHERE path IN ({})",
            placeholders
        ))?;

        // Build params vector manually
        let mut query_params: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for s in &path_strs {
            query_params.push(s);
        }

        let cached: HashMap<String, (u64, i64, i64)> = stmt
            .query_map(
                rusqlite::params_from_iter(query_params),
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        (row.get::<_, u64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?),
                    ))
                },
            )?
            .collect::<Result<HashMap<_, _>, _>>()?;

        // Check each file
        for path in paths {
            let path_str = normalize_path(&path);

            if let Some((cached_size, mtime_secs, mtime_nsecs)) = cached.get(&path_str) {
                // File is in cache, check if it changed
                match std::fs::metadata(path) {
                    Ok(metadata) => {
                        let current_size = metadata.len();
                        let current_mtime = metadata.modified().ok();

                        if let Some(mtime) = current_mtime {
                            let (current_secs, current_nsecs) = system_time_to_secs_nsecs(mtime);

                            if current_size != *cached_size
                                || current_secs != *mtime_secs
                                || current_nsecs != *mtime_nsecs
                            {
                                result.insert(path.clone(), FileStatus::Modified);
                            } else {
                                result.insert(path.clone(), FileStatus::Unchanged);
                            }
                        } else {
                            result.insert(path.clone(), FileStatus::Modified);
                        }
                    }
                    Err(_) => {
                        result.insert(path.clone(), FileStatus::Deleted);
                    }
                }
            } else {
                // File not in cache
                if path.exists() {
                    result.insert(path.clone(), FileStatus::New);
                } else {
                    result.insert(path.clone(), FileStatus::Deleted);
                }
            }
        }

        Ok(result)
    }

    /// Update/insert file record after scanning
    pub fn upsert_file(&mut self, sig: &FileSignature, category: &str, scan_id: i64) -> Result<()> {
        let path_str = normalize_path(&sig.path);
        let (mtime_secs, mtime_nsecs) = system_time_to_secs_nsecs(sig.mtime);
        let now = Utc::now().timestamp();

        // Handle potential integer overflow for very large files (>9PB)
        // SQLite INTEGER can store up to 8 bytes, so i64::MAX is safe
        let size_i64 = if sig.size > i64::MAX as u64 {
            i64::MAX // Cap at max value
        } else {
            sig.size as i64
        };

        self.db.execute(
            "INSERT INTO file_records (path, size, mtime_secs, mtime_nsecs, content_hash, category, last_scan_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(path) DO UPDATE SET
                size = ?2,
                mtime_secs = ?3,
                mtime_nsecs = ?4,
                content_hash = ?5,
                category = ?6,
                last_scan_id = ?7,
                updated_at = ?9",
            params![
                path_str,
                size_i64,
                mtime_secs,
                mtime_nsecs,
                sig.content_hash,
                category,
                scan_id,
                now,
                now
            ],
        )?;

        Ok(())
    }

    /// Batch upsert for efficiency
    pub fn upsert_files_batch(
        &mut self,
        records: &[(FileSignature, String)],
        scan_id: i64,
    ) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        let tx = self.db.transaction()
            .with_context(|| "Failed to start transaction")?;

        for (sig, category) in records {
            let path_str = normalize_path(&sig.path);
            let (mtime_secs, mtime_nsecs) = system_time_to_secs_nsecs(sig.mtime);
            let now = Utc::now().timestamp();

            // Handle potential integer overflow for very large files
            let size_i64 = if sig.size > i64::MAX as u64 {
                i64::MAX
            } else {
                sig.size as i64
            };

            if let Err(e) = tx.execute(
                "INSERT INTO file_records (path, size, mtime_secs, mtime_nsecs, content_hash, category, last_scan_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(path) DO UPDATE SET
                    size = ?2,
                    mtime_secs = ?3,
                    mtime_nsecs = ?4,
                    content_hash = ?5,
                    category = ?6,
                    last_scan_id = ?7,
                    updated_at = ?9",
                params![
                    path_str,
                    size_i64,
                    mtime_secs,
                    mtime_nsecs,
                    sig.content_hash,
                    category,
                    scan_id,
                    now,
                    now
                ],
            ) {
                // Transaction will be rolled back automatically on drop
                return Err(anyhow::anyhow!("Failed to upsert file record: {}", e));
            }
        }

        tx.commit()
            .with_context(|| "Failed to commit transaction")?;
        Ok(())
    }

    /// Get cached results for a category (unchanged files from previous scan)
    pub fn get_cached_category(&self, category: &str, previous_scan_id: i64) -> Result<Vec<PathBuf>> {
        let mut stmt = self.db.prepare(
            "SELECT path FROM file_records WHERE category = ?1 AND last_scan_id = ?2",
        )?;
        
        let mut paths = Vec::new();
        let mut rows = stmt.query_map(params![category, previous_scan_id], |row| {
            Ok(PathBuf::from(row.get::<_, String>(0)?))
        })?;
        
        for row in rows {
            paths.push(row?);
        }

        Ok(paths)
    }

    /// Remove entries for deleted files (files that were in cache but no longer exist)
    pub fn cleanup_stale(&mut self, current_scan_id: i64) -> Result<usize> {
        // Get all paths that weren't updated in current scan (they might be deleted)
        // We only check paths from the most recent previous scan to avoid checking everything
        let previous_scan_id: Option<i64> = {
            let mut stmt = self.db.prepare(
                "SELECT MAX(id) FROM scan_sessions WHERE id < ?1",
            )?;
            stmt.query_row([current_scan_id], |row| row.get(0)).ok()
        };
        
        if previous_scan_id.is_none() {
            return Ok(0); // No previous scan
        }
        
        let prev_id = previous_scan_id.unwrap();
        
        // Get paths from previous scan that weren't updated in current scan
        let paths: Vec<String> = {
            let mut stmt = self.db.prepare(
                "SELECT path FROM file_records WHERE last_scan_id = ?1",
            )?;
            
            let mut paths = Vec::new();
            let rows = stmt.query_map([prev_id], |row| Ok(row.get::<_, String>(0)?))?;
            
            for row in rows {
                paths.push(row?);
            }
            paths
        };

        if paths.is_empty() {
            return Ok(0);
        }

        // Check which paths are deleted (outside transaction for efficiency)
        let mut deleted_paths = Vec::new();
        for path_str in paths {
            let path = PathBuf::from(&path_str);
            // Use exists() check - if file doesn't exist, mark for deletion
            if !path.exists() {
                deleted_paths.push(path_str);
            }
        }

        if deleted_paths.is_empty() {
            return Ok(0);
        }

        // Batch deletions in a transaction for efficiency
        let tx = self.db.transaction()
            .with_context(|| "Failed to start cleanup transaction")?;
        
        // Batch delete in transaction
        let mut delete_stmt = tx.prepare("DELETE FROM file_records WHERE path = ?1")?;
        for path_str in &deleted_paths {
            if let Err(e) = delete_stmt.execute([path_str]) {
                // Transaction will rollback on drop
                return Err(anyhow::anyhow!("Failed to delete stale record: {}", e));
            }
        }
        drop(delete_stmt);

        let deleted_count = deleted_paths.len();
        tx.commit()
            .with_context(|| "Failed to commit cleanup transaction")?;

        Ok(deleted_count)
    }

    /// Finish scan session and cleanup
    pub fn finish_scan(&mut self, scan_id: i64, stats: ScanStats) -> Result<()> {
        let finished_at = Utc::now().timestamp();

        self.db.execute(
            "UPDATE scan_sessions SET
                finished_at = ?1,
                total_files = ?2,
                new_files = ?3,
                changed_files = ?4,
                removed_files = ?5
             WHERE id = ?6",
            params![
                finished_at,
                stats.total_files as i64,
                stats.new_files as i64,
                stats.changed_files as i64,
                stats.removed_files as i64,
                scan_id
            ],
        )?;

        self.current_scan_id = None;
        Ok(())
    }

    /// Force full rescan (clear cache for categories)
    pub fn invalidate(&mut self, categories: Option<&[&str]>) -> Result<()> {
        if let Some(cats) = categories {
            if !cats.is_empty() {
                let placeholders = cats.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                // Build params vector manually
                let mut query_params: Vec<&dyn rusqlite::ToSql> = Vec::new();
                for cat in cats {
                    query_params.push(cat);
                }
                self.db.execute(
                    &format!("DELETE FROM file_records WHERE category IN ({})", placeholders),
                    rusqlite::params_from_iter(query_params),
                )?;
            }
        } else {
            self.db.execute("DELETE FROM file_records", [])?;
        }
        Ok(())
    }

    /// Get the previous scan ID (for getting cached results)
    pub fn get_previous_scan_id(&self) -> Result<Option<i64>> {
        let result: Option<i64> = self.db.query_row(
            "SELECT MAX(id) FROM scan_sessions WHERE finished_at IS NOT NULL",
            [],
            |row| row.get(0),
        ).ok();
        Ok(result)
    }

    /// Get last scan info
    pub fn get_last_scan(&self) -> Result<Option<ScanSession>> {
        let result: Option<ScanSession> = self.db.query_row(
            "SELECT id, started_at, finished_at, scan_type, categories, total_files, new_files, changed_files, removed_files
             FROM scan_sessions
             ORDER BY id DESC LIMIT 1",
            [],
            |row| {
                let id: i64 = row.get(0)?;
                let started_at: i64 = row.get(1)?;
                let finished_at: Option<i64> = row.get(2)?;
                let scan_type: String = row.get(3)?;
                let categories_json: String = row.get(4)?;
                let total_files: Option<i64> = row.get(5)?;
                let new_files: Option<i64> = row.get(6)?;
                let changed_files: Option<i64> = row.get(7)?;
                let removed_files: Option<i64> = row.get(8)?;

                let categories: Vec<String> = serde_json::from_str(&categories_json).unwrap_or_default();

                Ok(ScanSession {
                    id,
                    started_at: DateTime::from_timestamp(started_at, 0)
                        .unwrap_or_else(Utc::now),
                    finished_at: finished_at.map(|ts| {
                        DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
                    }),
                    scan_type,
                    categories,
                    stats: ScanStats {
                        total_files: total_files.unwrap_or(0) as usize,
                        new_files: new_files.unwrap_or(0) as usize,
                        changed_files: changed_files.unwrap_or(0) as usize,
                        removed_files: removed_files.unwrap_or(0) as usize,
                    },
                })
            },
        ).ok();

        Ok(result)
    }

    /// Get current scan ID
    pub fn current_scan_id(&self) -> Option<i64> {
        self.current_scan_id
    }
}

/// Get cache directory path
fn get_cache_dir() -> Result<PathBuf> {
    let base_dir = if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("USERPROFILE")
                    .map(|p| PathBuf::from(p).join("AppData").join("Local"))
                    .unwrap_or_else(|_| PathBuf::from("."))
            })
    } else {
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".local").join("share"))
            .unwrap_or_else(|_| PathBuf::from("."))
    };

    Ok(base_dir.join("wole").join("cache"))
}

/// Normalize path for consistent storage and lookup
/// On Windows, converts to lowercase for case-insensitive matching
/// On Unix, preserves case
fn normalize_path(path: &Path) -> String {
    #[cfg(windows)]
    {
        path.to_string_lossy().to_lowercase().replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        path.to_string_lossy().replace('\\', "/")
    }
}

/// Convert SystemTime to (seconds, nanoseconds) tuple
fn system_time_to_secs_nsecs(time: SystemTime) -> (i64, i64) {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            let secs = duration.as_secs() as i64;
            let nsecs = duration.subsec_nanos() as i64;
            (secs, nsecs)
        }
        Err(_) => (0, 0), // Shouldn't happen for mtime, but handle gracefully
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_cache() -> (TempDir, ScanCache) {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("LOCALAPPDATA", temp_dir.path());
        let cache = ScanCache::open().unwrap();
        (temp_dir, cache)
    }

    #[test]
    fn test_open_cache() {
        let (_temp_dir, _cache) = setup_test_cache();
    }

    #[test]
    fn test_start_scan() {
        let (_temp_dir, mut cache) = setup_test_cache();
        let scan_id = cache.start_scan("full", &["cache", "temp"]).unwrap();
        assert!(scan_id > 0);
    }

    #[test]
    fn test_upsert_file() {
        let (temp_dir, mut cache) = setup_test_cache();
        let scan_id = cache.start_scan("full", &["cache"]).unwrap();
        
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();
        
        let sig = FileSignature::from_path(&test_file, false).unwrap();
        cache.upsert_file(&sig, "cache", scan_id).unwrap();
        
        // Check that file is in cache
        let status = cache.check_file(&test_file).unwrap();
        assert!(matches!(status, FileStatus::Unchanged));
    }

    #[test]
    fn test_check_file_new() {
        let (temp_dir, cache) = setup_test_cache();
        
        let test_file = temp_dir.path().join("new_file.txt");
        fs::write(&test_file, "new content").unwrap();
        
        let status = cache.check_file(&test_file).unwrap();
        assert!(matches!(status, FileStatus::New));
    }

    #[test]
    fn test_check_file_modified() {
        let (temp_dir, mut cache) = setup_test_cache();
        let scan_id = cache.start_scan("full", &["cache"]).unwrap();
        
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "original").unwrap();
        
        let sig = FileSignature::from_path(&test_file, false).unwrap();
        cache.upsert_file(&sig, "cache", scan_id).unwrap();
        
        // Modify file
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(&test_file, "modified").unwrap();
        
        let status = cache.check_file(&test_file).unwrap();
        assert!(matches!(status, FileStatus::Modified));
    }

    #[test]
    fn test_invalidate() {
        let (temp_dir, mut cache) = setup_test_cache();
        let scan_id = cache.start_scan("full", &["cache"]).unwrap();
        
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();
        
        let sig = FileSignature::from_path(&test_file, false).unwrap();
        cache.upsert_file(&sig, "cache", scan_id).unwrap();
        
        // Invalidate cache
        cache.invalidate(Some(&["cache"])).unwrap();
        
        // File should be gone from cache
        let status = cache.check_file(&test_file).unwrap();
        assert!(matches!(status, FileStatus::New));
    }
}
