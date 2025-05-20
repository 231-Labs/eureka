use crate::app::core::App;
use crate::app::{MessageType, ScriptStatus, PrintStatus};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{BufReader, AsyncBufReadExt};

impl App {
    // main function to run the print script
    pub async fn run_print_script(app: Arc<Mutex<App>>) -> Result<bool, String> {
        {
            let mut app_guard = app.lock().await;
            app_guard.script_status = ScriptStatus::Running;
            app_guard.print_status = PrintStatus::Printing;
            app_guard.clear_print_log();
            app_guard.set_message(MessageType::Info, "Starting print script...".to_string());
        }
        
        // Use channel to wait for script completion
        // FIXME: for test only
        #[allow(unused_mut)]
        let (_tx, mut _rx) = tokio::sync::mpsc::channel::<Result<bool, String>>(1);
        let app_clone = Arc::clone(&app);
        
        // Get current directory
        let current_dir = match std::env::current_dir()
        {
            Ok(dir) => dir,
            Err(e) => {
                let error_msg = format!("Failed to get current directory: {}", e);
                let mut app_locked = app_clone.lock().await;
                app_locked.print_output.push(format!("[ERROR] {}", error_msg));
                app_locked.script_status = ScriptStatus::Failed(error_msg.clone());
                app_locked.set_message(MessageType::Error, error_msg.clone());
                return Err(error_msg);
            }
        };
        
        // Check if Gcode-Transmit directory exists before executing script
        let transmit_dir = current_dir.join("Gcode-Transmit");
        {
            if !transmit_dir.exists() || !transmit_dir.is_dir() {
                let error_msg = format!("Gcode-Transmit directory does not exist at {}", transmit_dir.display());
                let mut app_locked = app_clone.lock().await;
                app_locked.print_output.push(format!("[ERROR] {}", error_msg));
                app_locked.script_status = ScriptStatus::Failed(error_msg.clone());
                app_locked.set_message(MessageType::Error, error_msg.clone());
                return Err(error_msg);
            }
        }
        
        // start print job on blockchain
        {
            let mut app_guard = app.lock().await;
            
            if app_guard.printer_id.eq("No Printer ID") {
                drop(app_guard);
            } else {
                let selected_index = app_guard.sculpt_state.selected();
                if selected_index.is_none() || selected_index.unwrap() >= app_guard.sculpt_items
                .len()
                { drop(app_guard) }
                else {
                    match app_guard.test_start_print_job().await {
                        Err(e) => app_guard.print_output
                        .push(format!("[INFO] Failed to start print job on blockchain: {}", e)),
                        Ok(_) => app_guard.print_output
                        .push("[INFO] Print job started on blockchain successfully".to_string()),
                    }
                }
            }
        }
        
        // start Gcode monitoring
        App::setup_gcode_monitoring(Arc::clone(&app_clone)).await;
        
        // TODO: text only 使用模擬腳本替代實際打印腳本
        // 成功模擬，執行時間為10秒鐘，產生日誌輸出
        return super::run_mock_print_script(
            app_clone, 
            super::MockPrintScriptResult::Success,
            10, // 執行10秒
            true // 產生日誌
        ).await;
        
        /* 註釋掉實際執行腳本的代碼，等有真實打印機時再恢復
        tokio::spawn(async move {
            let current_dir = std::env::current_dir().unwrap_or_default();
            let script_path = current_dir.join("Gcode-Transmit").join("Gcode-Process.sh");
            let script_path_str = script_path.to_string_lossy();
            let command = format!("{} --print", script_path_str);
            
            let mut child = match tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&command)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn() {
                    Ok(child) => child,
                    Err(e) => {
                        let error_msg = format!("Failed to start script: {}", e);
                        let mut app = app_clone.lock().await;
                        app.script_status = ScriptStatus::Failed(error_msg.clone());
                        app.set_message(MessageType::Error, error_msg.clone());
                        let _ = tx.send(Err(error_msg)).await;
                        return;
                    }
                };

            // Set up output handling
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            let app_clone_stdout = Arc::clone(&app_clone);
            let app_clone_stderr = Arc::clone(&app_clone);

            // Handle standard output
            let stdout_handle = tokio::spawn(async move {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let mut app = app_clone_stdout.lock().await;
                    app.print_output.push(format!("[STDOUT] {}", line));
                    if app.print_output.len() > 1000 {
                        app.print_output.remove(0);
                    }
                }
            });

            // Handle error output
            let stderr_handle = tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let mut app = app_clone_stderr.lock().await;
                    app.print_output.push(format!("[STDERR] {}", line));
                    if app.print_output.len() > 1000 {
                        app.print_output.remove(0);
                    }
                }
            });

            // Wait for script completion
            let status = match child.wait().await {
                Ok(status) => status,
                Err(e) => {
                    let error_msg = format!("Script execution failed: {}", e);
                    let mut app = app_clone.lock().await;
                    app.script_status = ScriptStatus::Failed(error_msg.clone());
                    app.set_message(MessageType::Error, error_msg.clone());
                    let _ = tx.send(Err(error_msg)).await;
                    return;
                }
            };

            // Wait for output handling to complete
            let _ = tokio::join!(stdout_handle, stderr_handle);

            // Update final status
            let mut app = app_clone.lock().await;
            if status.success() {
                app.script_status = ScriptStatus::Completed;
                app.print_status = PrintStatus::Completed;
                app.set_message(MessageType::Success, "Print completed successfully".to_string());
                
                // Try to update blockchain state
                let should_update_blockchain = 
                    !app.printer_id.eq("No Printer ID") && 
                    app.sculpt_state.selected()
                        .map(|index| index < app.sculpt_items.len())
                        .unwrap_or(false);
                
                if should_update_blockchain {
                    // Spawn a task to update blockchain status
                    let app_clone_for_completion = Arc::clone(&app_clone);
                    tokio::spawn(async move {
                        Self::update_blockchain_on_completion(app_clone_for_completion).await;
                    });
                }
                
                let _ = tx.send(Ok(true)).await;
            } else {
                let error_message = match status.code() {
                    Some(1) => "Printer not connected",
                    Some(2) => "Slicing process failed",
                    Some(3) => "Serial communication failed",
                    Some(code) => {
                        app.print_output
                        .push(format!("[ERROR] Unknown error code: {}", code));
                        "Unknown error"
                    },
                    None => "Process terminated with unknown status",
                };
                
                // build detailed error information
                let error_code = status.code().unwrap_or(-1);
                let full_error = format!("Print failed (code {}): {}", error_code, error_message);
                
                // update application state
                app.script_status = ScriptStatus::Failed(full_error.clone());
                app.set_message(MessageType::Error, full_error.clone());
                
                // send error result
                let _ = tx.send(Err(full_error)).await;
            }
        });
        
        // Wait for script completion and return result
        match rx.recv().await {
            Some(result) => result,
            None => Err("Communication channel with print script was closed unexpectedly".to_string())
        }
        */
    }

