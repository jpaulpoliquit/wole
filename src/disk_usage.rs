//! Disk usage analysis - scan filesystem and calculate folder sizes

use crate::utils;
use anyhow::Result;
use jwalk::WalkDir;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Represents a file in a directory
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

/// Represents a folder node in the directory tree
#[derive(Debug, Clone)]
pub struct FolderNode {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub file_count: u64,
    pub children: Vec<FolderNode>,
    pub files: Vec<FileInfo>, // Files directly in this directory (not in subdirectories)
    pub percentage: f64,      // % of parent's total size
}

/// Complete disk insights data
#[derive(Debug, Clone)]
pub struct DiskInsights {
    pub root: FolderNode,
    pub total_size: u64,
    pub total_files: u64,
    pub largest_files: Vec<(PathBuf, u64)>, // Top 10 largest files
    pub scan_duration: Duration,
}

/// Sort order for folder display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Size,
    Name,
    Files,
}

/// Scan a directory and build a folder tree with sizes
pub fn scan_directory(path: &Path, max_depth: u8) -> Result<DiskInsights> {
    let start_time = Instant::now();

    // First pass: collect all files and their sizes, grouped by directory
    let mut dir_sizes: HashMap<PathBuf, u64> = HashMap::new();
    let mut dir_file_counts: HashMap<PathBuf, u64> = HashMap::new();
    let mut dir_files: HashMap<PathBuf, Vec<(PathBuf, u64)>> = HashMap::new(); // Files per directory
    let mut file_sizes: Vec<(PathBuf, u64)> = Vec::new();

    let total_size = AtomicU64::new(0);
    let total_files = AtomicU64::new(0);

    // Use jwalk for parallel traversal
    WalkDir::new(path)
        .max_depth(max_depth as usize)
        .follow_links(false)
        .parallelism(jwalk::Parallelism::RayonDefaultPool {
            busy_timeout: Duration::from_secs(1),
        })
        .process_read_dir(|_depth, _path, _state, children| {
            // Filter out entries we want to skip
            children.retain(|entry| {
                if let Ok(ref e) = entry {
                    // Skip symlinks
                    if e.file_type().is_symlink() {
                        return false;
                    }
                    // Skip reparse points on Windows
                    if utils::is_windows_reparse_point(&e.path()) {
                        return false;
                    }
                    // Skip system directories
                    if utils::is_system_path(&e.path()) {
                        return false;
                    }
                }
                true
            });
        })
        .into_iter()
        .for_each(|entry| {
            if let Ok(e) = entry {
                if e.file_type().is_file() {
                    if let Ok(meta) = e.metadata() {
                        let size = meta.len();
                        total_size.fetch_add(size, Ordering::Relaxed);
                        total_files.fetch_add(1, Ordering::Relaxed);

                        // Track file size for largest files list
                        file_sizes.push((e.path().to_path_buf(), size));

                        // Add file to its parent directory's file list
                        if let Some(parent) = e.path().parent() {
                            dir_files
                                .entry(parent.to_path_buf())
                                .or_default()
                                .push((e.path().to_path_buf(), size));

                            *dir_sizes.entry(parent.to_path_buf()).or_insert(0) += size;
                            *dir_file_counts.entry(parent.to_path_buf()).or_insert(0) += 1;

                            // Also add to all ancestor directories
                            let mut current = parent;
                            while let Some(ancestor) = current.parent() {
                                *dir_sizes.entry(ancestor.to_path_buf()).or_insert(0) += size;
                                *dir_file_counts.entry(ancestor.to_path_buf()).or_insert(0) += 1;
                                current = ancestor;

                                // Stop if we've gone too deep or reached the root
                                if current == path
                                    || current.components().count() < path.components().count()
                                {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });

    let total_size = total_size.load(Ordering::Relaxed);
    let total_files = total_files.load(Ordering::Relaxed);

    // Get top 10 largest files
    file_sizes.sort_by(|a, b| b.1.cmp(&a.1));
    let largest_files = file_sizes.into_iter().take(10).collect();

    // Build folder tree starting from root
    let root = build_folder_tree(
        path,
        &dir_sizes,
        &dir_file_counts,
        &dir_files,
        total_size,
        max_depth,
    )?;

    Ok(DiskInsights {
        root,
        total_size,
        total_files,
        largest_files,
        scan_duration: start_time.elapsed(),
    })
}

/// Build a folder tree from directory size map
fn build_folder_tree(
    path: &Path,
    dir_sizes: &HashMap<PathBuf, u64>,
    dir_file_counts: &HashMap<PathBuf, u64>,
    dir_files: &HashMap<PathBuf, Vec<(PathBuf, u64)>>,
    parent_total: u64,
    max_depth: u8,
) -> Result<FolderNode> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.display().to_string());

    // Get children directories first
    let mut children = Vec::new();

    if max_depth > 0 {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let child_path = entry.path();

                // Skip if not a directory
                if !child_path.is_dir() {
                    continue;
                }

                // Skip symlinks and reparse points
                if utils::should_skip_entry(&child_path) {
                    continue;
                }

                // Skip system directories
                if utils::is_system_path(&child_path) {
                    continue;
                }

                // Include ALL directories (even if they have no direct files)
                // Build the tree recursively - this will calculate sizes from subdirectories
                // We'll pass the current directory's size as parent_total after calculating it
                // For now, pass a placeholder - we'll recalculate percentages after getting size
                let placeholder_total = *dir_sizes.get(&path.to_path_buf()).unwrap_or(&0);
                if let Ok(child_node) = build_folder_tree(
                    &child_path,
                    dir_sizes,
                    dir_file_counts,
                    dir_files,
                    placeholder_total.max(1), // Use placeholder, min 1 to avoid division by zero
                    max_depth - 1,
                ) {
                    // Add all children - they may have sizes from subdirectories
                    children.push(child_node);
                }
            }
        }
    }

    // Calculate size: dir_sizes already includes files in this directory AND all subdirectories
    // (because we add file sizes to all ancestor directories during scanning)
    let mut size = *dir_sizes.get(&path.to_path_buf()).unwrap_or(&0);

    // If size is 0 but we have children, sum their sizes (handles edge case where
    // a folder only has subdirectories but wasn't in dir_sizes)
    if size == 0 && !children.is_empty() {
        let children_size: u64 = children.iter().map(|c| c.size).sum();
        if children_size > 0 {
            // Use children's total size if dir_sizes wasn't populated
            // (this can happen if a folder only has subdirectories and wasn't in the scan path)
            size = children_size;
        }
    }

    // Calculate file count: dir_file_counts already includes files in this directory AND all subdirectories
    // (because we add file counts to all ancestor directories during scanning, just like sizes)
    let file_count = *dir_file_counts.get(&path.to_path_buf()).unwrap_or(&0);

    let percentage = if parent_total > 0 {
        (size as f64 / parent_total as f64) * 100.0
    } else {
        100.0
    };

    // Sort children by size descending
    children.sort_by(|a, b| b.size.cmp(&a.size));

    // Recalculate children's percentages now that we know the current directory's size
    // (they were calculated with a placeholder parent_total)
    if size > 0 {
        for child in &mut children {
            child.percentage = (child.size as f64 / size as f64) * 100.0;
        }
    }

    // Collect files directly in this directory
    let files: Vec<FileInfo> = dir_files
        .get(&path.to_path_buf())
        .map(|file_list| {
            file_list
                .iter()
                .map(|(file_path, file_size)| {
                    let name = file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| file_path.display().to_string());
                    FileInfo {
                        path: file_path.clone(),
                        name,
                        size: *file_size,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(FolderNode {
        path: path.to_path_buf(),
        name,
        size,
        file_count,
        children,
        files,
        percentage,
    })
}

/// Get top-level folders sorted by size
pub fn get_top_folders(node: &FolderNode, limit: usize) -> Vec<&FolderNode> {
    let mut folders: Vec<&FolderNode> = node.children.iter().collect();
    folders.sort_by(|a, b| b.size.cmp(&a.size));
    folders.into_iter().take(limit).collect()
}

/// Sort folder children by the specified criteria
pub fn sort_children(node: &mut FolderNode, sort_by: SortBy) {
    match sort_by {
        SortBy::Size => {
            node.children.sort_by(|a, b| b.size.cmp(&a.size));
        }
        SortBy::Name => {
            node.children.sort_by(|a, b| a.name.cmp(&b.name));
        }
        SortBy::Files => {
            node.children
                .sort_by(|a, b| b.file_count.cmp(&a.file_count));
        }
    }

    // Recursively sort children
    for child in &mut node.children {
        sort_children(child, sort_by);
    }
}

/// Find a folder node by path (for navigation)
pub fn find_folder_by_path<'a>(node: &'a FolderNode, target_path: &Path) -> Option<&'a FolderNode> {
    if node.path == target_path {
        return Some(node);
    }

    for child in &node.children {
        if let Some(found) = find_folder_by_path(child, target_path) {
            return Some(found);
        }
    }

    None
}

/// Get breadcrumb path from root to target
pub fn get_breadcrumb(root: &FolderNode, target: &Path) -> Vec<String> {
    let mut breadcrumb = Vec::new();

    // Build path components
    let mut current = target;
    let root_path = &root.path;

    // Collect path components from target back to root
    let mut components = Vec::new();
    while let Some(parent) = current.parent() {
        if parent == root_path || !current.starts_with(root_path) {
            break;
        }
        if let Some(name) = current.file_name().and_then(|n| n.to_str()) {
            components.push(name.to_string());
        }
        current = parent;
    }

    // Reverse to get root -> target order
    components.reverse();
    breadcrumb.extend(components);

    breadcrumb
}
