//! Cache context for category scanners

use crate::scan_cache::signature::FileSignature;
use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;

/// Context passed to category scanners for incremental scanning
///
/// Provides:
/// - Set of paths to skip (unchanged from cache)
/// - Method to record new file signatures
pub struct CacheContext {
    /// Paths that are unchanged and can be skipped
    pub unchanged_paths: HashSet<PathBuf>,
    /// New/modified files that need scanning
    pub files_to_scan: Vec<PathBuf>,
    /// Callback to record file signatures after scanning
    pub record_signature: Box<dyn Fn(&FileSignature, &str) -> Result<()> + Send + Sync>,
}

impl CacheContext {
    /// Create a new cache context
    pub fn new(
        unchanged_paths: HashSet<PathBuf>,
        files_to_scan: Vec<PathBuf>,
        record_signature: Box<dyn Fn(&FileSignature, &str) -> Result<()> + Send + Sync>,
    ) -> Self {
        Self {
            unchanged_paths,
            files_to_scan,
            record_signature,
        }
    }

    /// Check if a path should be skipped (unchanged)
    pub fn should_skip(&self, path: &PathBuf) -> bool {
        self.unchanged_paths.contains(path)
    }

    /// Record a file signature after scanning
    pub fn record(&self, sig: &FileSignature, category: &str) -> Result<()> {
        (self.record_signature)(sig, category)
    }
}

/// Builder for creating cache contexts
pub struct CacheContextBuilder {
    unchanged_paths: HashSet<PathBuf>,
    files_to_scan: Vec<PathBuf>,
    record_signature: Option<Box<dyn Fn(&FileSignature, &str) -> Result<()> + Send + Sync>>,
}

impl CacheContextBuilder {
    pub fn new() -> Self {
        Self {
            unchanged_paths: HashSet::new(),
            files_to_scan: Vec::new(),
            record_signature: None,
        }
    }

    pub fn with_unchanged_paths(mut self, paths: HashSet<PathBuf>) -> Self {
        self.unchanged_paths = paths;
        self
    }

    pub fn with_files_to_scan(mut self, paths: Vec<PathBuf>) -> Self {
        self.files_to_scan = paths;
        self
    }

    pub fn with_record_fn(
        mut self,
        f: Box<dyn Fn(&FileSignature, &str) -> Result<()> + Send + Sync>,
    ) -> Self {
        self.record_signature = Some(f);
        self
    }

    pub fn build(self) -> CacheContext {
        CacheContext::new(
            self.unchanged_paths,
            self.files_to_scan,
            self.record_signature
                .unwrap_or_else(|| Box::new(|_sig: &FileSignature, _cat: &str| Ok(()))),
        )
    }
}

impl Default for CacheContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}
