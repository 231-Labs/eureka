use ratatui::{
    layout::{Rect, Alignment},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::app::App;
use crate::constants::{EUREKA_FRAMES, PRINTER_ACTIVE_FRAMES, PRINTER_IDLE_FRAMES};
use super::utils::split_ascii_art;

/// 渲染 EUREKA ASCII 藝術動畫
pub fn render_eureka_animation(f: &mut Frame, area: Rect, highlight_color: Color) {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let animation_frame = (time % 3) as usize;
    let ascii_art = Paragraph::new(EUREKA_FRAMES[animation_frame])
        .style(Style::default().fg(highlight_color))
        .alignment(Alignment::Center);
    f.render_widget(ascii_art, area);
}

/// 渲染科技風格動畫
pub fn render_tech_animation(f: &mut Frame, app: &App, area: Rect, primary_color: Color) {
    let tech_block = Block::default()
        .title("SYSTEM")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(primary_color));
    
    let tech_text = Paragraph::new(app.get_tech_animation())
        .style(Style::default().fg(if app.is_online { Color::Cyan } else { Color::Magenta }))
        .alignment(Alignment::Center)
        .block(tech_block);
    
    f.render_widget(tech_text, area);
}

/// 渲染環境氛圍動畫效果
pub fn render_ambient_noise(f: &mut Frame, time: u64, dim_color: Color) {
    // 模擬太空船環境氛圍噪音效果
    for _ in 0..15 {
        let noise_char = match time % 3 {
            0 => "▓",
            1 => "▒",
            _ => "░",
        };
        
        let x = (time * 7 + 11) % (f.size().width as u64 - 4) + 2;
        let y = (time * 13 + 5) % (f.size().height as u64 - 4) + 2;
        
        if x < f.size().width as u64 && y < f.size().height as u64 {
            let noise = Paragraph::new(noise_char)
                .style(Style::default().fg(dim_color));
            f.render_widget(noise, Rect::new(x as u16, y as u16, 1, 1));
        }
    }
}

/// 獲取打印機動畫幀
pub fn get_printer_animation_frames(app: &App, animation_frame: usize, color: Color) -> Vec<Line<'static>> {
    if matches!(app.print_status, crate::app::PrintStatus::Printing) {
        // 打印狀態 - 顯示活動動畫
        split_ascii_art(PRINTER_ACTIVE_FRAMES[animation_frame], color)
    } else {
        // 待機狀態 - 顯示閒置動畫
        split_ascii_art(PRINTER_IDLE_FRAMES[animation_frame], color)
    }
} 