use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::app::App;
use super::status_display::{render_online_active_task, render_offline_printer};
use super::animations::{render_eureka_animation, render_tech_animation, render_ambient_noise};
use super::ascii_arts::UiConstants;
use textwrap;

/// Render the main application UI
pub fn draw_main(f: &mut Frame, app: &mut App) {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Get theme colors
    let (primary_color, secondary_color, accent_color) = if app.is_online {
        (Color::Cyan, Color::LightBlue, Color::DarkGray)  // Tech cool colors
    } else {
        (Color::Magenta, Color::Red, Color::DarkGray)  // Future warm colors
    };

    // Setup color theme
    let base_color = if app.is_online { Color::Cyan } else { Color::Magenta };
    let highlight_color = if app.is_online { Color::LightBlue } else { Color::LightRed };
    let dim_color = if app.is_online { Color::DarkGray } else { Color::DarkGray };
    
    // Full screen border
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(base_color));
    f.render_widget(main_block, f.size());

    // Main layout structure
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(8),   // EUREKA ASCII art
            Constraint::Length(3),   // System status information
            Constraint::Min(3),      // Printer registration information
            Constraint::Length(3),   // Input area
            Constraint::Length(1),   // Bottom padding
            Constraint::Length(3),   // Control information
        ])
        .split(f.size());

    // Add EUREKA ASCII art animation
    render_eureka_animation(f, main_layout[0], highlight_color);

    // System status indicators
    let status_indicators = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(main_layout[1]);
        
    // Display current network
    let network_status = format!("{} NETWORK: {}", UiConstants::NETWORK_PROGRESS, app.network_state.get_current_network().to_uppercase());
    let network_info = Paragraph::new(network_status)
        .style(Style::default().fg(base_color))
        .alignment(Alignment::Center);
    f.render_widget(network_info, status_indicators[0]);
    
    // Build on Sui
    let build_on_sui_text = format!("╔══════╡ {} ╞══════╗", UiConstants::BUILD_ON_SUI.to_uppercase());
    let build_on_sui = Paragraph::new(build_on_sui_text)
        .style(Style::default().fg(base_color))
        .alignment(Alignment::Center);
    f.render_widget(build_on_sui, status_indicators[1]);
    
    // Display wallet address
    let wallet_status = format!("{} WALLET: {}", UiConstants::WALLET_PROGRESS, app.wallet_address);
    let wallet_info = Paragraph::new(wallet_status)
        .style(Style::default().fg(base_color))
        .alignment(Alignment::Center);
    f.render_widget(wallet_info, status_indicators[2]);

    // Split main content area into left and right sections
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),  // Left area
            Constraint::Percentage(60),  // Right area
        ])
        .split(main_layout[2]);

    // Left section layout
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Network status
            Constraint::Length(3),  // Token balance
            Constraint::Length(3),  // Printer ID and wages
            Constraint::Length(3),  // Status
            Constraint::Length(3),  // Tech animation block
            Constraint::Min(0),     // Task history or model list
        ])
        .split(content_layout[0]);

    // Network status display
    render_network_status(f, app, left_chunks[0], primary_color, secondary_color);
    
    // Token balance display
    render_printer_id(f, app, left_chunks[1], primary_color, secondary_color);

    // Printer ID and wages display
    render_balance_and_rewards(f, app, left_chunks[2], primary_color, secondary_color);

    // Status toggle
    render_status_toggle(f, app, left_chunks[3], primary_color, secondary_color);

    // Tech animation block
    render_tech_animation(f, app, left_chunks[4], primary_color);

    // Display different lists based on status
    if app.is_online {
        render_task_list(f, app, left_chunks[5], primary_color, secondary_color, dim_color, accent_color);
    } else {
        render_sculpt_list(f, app, left_chunks[5], primary_color, secondary_color, accent_color);
    }

    // Right area layout
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),   // Message display area increased to 6 lines
            Constraint::Length(21),  // Main print job status
            Constraint::Min(0),      // Bottom area - Print output log
        ])
        .split(content_layout[1]);

    // Message display area
    render_message_area(f, app, right_chunks[0], primary_color);

    // Main print job status area
    if app.is_online {
        render_online_active_task(f, app, right_chunks[1], time);
    } else {
        render_offline_printer(f, app, right_chunks[1], time);
    }

    // Print output log area
    render_print_output(f, app, right_chunks[2], primary_color, secondary_color);

    // Bottom area
    let bottom_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(primary_color));

    f.render_widget(bottom_block, right_chunks[2]);

    // Controls at the bottom
    render_help_controls(f, app, main_layout[5], dim_color, highlight_color);
    
    // Simulate spaceship ambient noise effect
    render_ambient_noise(f, time, dim_color);
}

