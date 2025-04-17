use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::{io, time::Duration};

mod app;
mod constants;
mod utils;
mod wallet;

use app::App;
use crate::app::TaskStatus;

#[tokio::main]
async fn main() -> io::Result<()> {
    // 設置終端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 初始化應用程序狀態
    let mut app = App::new().await.map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("Failed to initialize app: {}", e))
    })?;

    // 主循環
    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|f| {
            let size = f.size();
            
            // 創建左右分欄布局
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(75),
                ])
                .split(size);

            // 獲取當前主題顏色
            let (primary_color, secondary_color, accent_color) = if app.is_online {
                (Color::Cyan, Color::LightBlue, Color::DarkGray)  // 科技感冷色調
            } else {
                (Color::Magenta, Color::Red, Color::DarkGray)  // 未來感暖色調
            };

            // 左側欄的垂直布局
            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // 錢包地址
                    Constraint::Length(3),  // 網絡狀態
                    Constraint::Length(3),  // 代幣餘額
                    Constraint::Length(3),  // WAL 餘額
                    Constraint::Length(3),  // 印表機 ID 和工資
                    Constraint::Min(0),     // 列表區域
                    Constraint::Length(5),  // 操作提示
                ])
                .split(chunks[0]);

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
                .title("PRINTER ID (DEVNET)") // TODO: add changeable printer id
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
                        "● ONLINE [Press T to toggle]".to_string()
                    } else {
                        "○ OFFLINE [Press T to toggle]".to_string()
                    },
                    Style::default().fg(secondary_color)
                )
            };
            
            let status = Paragraph::new(status_text)
                .style(status_style)
                .block(status_block);
            f.render_widget(status, left_chunks[4]);

            // 根據狀態顯示不同的列表
            if app.is_online {
                // 在線狀態顯示任務列表
                let task_area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),  // 當前打印任務
                        Constraint::Min(0),     // 歷史記錄
                    ])
                    .split(left_chunks[5]);

                // 顯示當前打印任務
                let current_task = app.tasks
                    .iter()
                    .find(|task| matches!(task.status, TaskStatus::Printing(_)))
                    .map(|task| {
                        if let TaskStatus::Printing(progress) = task.status {
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
                f.render_widget(current_task_text, task_area[0]);

                // 顯示已完成任務
                let completed_tasks: Vec<ListItem> = app.tasks
                    .iter()
                    .filter(|task| matches!(task.status, TaskStatus::Completed))
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
                f.render_stateful_widget(tasks_list, task_area[1], &mut app.tasks_state);
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
                f.render_stateful_widget(assets_list, left_chunks[5], &mut app.assets_state);
            }

            // 操作提示
            let help_text = if app.is_confirming {
                vec![
                    Line::from(vec![
                        Span::styled("Y", Style::default().fg(Color::Yellow)),
                        Span::raw(": Confirm"),
                    ]),
                    Line::from(vec![
                        Span::styled("N", Style::default().fg(Color::Yellow)),
                        Span::raw(": Cancel"),
                    ]),
                ]
            } else if app.is_harvesting {
                vec![
                    Line::from(vec![
                        Span::styled("Y", Style::default().fg(Color::Yellow)),
                        Span::raw(": Confirm"),
                    ]),
                    Line::from(vec![
                        Span::styled("N", Style::default().fg(Color::Yellow)),
                        Span::raw(": Cancel"),
                    ]),
                ]
            } else if app.is_switching_network {
                vec![
                    Line::from(vec![
                        Span::styled("Y", Style::default().fg(Color::Yellow)),
                        Span::raw(": Confirm"),
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
                    ]),
                    Line::from(vec![
                        Span::styled("T", Style::default().fg(secondary_color)),
                        Span::raw(" TOGGLE STATUS"),
                    ]),
                    Line::from(vec![
                        Span::styled("H", Style::default().fg(secondary_color)),
                        Span::raw(" HARVEST REWARDS"),
                    ]),
                    Line::from(vec![
                        Span::styled("↑↓", Style::default().fg(secondary_color)),
                        Span::raw(" NAVIGATE LIST"),
                    ]),
                    Line::from(vec![
                        Span::styled("N", Style::default().fg(secondary_color)),
                        Span::raw(" SWITCH NETWORK"),
                    ]),
                ]
            };
            let help_block = Block::default()
                .title("CONTROLS")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(primary_color));
            let help = Paragraph::new(help_text)
                .block(help_block)
                .style(Style::default().fg(accent_color));
            f.render_widget(help, left_chunks[6]);

            // 右側內容區
            let right_block = Block::default()
                .title("CONTENT")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(primary_color));

            let right_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // 錯誤訊息區域，與左側錢包地址區域一致
                    Constraint::Length(3),  // 機器狀態區域，與左側網絡狀態區域一致
                    Constraint::Min(0),     // 其他內容
                ])
                .split(chunks[1]);

            f.render_widget(right_block, chunks[1]);

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
        })?;

        // 處理事件
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => should_quit = true,
                    KeyCode::Char('t') if !app.is_confirming && !app.is_harvesting => {
                        app.clear_error();
                        app.start_toggle_confirm()
                    },
                    KeyCode::Char('h') if !app.is_confirming && !app.is_harvesting => {
                        app.clear_error();
                        app.start_harvest_confirm()
                    },
                    KeyCode::Char('y') if app.is_confirming => {
                        app.clear_error();
                        app.confirm_toggle()
                    },
                    KeyCode::Char('n') if app.is_confirming => {
                        app.clear_error();
                        app.cancel_toggle()
                    },
                    KeyCode::Char('y') if app.is_harvesting => {
                        app.clear_error();
                        app.confirm_harvest()
                    },
                    KeyCode::Char('n') if app.is_harvesting => {
                        app.clear_error();
                        app.cancel_harvest()
                    },
                    KeyCode::Char('n') if !app.is_confirming && !app.is_harvesting && !app.is_switching_network => {
                        app.start_network_switch();
                    }
                    KeyCode::Char('1') if app.is_switching_network => {
                        app.switch_to_network(0);  
                        if let Err(e) = app.update_network().await {
                            eprintln!("Failed to update network: {}", e);
                        }
                    }
                    KeyCode::Char('2') if app.is_switching_network => {
                        app.switch_to_network(1);  
                        if let Err(e) = app.update_network().await {
                            eprintln!("Failed to update network: {}", e);
                        }
                    }
                    KeyCode::Char('3') if app.is_switching_network => {
                        app.switch_to_network(2);  
                        if let Err(e) = app.update_network().await {
                            eprintln!("Failed to update network: {}", e);
                        }
                    }
                    KeyCode::Esc if app.is_switching_network => {
                        app.cancel_network_switch();
                    }
                    KeyCode::Down => app.next_item(),
                    KeyCode::Up => app.previous_item(),
                    _ => {}
                }
            }
        }
    }

    // 清理
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
