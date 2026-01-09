//! Confirmation screen overlay

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
    style::Style,
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table,
    },
    Frame,
};

/// Generate a fun comparison for the amount of space to be reclaimed
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
            Some(format!(
                "That's like ~{} AAA game installs (~{:.1} GB)!",
                count, gb
            ))
        } else {
            Some(format!(
                "That's like a partial game install (~{:.1} GB)!",
                gb
            ))
        }
    } else if bytes >= 500 * MB {
        let count = bytes / node_modules_size;
        let gb = bytes as f64 / GB as f64;
        Some(format!(
            "That's like ~{} node_modules folders (~{:.1} GB)!",
            count, gb
        ))
    } else if bytes >= 10 * MB {
        let count = bytes / floppy_size;
        let mb = bytes as f64 / MB as f64;
        Some(format!(
            "That's like ~{} floppy disks (~{:.0} MB)!",
            count, mb
        ))
    } else {
        None
    }
}

pub fn render(f: &mut Frame, app_state: &mut AppState) {
    let area = f.area();

    // Layout: logo+tagline, warning, items area (split into summary and file list), actions, shortcuts
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(LOGO_WITH_TAGLINE_HEIGHT), // Logo + 2 blank lines + tagline
            Constraint::Length(5),                        // Warning message
            Constraint::Min(12),                          // Items area (will be split horizontally)
            Constraint::Length(6),                        // Actions
            Constraint::Length(3),                        // Shortcuts
        ])
        .split(area);

    // Logo and tagline (using reusable widgets)
    render_logo(f, chunks[0]);
    render_tagline(f, chunks[0]);

    // Warning message
    let selected_count = app_state.selected_count();
    let selected_size = app_state.selected_size();

    let mut warning_lines = vec![Line::from("")];

    if selected_count == 0 {
        warning_lines.push(Line::from(vec![Span::styled(
            "  ⚠  NO ITEMS SELECTED",
            Styles::warning(),
        )]));
        warning_lines.push(Line::from(vec![Span::styled(
            "     Use Space to select items, then confirm deletion",
            Styles::secondary(),
        )]));
    } else {
        // Handle singular/plural
        let item_text = if selected_count == 1 { "ITEM" } else { "ITEMS" };

        warning_lines.push(Line::from(vec![
            Span::styled("  ⚠  DELETE ", Styles::warning()),
            Span::styled(
                format!("{} {}", selected_count, item_text),
                Styles::emphasis(),
            ),
            Span::styled(
                format!(" ({})", bytesize::to_string(selected_size, true)),
                Styles::secondary(),
            ),
        ]));

        // Add fun comparison if applicable
        if let Some(comparison) = fun_comparison(selected_size) {
            warning_lines.push(Line::from(vec![Span::styled(
                format!("     {}", comparison),
                Styles::emphasis(),
            )]));
        } else {
            warning_lines.push(Line::from(""));
        }

        warning_lines.push(Line::from(vec![Span::styled(
            "     Files will be moved to Recycle Bin (recoverable)",
            Styles::secondary(),
        )]));
    }

    let warning = Paragraph::new(warning_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Styles::warning())
            .title("CONFIRM DELETION"),
    );
    f.render_widget(warning, chunks[1]);

    // Split items area into summary (left) and file list (right)
    let items_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(35), // Category summary (smaller)
            Constraint::Min(20),    // File list (larger, takes remaining space)
        ])
        .split(chunks[2]);

    // Category summary table (smaller, on the left)
    render_summary_table(f, items_chunks[0], app_state);

    // File list (larger, on the right)
    render_file_list(f, items_chunks[1], app_state);

    // Actions
    let actions_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("    [Y] ", Styles::emphasis()),
            Span::styled("Delete (to Recycle Bin)", Styles::primary()),
            Span::styled("       [N] ", Styles::secondary()),
            Span::styled("Cancel", Styles::secondary()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    [P] ", Styles::warning()),
            Span::styled("Permanent Delete", Styles::warning()),
            Span::styled(
                " (bypass Recycle Bin - cannot be undone!)",
                Styles::secondary(),
            ),
        ]),
    ];
    let actions = Paragraph::new(actions_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Styles::border())
            .title("ACTIONS"),
    );
    f.render_widget(actions, chunks[3]);

    // Shortcuts
    let shortcuts = get_shortcuts(&app_state.screen, Some(app_state));
    render_shortcuts(f, chunks[4], &shortcuts);
}

