//! Results screen with grouped categories

use crate::tui::{
    state::AppState,
    theme::{category_style, Styles},
    widgets::{
        logo::{render_logo, render_tagline, LOGO_WITH_TAGLINE_HEIGHT},
        shortcuts::{get_shortcuts, render_shortcuts},
    },
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

/// Disk space information
#[derive(Debug, Clone, Copy)]
struct DiskSpace {
    free_bytes: u64,
    total_bytes: u64,
}

/// Get disk space information on the system drive
fn get_disk_space() -> Option<DiskSpace> {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let path: Vec<u16> = OsStr::new("C:\\")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut free_bytes_available: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut total_free_bytes: u64 = 0;

        unsafe {
            extern "system" {
                fn GetDiskFreeSpaceExW(
                    lpDirectoryName: *const u16,
                    lpFreeBytesAvailableToCaller: *mut u64,
                    lpTotalNumberOfBytes: *mut u64,
                    lpTotalNumberOfFreeBytes: *mut u64,
                ) -> i32;
            }

            let result = GetDiskFreeSpaceExW(
                path.as_ptr(),
                &mut free_bytes_available,
                &mut total_bytes,
                &mut total_free_bytes,
            );

            if result != 0 {
                return Some(DiskSpace {
                    free_bytes: free_bytes_available,
                    total_bytes,
                });
            }
        }
        None
    }

    #[cfg(not(windows))]
    {
        None
    }
}

/// Generate a fun comparison for the amount of space
fn fun_comparison(bytes: u64) -> Option<String> {
    const MB: u64 = 1_000_000;
    const GB: u64 = 1_000_000_000;

    let game_size: u64 = 50 * GB; // ~50 GB for AAA game
    let node_modules_size: u64 = 500 * MB; // ~500 MB average node_modules
    let floppy_size: u64 = 1_440_000; // 1.44 MB floppy disk

    if bytes >= 10 * GB {
        let count = bytes / game_size;
        let gb = bytes as f64 / GB as f64;
        if count >= 1 {
            Some(format!("~{} AAA game installs (~{:.1} GB)", count, gb))
        } else {
            Some(format!("a partial game install (~{:.1} GB)", gb))
        }
    } else if bytes >= 500 * MB {
        let count = bytes / node_modules_size;
        let gb = bytes as f64 / GB as f64;
        Some(format!("~{} node_modules folders (~{:.1} GB)", count, gb))
    } else if bytes >= 10 * MB {
        let count = bytes / floppy_size;
        let mb = bytes as f64 / MB as f64;
        Some(format!("~{} floppy disks (~{:.0} MB)", count, mb))
    } else {
        None
    }
}

pub fn render(f: &mut Frame, app_state: &mut AppState) {
    let area = f.area();

    // Layout: logo+tagline, summary, search bar (always visible), grouped results, shortcuts
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(LOGO_WITH_TAGLINE_HEIGHT), // Logo + 2 blank lines + tagline
            Constraint::Length(5),                        // Summary (increased for extra info)
            Constraint::Length(3),                        // Search bar (always visible)
            Constraint::Min(10),                          // Grouped results
            Constraint::Length(3),                        // Shortcuts
        ])
        .split(area);

    // Logo and tagline (using reusable widgets)
    render_logo(f, chunks[0]);
    render_tagline(f, chunks[0]);

    // Summary
    let total_size = app_state.selected_size();
    let total_items = app_state.all_items.len();
    let selected_count = app_state.selected_count();
    let categories_count = app_state.category_groups.len();
    let disk_space = get_disk_space();
    let show_storage_info = app_state.config.ui.show_storage_info;

    let mut summary_lines = vec![Line::from(vec![
        Span::styled("  Found: ", Styles::secondary()),
        Span::styled(format!("{} items", total_items), Styles::emphasis()),
        Span::styled(" │ ", Styles::secondary()),
        Span::styled("Selected: ", Styles::secondary()),
        Span::styled(format!("{}", selected_count), Styles::checked()),
        Span::styled(" │ ", Styles::secondary()),
        Span::styled("Reclaimable: ", Styles::secondary()),
        Span::styled(bytesize::to_string(total_size, true), Styles::emphasis()),
        Span::styled(" │ ", Styles::secondary()),
        Span::styled("Categories: ", Styles::secondary()),
        Span::styled(format!("{}", categories_count), Styles::emphasis()),
    ])];

    // Second line: storage info or free space and fun comparison
    let mut line2_spans = vec![Span::styled("  ", Styles::secondary())];

    if let Some(disk) = disk_space {
        if show_storage_info {
            // Show current storage (used) and storage after deletion (used after reclaiming space)
            let current_storage = disk.total_bytes - disk.free_bytes;
            let storage_after = current_storage.saturating_sub(total_size);

            line2_spans.push(Span::styled("Current storage: ", Styles::secondary()));
            line2_spans.push(Span::styled(
                bytesize::to_string(current_storage, true),
                Styles::emphasis(),
            ));
            line2_spans.push(Span::styled(" │ ", Styles::secondary()));
            line2_spans.push(Span::styled("Storage after: ", Styles::secondary()));
            line2_spans.push(Span::styled(
                bytesize::to_string(storage_after, true),
                Styles::emphasis(),
            ));
        } else {
            // Show free space (original behavior)
            line2_spans.push(Span::styled("Free space: ", Styles::secondary()));
            line2_spans.push(Span::styled(
                bytesize::to_string(disk.free_bytes, true),
                Styles::emphasis(),
            ));
        }
    }

    if let Some(comparison) = fun_comparison(total_size) {
        if disk_space.is_some() {
            line2_spans.push(Span::styled(" │ ", Styles::secondary()));
        }
        line2_spans.push(Span::styled(
            format!("That's like {} worth of space!", comparison),
            Styles::secondary(),
        ));
    }

    summary_lines.push(Line::from(line2_spans));
    summary_lines.push(Line::from(""));
    summary_lines.push(Line::from(vec![
        Span::styled("  Press ", Styles::secondary()),
        Span::styled("[C]", Styles::emphasis()),
        Span::styled(" to clean selected items", Styles::secondary()),
    ]));

    let summary = Paragraph::new(summary_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Styles::border())
            .title("SCAN RESULTS"),
    );
    f.render_widget(summary, chunks[1]);

    // Search bar (always visible)
    render_search_bar(f, chunks[2], app_state);

    // Grouped results
    render_grouped_results(f, chunks[3], app_state);

    // Shortcuts - always in chunks[4]
    let shortcuts = get_shortcuts(&app_state.screen, Some(app_state));
    render_shortcuts(f, chunks[4], &shortcuts);
}

