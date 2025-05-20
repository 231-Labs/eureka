use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::app::{App, TaskStatus, PrintStatus};
use super::ascii_arts::{PRINTER_ACTIVE_FRAMES, PRINTER_IDLE_FRAMES};
use super::utils::split_ascii_art;
use super::animations::get_printer_animation_frames;

/// Renders the online mode active task display
pub fn render_online_active_task(f: &mut Frame, app: &mut App, area: Rect, time: u64) {
    let animation_frame = (time % 4) as usize;
    let highlight_color = Color::LightBlue;
    let secondary_color = Color::LightBlue;
    let dim_color = Color::DarkGray;
    
    // Block for the print job
    let task_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .title(" ACTIVE PRINT JOB ")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Cyan));
    
    if let Some(task) = app.tasks.iter().find(|task| matches!(task.status, TaskStatus::Printing)) {
        // Online mode - with delegated task display
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let elapsed_time = task.start_time
            .map(|start| current_time.saturating_sub(start))
            .unwrap_or(0);
        
        let _elapsed_hours = elapsed_time / 3600;
        let _elapsed_minutes = (elapsed_time % 3600) / 60;

        let mut task_info = vec![
            Line::from("").alignment(Alignment::Center),
            Line::from(vec![
                Span::raw("╭─"),
                Span::styled("MODEL", Style::default().fg(highlight_color)),
                Span::raw("─╮"),
            ]).alignment(Alignment::Center),
            Line::from(vec![
                Span::styled(&task.name, Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)),
            ]).alignment(Alignment::Center),
            Line::from(vec![
                Span::raw("╰"),
                Span::styled("──────", Style::default().fg(dim_color)),
                Span::raw("╯"),
            ]).alignment(Alignment::Center),
            Line::from("").alignment(Alignment::Center),
        ];

        // Add printer animation frames
        task_info.extend(get_printer_animation_frames(app, animation_frame, Color::Cyan));

        task_info.extend(vec![
            Line::from(vec![
                Span::styled("BLOB ID: ", Style::default().fg(dim_color)),
                Span::styled(task.get_short_sculpt_id(), Style::default().fg(secondary_color)),
            ]).alignment(Alignment::Center),
            Line::from(vec![
                Span::styled("CUSTOMER: ", Style::default().fg(dim_color)),
                Span::styled(task.get_short_customer(), Style::default().fg(secondary_color)),
            ]).alignment(Alignment::Center),
            Line::from("").alignment(Alignment::Center),
            Line::from(vec![
                Span::styled("◈ ", Style::default().fg(highlight_color)),
                Span::styled(
                    task.format_paid_amount(),
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                ),
                Span::styled(" ◈", Style::default().fg(highlight_color)),
            ]).alignment(Alignment::Center),
            Line::from(vec![
                Span::styled("ELAPSED: ", Style::default().fg(dim_color)),
                Span::styled(
                    task.format_elapsed_time(),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                ),
            ]).alignment(Alignment::Center),
        ]);

        // Add action hint based on printer status
        if !matches!(app.print_status, PrintStatus::Printing) {
            task_info.push(Line::from(vec![
                Span::styled("Press ", Style::default().fg(dim_color)),
                Span::styled("P", Style::default().fg(highlight_color)),
                Span::styled(" to start printing", Style::default().fg(dim_color)),
            ]).alignment(Alignment::Center));
        } else {
            task_info.push(Line::from(vec![
                Span::styled("Press ", Style::default().fg(dim_color)),
                Span::styled("E", Style::default().fg(highlight_color)),
                Span::styled(" to stop printing", Style::default().fg(dim_color)),
            ]).alignment(Alignment::Center));
        }

        let task_widget = Paragraph::new(task_info)
            .style(Style::default())
            .alignment(Alignment::Center)
            .block(task_block);

        f.render_widget(task_widget, area);
    } else {
        // Online mode - no delegated task
        let mut idle_text = vec![
            // Add some empty lines to center content better
            Line::from("").alignment(Alignment::Center),
            Line::from("").alignment(Alignment::Center),
            Line::from("").alignment(Alignment::Center),
            Line::from("").alignment(Alignment::Center),
            Line::from(vec![
                Span::styled("◢ ", Style::default().fg(highlight_color)),
                Span::styled("ONLINE MODE - AWAITING TASKS", Style::default().fg(secondary_color)),
                Span::styled(" ◣", Style::default().fg(highlight_color)),
            ]).alignment(Alignment::Center),
            Line::from("").alignment(Alignment::Center),
        ];
        
        // Add idle animation
        let frames = split_ascii_art(PRINTER_IDLE_FRAMES[animation_frame], Color::Cyan);
        idle_text.extend(frames);
        
        idle_text.extend(vec![
            Line::from("").alignment(Alignment::Center),
            Line::from("").alignment(Alignment::Center),
            Line::from(vec![
                Span::styled("◈ Waiting for tasks ⦿", Style::default().fg(dim_color)),
            ]).alignment(Alignment::Center),
        ]);
        
        let idle_widget = Paragraph::new(idle_text)
            .style(Style::default())
            .alignment(Alignment::Center)
            .block(task_block);
            
        f.render_widget(idle_widget, area);
    }
}

