// UI module for EUREKA 3D printing TUI application
// This module handles all the terminal UI rendering logic

mod main_view;
mod registration;
mod status_display;
mod draw;
mod utils;
mod animations;
mod ascii_arts;

// Re-export the public functions
pub use draw::draw;