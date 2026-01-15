//! File signature computation and comparison

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// File signature for change detection
///
/// Uses a two-tier approach:
/// 1. Fast signature: path + mtime + size (sufficient for 99% of cases)
/// 2. Content hash: optional blake3 hash (for content verification)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSignature {
    pub path: PathBuf,
    pub size: u64,
    pub mtime: SystemTime,
    pub content_hash: Option<String>,
}

impl FileSignature {
    /// Compute signature for a file path
    ///
    /// Only computes content hash if explicitly requested (for duplicates, etc.)
    pub fn from_path(path: &Path, compute_hash: bool) -> Result<Self> {
        // Use safe_metadata on Windows to handle long paths (>260 chars) gracefully
        #[cfg(windows)]
        let metadata = crate::utils::safe_metadata(path)
            .with_context(|| format!("Failed to read metadata: {}", path.display()))?;

        #[cfg(not(windows))]
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to read metadata: {}", path.display()))?;

        let size = metadata.len();
        let mtime = metadata
            .modified()
            .with_context(|| format!("Failed to get mtime: {}", path.display()))?;

        let content_hash = if compute_hash {
            Some(Self::compute_hash(path)?)
        } else {
            None
        };

        Ok(Self {
            path: path.to_path_buf(),
            size,
            mtime,
            content_hash,
        })
    }

    /// Compute blake3 hash of file content
    ///
    /// Uses memory mapping for large files (similar to duplicates.rs)
    fn compute_hash(path: &Path) -> Result<String> {
        use blake3::Hasher;
        use memmap2::MmapOptions;
        use std::fs::File;
        use std::io::{BufReader, Read};

        const MEMMAP_THRESHOLD: u64 = 10 * 1024 * 1024; // 10MB
        const BUFFER_SIZE: usize = 8 * 1024 * 1024; // 8MB

        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to get metadata: {}", path.display()))?;
        let file_size = metadata.len();

        // Use memory mapping for large files
        if file_size >= MEMMAP_THRESHOLD {
            let file = File::open(path)
                .with_context(|| format!("Failed to open file: {}", path.display()))?;

            // Safety: We're only reading the file, not modifying it
            let mmap = unsafe {
                MmapOptions::new()
                    .map(&file)
                    .with_context(|| format!("Failed to memory map file: {}", path.display()))?
            };

            let mut hasher = Hasher::new();
            hasher.update(&mmap[..]);
            let hash = hasher.finalize();
            return Ok(format!("{}", hash.to_hex()));
        }

        // Use buffered reads for smaller files
        let file =
            File::open(path).with_context(|| format!("Failed to open file: {}", path.display()))?;

        let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
        let mut hasher = Hasher::new();
        let mut buffer = vec![0u8; BUFFER_SIZE];

        loop {
            let bytes_read = reader
                .read(&mut buffer)
                .with_context(|| format!("Failed to read file: {}", path.display()))?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
        }

        let hash = hasher.finalize();
        Ok(format!("{}", hash.to_hex()))
    }

    /// Compare two signatures to determine if file changed
    pub fn compare(&self, other: &Self) -> FileStatus {
        if self.path != other.path {
            return FileStatus::New;
        }

        // Check if size or mtime changed (fast check)
        if self.size != other.size || self.mtime != other.mtime {
            return FileStatus::Modified;
        }

        // If both have content hashes, compare them
        if let (Some(hash1), Some(hash2)) = (&self.content_hash, &other.content_hash) {
            if hash1 != hash2 {
                return FileStatus::Modified;
            }
        }

        FileStatus::Unchanged
    }
}

/// Status of a file compared to cache
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    /// File not in cache (new file)
    New,
    /// File unchanged since last scan
    Unchanged,
    /// File modified (size or mtime changed)
    Modified,
    /// File is in recycle bin (was cleaned, but still recoverable)
    InRecycleBin,
    /// File deleted (was in cache but no longer exists and not in recycle bin)
    Deleted,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_signature_from_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "hello world").unwrap();

        let sig = FileSignature::from_path(&file_path, false).unwrap();
        assert_eq!(sig.size, 11);
        assert_eq!(sig.path, file_path);
        assert!(sig.content_hash.is_none());
    }

    #[test]
    fn test_signature_with_hash() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "hello world").unwrap();

        let sig = FileSignature::from_path(&file_path, true).unwrap();
        assert_eq!(sig.size, 11);
        assert!(sig.content_hash.is_some());
    }

    #[test]
    fn test_signature_compare_unchanged() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "hello world").unwrap();

        let sig1 = FileSignature::from_path(&file_path, false).unwrap();
        let sig2 = FileSignature::from_path(&file_path, false).unwrap();

        assert_eq!(sig1.compare(&sig2), FileStatus::Unchanged);
    }

    #[test]
    fn test_signature_compare_modified() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "hello world").unwrap();

        let sig1 = FileSignature::from_path(&file_path, false).unwrap();

        // Modify file
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(&file_path, "hello world modified").unwrap();

        let sig2 = FileSignature::from_path(&file_path, false).unwrap();

        assert_eq!(sig1.compare(&sig2), FileStatus::Modified);
    }
}