fn render_search_bar(f: &mut Frame, area: Rect, app_state: &AppState) {
    let search_text = if app_state.search_mode {
        format!("/ {}_", app_state.search_query) // Cursor indicator
    } else if app_state.search_query.is_empty() {
        "Press / to filter results...".to_string()
    } else {
        format!("Filter: {} (Esc to clear)", app_state.search_query)
    };

    let style = if app_state.search_mode {
        Styles::emphasis()
    } else {
        Styles::secondary()
    };

    let paragraph = Paragraph::new(search_text).style(style).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Styles::border())
            .title("SEARCH"),
    );

    f.render_widget(paragraph, area);
}

fn render_grouped_results(f: &mut Frame, area: Rect, app_state: &mut AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Styles::border())
        .title("CATEGORIES");

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app_state.category_groups.is_empty() {
        let empty = Paragraph::new(Line::from(vec![Span::styled(
            "  No items found",
            Styles::secondary(),
        )]));
        f.render_widget(empty, inner);
        return;
    }

    // Build display lines from a flattened row model so navigation matches rendering.
    let mut lines: Vec<Line> = Vec::new();
    let rows = if app_state.search_query.is_empty() {
        app_state.results_rows()
    } else {
        app_state.filtered_results_rows()
    };

    // If rows is empty but we have category groups, something went wrong
    // Try to show items directly as a fallback - show ALL categories
    if rows.is_empty() && !app_state.category_groups.is_empty() {
        // Fallback: show items directly from all category groups
        for (group_idx, group) in app_state.category_groups.iter().enumerate() {
            let item_indices = app_state.category_item_indices(group_idx);
            if !item_indices.is_empty() {
                // Show category name as header
                if app_state.category_groups.len() > 1 {
                    lines.push(Line::from(vec![Span::styled(
                        format!("  {} ({} items)", group.name, item_indices.len()),
                        Styles::emphasis(),
                    )]));
                }
                // Show items directly without folder grouping
                for &item_idx in &item_indices {
                    let Some(item) = app_state.all_items.get(item_idx) else {
                        continue;
                    };
                    let is_selected = app_state.selected_items.contains(&item_idx);
                    let checkbox = if is_selected { "[X]" } else { "[ ]" };
                    let checkbox_style = if is_selected {
                        Styles::checked()
                    } else {
                        Styles::secondary()
                    };
                    let path_str = crate::utils::to_relative_path(&item.path, &app_state.scan_path);
                    let size_str = bytesize::to_string(item.size_bytes, true);
                    let indent = if app_state.category_groups.len() > 1 {
                        "    "
                    } else {
                        "  "
                    };
                    lines.push(Line::from(vec![
                        Span::styled(indent, Style::default()),
                        Span::styled(checkbox, checkbox_style),
                        Span::styled(" ", Style::default()),
                        Span::styled(path_str, Styles::primary()),
                        Span::styled(format!("  {:>8}", size_str), Styles::secondary()),
                    ]));
                }
                if app_state.category_groups.len() > 1
                    && group_idx < app_state.category_groups.len() - 1
                {
                    lines.push(Line::from(""));
                }
            }
        }
        if lines.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "  No items to display",
                Styles::secondary(),
            )]));
        }
    }

    fn tri_checkbox(selected: usize, total: usize) -> (&'static str, Style) {
        if total == 0 || selected == 0 {
            ("[ ]", Styles::secondary())
        } else if selected == total {
            ("[X]", Styles::checked())
        } else {
            ("[-]", Styles::warning())
        }
    }

    enum Context {
        None,
        FlatItems,
        FolderItems,
    }
    let mut ctx = Context::None;
    let mut current_folder_path: Option<String> = None; // Track current folder path to strip from items

    // When there's only one category, skip category header and adjust indentation
    let skip_category_header = app_state.category_groups.len() == 1;

    for (row_idx, row) in rows.iter().enumerate() {
        let is_cursor = row_idx == app_state.cursor;
        let row_style = if is_cursor {
            Styles::selected()
        } else {
            Style::default()
        };
        let apply_sel = |s: Style| {
            if is_cursor {
                s.patch(Styles::selected())
            } else {
                s
            }
        };
        let prefix = if is_cursor { ">" } else { " " };

        match *row {
            crate::tui::state::ResultsRow::CategoryHeader { group_idx } => {
                // Skip rendering category header if there's only one category
                if skip_category_header {
                    continue;
                }

                let Some(group) = app_state.category_groups.get(group_idx) else {
                    continue;
                };

                ctx = if group.grouped_by_folder {
                    Context::None
                } else {
                    Context::FlatItems
                };
                current_folder_path = None; // Clear folder path when leaving folder context

                let icon = if group.safe { "✓" } else { "!" };
                let icon_style = category_style(group.safe);

                let item_indices = app_state.category_item_indices(group_idx);
                let selected_in_group = item_indices
                    .iter()
                    .filter(|&&idx| app_state.selected_items.contains(&idx))
                    .count();
                let total_in_group = item_indices.len();

                let (checkbox, checkbox_style) = tri_checkbox(selected_in_group, total_in_group);
                let exp_marker = if group.expanded { "▾" } else { "▸" };

                let header_line = Line::from(vec![
                    Span::styled(format!(" {} ", prefix), row_style),
                    Span::styled(checkbox, apply_sel(checkbox_style)),
                    Span::styled(" ", row_style),
                    Span::styled(format!("{} {} ", exp_marker, icon), apply_sel(icon_style)),
                    Span::styled(format!("{:<12}", group.name), apply_sel(Styles::emphasis())),
                    Span::styled(
                        format!("{:>8}", bytesize::to_string(group.total_size, true)),
                        apply_sel(Styles::primary()),
                    ),
                    Span::styled("    ", apply_sel(Styles::secondary())),
                    Span::styled(
                        format!("{}/{} items", selected_in_group, total_in_group),
                        apply_sel(Styles::secondary()),
                    ),
                    if group.safe {
                        Span::styled("  [safe to delete]", apply_sel(Styles::checked()))
                    } else {
                        Span::styled("  [review recommended]", apply_sel(Styles::warning()))
                    },
                ]);
                lines.push(header_line);
            }
            crate::tui::state::ResultsRow::FolderHeader {
                group_idx,
                folder_idx,
            } => {
                let Some(group) = app_state.category_groups.get(group_idx) else {
                    continue;
                };
                let Some(folder) = group.folder_groups.get(folder_idx) else {
                    continue;
                };

                ctx = Context::FolderItems;

                // Store the folder path (relative) to strip from child items
                let folder_path = std::path::PathBuf::from(&folder.folder_name);
                current_folder_path = Some(crate::utils::to_relative_path(
                    &folder_path,
                    &app_state.scan_path,
                ));

                let selected_in_folder = folder
                    .items
                    .iter()
                    .filter(|&&idx| app_state.selected_items.contains(&idx))
                    .count();
                let total_in_folder = folder.items.len();
                let (checkbox, checkbox_style) = tri_checkbox(selected_in_folder, total_in_folder);
                let exp_marker = if folder.expanded { "▾" } else { "▸" };

                // Truncate folder path to avoid line wrapping.
                // Convert folder_name (which may be absolute) to relative path
                let folder_path = std::path::PathBuf::from(&folder.folder_name);
                let folder_str = crate::utils::to_relative_path(&folder_path, &app_state.scan_path);
                let size_str = bytesize::to_string(folder.total_size, true);

                // Adjust indent based on whether category header is shown
                let indent = if skip_category_header { "" } else { "    " };
                let fixed = indent.len() + 2 /*prefix*/ + 1 /*space*/ + 3 /*checkbox*/ + 1 /*space*/ + 2 /*exp*/ + 2 /*space*/ + 2 /*two spaces before size*/ + 8 + 2 /*two spaces before count*/ + 10;
                let max_len = (inner.width as usize).saturating_sub(fixed).max(8);
                let folder_display = if folder_str.len() > max_len {
                    format!(
                        "...{}",
                        &folder_str[folder_str.len().saturating_sub(max_len.saturating_sub(3))..]
                    )
                } else {
                    folder_str
                };

                let folder_header = Line::from(vec![
                    Span::styled(format!("{}{} ", indent, prefix), row_style),
                    Span::styled(checkbox, apply_sel(checkbox_style)),
                    Span::styled(" ", row_style),
                    Span::styled(format!("{} ", exp_marker), apply_sel(Styles::secondary())),
                    Span::styled(folder_display, apply_sel(Styles::emphasis())),
                    Span::styled(format!("  {:>8}", size_str), apply_sel(Styles::primary())),
                    Span::styled(
                        format!("  ({}/{})", selected_in_folder, total_in_folder),
                        apply_sel(Styles::secondary()),
                    ),
                ]);
                lines.push(folder_header);
            }
            crate::tui::state::ResultsRow::Item { item_idx } => {
                let Some(item) = app_state.all_items.get(item_idx) else {
                    continue;
                };

                let is_selected = app_state.selected_items.contains(&item_idx);
                let checkbox = if is_selected { "[X]" } else { "[ ]" };
                let checkbox_style = if is_selected {
                    Styles::checked()
                } else {
                    Styles::secondary()
                };

                // Adjust indent based on context and whether category header is shown
                let indent = match ctx {
                    Context::FolderItems => {
                        if skip_category_header {
                            "  "
                        } else {
                            "      "
                        }
                    }
                    Context::FlatItems => {
                        if skip_category_header {
                            ""
                        } else {
                            "    "
                        }
                    }
                    Context::None => {
                        if skip_category_header {
                            ""
                        } else {
                            "    "
                        }
                    }
                };

                // Truncate the path to avoid line wrapping.
                let mut path_str = crate::utils::to_relative_path(&item.path, &app_state.scan_path);

                // If we're in a folder group, strip the folder prefix from the path
                if let Context::FolderItems = ctx {
                    if let Some(ref folder_path) = current_folder_path {
                        // Normalize paths for comparison (handle both / and \)
                        let normalized_folder = folder_path.replace('\\', "/");
                        let normalized_path = path_str.replace('\\', "/");

                        // Strip the folder path prefix from the item path
                        if normalized_path.starts_with(&normalized_folder) {
                            // Remove the folder path and the following separator
                            let remaining = &normalized_path[normalized_folder.len()..];
                            path_str = if remaining.starts_with('/') {
                                remaining[1..].to_string()
                            } else if remaining.is_empty() {
                                // If the item path is exactly the folder path, show just the filename
                                item.path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| remaining.to_string())
                            } else {
                                remaining.to_string()
                            };
                        }
                    }
                }

                let size_str = bytesize::to_string(item.size_bytes, true);
                let fixed = indent.len() + 3 /*prefix+spaces*/ + 3 /*checkbox*/ + 1 /*space*/ + 2 /*two spaces before size*/ + 8;
                let max_len = (inner.width as usize).saturating_sub(fixed).max(8);
                let path_display = if path_str.len() > max_len {
                    format!(
                        "...{}",
                        &path_str[path_str.len().saturating_sub(max_len.saturating_sub(3))..]
                    )
                } else {
                    path_str
                };

                // Add underline to path when cursor is on this item
                let path_style = if is_cursor {
                    row_style.add_modifier(Modifier::UNDERLINED)
                } else {
                    row_style
                };

                let item_line = Line::from(vec![
                    Span::styled(format!("{}{} ", indent, prefix), row_style),
                    Span::styled(checkbox, apply_sel(checkbox_style)),
                    Span::styled(" ", row_style),
                    Span::styled(path_display, path_style),
                    Span::styled(format!("  {:>8}", size_str), apply_sel(Styles::secondary())),
                ]);
                lines.push(item_line);
            }
            crate::tui::state::ResultsRow::Spacer => {
                ctx = Context::None;
                current_folder_path = None; // Clear folder path when leaving folder context
                lines.push(Line::from(""));
            }
        }
    }

    // Handle scrolling
    let visible_height = inner.height as usize;
    // Update cached visible height in app state for event handlers
    app_state.visible_height = visible_height;
    let total_lines = lines.len();
    let scroll = app_state
        .scroll_offset
        .min(total_lines.saturating_sub(visible_height));

    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();

    let paragraph = Paragraph::new(visible_lines);
    f.render_widget(paragraph, inner);

    // Scrollbar
    if total_lines > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        let mut scrollbar_state = ScrollbarState::new(total_lines).position(scroll);
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
