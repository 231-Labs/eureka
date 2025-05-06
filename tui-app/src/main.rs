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

use app::App;

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
                    // 在主畫面時，處理所有快捷鍵
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Esc => return Ok(()),
                        KeyCode::Up => app_guard.previous_item(),
                        KeyCode::Down => app_guard.next_item(),
                        KeyCode::Char('o') => {
                            app_guard.clear_error();
                            app_guard.start_toggle_confirm();
                        }
                        KeyCode::Char('h') => {
                            app_guard.clear_error();
                            app_guard.start_harvest_confirm();
                        }
                        KeyCode::Char('n') => {
                            if app_guard.is_confirming {
                                app_guard.cancel_toggle();
                            } else if app_guard.is_harvesting {
                                app_guard.cancel_harvest();
                            } else if app_guard.is_switching_network {
                                app_guard.cancel_network_switch();
                            } else {
                                app_guard.clear_error();
                                app_guard.start_network_switch();
                            }
                        }
                        KeyCode::Char('1') if app_guard.is_switching_network => {
                            app_guard.switch_to_network(1);
                            if let Err(e) = app_guard.update_network().await {
                                eprintln!("Failed to update network: {}", e);
                            }
                        }
                        KeyCode::Char('2') if app_guard.is_switching_network => {
                            app_guard.switch_to_network(2);
                            if let Err(e) = app_guard.update_network().await {
                                eprintln!("Failed to update network: {}", e);
                            }
                        }
                        KeyCode::Char('3') if app_guard.is_switching_network => {
                            app_guard.switch_to_network(3);
                            if let Err(e) = app_guard.update_network().await {
                                eprintln!("Failed to update network: {}", e);
                            }
                        }
                        KeyCode::Char('y') => {
                            if app_guard.is_confirming {
                                app_guard.confirm_toggle().await?;
                            } else if app_guard.is_harvesting {
                                app_guard.clear_error();
                                app_guard.confirm_harvest();
                            }
                        }
                        //Start-Start Printing
                        KeyCode::Char('p') => {
                            if let Err(e) = App::handle_model_selection(Arc::clone(&app_arc), false).await {
                                let mut app_guard = app_arc.lock().await;
                                app_guard.error_message = Some(format!("Error: {}", e));
                            }
                        }
                        KeyCode::Enter => {
                            if let Err(e) = App::handle_model_selection(Arc::clone(&app_arc), true).await {
                                let mut app_guard = app_arc.lock().await;
                                app_guard.error_message = Some(format!("Error: {}", e));
                            }
                        }
                        //End

                        //Start-Stop Printing
                        KeyCode::Char('s') => {
                            if let Err(e) = App::run_stop_script(Arc::clone(&app_arc)).await {
                                let mut app_guard = app_arc.lock().await;
                                app_guard.error_message = Some(format!("Error: {}", e));
                            }
                        }
                        KeyCode::Char('e') => {
                            if let Err(e) = App::run_stop_script(Arc::clone(&app_arc)).await {
                                let mut app_guard = app_arc.lock().await;
                                app_guard.error_message = Some(format!("Error: {}", e));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
