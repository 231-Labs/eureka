use ratatui::Frame;
use crate::app::App;
use super::{registration, main_view};

/// Main entry point for UI rendering
/// Decides whether to show registration or main UI
pub fn draw(f: &mut Frame, app: &mut App) {
    if app.is_registering_printer {
        registration::draw_registration(f, app);
    } else {
        main_view::draw_main(f, app);
    }
} 