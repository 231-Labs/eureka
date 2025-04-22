use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::app::App;
use crate::constants::{EUREKA_FRAMES, BUILD_ON_SUI};

pub fn draw(f: &mut Frame, app: &mut App) {
    if app.is_registering_printer {
        draw_printer_registration(f, app);
        return;
    }

    // 將整個畫面分為上下兩個部分
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),      // 主要內容區域
            Constraint::Length(3),   // 控制項區域
        ])
        .split(f.size());

    // 將主要內容區域分為左右兩部分
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),  // 左側區域
            Constraint::Percentage(60),  // 右側區域
        ])
        .split(main_layout[0]);

    // 獲取當前主題顏色
    let (primary_color, secondary_color, accent_color) = if app.is_online {
        (Color::Cyan, Color::LightBlue, Color::DarkGray)  // 科技感冷色調
    } else {
        (Color::Magenta, Color::Red, Color::DarkGray)  // 未來感暖色調
    };

    // 左側佈局結構
    let left_chunks = if app.is_online {
        // 在線模式需要顯示當前打印和歷史任務
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // 錢包地址
                Constraint::Length(3),  // 網絡狀態
                Constraint::Length(3),  // 代幣餘額
                Constraint::Length(3),  // 印表機 ID 和工資
                Constraint::Length(3),  // 狀態
                Constraint::Length(3),  // 科技動畫區塊
                Constraint::Length(3),  // 當前打印任務
                Constraint::Min(0),     // 任務歷史（填充剩餘空間）
            ])
            .split(content_layout[0])
    } else {
        // 離線模式只需要顯示3D模型列表
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // 錢包地址
                Constraint::Length(3),  // 網絡狀態
                Constraint::Length(3),  // 代幣餘額
                Constraint::Length(3),  // 印表機 ID 和工資
                Constraint::Length(3),  // 狀態
                Constraint::Length(3),  // 科技動畫區塊
                Constraint::Min(0),     // 3D模型列表（填充剩餘空間）
            ])
            .split(content_layout[0])
    };

    // 錢包地址顯示
    let wallet_block = Block::default()
        .title("WALLET ADDRESS")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color));
    let wallet_text = Paragraph::new(app.wallet_address.clone())
        .block(wallet_block)
        .style(Style::default().fg(secondary_color))
        .alignment(Alignment::Left);
    f.render_widget(wallet_text, left_chunks[0]);

    // 網絡狀態顯示
    let network_block = Block::default()
        .title(if app.is_switching_network {
            "SELECT NETWORK (1-3)"
        } else {
            "CURRENT NETWORK"
        })
        .borders(Borders::ALL)
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
    f.render_widget(network_paragraph, left_chunks[1]);

    // 代幣餘額顯示
    let balance_chunks = Layout::default()
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
        .border_style(Style::default().fg(primary_color));
    let sui_text = Paragraph::new(format!("{:.2} SUI", app.sui_balance as f64 / 1_000_000_000.0))
        .block(sui_block)
        .style(Style::default().fg(secondary_color))
        .alignment(Alignment::Left);
    f.render_widget(sui_text, balance_chunks[0]);

    // WAL 餘額顯示
    let wal_block = Block::default()
        .title("WAL BALANCE")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color));
    let wal_text = Paragraph::new(format!("{:.2} WAL", app.wal_balance as f64 / 1_000_000_000.0))
        .block(wal_block)
        .style(Style::default().fg(secondary_color))
        .alignment(Alignment::Left);
    f.render_widget(wal_text, balance_chunks[1]);

    // 印表機 ID 和工資顯示
    let printer_reward_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(left_chunks[3]);

    // 印表機 ID
    let printer_block = Block::default()
        .title("PRINTER ID")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color));
    let printer_text = Paragraph::new(app.printer_id.clone())
        .block(printer_block)
        .style(Style::default().fg(secondary_color))
        .alignment(Alignment::Left);
    f.render_widget(printer_text, printer_reward_chunks[0]);

    // 可收穫工資
    let reward_block = Block::default()
        .title("REWARDS")
        .borders(Borders::ALL)
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
    f.render_widget(status, left_chunks[4]);

    // 添加科技動畫區塊
    let tech_block = Block::default()
        .title("SYSTEM")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color));
    
    // 根據當前時間生成不同的科技動畫效果
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let animation_frame = (time % 8) as usize;
    
    // 不同的科技動畫幀
    let tech_animations = vec![
        "║▓▒░ SYS ACTIVE ░▒▓║",
        "║▒▓░ SCANNING... ░▓▒║",
        "║▓▒░ DATA FLOW ░▒▓║",
        "║▒▓░ COMPUTING ░▓▒║",
        "║▓▒░ ANALYZING ░▒▓║",
        "║▒▓░ CONNECTING ░▓▒║",
        "║▓▒░ PROCESSING ░▒▓║",
        "║▒▓░ SYNCING... ░▓▒║",
    ];
    
    let tech_text = Paragraph::new(tech_animations[animation_frame])
        .style(Style::default().fg(if app.is_online { Color::Cyan } else { Color::Magenta }))
        .alignment(Alignment::Center)
        .block(tech_block);
    
    f.render_widget(tech_text, left_chunks[5]);

    // 根據狀態顯示不同的列表
    if app.is_online {
        // 在線狀態顯示任務列表

        // 顯示當前打印任務
        let current_task = app.tasks
            .iter()
            .find(|task| matches!(task.status, crate::app::TaskStatus::Printing(_)))
            .map(|task| {
                if let crate::app::TaskStatus::Printing(progress) = task.status {
                    format!("▶ {} - {} [{}%]", task.id, task.name, progress)
                } else {
                    unreachable!()
                }
            })
            .unwrap_or_else(|| "◇ No active print job".to_string());

        let current_task_block = Block::default()
            .title("NOW PRINTING")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(primary_color));
        let current_task_text = Paragraph::new(current_task)
            .style(Style::default().fg(secondary_color))
            .block(current_task_block);
        f.render_widget(current_task_text, left_chunks[6]);

        // 顯示已完成任務
        let completed_tasks: Vec<ListItem> = app.tasks
            .iter()
            .filter(|task| matches!(task.status, crate::app::TaskStatus::Completed))
            .map(|task| {
                ListItem::new(format!("✓ {} - {}", task.id, task.name))
                    .style(Style::default().fg(accent_color))
            })
            .collect();

        let tasks_list = List::new(completed_tasks)
            .block(Block::default()
                .title("TASKS HISTORY")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(primary_color)))
            .highlight_style(Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(secondary_color));
        f.render_stateful_widget(tasks_list, left_chunks[7], &mut app.tasks_state);
    } else {
        // 離線狀態顯示資產列表
        let assets: Vec<ListItem> = app.assets
            .iter()
            .map(|asset| {
                ListItem::new(format!("◈ {}", asset))
                    .style(Style::default().fg(accent_color))
            })
            .collect();
        let assets_list = List::new(assets)
            .block(Block::default()
                .title("3D MODELS")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(primary_color)))
            .highlight_style(Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(secondary_color));
        f.render_stateful_widget(assets_list, left_chunks[6], &mut app.assets_state);
    }

    // 底部控制項
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
                Span::styled("Q", Style::default().fg(secondary_color)),
                Span::raw(" QUIT"),
                Span::raw("  |  "),
                Span::styled("O", Style::default().fg(secondary_color)),
                Span::raw(" TOGGLE STATUS"),
                Span::raw("  |  "),
                Span::styled("H", Style::default().fg(secondary_color)),
                Span::raw(" HARVEST REWARDS"),
            ]),
        ]
    };
    let help = Paragraph::new(help_text)
        .block(Block::default()
            .title("CONTROLS")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(primary_color)))
        .style(Style::default().fg(accent_color));
    f.render_widget(help, main_layout[1]);

    // 右側內容區
    let right_block = Block::default()
        .title("CONTENT")
        .borders(Borders::ALL)
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

    // 錯誤訊息區域
    if let Some(error) = &app.error_message {
        let error_block = Block::default()
            .title("ERROR")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));

        let error_text = Paragraph::new(error.clone())
            .style(Style::default().fg(Color::Red))
            .block(error_block)
            .alignment(Alignment::Left);

        f.render_widget(error_text, right_area[0]);
    }

    // 機器狀態區域
    let status_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(right_area[1]);

    // 噴嘴溫度
    let nozzle_block = Block::default()
        .title("NOZZLE TEMP")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(secondary_color));
    
    let nozzle_text = Paragraph::new(format!("{:.1}°C", app.nozzle_temp))
        .style(Style::default().fg(if app.nozzle_temp > 50.0 { Color::Red } else { secondary_color }))
        .alignment(Alignment::Center)
        .block(nozzle_block);

    // 加熱板溫度
    let bed_block = Block::default()
        .title("BED TEMP")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(secondary_color));
    
    let bed_text = Paragraph::new(format!("{:.1}°C", app.bed_temp))
        .style(Style::default().fg(if app.bed_temp > 50.0 { Color::Red } else { secondary_color }))
        .alignment(Alignment::Center)
        .block(bed_block);

    f.render_widget(nozzle_text, status_layout[0]);
    f.render_widget(bed_text, status_layout[1]);
}

