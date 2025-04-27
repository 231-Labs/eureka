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
    let mut app = App::new().await?;

    // 運行應用
    let result = run_app(&mut terminal, &mut app).await;

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
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if crossterm_event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = crossterm_event::read()? {
                if app.is_registering_printer {
                    // 在註冊畫面時，只處理註冊相關的按鍵
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Esc => return Ok(()),
                        KeyCode::Char(c) => {
                            if let Err(e) = app.handle_printer_registration_input(c).await {
                                app.error_message = Some(format!("Error: {}", e));
                            }
                        }
                        KeyCode::Backspace => {
                            if let Err(e) = app.handle_printer_registration_input('\x08').await {
                                app.error_message = Some(format!("Error: {}", e));
                            }
                        }
                        KeyCode::Enter => {
                            if let Err(e) = app.handle_printer_registration_input('\n').await {
                                app.error_message = Some(format!("Error: {}", e));
                            }
                        }
                        _ => {}
                    }
                } else {
                    // 在主畫面時，處理所有快捷鍵
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Esc => return Ok(()),
                        KeyCode::Up => app.previous_item(),
                        KeyCode::Down => app.next_item(),
                        KeyCode::Char('o') => {
                            app.clear_error();
                            app.start_toggle_confirm();
                        }
                        KeyCode::Char('h') => {
                            app.clear_error();
                            app.start_harvest_confirm();
                        }
                        KeyCode::Char('n') => {
                            if app.is_confirming {
                                app.cancel_toggle();
                            } else if app.is_harvesting {
                                app.cancel_harvest();
                            } else if app.is_switching_network {
                                app.cancel_network_switch();
                            } else {
                                app.clear_error();
                                app.start_network_switch();
                            }
                        }
                        KeyCode::Char('1') if app.is_switching_network => {
                            app.switch_to_network(1);
                            if let Err(e) = app.update_network().await {
                                eprintln!("Failed to update network: {}", e);
                            }
                        }
                        KeyCode::Char('2') if app.is_switching_network => {
                            app.switch_to_network(2);
                            if let Err(e) = app.update_network().await {
                                eprintln!("Failed to update network: {}", e);
                            }
                        }
                        KeyCode::Char('3') if app.is_switching_network => {
                            app.switch_to_network(3);
                            if let Err(e) = app.update_network().await {
                                eprintln!("Failed to update network: {}", e);
                            }
                        }
                        KeyCode::Char('y') => {
                            if app.is_confirming {
                                app.clear_error();
                                app.confirm_toggle();
                            } else if app.is_harvesting {
                                app.clear_error();
                                app.confirm_harvest();
                            }
                        }
                        //Start-New
                        KeyCode::Char('s') => {

                            app.run_custom_script();        // 3D列印機按鍵

                        }
                        _ => {}
                        //End
                    }
                }
            }
        }
    }
}