fn render_summary_table(f: &mut Frame, area: Rect, app_state: &AppState) {
    // Group currently selected items by category
    use std::collections::HashMap;
    let mut category_stats: HashMap<String, (usize, u64)> = HashMap::new();

    for &index in &app_state.selected_items {
        if let Some(item) = app_state.all_items.get(index) {
            let entry = category_stats
                .entry(item.category.clone())
                .or_insert((0, 0));
            entry.0 += 1;
            entry.1 += item.size_bytes;
        }
    }

    // Build table rows
    let mut rows = vec![Row::new(vec![
        Cell::from("CATEGORY").style(Styles::header()),
        Cell::from("ITEMS").style(Styles::header()),
        Cell::from("SIZE").style(Styles::header()),
    ])];

    let mut category_vec: Vec<_> = category_stats.iter().collect();
    // Sort by size descending, then by category name for stable ordering when sizes are equal
    category_vec.sort_by(|a, b| {
        let size_cmp = b.1 .1.cmp(&a.1 .1);
        if size_cmp == std::cmp::Ordering::Equal {
            a.0.cmp(b.0) // Secondary sort by category name for stability
        } else {
            size_cmp
        }
    });

    for (category, (count, size)) in category_vec {
        rows.push(Row::new(vec![
            Cell::from(format!("  {}", category)),
            Cell::from(format!("{}", count)),
            Cell::from(bytesize::to_string(*size, true)),
        ]));
    }

    // Add total row
    rows.push(Row::new(vec![
        Cell::from(""),
        Cell::from(""),
        Cell::from(""),
    ]));
    rows.push(Row::new(vec![
        Cell::from("  TOTAL").style(Styles::emphasis()),
        Cell::from(format!("{}", app_state.selected_count())).style(Styles::emphasis()),
        Cell::from(bytesize::to_string(app_state.selected_size(), true)).style(Styles::emphasis()),
    ]));

    let table = Table::new(
        rows,
        &[
            Constraint::Percentage(50),
            Constraint::Length(8),
            Constraint::Length(12),
        ],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Styles::border())
            .title("SUMMARY"),
    );

    f.render_widget(table, area);
}

