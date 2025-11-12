use crate::app::core::App;
use crate::app::{MessageType, ScriptStatus, PrintStatus};
use crate::constants::PRINT_OUTPUT_MAX_LINES;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{BufReader, AsyncBufReadExt};

impl App {
    /// Run slicing test without printer connection
    pub async fn run_slice_test(app: Arc<Mutex<App>>) -> Result<bool, String> {
        {
            let mut app_guard = app.lock().await;
            app_guard.print_output.push("[TEST] Starting slicing test...".to_string());
        }
        
        let current_dir = match std::env::current_dir() {
            Ok(dir) => dir,
            Err(e) => {
                let error_msg = format!("Failed to get current directory: {}", e);
                let mut app_locked = app.lock().await;
                app_locked.print_output.push(format!("[TEST] ERROR: {}", error_msg));
                return Err(error_msg);
            }
        };
        
        let test_dir = current_dir.join("test_decryption");
        let input_stl = test_dir.join("decrypted.stl");
        let output_gcode = test_dir.join("output.gcode");
        
        // Check if input file exists
        if !input_stl.exists() {
            let error_msg = format!("Input STL not found at {}", input_stl.display());
            let mut app = app.lock().await;
            app.print_output.push(format!("[TEST] ERROR: {}", error_msg));
            return Err(error_msg);
        }
        
        {
            let mut app = app.lock().await;
            app.print_output.push(format!("[TEST] Running PrusaSlicer on: {}", input_stl.display()));
        }
        
        // Get PrusaSlicer config
        let transmit_dir = current_dir.join("Gcode-Transmit").join("main");
        let config_file = transmit_dir.join("Ender-3_set.ini");
        
        if !config_file.exists() {
            let error_msg = format!("Config file not found at {}", config_file.display());
            let mut app = app.lock().await;
            app.print_output.push(format!("[TEST] ERROR: {}", error_msg));
            return Err(error_msg);
        }
        
        // Run PrusaSlicer
        let mut child = match tokio::process::Command::new("prusa-slicer")
            .arg("--load")
            .arg(&config_file)
            .arg("--export-gcode")
            .arg("--output")
            .arg(&output_gcode)
            .arg(&input_stl)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn() {
                Ok(child) => child,
                Err(e) => {
                    let error_msg = format!("Failed to start PrusaSlicer: {}", e);
                    let mut app = app.lock().await;
                    app.print_output.push(format!("[TEST] ERROR: {}", error_msg));
                    return Err(error_msg);
                }
            };
        
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let app_clone_stdout = Arc::clone(&app);
        let app_clone_stderr = Arc::clone(&app);
        
        let stdout_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let mut app = app_clone_stdout.lock().await;
                app.print_output.push(format!("[TEST] {}", line));
            }
        });
        
        let stderr_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let mut app = app_clone_stderr.lock().await;
                app.print_output.push(format!("[TEST] {}", line));
            }
        });
        
        let status = match child.wait().await {
            Ok(status) => status,
            Err(e) => {
                let error_msg = format!("PrusaSlicer execution failed: {}", e);
                let mut app = app.lock().await;
                app.print_output.push(format!("[TEST] ERROR: {}", error_msg));
                return Err(error_msg);
            }
        };
        
        let _ = tokio::join!(stdout_handle, stderr_handle);
        
        let mut app = app.lock().await;
        if status.success() {
            app.print_output.push(format!("[TEST] G-code saved to: {}", output_gcode.display()));
            Ok(true)
        } else {
            let error_msg = format!("PrusaSlicer failed with exit code: {:?}", status.code());
            app.print_output.push(format!("[TEST] ERROR: {}", error_msg));
            Err(error_msg)
        }
    }

    pub async fn run_print_script(app: Arc<Mutex<App>>) -> Result<bool, String> {
        {
            let mut app_guard = app.lock().await;
            app_guard.script_status = ScriptStatus::Running;
            app_guard.print_status = PrintStatus::Printing;
            app_guard.clear_print_log();
            app_guard.set_message(MessageType::Info, "Starting print script...".to_string());
        }
        
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<bool, String>>(1);
        let app_clone = Arc::clone(&app);
        
        let current_dir = match std::env::current_dir() {
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
        
        App::setup_gcode_monitoring(Arc::clone(&app_clone)).await;
        
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

            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            let app_clone_stdout = Arc::clone(&app_clone);
            let app_clone_stderr = Arc::clone(&app_clone);

            let stdout_handle = tokio::spawn(async move {
                let mut reader = BufReader::new(stdout).lines();
                        while let Ok(Some(line)) = reader.next_line().await {
                            let mut app = app_clone_stdout.lock().await;
                            app.print_output.push(format!("[STDOUT] {}", line));
                            if app.print_output.len() > PRINT_OUTPUT_MAX_LINES {
                                app.print_output.remove(0);
                            }
                        }
            });

            let stderr_handle = tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                        while let Ok(Some(line)) = reader.next_line().await {
                            let mut app = app_clone_stderr.lock().await;
                            app.print_output.push(format!("[STDERR] {}", line));
                            if app.print_output.len() > PRINT_OUTPUT_MAX_LINES {
                                app.print_output.remove(0);
                            }
                        }
            });

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

            let _ = tokio::join!(stdout_handle, stderr_handle);

            let mut app = app_clone.lock().await;
            if status.success() {
                app.script_status = ScriptStatus::Completed;
                app.print_status = PrintStatus::Completed;
                app.set_message(MessageType::Success, "Print completed successfully".to_string());
                
                let should_update_blockchain = 
                    !app.printer_id.eq("No Printer ID") && 
                    app.sculpt_state.selected()
                        .map(|index| index < app.sculpt_items.len())
                        .unwrap_or(false);
                
                if should_update_blockchain {
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
                
                let error_code = status.code().unwrap_or(-1);
                let full_error = format!("Print failed (code {}): {}", error_code, error_message);
                
                app.script_status = ScriptStatus::Failed(full_error.clone());
                app.set_message(MessageType::Error, full_error.clone());
                
                let _ = tx.send(Err(full_error)).await;
            }
        });
        
        match rx.recv().await {
            Some(result) => result,
            None => Err("Communication channel with print script was closed unexpectedly".to_string())
        }
        
    }

    #[allow(dead_code)]
    pub async fn run_stop_script(&mut self) -> Result<()> {
        self.set_message(MessageType::Info, "Stopping print...".to_string());
        
        let current_dir = std::env::current_dir()?;
        let script_path = current_dir.join("Gcode-Transmit").join("Gcode-Process.sh");
        
        if !script_path.exists() {
            let error_msg = format!("Script file does not exist: {}", script_path.display());
            self.print_output.push(format!("[ERROR] {}", error_msg));
            self.script_status = ScriptStatus::Failed(error_msg.clone());
            self.set_message(MessageType::Error, error_msg);
            return Ok(());
        }
        
        let script_path_str = script_path.to_string_lossy();
        let command = format!("{} --stop", script_path_str);
        
        let output = match tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .await {
                Ok(output) => output,
                Err(e) => {
                    let error_msg = format!("Failed to execute stop script: {}", e);
                    self.print_output.push(format!("[ERROR] {}", error_msg));
                    self.script_status = ScriptStatus::Failed(error_msg.clone());
                    self.print_status = PrintStatus::Error("Failed to execute stop script".to_string());
                    self.set_message(MessageType::Error, error_msg);
                    return Ok(());
                }
            };

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            self.script_status = ScriptStatus::Idle;
            self.print_status = PrintStatus::Idle;
            
            if !stdout.is_empty() {
                self.print_output.push(format!("[STDOUT] {}", stdout));
            }
            
            self.set_message(MessageType::Success, 
                if stdout.is_empty() { "Print stopped successfully".to_string() } else { stdout }
            );
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            
            if !stdout.is_empty() {
                self.print_output.push(format!("[STDOUT] {}", stdout));
            }
            if !stderr.is_empty() {
                self.print_output.push(format!("[STDERR] {}", stderr));
            }
            
            let error_msg = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                format!("Failed to stop print (exit code: {})", output.status.code().unwrap_or(-1))
            };
            
            self.script_status = ScriptStatus::Failed(error_msg.clone());
            self.print_status = PrintStatus::Error(error_msg.clone());
            self.set_message(MessageType::Error, error_msg);
        }
        
        Ok(())
    }

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