/// Renders the offline mode printer status and display
pub fn render_offline_printer(f: &mut Frame, app: &mut App, area: Rect, time: u64) {
    let animation_frame = (time % 4) as usize;
    let highlight_color = Color::LightRed;
    let secondary_color = Color::Red;
    let dim_color = Color::DarkGray;
    
    // Block for the print job
    let task_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .title(" ACTIVE PRINT JOB ")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Magenta));
        
    if matches!(app.print_status, PrintStatus::Idle) {
        // Offline mode - idle state
        let mut idle_text = vec![
            // Add some empty lines to center content better
            Line::from("").alignment(Alignment::Center),
            Line::from("").alignment(Alignment::Center),
            Line::from("").alignment(Alignment::Center),
            Line::from("").alignment(Alignment::Center),
            Line::from(vec![
                Span::styled("◢ ", Style::default().fg(highlight_color)),
                Span::styled("OFFLINE MODE - PERSONAL USE", Style::default().fg(secondary_color)),
                Span::styled(" ◣", Style::default().fg(highlight_color)),
            ]).alignment(Alignment::Center),
            Line::from("").alignment(Alignment::Center),
        ];
        
        // Add idle animation
        let frames = split_ascii_art(PRINTER_IDLE_FRAMES[animation_frame], Color::Magenta);
        idle_text.extend(frames);
        
        idle_text.extend(vec![
            Line::from("").alignment(Alignment::Center),
            Line::from("").alignment(Alignment::Center),
            Line::from(vec![
                Span::styled("Select a model and press ", Style::default().fg(dim_color)),
                Span::styled("P", Style::default().fg(highlight_color)),
                Span::styled(" to print", Style::default().fg(dim_color)),
            ]).alignment(Alignment::Center),
            // Add some empty lines for better centering
            Line::from("").alignment(Alignment::Center),
            Line::from("").alignment(Alignment::Center),
        ]);
        
        let idle_widget = Paragraph::new(idle_text)
            .style(Style::default())
            .alignment(Alignment::Center)
            .block(task_block);
            
        f.render_widget(idle_widget, area);
    } else {
        // Offline mode - printing personal model
        if let Some(selected) = app.sculpt_state.selected() {
            if let Some(sculpt) = app.sculpt_items.get(selected) {
                let mut printing_text = vec![
                    // Add some empty lines to center content better
                    Line::from("").alignment(Alignment::Center),
                    Line::from("").alignment(Alignment::Center),
                    Line::from("").alignment(Alignment::Center),
                    Line::from(vec![
                        Span::styled("◢ ", Style::default().fg(highlight_color)),
                        Span::styled("PERSONAL MODEL PRINTING", Style::default().fg(secondary_color)),
                        Span::styled(" ◣", Style::default().fg(highlight_color)),
                    ]).alignment(Alignment::Center),
                    Line::from("").alignment(Alignment::Center),
                    Line::from(vec![
                        Span::styled(sculpt.alias.clone(), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                    ]).alignment(Alignment::Center),
                    Line::from("").alignment(Alignment::Center),
                ];
                
                // Add animation
                let frames = split_ascii_art(PRINTER_ACTIVE_FRAMES[animation_frame], Color::Magenta);
                printing_text.extend(frames);
                
                printing_text.extend(vec![
                    Line::from("").alignment(Alignment::Center),
                    Line::from(vec![
                        Span::styled("BLOB ID: ", Style::default().fg(dim_color)),
                        Span::styled(&sculpt.blob_id, Style::default().fg(secondary_color)),
                    ]).alignment(Alignment::Center),
                    Line::from(vec![
                        Span::styled("PRINT COUNT: ", Style::default().fg(dim_color)),
                        Span::styled(sculpt.printed_count.to_string(), Style::default().fg(secondary_color)),
                    ]).alignment(Alignment::Center),
                    Line::from("").alignment(Alignment::Center),
                    Line::from(vec![
                        Span::styled("Press ", Style::default().fg(dim_color)),
                        Span::styled("E", Style::default().fg(highlight_color)),
                        Span::styled(" to stop printing", Style::default().fg(dim_color)),
                    ]).alignment(Alignment::Center),
                    Line::from("").alignment(Alignment::Center),
                    Line::from("").alignment(Alignment::Center),
                ]);
                
                let printing_widget = Paragraph::new(printing_text)
                    .style(Style::default())
                    .alignment(Alignment::Center)
                    .block(task_block);
                    
                f.render_widget(printing_widget, area);
            }
        }
    }
} 