use anyhow::Result;
use crossterm::{
    event::{self as crossterm_event, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{io, time::Duration};
use std::sync::Arc;
use tokio::sync::Mutex;
use app::{MessageType, TaskStatus};

mod app;
mod constants;
mod utils;
mod wallet;
mod ui;
mod transactions;

use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    execute!(stdout, Clear(ClearType::All))?;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize application state
    let app = Arc::new(Mutex::new(App::new().await?));

    // Run application
    let result = run_app(&mut terminal, Arc::clone(&app)).await;

    // Restore terminal
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
    // Retry interval
    let mut last_update_time = std::time::Instant::now();
    let retry_interval = std::time::Duration::from_secs(5); // Retry getting printer ID every 3 second
    
    // Track if printer ID is acquired
    let mut printer_id_acquired = false;
    
    loop {
        let app_arc = Arc::clone(&app);
        
        // Check if printer ID is acquired
        if !printer_id_acquired {
            let should_update = {
                let app_guard = app_arc.lock().await;
                // Only update if not registering printer and printer ID is missing
                !app_guard.is_registering_printer && 
                app_guard.printer_id == "No Printer ID" && 
                last_update_time.elapsed() >= retry_interval
            };
            
            if should_update {
                let mut app_guard = app_arc.lock().await;
                // Try to update basic info
                if let Err(e) = app_guard.update_basic_info().await {
                    println!("Failed to update basic info: {}", e);
                } else if app_guard.printer_id != "No Printer ID" {
                    // If successfully acquired printer ID, mark as acquired
                    printer_id_acquired = true;
                    println!("Successfully acquired printer ID: {}", app_guard.printer_id);
                }
                last_update_time = std::time::Instant::now();
            }
        }
        
        {
            let mut app_guard = app_arc.lock().await;
            terminal.draw(|f| ui::draw(f, &mut app_guard)).unwrap();
        }

        if crossterm_event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = crossterm_event::read()? {
                let mut app_guard = app_arc.lock().await;
                if app_guard.is_registering_printer {
                    // Only handle registration related keys on registration page
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
                    // Handle all keys on main page
                    match key.code {
                        KeyCode::Char('q') => {
                            // Check if app is in online mode before exiting
                            if app_guard.is_online {
                                app_guard
                                .set_message
                                (MessageType::Error, "Please switch to OFFLINE mode before exiting the application."
                                .to_string());
                            } else {
                                return Ok(());
                            }
                        },
                        KeyCode::Esc => return Ok(()),
                        // Confirmation related keys
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
                        // Feature keys
                        KeyCode::Char('o') => {
                            if !app_guard.is_confirming && !app_guard.is_harvesting && !app_guard.is_switching_network {
                                // check if there is a printing task in online mode
                                if app_guard.is_online && app_guard.tasks.iter().any(|task| matches!(task.status, TaskStatus::Active)) {
                                    app_guard.set_message(MessageType::Error, "Cannot switch mode while a print job is in progress. Please complete the current job first.".to_string());
                                } else {
                                    app_guard.start_toggle_confirm();
                                }
                            }
                        }
                        KeyCode::Char('h') => {
                            if !app_guard.is_confirming && !app_guard.is_harvesting && !app_guard.is_switching_network {
                                app_guard.start_harvest_confirm();
                            }
                        }
                        KeyCode::Char('p') => {
                            if !app_guard.is_confirming && !app_guard.is_harvesting && !app_guard.is_switching_network {
                                if app_guard.is_online {
                                    // Process print tasks in online mode
                                    App::handle_task_print(Arc::clone(&app_arc), false).await?;
                                } else {
                                    // Process local models in offline mode
                                    App::handle_model_selection(Arc::clone(&app_arc), false).await?;
                                }
                            }
                        }
                        KeyCode::Char('e') => {
                            if !app_guard.is_confirming && !app_guard.is_harvesting && !app_guard.is_switching_network {
                                if let Err(e) = app_guard.run_stop_script().await {
                                    app_guard.set_message(MessageType::Error, format!("Failed to stop print: {}", e));
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if !app_guard.is_confirming && !app_guard.is_harvesting && !app_guard.is_switching_network {
                                if app_guard.is_online {
                                    // Only download without printing in online mode
                                    App::handle_task_print(Arc::clone(&app_arc), true).await?;
                                } else {
                                    // Only download without printing in offline mode
                                    App::handle_model_selection(Arc::clone(&app_arc), true).await?;
                                }
                            }
                        }
                        // Network switch
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
                        // list navigation
                        KeyCode::Up => {
                            app_guard.previous_item();
                        }
                        KeyCode::Down => {
                            app_guard.next_item();
                        }
                        _ => {
                            // clear any messages
                            app_guard.clear_error();
                            app_guard.success_message = None;
                        }
                    }
                }
            }
        }
    }
}

