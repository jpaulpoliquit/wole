//! Theme and styling for TUI - No colors, typography-based hierarchy

use ratatui::style::{Modifier, Style};

/// Style definitions - using only typography (bold, underline, etc.)
pub struct Styles;

impl Styles {
    /// Main title style - bold, large emphasis
    pub fn title() -> Style {
        Style::default()
            .add_modifier(Modifier::BOLD)
    }
    
    /// Header style - bold
    pub fn header() -> Style {
        Style::default()
            .add_modifier(Modifier::BOLD)
    }
    
    /// Primary text style - normal
    pub fn primary() -> Style {
        Style::default()
    }
    
    /// Secondary/muted text style - dimmed
    pub fn secondary() -> Style {
        Style::default()
            .add_modifier(Modifier::DIM)
    }
    
    /// Emphasized text - bold
    pub fn emphasis() -> Style {
        Style::default()
            .add_modifier(Modifier::BOLD)
    }
    
    /// Underlined text for emphasis
    pub fn underlined() -> Style {
        Style::default()
            .add_modifier(Modifier::UNDERLINED)
    }
    
    /// Selected/highlighted row style - reverse video
    pub fn selected() -> Style {
        Style::default()
            .add_modifier(Modifier::REVERSED | Modifier::BOLD)
    }
    
    /// Checkbox checked style - bold
    pub fn checked() -> Style {
        Style::default()
            .add_modifier(Modifier::BOLD)
    }
    
    /// Border style - normal
    pub fn border() -> Style {
        Style::default()
    }
    
    /// Success style - bold
    pub fn success() -> Style {
        Style::default()
            .add_modifier(Modifier::BOLD)
    }
    
    /// Warning style - bold + underlined
    pub fn warning() -> Style {
        Style::default()
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    }
    
    /// Danger style - bold + underlined
    pub fn danger() -> Style {
        Style::default()
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    }
    
    /// Accent/highlight style - bold
    pub fn accent() -> Style {
        Style::default()
            .add_modifier(Modifier::BOLD)
    }
    
    /// Error style - bold + underlined (same as danger)
    pub fn error() -> Style {
        Style::default()
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    }
    
    /// Muted/dimmed text style - dimmed (same as secondary)
    pub fn muted() -> Style {
        Style::default()
            .add_modifier(Modifier::DIM)
    }
}

/// Get style for a category based on safety - no color difference
pub fn category_style(_safe: bool) -> Style {
    // No visual difference, just use normal style
    Style::default()
}