// Helper functions for rendering different components

fn render_network_status(f: &mut Frame, app: &App, area: Rect, primary_color: Color, secondary_color: Color) {
    let network_block = Block::default()
        .title(if app.is_switching_network {
            "SELECT NETWORK (1-3)"
        } else {
            "CURRENT NETWORK"
        })
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(primary_color));
    
    let network_text = if app.is_switching_network {
        app.get_network_options()
    } else {
        format!("{}  [Press N to switch]", app.network_state.get_current_network().to_uppercase())
    };
    
    let network_paragraph = Paragraph::new(network_text)
        .block(network_block)
        .style(Style::default().fg(if app.is_switching_network { Color::Yellow } else { secondary_color }))
        .alignment(Alignment::Left);
    
    f.render_widget(network_paragraph, area);
}

fn render_printer_id(f: &mut Frame, app: &App, area: Rect, primary_color: Color, secondary_color: Color) {
    let printer_block = Block::default()
        .title("PRINTER ID")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(primary_color));
    
    let printer_text = Paragraph::new(app.printer_id.clone())
        .block(printer_block)
        .style(Style::default().fg(secondary_color))
        .alignment(Alignment::Left);
    
    f.render_widget(printer_text, area);
}

fn render_balance_and_rewards(f: &mut Frame, app: &App, area: Rect, primary_color: Color, secondary_color: Color) {
    let balance_reward_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // SUI balance display
    let sui_block = Block::default()
        .title("SUI BALANCE")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(primary_color));
    
    let sui_text = Paragraph::new(format!("{:.2} SUI", app.sui_balance as f64 / 1_000_000_000.0))
        .block(sui_block)
        .style(Style::default().fg(secondary_color))
        .alignment(Alignment::Left);
    
    f.render_widget(sui_text, balance_reward_chunks[0]);

    // Harvestable rewards
    let reward_block = Block::default()
        .title("REWARDS")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(primary_color));
    
    let reward_text = if app.is_harvesting {
        "Harvest? (Y/N)".to_string()
    } else {
        app.harvestable_rewards.clone()
    };
    
    let reward_style = if app.is_harvesting {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(secondary_color)
    };
    
    let reward = Paragraph::new(reward_text)
        .block(reward_block)
        .style(reward_style);
    
    f.render_widget(reward, balance_reward_chunks[1]);
}

fn render_status_toggle(f: &mut Frame, app: &App, area: Rect, primary_color: Color, secondary_color: Color) {
    let status_block = Block::default()
        .title("STATUS")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(primary_color));
    
    let (status_text, status_style) = if app.is_confirming {
        let target_status = if app.is_online { "OFFLINE" } else { "ONLINE" };
        (
            format!("Switch to {}? (Y/N)", target_status),
            Style::default().fg(Color::Yellow)
        )
    } else {
        (
            if app.is_online {
                "● ONLINE [Press O to toggle]".to_string()
            } else {
                "○ OFFLINE [Press O to toggle]".to_string()
            },
            Style::default().fg(secondary_color)
        )
    };
    
    let status = Paragraph::new(status_text)
        .style(status_style)
        .block(status_block);
    
    f.render_widget(status, area);
}

fn render_task_list(
    f: &mut Frame, 
    app: &mut App, 
    area: Rect, 
    primary_color: Color, 
    secondary_color: Color,
    dim_color: Color,
    _accent_color: Color
) {
    // Online mode task list
    let completed_tasks: Vec<ListItem> = app.tasks
        .iter()
        .filter(|task| task.is_completed())
        .map(|task| {
            ListItem::new(Line::from(vec![
                Span::styled("[", Style::default().fg(dim_color)),
                Span::styled(task.format_end_time(), Style::default().fg(Color::Cyan)),
                Span::styled("] ", Style::default().fg(dim_color)),
                Span::styled(task.name.clone(), Style::default().fg(secondary_color)),
                Span::styled(" - ", Style::default().fg(dim_color)),
                Span::styled(
                    task.format_paid_amount(),
                    Style::default().fg(Color::Green),
                ),
            ]))
        })
        .collect();

    let tasks_list = List::new(completed_tasks)
        .block(Block::default()
            .title("TASKS HISTORY")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(primary_color)))
        .highlight_style(Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(secondary_color));
            
    f.render_stateful_widget(tasks_list, area, &mut app.tasks_state);
}

