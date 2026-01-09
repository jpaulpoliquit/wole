use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Create a spinner for indeterminate progress
pub fn create_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")
            .unwrap()
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Create a progress bar for determinate progress
pub fn create_progress_bar(total: u64, msg: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓░")
    );
    pb.set_message(msg.to_string());
    pb
}

/// Finish progress bar with a success message
pub fn finish_with_message(pb: &ProgressBar, msg: &str) {
    pb.finish_with_message(msg.to_string());
}

/// Finish and clear progress bar
pub fn finish_and_clear(pb: &ProgressBar) {
    pb.finish_and_clear();
}
