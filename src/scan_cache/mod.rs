//! Incremental scan cache system
//!
//! Provides persistent file tracking to enable fast incremental scans by caching
//! file signatures (metadata + optional content hash) and only rescanning files
//! that are new or have changed.

pub mod context;
pub mod database;
pub mod session;
pub mod signature;

pub use context::CacheContext;
pub use database::ScanCache;
pub use session::{ScanSession, ScanStats};
pub use signature::{FileSignature, FileStatus};