    // stop print script
    pub async fn run_stop_script(&mut self) -> Result<()> {
        // immediately show stopping state
        self.set_message(MessageType::Info, "Stopping print...".to_string());
        
        // get current execution directory
        let current_dir = std::env::current_dir()?;
        let script_dir = current_dir.join("Gcode-Transmit");
        
        // output path log for debugging
        self.print_output.push(format!("[DEBUG] Current directory: {}", current_dir.display()));
        self.print_output.push(format!("[DEBUG] Script directory: {}", script_dir.display()));
        
        // use spawn to start command, capture output to display
        let output = match tokio::process::Command::new("sh")
            .current_dir(&script_dir)
            .arg("Gcode-Process.sh")
            .arg("--stop")
            .output()
            .await {
                Ok(output) => output,
                Err(e) => {
                    self.script_status = ScriptStatus::Failed(e.to_string());
                    self.print_status = PrintStatus::Error("Failed to execute stop script".to_string());
                    self.set_message(MessageType::Error, format!("Failed to execute stop script: {}", e));
                    return Ok(());
                }
            };

        // process output
        if output.status.success() {
            // get message from output
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // reset state
            self.script_status = ScriptStatus::Idle;
            self.print_status = PrintStatus::Idle;
            self.set_message(MessageType::Success, 
                if stdout.is_empty() { "Print stopped successfully".to_string() } else { stdout }
            );
        } else {
            // process error
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let error_msg = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                "Failed to stop print".to_string()
            };
            self.script_status = ScriptStatus::Failed(error_msg.clone());
            self.print_status = PrintStatus::Error(error_msg.clone());
            self.set_message(MessageType::Error, error_msg);
        }
        
        Ok(())
    }

    // Helper method to update blockchain status after print completion
    pub async fn update_blockchain_on_completion(app_clone: Arc<Mutex<App>>) {
        let mut completion_app = app_clone.lock().await;
        
        match completion_app.test_complete_print_job().await {
            Ok(_) => {
                completion_app.print_output
                    .push("[INFO] Print job completed successfully on blockchain".to_string());
            }
            Err(e) => {
                completion_app.print_output
                    .push(format!("[WARNING] Failed to complete print job on blockchain: {}", e));
                completion_app.set_message(
                    MessageType::Error, 
                    format!("Print completed locally but failed to update blockchain: {}", e)
                );
            }
        }
    }
} 