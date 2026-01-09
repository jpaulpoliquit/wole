use crate::output::CategoryResult;
use anyhow::{Context, Result};
use trash::os_limited;

/// Scan the Recycle Bin for items
///
/// Note: Size calculation is skipped as it would require reading each file,
/// which is expensive. Only item count is tracked.
pub fn scan() -> Result<CategoryResult> {
    let mut result = CategoryResult::default();

    match os_limited::list() {
        Ok(items) => {
            result.items = items.len();
            // TrashItem doesn't expose size, so we just count items
            // Size would require reading each file which is expensive
            result.size_bytes = 0;
            result.paths = items
                .iter()
                .map(|i| i.original_parent.join(&i.name))
                .collect();
        }
        Err(e) => {
            eprintln!("Warning: Could not read Recycle Bin: {}", e);
        }
    }

    Ok(result)
}

/// Empty the Recycle Bin by purging all items
pub fn clean() -> Result<()> {
    let items = os_limited::list().context("Failed to list Recycle Bin items")?;

    if !items.is_empty() {
        os_limited::purge_all(&items).context("Failed to empty Recycle Bin")?;
    }

    Ok(())
}
