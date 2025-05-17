use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

// Helper function to split ASCII art into multiple lines and apply color
pub fn split_ascii_art(art: &str, color: Color) -> Vec<Line> {
    art.trim().lines()
        .map(|line| Line::from(vec![Span::styled(line.to_string(), Style::default().fg(color))]))
        .collect()
} 