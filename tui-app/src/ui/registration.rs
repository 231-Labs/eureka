use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::app::{App, RegistrationStatus};
use super::ascii_arts::{EUREKA_FRAMES, UiConstants};

/// Renders the printer registration UI
pub fn draw_registration(f: &mut Frame, app: &mut App) {
    // Setup color theme
    let base_color = if app.is_online { Color::Cyan } else { Color::Magenta };
    let highlight_color = if app.is_online { Color::LightBlue } else { Color::LightRed };
    let dim_color = if app.is_online { Color::DarkGray } else { Color::DarkGray };
    
    // Full screen layout
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
            Constraint::Length(3),   // System status indicators
            Constraint::Min(3),      // Printer registration info
            Constraint::Length(3),   // Input area
            Constraint::Length(1),   // Bottom padding
            Constraint::Length(3),   // Control information
        ])
        .split(f.size());
    
    // Add EUREKA ASCII art animation
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let animation_frame = (time % 3) as usize;
    let ascii_art = Paragraph::new(EUREKA_FRAMES[animation_frame])
        .style(Style::default().fg(highlight_color))
        .alignment(Alignment::Center);
    f.render_widget(ascii_art, main_layout[0]);
    
    // System status indicators
    let status_indicators = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(main_layout[1]);
        
    // Display current network environment
    let network_status = format!("[■■■■■□□□□□] NETWORK: {}", app.network_state.get_current_network().to_uppercase());
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
    let wallet_status = format!("[■■■■■■■□□□] WALLET: {}", app.wallet_address);
    let wallet_info = Paragraph::new(wallet_status)
        .style(Style::default().fg(base_color))
        .alignment(Alignment::Center);
    f.render_widget(wallet_info, status_indicators[2]);
    
    // Registration information area
    let registration_block = Block::default()
        .title(" << SYSTEM STATUS >> ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(base_color));
    
    // Different info based on registration stage
    let mut registration_text = vec![
        Line::from(vec![
            Span::styled(">> ", Style::default().fg(highlight_color)),
            Span::styled("SYSTEM STATUS", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
    ];

    // Add status information
    match &app.registration_status {
        RegistrationStatus::Inputting => {
            registration_text.extend(vec![
                Line::from(vec![
                    Span::styled(">> ", Style::default().fg(highlight_color)),
                    Span::raw("3D Printer Not Found"),
                ]),
                Line::from(vec![
                    Span::styled(">> ", Style::default().fg(highlight_color)),
                    Span::raw("Establish connection by registering your printer"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("STATUS: ", Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)),
                    Span::raw("Please enter your printer alias below."),
                ]),
            ]);
        }
        RegistrationStatus::Submitting => {
            registration_text.extend(vec![
                Line::from(vec![
                    Span::styled(">> ", Style::default().fg(highlight_color)),
                    Span::raw("Processing Registration"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("STATUS: ", Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)),
                    Span::styled("⟳ ", Style::default().fg(Color::Yellow)),
                    Span::raw("Sending transaction to network..."),
                ]),
                Line::from(vec![
                    Span::raw("Please wait while we process your registration."),
                ]),
            ]);
        }
        RegistrationStatus::Success(tx_id) => {
            registration_text.extend(vec![
                Line::from(vec![
                    Span::styled(">> ", Style::default().fg(highlight_color)),
                    Span::styled("✓ ", Style::default().fg(Color::Green)),
                    Span::styled("Registration Successful!", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Printer Name: ", Style::default().fg(highlight_color)),
                    Span::raw(&app.printer_alias),
                ]),
                Line::from(vec![
                    Span::styled("Transaction: ", Style::default().fg(highlight_color)),
                    Span::raw(tx_id),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(">> ", Style::default().fg(highlight_color)),
                    Span::styled("Press any key to continue...", Style::default().fg(Color::Yellow)),
                ]),
            ]);
        }
        RegistrationStatus::Failed(_) => {
            registration_text.extend(vec![
                Line::from(vec![
                    Span::styled(">> ", Style::default().fg(highlight_color)),
                    Span::styled("✗ ", Style::default().fg(Color::Red)),
                    Span::styled("Registration Failed", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                ]),
            ]);
        }
    };

    // Display error message
    if let Some(error) = &app.error_message {
        registration_text.push(Line::from(""));
        registration_text.push(Line::from(vec![
            Span::styled("ERROR: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(error, Style::default().fg(Color::Red)),
        ]));
    }
    
    let registration_para = Paragraph::new(registration_text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .block(registration_block);
    f.render_widget(registration_para, main_layout[2]);
    
    // Input area
    let input_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),   // Prompt text
            Constraint::Min(3),      // Input box (increased min height)
        ])
        .split(main_layout[3]);
    
    let input_prompt = if matches!(app.registration_status, RegistrationStatus::Inputting) {
        Paragraph::new("ENTER PRINTER ALIAS:")
            .style(Style::default().fg(highlight_color))
            .alignment(Alignment::Left)
    } else {
        Paragraph::new("")
            .style(Style::default().fg(highlight_color))
            .alignment(Alignment::Left)
    };
    f.render_widget(input_prompt, input_area[0]);
    
    // Only show input box in input state
    if matches!(app.registration_status, RegistrationStatus::Inputting) {
        // Input box
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(base_color));
        
        let blink_cursor = time % 2 == 0;
        let cursor = if blink_cursor { "█" } else { " " };
        
        let input_text = format!("{}{}",
            app.printer_alias,
            if app.registration_status == RegistrationStatus::Inputting && app.printer_alias.len() < 30 { cursor } else { "" }
        );
        
        let input = Paragraph::new(input_text)
            .style(Style::default().fg(Color::White))
            .block(input_block);
        f.render_widget(input, input_area[1]);
    }
    
    // Control items
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
            ]),
        ]
    };
    
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(dim_color))
        .alignment(Alignment::Center)
        .block(help_block);
    f.render_widget(help, main_layout[5]);
    
    // Simulate spaceship ambient noise effect
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
            f.render_widget(noise, ratatui::layout::Rect::new(x as u16, y as u16, 1, 1));
        }
    }
} 