fn draw_printer_registration(f: &mut Frame, app: &App) {
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
    
    // 主佈局區域
    let main_area = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(8),  // EUREKA ASCII藝術
            Constraint::Length(3),  // 太空船狀態信息
            Constraint::Length(4),  // 打印機註冊信息
            Constraint::Min(2),     // 輸入區域
            Constraint::Length(1),  // 底部留白
            Constraint::Length(3),  // 控制項信息
        ])
        .split(f.size());
    
    // 根據當前時間選擇動畫框架
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let animation_frame = (time % 3) as usize;
    
    // 添加 EUREKA ASCII藝術
    let ascii_art = Paragraph::new(EUREKA_FRAMES[animation_frame])
        .style(Style::default().fg(highlight_color))
        .alignment(Alignment::Center);
    f.render_widget(ascii_art, main_area[0]);
    
    // 添加太空船狀態信息
    let status_indicators = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(main_area[1]);
        
    // 生命支持系統
    let life_support = Paragraph::new("[■■■■■□□□□□] LIFE SUPPORT: NOMINAL")
        .style(Style::default().fg(base_color))
        .alignment(Alignment::Center);
    f.render_widget(life_support, status_indicators[0]);
    
    // 導航系統
    let navigation = Paragraph::new("[■■■■■■■□□□] NAVIGATION: STANDBY")
        .style(Style::default().fg(base_color))
        .alignment(Alignment::Center);
    f.render_widget(navigation, status_indicators[1]);
    
    // 構建於Sui (替換原來的通訊系統)
    let build_on_sui_text = format!("[■■■■■■■■□□] {} ", BUILD_ON_SUI.to_uppercase());
    let build_on_sui = Paragraph::new(build_on_sui_text)
        .style(Style::default().fg(base_color))
        .alignment(Alignment::Center);
    f.render_widget(build_on_sui, status_indicators[2]);
    
    // 添加註冊信息
    let registration_block = Block::default()
        .title(" << PRINTER REGISTRATION >> ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(base_color));
    
    let registration_text = vec![
        Line::from(vec![
            Span::styled(">> ", Style::default().fg(highlight_color)),
            Span::styled("SYSTEM ALERT:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" 3D Printer Not Found"),
        ]),
        Line::from(vec![
            Span::styled(">> ", Style::default().fg(highlight_color)),
            Span::raw("Establish connection by registering your printer"),
        ]),
    ];
    
    let registration_para = Paragraph::new(registration_text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .block(registration_block);
    f.render_widget(registration_para, main_area[2]);
    
    // 輸入區域
    let input_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(main_area[3]);
    
    let input_prompt = Paragraph::new("ENTER PRINTER DESIGNATION:")
        .style(Style::default().fg(highlight_color))
        .alignment(Alignment::Left);
    f.render_widget(input_prompt, input_area[0]);
    
    // 輸入框
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(base_color));
    
    let blink_cursor = time % 2 == 0;
    let cursor = if blink_cursor { "█" } else { " " };
    
    let input_text = format!("{}{}",
        app.printer_alias,
        if blink_cursor && app.printer_alias.len() < 30 { cursor } else { "" }
    );
    
    let input = Paragraph::new(input_text)
        .style(Style::default().fg(Color::White))
        .block(input_block);
    f.render_widget(input, input_area[1]);
    
    // 控制項信息
    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(dim_color));
    
    let help_text = vec![
        Line::from(vec![
            Span::styled("ENTER", Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)),
            Span::raw(" to confirm"),
            Span::raw("   "),
            Span::styled("BACKSPACE", Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)),
            Span::raw(" to edit"),
            Span::raw("   "),
            Span::styled("Q/ESC", Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)),
            Span::raw(" to quit"),
        ]),
    ];
    
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(dim_color))
        .alignment(Alignment::Center)
        .block(help_block);
    f.render_widget(help, main_area[5]);
    
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