use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::app::{App, RegistrationStatus, MessageType};
use crate::constants::{EUREKA_FRAMES, BUILD_ON_SUI};
use textwrap;

pub fn draw(f: &mut Frame, app: &mut App) {
    if app.is_registering_printer {
        draw_registration(f, app);
    } else {
        draw_main(f, app);
    }
}

fn draw_registration(f: &mut Frame, app: &mut App) {

    // 設置黑底色彩
    let base_color = if app.is_online { Color::Cyan } else { Color::Magenta };
    let highlight_color = if app.is_online { Color::LightBlue } else { Color::LightRed };
    let dim_color = if app.is_online { Color::DarkGray } else { Color::DarkGray };
    
    // 全屏佈局
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(base_color));
    f.render_widget(main_block, f.size());
    
    // 全屏佈局
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(8),   // EUREKA ASCII藝術
            Constraint::Length(3),   // 太空船狀態信息
            Constraint::Min(3),      // 打印機註冊信息
            Constraint::Length(3),   // 輸入區域
            Constraint::Length(1),   // 底部留白
            Constraint::Length(3),   // 控制項信息
        ])
        .split(f.size());
    
    // 添加 EUREKA ASCII藝術
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let animation_frame = (time % 3) as usize;
    let ascii_art = Paragraph::new(EUREKA_FRAMES[animation_frame])
        .style(Style::default().fg(highlight_color))
        .alignment(Alignment::Center);
    f.render_widget(ascii_art, main_layout[0]);
    
    // 添加太空船狀態信息
    let status_indicators = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(main_layout[1]);
        
    // 顯示當前網路環境
    let network_status = format!("[■■■■■□□□□□] NETWORK: {}", app.network_state.get_current_network().to_uppercase());
    let network_info = Paragraph::new(network_status)
        .style(Style::default().fg(base_color))
        .alignment(Alignment::Center);
    f.render_widget(network_info, status_indicators[0]);
    
    // 構建於Sui
    let build_on_sui_text = format!("╔══════╡ {} ╞══════╗", BUILD_ON_SUI.to_uppercase());
    let build_on_sui = Paragraph::new(build_on_sui_text)
        .style(Style::default().fg(base_color))
        .alignment(Alignment::Center);
    f.render_widget(build_on_sui, status_indicators[1]);
    
    // 顯示錢包地址
    let wallet_status = format!("[■■■■■■■□□□] WALLET: {}", app.wallet_address);
    let wallet_info = Paragraph::new(wallet_status)
        .style(Style::default().fg(base_color))
        .alignment(Alignment::Center);
    f.render_widget(wallet_info, status_indicators[2]);
    
    // 添加註冊信息區
    let registration_block = Block::default()
        .title(" << SYSTEM STATUS >> ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(base_color));
    
    // 根據不同的註冊階段顯示不同的信息
    let mut registration_text = vec![
        Line::from(vec![
            Span::styled(">> ", Style::default().fg(highlight_color)),
            Span::styled("SYSTEM STATUS", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
    ];

    // 添加狀態信息
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

    // 顯示錯誤消息
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
    
    // 輸入區域
    let input_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),   // 提示文字
            Constraint::Min(3),      // 輸入框（增加最小高度）
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
    
    // 只在輸入狀態下顯示輸入框
    if matches!(app.registration_status, RegistrationStatus::Inputting) {
        // 輸入框
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
    
    // 控制項信息
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
    
    // 模擬太空船環境噪音效果 - 在不同位置添加隨機"雜訊"
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

fn draw_main(f: &mut Frame, app: &mut App) {
    // 獲取當前主題顏色
    let (primary_color, secondary_color, accent_color) = if app.is_online {
        (Color::Cyan, Color::LightBlue, Color::DarkGray)  // 科技感冷色調
    } else {
        (Color::Magenta, Color::Red, Color::DarkGray)  // 未來感暖色調
    };

    // 設置黑底色彩
    let base_color = if app.is_online { Color::Cyan } else { Color::Magenta };
    let highlight_color = if app.is_online { Color::LightBlue } else { Color::LightRed };
    let dim_color = if app.is_online { Color::DarkGray } else { Color::DarkGray };
    
    // 全屏佈局
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(base_color));
    f.render_widget(main_block, f.size());

    // 全屏佈局
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(8),   // EUREKA ASCII藝術
            Constraint::Length(3),   // 太空船狀態信息
            Constraint::Min(3),      // 打印機註冊信息
            Constraint::Length(3),   // 輸入區域
            Constraint::Length(1),   // 底部留白
            Constraint::Length(3),   // 控制項信息
        ])
        .split(f.size());

    // 添加 EUREKA ASCII藝術
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let animation_frame = (time % 3) as usize;
    let ascii_art = Paragraph::new(EUREKA_FRAMES[animation_frame])
        .style(Style::default().fg(highlight_color))
        .alignment(Alignment::Center);
    f.render_widget(ascii_art, main_layout[0]);

    // 添加太空船狀態信息
    let status_indicators = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(main_layout[1]);
        
    // 顯示當前網路環境
    let network_status = format!("[■■■■■□□□□□] NETWORK: {}", app.network_state.get_current_network().to_uppercase());
    let network_info = Paragraph::new(network_status)
        .style(Style::default().fg(base_color))
        .alignment(Alignment::Center);
    f.render_widget(network_info, status_indicators[0]);
    
    // 構建於Sui
    let build_on_sui_text = format!("╔══════╡ {} ╞══════╗", BUILD_ON_SUI.to_uppercase());
    let build_on_sui = Paragraph::new(build_on_sui_text)
        .style(Style::default().fg(base_color))
        .alignment(Alignment::Center);
    f.render_widget(build_on_sui, status_indicators[1]);
    
    // 顯示錢包地址
    let wallet_status = format!("[■■■■■■■□□□] WALLET: {}", app.wallet_address);
    let wallet_info = Paragraph::new(wallet_status)
        .style(Style::default().fg(base_color))
        .alignment(Alignment::Center);
    f.render_widget(wallet_info, status_indicators[2]);

    // 將主要內容區域分為左右兩部分
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),  // 左側區域
            Constraint::Percentage(60),  // 右側區域
        ])
        .split(main_layout[2]);

    // 左側佈局結構
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 網絡狀態
            Constraint::Length(3),  // 代幣餘額
            Constraint::Length(3),  // 印表機 ID 和工資
            Constraint::Length(3),  // 狀態
            Constraint::Length(3),  // 科技動畫區塊
            Constraint::Min(0),     // 任務歷史或模型列表
        ])
        .split(content_layout[0]);

    // 網絡狀態顯示
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
    f.render_widget(network_paragraph, left_chunks[0]);

    // 代幣餘額顯示
    let printer_block = Block::default()
        .title("PRINTER ID")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(primary_color));
    let printer_text = Paragraph::new(app.printer_id.clone())
        .block(printer_block)
        .style(Style::default().fg(secondary_color))
        .alignment(Alignment::Left);
    f.render_widget(printer_text, left_chunks[1]);

    // 印表機 ID 和工資顯示
    let printer_reward_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(left_chunks[2]);

    // SUI 餘額顯示
    let sui_block = Block::default()
        .title("SUI BALANCE")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(primary_color));
    let sui_text = Paragraph::new(format!("{:.2} SUI", app.sui_balance as f64 / 1_000_000_000.0))
        .block(sui_block)
        .style(Style::default().fg(secondary_color))
        .alignment(Alignment::Left);
    f.render_widget(sui_text, printer_reward_chunks[0]);

    // 可收穫工資
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
    f.render_widget(reward, printer_reward_chunks[1]);

    // 在線狀態切換
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
    f.render_widget(status, left_chunks[3]);

    // 添加科技動畫區塊
    let tech_block = Block::default()
        .title("SYSTEM")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(primary_color));
    
    let tech_text = Paragraph::new(app.get_tech_animation())
        .style(Style::default().fg(if app.is_online { Color::Cyan } else { Color::Magenta }))
        .alignment(Alignment::Center)
        .block(tech_block);
    
    f.render_widget(tech_text, left_chunks[4]);

    // 根據狀態顯示不同的列表
    if app.is_online {
        // 在線狀態顯示任務列表
        let completed_tasks: Vec<ListItem> = app.tasks
            .iter()
            .filter(|task| matches!(task.status, crate::app::TaskStatus::Completed))
            .map(|task| {
                let status_text = match task.status {
                    crate::app::TaskStatus::Printing(progress) => format!("[{}%] {}", progress, task.name),
                    crate::app::TaskStatus::Completed => format!("✓ {}", task.name),
                };
                ListItem::new(format!("{} - {}", task.id, status_text))
                    .style(Style::default().fg(accent_color))
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
        f.render_stateful_widget(tasks_list, left_chunks[5], &mut app.tasks_state);
    } else {
        // 離線狀態顯示 sculpt 列表
        let sculpt_items: Vec<ListItem> = app.sculpt_items
            .iter()
            .map(|item| {
                ListItem::new(format!("◈ {} [Printed: {}]", item.alias, item.printed_count))
                    .style(Style::default().fg(accent_color))
            })
            .collect();
        let sculpt_list = List::new(sculpt_items)
            .block(Block::default()
                .title("3D MODELS")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(primary_color)))
            .highlight_style(Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(secondary_color))
            .highlight_symbol(">> ");
        f.render_stateful_widget(sculpt_list, left_chunks[5], &mut app.sculpt_state);
    }

    // 右側內容區
    let right_block = Block::default()
        .title("CONTENT")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(primary_color));

    let right_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 錯誤訊息區域
            Constraint::Length(3),  // 機器狀態區域
            Constraint::Min(0),     // 其他內容
        ])
        .split(content_layout[1]);

    f.render_widget(right_block, content_layout[1]);
    //右側content內容動畫
    let content_block = Block::default().title("Status").borders(Borders::ALL);
    let content = Paragraph::new(app.content.as_str())
        .block(content_block)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(base_color));
    f.render_widget(content, right_chunks[1]);

    // 錯誤訊息區域
    if let Some(error) = &app.error_message {
        let (style, title) = match app.message_type {
            MessageType::Error => (
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                "ERROR"
            ),
            MessageType::Info => (
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                "INFO"
            ),
            MessageType::Success => (
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                "SUCCESS"
            ),
        };

        let message_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(style);

        // 計算可用寬度（減去邊框和邊距）
        let available_width = right_area[0].width.saturating_sub(4);
        
        // 將錯誤訊息按可用寬度換行
        let wrapped_text = textwrap::wrap(error, available_width as usize)
            .into_iter()
            .map(|line| Line::from(Span::styled(line, style)))
            .collect::<Vec<_>>();

        // 根據換行後的內容調整高度
        let message_height = wrapped_text.len() as u16;
        let message_area = Rect::new(
            right_area[0].x,
            right_area[0].y,
            right_area[0].width,
            message_height + 2, // 加上邊框的高度
        );

        f.render_widget(
            Paragraph::new(wrapped_text)
                .block(message_block)
                .alignment(Alignment::Left),
            message_area,
        );
    }

    // 成功訊息區域
    if let Some(success) = &app.success_message {
        let message_block = Block::default()
            .title("SUCCESS")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Green));

        // 計算可用寬度（減去邊框和邊距）
        let available_width = right_area[0].width.saturating_sub(4);
        
        // 將成功訊息按可用寬度換行
        let wrapped_text = textwrap::wrap(success, available_width as usize)
            .into_iter()
            .map(|line| Line::from(Span::styled(
                line,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )))
            .collect::<Vec<_>>();

        // 根據換行後的內容調整高度
        let message_height = wrapped_text.len() as u16;
        let message_area = Rect::new(
            right_area[0].x,
            right_area[0].y,
            right_area[0].width,
            message_height + 2, // 加上邊框的高度
        );

        f.render_widget(
            Paragraph::new(wrapped_text)
                .block(message_block)
                .alignment(Alignment::Left),
            message_area,
        );
    }

    // 底部控制項
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
    
    // Simulate spaceship ambient noise effect - add random "noise" at different positions
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
