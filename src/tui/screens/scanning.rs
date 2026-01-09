//! Scanning screen with progress bars

use crate::tui::{
    state::AppState,
    theme::Styles,
    widgets::{
        logo::{render_logo, render_tagline, LOGO_WITH_TAGLINE_HEIGHT},
        progress::render_category_progress,
        shortcuts::{get_shortcuts, render_shortcuts},
    },
};
use bytesize;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Spinner frames for animation
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn get_spinner(tick: u64) -> &'static str {
    SPINNER_FRAMES[(tick as usize / 2) % SPINNER_FRAMES.len()]
}

/// Generate a short fun comparison for the amount of space found
fn fun_comparison_short(bytes: u64) -> Option<String> {
    const MB: u64 = 1_000_000;
    const GB: u64 = 1_000_000_000;

    let game_size: u64 = 50 * GB; // ~50 GB for AAA game
    let node_modules_size: u64 = 500 * MB; // ~500 MB average node_modules
    let floppy_size: u64 = 1_440_000; // 1.44 MB floppy disk

    if bytes >= 10 * GB {
        let count = bytes / game_size;
        if count >= 1 {
            Some(format!("(~{} game installs!)", count))
        } else {
            Some(format!("(partial game install!)"))
        }
    } else if bytes >= 500 * MB {
        let count = bytes / node_modules_size;
        Some(format!("(~{} node_modules!)", count))
    } else if bytes >= 10 * MB {
        let count = bytes / floppy_size;
        Some(format!("(~{} floppies!)", count))
    } else {
        None
    }
}

pub fn render(f: &mut Frame, app_state: &AppState) {
    let area = f.area();
    let spinner = get_spinner(app_state.tick);

    // Layout: logo+tagline, status, progress, shortcuts
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(LOGO_WITH_TAGLINE_HEIGHT), // Logo + 2 blank lines + tagline
            Constraint::Length(3),                        // Status with spinner
            Constraint::Min(8),                           // Progress bars
            Constraint::Length(6),                        // Stats (1 for label + 5 for box)
            Constraint::Length(3),                        // Shortcuts
        ])
        .split(area);

    // Logo and tagline (using reusable widgets)
    render_logo(f, chunks[0]);
    render_tagline(f, chunks[0]);

    // Status with animated spinner
    if let crate::tui::state::Screen::Scanning { ref progress } = app_state.screen {
        let status_text = if progress.current_category.is_empty() {
            format!("{}  Scanning...", spinner)
        } else {
            format!("{}  Scanning {}...", spinner, progress.current_category)
        };

        let status_lines = vec![Line::from(vec![Span::styled(
            status_text,
            Styles::emphasis(),
        )])];
        let status = Paragraph::new(status_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Styles::border())
                .title("SCANNING")
                .padding(ratatui::widgets::Padding::uniform(1)),
        );
        f.render_widget(status, chunks[1]);

        // Progress bars
        if progress.category_progress.is_empty() {
            let empty_msg = Paragraph::new(Line::from(vec![Span::styled(
                format!("{}  Initializing scan...", spinner),
                Styles::emphasis(),
            )]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Styles::border())
                    .title("CATEGORIES"),
            );
            f.render_widget(empty_msg, chunks[2]);
        } else {
            render_category_progress(f, chunks[2], &progress.category_progress, app_state.tick);
        }

        // Stats section with label outside
        let stats_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Label
                Constraint::Min(3),    // Stats box
            ])
            .split(chunks[3]);

        // Section label outside the box
        let label = Paragraph::new(Line::from(vec![Span::styled("PROGRESS", Styles::header())]))
            .alignment(ratatui::layout::Alignment::Left);
        f.render_widget(label, stats_chunks[0]);

        // Stats content in box without title
        let mut size_spans = vec![
            Span::styled("  Found: ", Styles::header()),
            Span::styled(
                format!("{} items", progress.total_found),
                Styles::emphasis(),
            ),
            Span::styled("    │    ", Styles::secondary()),
            Span::styled("Size: ", Styles::header()),
            Span::styled(
                bytesize::to_string(progress.total_size, true),
                Styles::emphasis(),
            ),
        ];

        // Add fun comparison if size is significant
        if let Some(comparison) = fun_comparison_short(progress.total_size) {
            size_spans.push(Span::styled("  ", Styles::secondary()));
            size_spans.push(Span::styled(comparison, Styles::secondary()));
        }

        let stats_lines = vec![
            Line::from(size_spans),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Scanned: ", Styles::secondary()),
                Span::styled(
                    format!("{} locations", progress.total_scanned),
                    Styles::secondary(),
                ),
            ]),
        ];
        let stats = Paragraph::new(stats_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Styles::border()),
        );
        f.render_widget(stats, stats_chunks[1]);
    } else {
        // Fallback
        let empty_msg = Paragraph::new(Line::from(vec![Span::styled(
            "No scan in progress",
            Styles::secondary(),
        )]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Styles::border()),
        );
        f.render_widget(empty_msg, chunks[2]);
    }

    // Shortcuts
    let shortcuts = get_shortcuts(&app_state.screen, Some(app_state));
    render_shortcuts(f, chunks[4], &shortcuts);
}

