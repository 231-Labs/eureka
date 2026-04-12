#[allow(dead_code)]
use std::time::Duration;
use tokio::time::sleep;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::app::core::App;
use crate::app::{MessageType, ScriptStatus, PrintStatus};

/// Outcome type for the mock print script.
#[allow(dead_code)]
pub enum MockPrintScriptResult {
    /// Finished successfully
    Success,
    /// Printer not connected
    PrinterNotConnected,
    /// Slicing failed
    SlicingFailed,
    /// Serial communication failed
    SerialCommFailed,
    /// Custom exit code + message
    CustomError(i32, String),
}

/// Run a mock print script with configurable outcome and duration.
#[allow(unused_mut, dead_code)]
pub async fn run_mock_print_script(
    app: Arc<Mutex<App>>, 
    result: MockPrintScriptResult, 
    execution_time_secs: u64,
    should_generate_logs: bool
) -> Result<bool, String> {
    // Initialize print state
    {
        let mut app_guard = app.lock().await;
        app_guard.script_status = ScriptStatus::Running;
        app_guard.print_status = PrintStatus::Printing;
        app_guard.clear_print_log();
        app_guard.set_message(MessageType::Info, "Starting mock print script...".to_string());
        
        if should_generate_logs {
            // Seed a few mock log lines
            app_guard.print_output.push("[STDOUT] initializing...".to_string());
            app_guard.print_output.push("[STDOUT] checking connection...".to_string());
        }
    }
    
    // Stream mock logs during the run
    if should_generate_logs {
        // Number of log intervals
        let log_intervals = execution_time_secs.min(30); // cap at ~30 log lines
        if log_intervals > 0 {
            let interval_duration = Duration::from_secs(execution_time_secs / log_intervals);
            
            for i in 0..log_intervals {
                // Wait, then append a log line
                sleep(interval_duration).await;
                
                let progress = (i as f32 / log_intervals as f32 * 100.0) as u8;
                let mut app_guard = app.lock().await;
                
                match &result {
                    MockPrintScriptResult::Success => {
                        app_guard.print_output.push(format!("[STDOUT] Print progress: {}%", progress));
                        if i == log_intervals / 2 {
                            app_guard.print_output.push("[STDOUT] Heating bed to 60C".to_string());
                        }
                        if i == log_intervals / 3 {
                            app_guard.print_output.push("[STDOUT] Heating nozzle to 215C".to_string());
                        }
                    },
                    MockPrintScriptResult::PrinterNotConnected => {
                        if i == log_intervals / 2 {
                            app_guard.print_output.push("[STDERR] warning: temperature too high".to_string());
                        }
                        if i == log_intervals - 1 {
                            app_guard.print_output.push("[STDERR] error: printer not connected".to_string());
                        }
                    },
                    MockPrintScriptResult::SlicingFailed => {
                        if i == log_intervals / 3 {
                            app_guard.print_output.push("[STDOUT] Preparing model for slicing".to_string());
                        }
                        if i == log_intervals - 1 {
                            app_guard.print_output.push("[STDERR] error: slicing process failed".to_string());
                        }
                    },
                    MockPrintScriptResult::SerialCommFailed => {
                        if i == log_intervals / 2 {
                            app_guard.print_output.push("[STDOUT] calibration completed".to_string());
                        }
                        if i == log_intervals - 1 {
                            app_guard.print_output.push("[STDERR] error: serial communication failed".to_string());
                        }
                    },
                    MockPrintScriptResult::CustomError(_, msg) => {
                        if i == log_intervals - 1 {
                            app_guard.print_output.push(format!("[STDERR] error: {}", msg));
                        }
                    }
                }
            }
        }
    } else {
        // No logs: just sleep for the full duration
        sleep(Duration::from_secs(execution_time_secs)).await;
    }
    
    // Final state after mock completion
    let mut app_guard = app.lock().await;
    
    match result {
        MockPrintScriptResult::Success => {
            app_guard.script_status = ScriptStatus::Completed;
            app_guard.print_status = PrintStatus::Completed;
            app_guard.set_message(MessageType::Success, "Print completed successfully".to_string());
            
            // Try to update blockchain state
            let should_update_blockchain = 
                !app_guard.printer_id.eq("No Printer ID") && 
                app_guard.sculpt_state.selected()
                    .map(|index| index < app_guard.sculpt_items.len())
                    .unwrap_or(false);
            
            if should_update_blockchain {
                // Spawn a task to update blockchain status
                let app_clone_for_completion = Arc::clone(&app);
                drop(app_guard); // release lock before spawning
                
                tokio::spawn(async move {
                    App::update_blockchain_on_completion(app_clone_for_completion).await;
                });
                
                return Ok(true);
            }
            
            Ok(true)
        },
        MockPrintScriptResult::PrinterNotConnected => {
            let error_msg = "Print failed (code 1): Printer not connected";
            app_guard.script_status = ScriptStatus::Failed(error_msg.to_string());
            app_guard.set_message(MessageType::Error, error_msg.to_string());
            Err(error_msg.to_string())
        },
        MockPrintScriptResult::SlicingFailed => {
            let error_msg = "Print failed (code 2): Slicing process failed";
            app_guard.script_status = ScriptStatus::Failed(error_msg.to_string());
            app_guard.set_message(MessageType::Error, error_msg.to_string());
            Err(error_msg.to_string())
        },
        MockPrintScriptResult::SerialCommFailed => {
            let error_msg = "Print failed (code 3): Serial communication failed";
            app_guard.script_status = ScriptStatus::Failed(error_msg.to_string());
            app_guard.set_message(MessageType::Error, error_msg.to_string());
            Err(error_msg.to_string())
        },
        MockPrintScriptResult::CustomError(code, message) => {
            let error_msg = format!("Print failed (code {}): {}", code, message);
            app_guard.script_status = ScriptStatus::Failed(error_msg.clone());
            app_guard.set_message(MessageType::Error, error_msg.clone());
            Err(error_msg)
        },
    }
} 