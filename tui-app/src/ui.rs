use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::app::App;

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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.size());

    // 註冊信息
    let registration_text = format!("{}\n\nYour input: {}", app.printer_registration_message, app.printer_alias);
    let registration = Paragraph::new(registration_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(registration, chunks[0]);

    // 提示信息
    let help = Paragraph::new("Press Enter to register your printer")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[1]);
} 