/// Render cleaning progress (similar to scanning)
pub fn render_cleaning(f: &mut Frame, app_state: &AppState) {
    let area = f.area();
    let spinner = get_spinner(app_state.tick);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(LOGO_WITH_TAGLINE_HEIGHT), // Logo + 2 blank lines + tagline
            Constraint::Length(3),                        // Status
            Constraint::Min(1),                           // Progress
            Constraint::Length(3),                        // Stats
            Constraint::Length(3),                        // Shortcuts
        ])
        .split(area);

    // Logo and tagline (using reusable widgets)
    render_logo(f, chunks[0]);
    render_tagline(f, chunks[0]);

    // Header with spinner
    let header = Paragraph::new(Line::from(vec![Span::styled(
        format!("{}  Cleaning files...", spinner),
        Styles::emphasis(),
    )]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Styles::border())
            .title("CLEANING"),
    );
    f.render_widget(header, chunks[1]);

    // Progress
    if let crate::tui::state::Screen::Cleaning { ref progress } = app_state.screen {
        let progress_pct = if progress.total > 0 {
            progress.cleaned as f32 / progress.total as f32
        } else {
            0.0
        };

        use crate::tui::widgets::progress::render_progress_bar;
        use ratatui::layout::{Constraint, Direction, Layout};

        let progress_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Progress bar
                Constraint::Min(1),    // Current file display
            ])
            .split(chunks[2]);

        render_progress_bar(
            f,
            progress_chunks[0],
            &progress.current_category,
            progress_pct,
            None,
            &format!("{}/{}", progress.cleaned, progress.total),
            app_state.tick,
        );

        // Display current file being deleted
        let current_file_text = if let Some(ref current_path) = progress.current_path {
            // Truncate path if too long
            let path_str = current_path.display().to_string();
            let max_len = (progress_chunks[1].width as usize).saturating_sub(4); // Account for padding
            let display_path = if path_str.len() > max_len {
                format!(
                    "...{}",
                    &path_str[path_str.len().saturating_sub(max_len.saturating_sub(3))..]
                )
            } else {
                path_str
            };
            format!("  Deleting: {}", display_path)
        } else {
            "  Preparing...".to_string()
        };

        let current_file_paragraph = Paragraph::new(Line::from(vec![Span::styled(
            current_file_text,
            Styles::primary(),
        )]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Styles::border())
                .title("CURRENT FILE"),
        );
        f.render_widget(current_file_paragraph, progress_chunks[1]);

        // Status
        let status_text = format!(
            "  Cleaned: {} items   │   Errors: {}",
            progress.cleaned, progress.errors
        );
        let status_paragraph = Paragraph::new(status_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Styles::border())
                .title("STATUS"),
        );
        f.render_widget(status_paragraph, chunks[3]);
    }

    // Shortcuts (empty for cleaning)
    let shortcuts = get_shortcuts(&app_state.screen, Some(app_state));
    render_shortcuts(f, chunks[4], &shortcuts);
}
