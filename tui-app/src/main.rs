use anyhow::Result;
use crossterm::{
    event::{self as crossterm_event, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{io, time::Duration};
use std::sync::Arc;
use tokio::sync::Mutex;

mod app;
mod constants;
mod utils;
mod wallet;
mod ui;
mod transactions;

use app::{App, MessageType};

#[tokio::main]
async fn main() -> Result<()> {
    // 設置終端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 初始化應用程序狀態
    let app = Arc::new(Mutex::new(App::new().await?));

    // 運行應用
    let result = run_app(&mut terminal, Arc::clone(&app)).await;

    // 恢復終端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: Arc<Mutex<App>>,
) -> Result<()> {
    loop {
        let app_arc = Arc::clone(&app);
        {
            let mut app_guard = app_arc.lock().await;
            terminal.draw(|f| ui::draw(f, &mut app_guard)).unwrap();
        }

        if crossterm_event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = crossterm_event::read()? {
                let mut app_guard = app_arc.lock().await;
                if app_guard.is_registering_printer {
                    // 在註冊畫面時，只處理註冊相關的按鍵
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Esc => return Ok(()),
                        KeyCode::Char(c) => {
                            if let Err(e) = app_guard.handle_printer_registration_input(c).await {
                                app_guard.error_message = Some(format!("Error: {}", e));
                            }
                        }
                        KeyCode::Backspace => {
                            if let Err(e) = app_guard.handle_printer_registration_input('\x08').await {
                                app_guard.error_message = Some(format!("Error: {}", e));
                            }
                        }
                        KeyCode::Enter => {
                            if let Err(e) = app_guard.handle_printer_registration_input('\n').await {
                                app_guard.error_message = Some(format!("Error: {}", e));
                            }
                        }
                        _ => {}
                    }
                } else {
                    // 在主畫面時，處理所有按鍵
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Esc => return Ok(()),
                        // 二次確認相關的按鍵
                        KeyCode::Char('y') => {
                            if app_guard.is_confirming {
                                app_guard.confirm_toggle().await?;
                            } else if app_guard.is_harvesting {
                                app_guard.confirm_harvest();
                            }
                        }
                        KeyCode::Char('n') => {
                            if app_guard.is_confirming {
                                app_guard.cancel_toggle();
                            } else if app_guard.is_harvesting {
                                app_guard.cancel_harvest();
                            } else if app_guard.is_switching_network {
                                app_guard.cancel_network_switch();
                            } else {
                                app_guard.start_network_switch();
                            }
                        }
                        // 功能按鍵
                        KeyCode::Char('o') => {
                            if !app_guard.is_confirming && !app_guard.is_harvesting && !app_guard.is_switching_network {
                                app_guard.start_toggle_confirm();
                            }
                        }
                        KeyCode::Char('h') => {
                            if !app_guard.is_confirming && !app_guard.is_harvesting && !app_guard.is_switching_network {
                                app_guard.start_harvest_confirm();
                            }
                        }
                        KeyCode::Char('p') => {
                            if !app_guard.is_online && !app_guard.is_confirming && !app_guard.is_harvesting && !app_guard.is_switching_network {
                                App::handle_model_selection(Arc::clone(&app_arc), false).await?;
                            }
                        }
                        KeyCode::Char('s') => {
                            if !app_guard.is_confirming && !app_guard.is_harvesting && !app_guard.is_switching_network {
                                if let Err(e) = App::run_stop_script(Arc::clone(&app_arc)).await {
                                    app_guard.set_message(MessageType::Error, format!("Failed to stop script: {}", e));
                                }
                            }
                        }
                        KeyCode::Char('e') => {
                            if !app_guard.is_confirming && !app_guard.is_harvesting && !app_guard.is_switching_network {
                                if let Err(e) = App::run_stop_script(Arc::clone(&app_arc)).await {
                                    app_guard.set_message(MessageType::Error, format!("Failed to stop print: {}", e));
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if !app_guard.is_online && !app_guard.is_confirming && !app_guard.is_harvesting && !app_guard.is_switching_network {
                                App::handle_model_selection(Arc::clone(&app_arc), true).await?;
                            }
                        }
                        // 網絡切換
                        KeyCode::Char('1') | KeyCode::Char('2') | KeyCode::Char('3') => {
                            if app_guard.is_switching_network {
                                let network_index = match key.code {
                                    KeyCode::Char('1') => 2,  // MAINNET
                                    KeyCode::Char('2') => 0,  // DEVNET
                                    KeyCode::Char('3') => 1,  // TESTNET
                                    _ => unreachable!(),
                                };
                                app_guard.switch_to_network(network_index);
                                app_guard.update_network().await?;
                            }
                        }
                        // 列表導航
                        KeyCode::Up => {
                            app_guard.previous_item();
                        }
                        KeyCode::Down => {
                            app_guard.next_item();
                        }
                        _ => {
                            // 清除任何消息
                            app_guard.clear_error();
                            app_guard.success_message = None;
                        }
                    }
                }
            }
        }
    }
}