fn render_file_list(f: &mut Frame, area: Rect, app_state: &mut AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Styles::border())
        .title("FILES TO DELETE");

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Build rows for confirm screen
    let rows = app_state.confirm_rows();

    if rows.is_empty() {
        let empty = Paragraph::new(Line::from(vec![Span::styled(
            "  No items selected",
            Styles::secondary(),
        )]));
        f.render_widget(empty, inner);
        return;
    }

    // Get confirm category groups for rendering
    let confirm_groups = app_state.confirm_category_groups();
    let skip_category_header = confirm_groups.len() == 1;

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

    // Build display lines from row model
    let mut lines: Vec<Line> = Vec::new();
    // Track mapping from line index to row index for proper cursor positioning
    let mut line_to_row: Vec<usize> = Vec::new();

    for (row_idx, row) in rows.iter().enumerate() {
        let is_cursor = row_idx == app_state.cursor;
        let row_style = if is_cursor {
            Styles::selected()
        } else {
            Style::default()
        };
        let prefix = if is_cursor { ">" } else { " " };

        match *row {
            crate::tui::state::ConfirmRow::CategoryHeader { cat_idx } => {
                if skip_category_header {
                    continue;
                }

                let Some(group) = confirm_groups.get(cat_idx) else {
                    continue;
                };

                ctx = if group.grouped_by_folder {
                    Context::None
                } else {
                    Context::FlatItems
                };

                let icon = if group.safe { "✓" } else { "!" };
                let icon_style = category_style(group.safe);

                // Calculate selected items in this category
                let item_indices: Vec<usize> = if group.grouped_by_folder {
                    group
                        .folder_groups
                        .iter()
                        .flat_map(|fg| fg.items.iter().copied())
                        .collect()
                } else {
                    group.items.clone()
                };
                let selected_in_group = item_indices
                    .iter()
                    .filter(|&&idx| app_state.selected_items.contains(&idx))
                    .count();
                let total_in_group = item_indices.len();

                let (checkbox, checkbox_style) = tri_checkbox(selected_in_group, total_in_group);
                let exp_marker = if group.expanded { "▾" } else { "▸" };

                lines.push(Line::from(vec![
                    Span::styled(format!(" {} ", prefix), row_style),
                    Span::styled(checkbox, checkbox_style),
                    Span::raw(" "),
                    Span::styled(format!("{} {} ", exp_marker, icon), icon_style),
                    Span::styled(format!("{:<12}", group.name), Styles::emphasis()),
                    Span::styled(
                        format!("{:>8}", bytesize::to_string(group.total_size, true)),
                        Styles::primary(),
                    ),
                    Span::styled("    ", Styles::secondary()),
                    Span::styled(
                        format!("{}/{} items", selected_in_group, total_in_group),
                        Styles::secondary(),
                    ),
                    if group.safe {
                        Span::styled("  [safe to delete]", Styles::checked())
                    } else {
                        Span::styled("  [review recommended]", Styles::warning())
                    },
                ]));
                line_to_row.push(row_idx);
            }
            crate::tui::state::ConfirmRow::FolderHeader {
                cat_idx,
                folder_idx,
            } => {
                let Some(group) = confirm_groups.get(cat_idx) else {
                    continue;
                };
                let Some(folder) = group.folder_groups.get(folder_idx) else {
                    continue;
                };

                ctx = Context::FolderItems;

                let selected_in_folder = folder
                    .items
                    .iter()
                    .filter(|&&idx| app_state.selected_items.contains(&idx))
                    .count();
                let total_in_folder = folder.items.len();
                let (checkbox, checkbox_style) = tri_checkbox(selected_in_folder, total_in_folder);
                let exp_marker = if folder.expanded { "▾" } else { "▸" };

                // Convert folder path to relative path
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

                lines.push(Line::from(vec![
                    Span::styled(format!("{}{} ", indent, prefix), row_style),
                    Span::styled(checkbox, checkbox_style),
                    Span::raw(" "),
                    Span::styled(format!("{} ", exp_marker), Styles::secondary()),
                    Span::styled(folder_display, Styles::emphasis()),
                    Span::styled(format!("  {:>8}", size_str), Styles::primary()),
                    Span::styled(
                        format!("  ({}/{})", selected_in_folder, total_in_folder),
                        Styles::secondary(),
                    ),
                ]));
                line_to_row.push(row_idx);
            }
            crate::tui::state::ConfirmRow::Item { item_idx } => {
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

                let path_str = crate::utils::to_relative_path(&item.path, &app_state.scan_path);
                let size_str = bytesize::to_string(item.size_bytes, true);

                // Truncate path if needed
                let fixed = indent.len() + 3 /*prefix+spaces*/ + 3 /*checkbox*/ + 1 /*space*/ + 2 /*two spaces before size*/ + 8;
                let max_len = (inner.width as usize).saturating_sub(fixed).max(10);
                let path_display = if path_str.len() > max_len {
                    format!(
                        "...{}",
                        &path_str[path_str.len().saturating_sub(max_len.saturating_sub(3))..]
                    )
                } else {
                    path_str
                };

                lines.push(Line::from(vec![
                    Span::styled(format!("{}{} ", indent, prefix), row_style),
                    Span::styled(checkbox, checkbox_style),
                    Span::raw(" "),
                    Span::styled(path_display, Styles::primary()),
                    Span::styled(format!("  {:>8}", size_str), Styles::secondary()),
                ]));
                line_to_row.push(row_idx);
            }
            crate::tui::state::ConfirmRow::Spacer => {
                ctx = Context::None;
                lines.push(Line::from(""));
                line_to_row.push(row_idx);
            }
        }
    }

    // Handle scrolling - find the line index for current cursor
    let cursor_line_idx = line_to_row
        .iter()
        .position(|&r| r == app_state.cursor)
        .unwrap_or(0);

    let visible_height = inner.height as usize;
    // Update cached visible height in app state for event handlers
    app_state.visible_height = visible_height;
    let total_lines = lines.len();

    // Calculate scroll to keep cursor visible
    let scroll = if cursor_line_idx < app_state.scroll_offset {
        cursor_line_idx
    } else if cursor_line_idx >= app_state.scroll_offset + visible_height {
        cursor_line_idx.saturating_sub(visible_height.saturating_sub(1))
    } else {
        app_state.scroll_offset
    }
    .min(total_lines.saturating_sub(visible_height));

    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();

    let paragraph = Paragraph::new(visible_lines);
    f.render_widget(paragraph, inner);

    // Scrollbar if needed
    if total_lines > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        let mut scrollbar_state = ScrollbarState::new(total_lines).position(scroll);
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
