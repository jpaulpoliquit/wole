use anyhow::Result;
use chrono::{DateTime, Utc};
use git2::Repository;
use std::path::{Path, PathBuf};

/// Find the git root directory by walking up from the given path
pub fn find_git_root(path: &Path) -> Option<PathBuf> {
    let mut current = path.to_path_buf();
    
    loop {
        let git_dir = current.join(".git");
        if git_dir.exists() {
            return Some(current);
        }
        
        if !current.pop() {
            break;
        }
    }
    
    None
}

/// Check if a git repository has uncommitted changes (dirty)
pub fn is_dirty(repo_path: &Path) -> Result<bool> {
    let repo = match Repository::open(repo_path) {
        Ok(repo) => repo,
        Err(_) => return Ok(false), // Not a git repo or can't open - not dirty
    };
    
    let mut status_options = git2::StatusOptions::new();
    status_options.include_ignored(false);
    status_options.include_untracked(true);
    
    let statuses = match repo.statuses(Some(&mut status_options)) {
        Ok(statuses) => statuses,
        Err(_) => return Ok(false), // Can't get status - assume not dirty
    };
    
    // If there are any status entries, the repo is dirty
    Ok(!statuses.is_empty())
}

/// Get the date of the last commit in a git repository
pub fn last_commit_date(repo_path: &Path) -> Result<Option<DateTime<Utc>>> {
    let repo = match Repository::open(repo_path) {
        Ok(repo) => repo,
        Err(_) => return Ok(None), // Not a git repo
    };
    
    let head = match repo.head() {
        Ok(head) => head,
        Err(_) => return Ok(None), // No HEAD
    };
    
    let commit = match head.peel_to_commit() {
        Ok(commit) => commit,
        Err(_) => return Ok(None), // Can't get commit
    };
    
    let time = commit.time();
    let datetime = DateTime::from_timestamp(time.seconds(), 0)
        .unwrap_or_else(|| Utc::now());
    
    Ok(Some(datetime))
}
