//! Restore screen - restore files from last deletion

use crate::tui::{
    state::AppState,
    theme::Styles,
    widgets::{
        logo::{render_logo, render_tagline, LOGO_WITH_TAGLINE_HEIGHT},
        shortcuts::{get_shortcuts, render_shortcuts},
    },
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app_state: &AppState) {
    let area = f.area();

    let is_small = area.height < 20 || area.width < 60;
    let shortcuts_height = if is_small { 2 } else { 3 };

    let header_height = LOGO_WITH_TAGLINE_HEIGHT;

    // Layout: header, content, shortcuts
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Min(1),
            Constraint::Length(shortcuts_height),
        ])
        .split(area);

    // Render header
    render_header(f, chunks[0], is_small);

    // Render content
    render_content(f, chunks[1], app_state, is_small);

    // Shortcuts
    let shortcuts = get_shortcuts(&app_state.screen, Some(app_state));
    render_shortcuts(f, chunks[2], &shortcuts);
}

fn render_header(f: &mut Frame, area: Rect, _is_small: bool) {
    render_logo(f, area);
    render_tagline(f, area);
}

fn render_content(f: &mut Frame, area: Rect, app_state: &AppState, _is_small: bool) {
    if let crate::tui::state::Screen::Restore { ref result } = app_state.screen {
        if let Some(ref restore_result) = result {
            // Show restore results
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(1)])
                .split(area);

            // Title
            let title = Paragraph::new("Restore Complete")
                .style(Styles::header())
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Styles::border())
                        .title("Restore"),
                );
            f.render_widget(title, chunks[0]);

            // Results
            let mut lines = vec![Line::from(vec![
                Span::styled("Restored: ", Styles::primary()),
                Span::styled(
                    format!("{} items", restore_result.restored),
                    if restore_result.restored > 0 {
                        Styles::success()
                    } else {
                        Styles::secondary()
                    },
                ),
            ])];

            if restore_result.restored > 0 {
                lines.push(Line::from(vec![
                    Span::styled("Size: ", Styles::primary()),
                    Span::styled(
                        bytesize::to_string(restore_result.restored_bytes, true),
                        Styles::success(),
                    ),
                ]));
            }

            if restore_result.errors > 0 {
                lines.push(Line::from(vec![
                    Span::styled("Errors: ", Styles::primary()),
                    Span::styled(format!("{}", restore_result.errors), Styles::error()),
                ]));
            }

            if restore_result.not_found > 0 {
                lines.push(Line::from(vec![
                    Span::styled("Not found: ", Styles::primary()),
                    Span::styled(
                        format!("{} items", restore_result.not_found),
                        Styles::muted(),
                    ),
                ]));
            }

            if restore_result.restored == 0
                && restore_result.errors == 0
                && restore_result.not_found == 0
            {
                lines.push(Line::from(vec![Span::styled(
                    "No files to restore from last deletion session.",
                    Styles::muted(),
                )]));
            }

            let content = Paragraph::new(lines)
                .style(Styles::primary())
                .alignment(Alignment::Left)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Styles::border()),
                );
            f.render_widget(content, chunks[1]);
        } else {
            // Show "Restoring..." message
            let message = Paragraph::new("Restoring files from last deletion session...")
                .style(Styles::primary())
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Styles::border())
                        .title("Restore"),
                );
            f.render_widget(message, area);
        }
    }
}