fn render_sculpt_list(
    f: &mut Frame, 
    app: &mut App, 
    area: Rect, 
    primary_color: Color, 
    secondary_color: Color,
    accent_color: Color
) {
    // Offline mode sculpt list
    let sculpt_items: Vec<ListItem> = app.sculpt_items
        .iter()
        .map(|item| {
            ListItem::new(format!("◈ {} [Printed: {}]", item.alias, item.printed_count))
                .style(Style::default().fg(accent_color))
        })
        .collect();
        
    let sculpt_list = List::new(sculpt_items)
        .block(Block::default()
            .title(" MY SCULPTS ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(primary_color)))
        .highlight_style(Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(secondary_color))
        .highlight_symbol(">> ");
            
    f.render_stateful_widget(sculpt_list, area, &mut app.sculpt_state);
}

fn render_message_area(f: &mut Frame, app: &App, area: Rect, primary_color: Color) {
    let message_block = Block::default()
        .title(" MESSAGE ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(primary_color));

    if let Some(error) = &app.error_message {
        // Calculate available width (minus borders and margins)
        let available_width = area.width.saturating_sub(4);
        let wrapped_text = textwrap::wrap(error, available_width as usize)
            .join("\n");
        
        let message_text = Paragraph::new(wrapped_text)
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Left)
            .block(message_block);
            
        f.render_widget(message_text, area);
    } else if let Some(success) = &app.success_message {
        // Calculate available width (minus borders and margins)
        let available_width = area.width.saturating_sub(4);
        let wrapped_text = textwrap::wrap(success, available_width as usize)
            .join("\n");
        
        let message_text = Paragraph::new(wrapped_text)
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Left)
            .block(message_block);
            
        f.render_widget(message_text, area);
    } else {
        // Just show the border when no message
        f.render_widget(message_block, area);
    }
}

fn render_help_controls(f: &mut Frame, app: &App, area: Rect, dim_color: Color, highlight_color: Color) {
    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(dim_color));
    
    let help_text = if app.is_confirming {
        vec![
            Line::from(vec![
                Span::styled("Y", Style::default().fg(Color::Yellow)),
                Span::raw(": Confirm"),
                Span::raw("  |  "),
                Span::styled("N", Style::default().fg(Color::Yellow)),
                Span::raw(": Cancel"),
            ]),
        ]
    } else if app.is_harvesting {
        vec![
            Line::from(vec![
                Span::styled("Y", Style::default().fg(Color::Yellow)),
                Span::raw(": Confirm"),
                Span::raw("  |  "),
                Span::styled("N", Style::default().fg(Color::Yellow)),
                Span::raw(": Cancel"),
            ]),
        ]
    } else if app.is_switching_network {
        vec![
            Line::from(vec![
                Span::styled("1", Style::default().fg(Color::Yellow)),
                Span::raw(": MAINNET"),
                Span::raw("  |  "),
                Span::styled("2", Style::default().fg(Color::Yellow)),
                Span::raw(": DEVNET"),
                Span::raw("  |  "),
                Span::styled("3", Style::default().fg(Color::Yellow)),
                Span::raw(": TESTNET"),
            ]),
            Line::from(vec![
                Span::styled("N", Style::default().fg(Color::Yellow)),
                Span::raw(": Cancel"),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::styled("Q", Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)),
                Span::raw(" QUIT"),
                Span::raw("   "),
                Span::styled("O", Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)),
                Span::raw(" TOGGLE STATUS"),
                Span::raw("   "),
                Span::styled("H", Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)),
                Span::raw(" HARVEST REWARDS"),
                Span::raw("   "),
                Span::styled("P", Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)),
                Span::raw(" 3D Print"),
                Span::raw("   "),
                Span::styled("E", Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)),
                Span::raw(" Stop Printing"),
                Span::raw("   "),
                // FIXME: test only
                Span::styled("C", Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)),
                Span::raw(" Create Job"),
                Span::raw("   "),
                // FIXME: test only
                Span::styled("T", Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)),
                Span::raw(" Test Job"),
                Span::raw("   "),
                // FIXME: test only
                Span::styled("F", Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)),
                Span::raw(" Finish Job"),
            ]),
        ]
    };
    
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(dim_color))
        .alignment(Alignment::Center)
        .block(help_block);
        
    f.render_widget(help, area);
}

/// Renders the print output log
fn render_print_output(f: &mut Frame, app: &App, area: Rect, primary_color: Color, secondary_color: Color) {
    let output_block = Block::default()
        .title(" PRINT OUTPUT LOG ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(primary_color));

    // Convert print output to list items with appropriate styling
    let items: Vec<ListItem> = app.print_output
        .iter()
        .map(|line| {
            let (style, prefix) = if line.contains("[STDERR]") {
                (Style::default().fg(Color::Red), "")
            } else if line.contains("[STDOUT]") {
                (Style::default().fg(secondary_color), "")
            } else {
                (Style::default().fg(Color::DarkGray), "│ ")
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(line, style),
            ]))
        })
        .collect();

    let output_list = List::new(items)
        .block(output_block)
        .style(Style::default());

    f.render_widget(output_list, area);
} 