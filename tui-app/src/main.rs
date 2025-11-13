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
use tokio::time;

mod app;
mod constants;
mod utils;
mod wallet;
mod blockchain;
mod model;

use constants::{PRINT_JOB_POLL_INTERVAL_SECS, RETRY_INTERVAL_SECS, SCULPT_LOAD_DELAY_MILLIS};
mod ui;
mod transactions;
mod seal;

use app::{App, MessageType, TaskStatus};

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

    // Start loading Sculpts asynchronously
    start_sculpt_loading_task(Arc::clone(&app));

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
    let mut last_update_time = std::time::Instant::now();
    let retry_interval = std::time::Duration::from_secs(RETRY_INTERVAL_SECS);
    
    let mut printer_id_acquired = false;
    let mut sculpt_loading_started = false;
    
    start_print_job_polling(Arc::clone(&app));
    
    loop {
        let app_arc = Arc::clone(&app);
        
        if !printer_id_acquired {
            let should_update = {
                let app_guard = app_arc.lock().await;
                !app_guard.is_registering_printer && 
                app_guard.printer_id == "No Printer ID" && 
                last_update_time.elapsed() >= retry_interval
            };
            
            if should_update {
                let mut app_guard = app_arc.lock().await;
                if let Err(e) = app_guard.update_basic_info().await {
                    println!("Failed to update basic info: {}", e);
                } else if app_guard.printer_id != "No Printer ID" {
                    printer_id_acquired = true;
                    println!("Successfully acquired printer ID: {}", app_guard.printer_id);
                }
                last_update_time = std::time::Instant::now();
            }
        }
        
        {
            let app_guard = app_arc.lock().await;
            let is_online = app_guard.is_online;
            drop(app_guard);
            
            if is_online {
                sculpt_loading_started = false;
            }
            
            let should_load = {
                let app_guard = app_arc.lock().await;
                app_guard.is_loading_sculpts && app_guard.sculpt_items.is_empty() && !sculpt_loading_started
            };
            if should_load {
                sculpt_loading_started = true;
                start_sculpt_loading_task(Arc::clone(&app_arc));
            }
            
            let loading_complete = {
                let app_guard = app_arc.lock().await;
                !app_guard.is_loading_sculpts && sculpt_loading_started
            };
            if loading_complete {
                sculpt_loading_started = false;
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
                        KeyCode::Char('q') => {
                            terminal.clear()?;
                            return Ok(());
                        },
                        KeyCode::Esc => {
                            terminal.clear()?;
                            return Ok(());
                        },
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
                    match key.code {
                        KeyCode::Char('q') => {
                            if app_guard.is_online {
                                app_guard
                                .set_message
                                (MessageType::Error, "Please switch to OFFLINE mode before exiting the application."
                                .to_string());
                            } else {
                                terminal.clear()?;
                                return Ok(());
                            }
                        },
                        KeyCode::Esc => {
                            terminal.clear()?;
                            return Ok(());
                        },
                        KeyCode::Char('y') => {
                            if app_guard.is_confirming {
                                app_guard.confirm_toggle_immediate();
                                drop(app_guard);
                                start_toggle_task(Arc::clone(&app_arc));
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
                        KeyCode::Char('o') => {
                            if !app_guard.is_confirming && !app_guard.is_harvesting && !app_guard.is_switching_network {
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
                                    App::handle_task_print(Arc::clone(&app_arc), false).await?;
                                } else {
                                    App::handle_model_selection(Arc::clone(&app_arc), false).await?;
                                }
                            }
                        }
                        KeyCode::Char('t') => {
                            if !app_guard.is_confirming && !app_guard.is_harvesting && !app_guard.is_switching_network {
                                app_guard.print_output.push("[INFO] Starting mock print mode (T key pressed)".to_string());
                                drop(app_guard);
                                App::handle_mock_print_with_printjob(Arc::clone(&app_arc)).await?;
                            }
                        }
                        KeyCode::Char('1') | KeyCode::Char('2') | KeyCode::Char('3') => {
                            if app_guard.is_switching_network {
                                let network_index = match key.code {
                                    KeyCode::Char('1') => 2,  // MAINNET
                                    KeyCode::Char('2') => 0,  // DEVNET
                                    KeyCode::Char('3') => 1,  // TESTNET
                                    _ => unreachable!(),
                                };
                                app_guard.switch_to_network(network_index);
                                drop(app_guard);
                                let mut app_guard = app_arc.lock().await;
                                app_guard.update_network().await?;
                            }
                        }
                        KeyCode::Up => {
                            app_guard.previous_item();
                        }
                        KeyCode::Down => {
                            app_guard.next_item();
                        }
                        _ => {
                            app_guard.clear_error();
                            app_guard.success_message = None;
                        }
                    }
                }
            }
        }
    }
}

fn start_sculpt_loading_task(app: Arc<Mutex<App>>) {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(SCULPT_LOAD_DELAY_MILLIS)).await;
        
        let address = {
            let app_guard = app.lock().await;
            app_guard.wallet.get_active_address().await.ok()
        };
        
        if let Some(addr) = address {
            let sculpts_result = {
                let app_guard = app.lock().await;
                let wallet = app_guard.wallet.clone();
                drop(app_guard);
                wallet.get_user_sculpt(addr).await
            };
            
            match sculpts_result {
                Ok(sculpts) => {
                    let mut app_guard = app.lock().await;
                    app_guard.sculpt_items = sculpts;
                    app_guard.is_loading_sculpts = false;
                }
                Err(e) => {
                    let mut app_guard = app.lock().await;
                    app_guard.sculpt_items = vec![crate::wallet::SculptItem {
                        alias: format!("Error loading models: {}", e),
                        blob_id: String::new(),
                        printed_count: 0,
                        id: String::new(),
                        is_encrypted: false,
                        seal_resource_id: None,
                    }];
                    app_guard.is_loading_sculpts = false;
                }
            }
        }
    });
}

fn start_toggle_task(app: Arc<Mutex<App>>) {
    tokio::spawn(async move {
        let mut app_guard = app.lock().await;
        if let Err(e) = app_guard.confirm_toggle().await {
            app_guard.set_message(crate::app::MessageType::Error, format!("Failed to toggle mode: {}", e));
        }
    });
}

fn start_print_job_polling(app: Arc<Mutex<App>>) {
    tokio::spawn(async move {
        let poll_interval = time::Duration::from_secs(PRINT_JOB_POLL_INTERVAL_SECS);
        let mut interval = time::interval(poll_interval);
        
        loop {
            interval.tick().await;
            let mut app_guard = app.lock().await;
            
            if app_guard.is_online && app_guard.printer_id != "No Printer ID" {
                let has_active_task = app_guard.tasks
                .iter()
                .any(|task| !task
                .is_completed());
                if !has_active_task {
                    if let Err(e) = app_guard.update_print_tasks().await {
                        println!("Failed to update print tasks: {:?}", e);
                    }
                }
            }
        }
    });
}

