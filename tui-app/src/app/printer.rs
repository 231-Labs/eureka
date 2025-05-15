use crate::app::core::App;
use crate::app::{MessageType, RegistrationStatus, ScriptStatus, PrintStatus};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{BufReader, AsyncBufReadExt};
use sui_sdk::types::base_types::ObjectID;

impl App {
    pub fn start_toggle_confirm(&mut self) {
        self.is_confirming = true;
    }

    pub async fn confirm_toggle(&mut self) -> Result<()> {
        // track original state
        let original_state = self.is_online;
        
        // first toggle state
        self.is_confirming = false;

        // try to update printer status
        if self.printer_id != "No Printer ID" {
            self.set_message(MessageType::Info, "Sending status update to blockchain...".to_string());
            
            let builder = crate::transactions::TransactionBuilder::new(
                Arc::clone(&self.sui_client),
                ObjectID::from(self.wallet.get_active_address().await?),
                self.network_state.clone()
            ).await;
            
            // get printer info
            match self.wallet.get_printer_info(self.wallet.get_active_address().await?).await {
                Ok(info) => {
                    let printer_id_str = info.id;
                    self.printer_id = printer_id_str.clone();
                    self.set_message(MessageType::Info, format!("Using printer ID: {}", printer_id_str));
                    
                    let printer_object_id = match ObjectID::from_hex_literal(&printer_id_str) {
                        Ok(id) => id,
                        Err(e) => {
                            self.set_message(MessageType::Error, format!("Invalid printer ID format: {} - {}", printer_id_str, e));
                            return Ok(());
                        }
                    };
                    
                    match builder.update_printer_status(printer_object_id).await {
                        Ok(tx_digest) => {
                            // only update UI state after transaction success
                            self.is_online = !original_state;
                            
                            let status_text = if self.is_online { "ONLINE" } else { "OFFLINE" };
                            
                            // set success message directly
                            self.set_message(
                                MessageType::Success, 
                                format!("Printer status updated to {} (Transaction ID: {})", status_text, tx_digest)
                            );
                        },
                        Err(e) => {
                            // only set error message
                            self.set_message(MessageType::Error, format!("Failed to update printer status: {}", e));
                            return Ok(());
                        }
                    };
                },
                Err(e) => {
                    self.set_message(MessageType::Error, format!("Failed to get printer info: {}", e));
                    return Ok(());
                }
            }
        } else {
            // if no printer, directly update UI state
            self.is_online = !original_state;
        }

        // if offline, update sculpt items
        if !self.is_online {
            match self.wallet.get_user_sculpt(self.wallet.get_active_address().await?).await {
                Ok(items) => {
                    self.sculpt_items = items;
                    // reset selection state
                    if !self.sculpt_items.is_empty() {
                        self.sculpt_state.select(Some(0));
                    }
                }
                Err(e) => {
                    self.set_message(MessageType::Error, format!("Failed to load 3D models: {}", e));
                }
            }
        }
        
        Ok(())
    }

    pub fn cancel_toggle(&mut self) {
        self.is_confirming = false;
    }

    // printer registration
    pub async fn handle_printer_registration_input(&mut self, input: char) -> Result<()> {
        match input {
            '\n' => {
                if !self.printer_alias.is_empty() && self.registration_status == RegistrationStatus::Inputting {
                    self.registration_status = RegistrationStatus::Submitting;
                    self.printer_registration_message = "Sending transaction to network...\nPlease wait...".to_string();
                    
                    let builder = crate::transactions::TransactionBuilder::new(
                        Arc::clone(&self.sui_client),
                        ObjectID::from(self.wallet.get_active_address().await?),
                        self.network_state.clone()
                    ).await;

                    self.printer_registration_message = "Transaction sent. Waiting for confirmation...\nThis may take a few seconds...".to_string();

                    match builder.register_printer(
                        self.network_state.get_current_package_ids().eureka_printer_registry_id.parse()?,
                        &self.printer_alias
                    ).await {
                        Ok(tx_digest) => {
                            self.registration_status = RegistrationStatus::Success(tx_digest.clone());
                            self.printer_registration_message = format!(
                                "Registration Successful!\n\
                                 Printer Name: {}\n\
                                 Transaction ID: {}\n\n\
                                 Press ENTER to continue...",
                                self.printer_alias,
                                tx_digest
                            );
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Registration failed: {}", e));
                            self.registration_status = RegistrationStatus::Failed(e.to_string());
                            self.printer_registration_message = "Registration failed. Press ESC to exit, or try registering again...".to_string();
                        }
                    };
                } else if matches!(self.registration_status, RegistrationStatus::Success(_)) {
                    // immediately exit registration page, let UI continue refreshing
                    self.is_registering_printer = false;
                    
                    // update application state in next loop
                    self.update_basic_info().await?;
                }
            }
            '\x08' | '\x7f' => {
                if self.registration_status == RegistrationStatus::Inputting {
                    self.printer_alias.pop();
                }
            }
            c if c.is_ascii() && !c.is_control() => {
                if self.registration_status == RegistrationStatus::Inputting && self.printer_alias.len() < 30 {
                    self.printer_alias.push(c);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn run_print_script(app: Arc<Mutex<App>>) -> bool {
        {
            let mut app_guard = app.lock().await;
            app_guard.script_status = ScriptStatus::Running;
            app_guard.print_status = PrintStatus::Idle;
            app_guard.clear_print_output();
            app_guard.set_message(MessageType::Info, "Printing...".to_string());
        }
        
        // Use channel to wait for script completion
        let (tx, mut rx) = tokio::sync::mpsc::channel::<bool>(1);
        let app_clone = Arc::clone(&app);
        
        // Check if Gcode-Transmit directory exists before executing script
        {
            let transmit_dir = std::path::Path::new("Gcode-Transmit");
            if !transmit_dir.exists() || !transmit_dir.is_dir() {
                let mut app_locked = app_clone.lock().await;
                app_locked.print_output.push("[ERROR] Gcode-Transmit directory does not exist".to_string());
                return false;
            }
        }
        
        tokio::spawn(async move {
            // Use absolute path
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
                        let mut app = app_clone.lock().await;
                        app.script_status = ScriptStatus::Failed(format!("Failed to start script: {}", e));
                        app.print_status = PrintStatus::Error(format!("Failed to start script: {}", e));
                        app.set_message(MessageType::Error, format!("Failed to start script: {}", e));
                        let _ = tx.send(false).await;
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
                    let mut app = app_clone.lock().await;
                    app.script_status = ScriptStatus::Failed(format!("Script execution failed: {}", e));
                    app.print_status = PrintStatus::Error(format!("Script execution failed: {}", e));
                    app.set_message(MessageType::Error, format!("Script execution failed: {}", e));
                    let _ = tx.send(false).await;
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
                let _ = tx.send(true).await;
            } else {
                let error_code = status.code().unwrap_or(-1);
                let error_msg = match error_code {
                    1 => "Printer not connected",
                    2 => "Slicing process failed",
                    3 => "Serial communication failed",
                    _ => "Unknown error",
                };
                app.script_status = ScriptStatus::Failed(format!("Script execution failed (Error code: {}): {}", error_code, error_msg));
                app.print_status = PrintStatus::Error(format!("Script execution failed (Error code: {}): {}", error_code, error_msg));
                app.set_message(MessageType::Error, format!("Script execution failed (Error code: {}): {}", error_code, error_msg));
                let _ = tx.send(false).await;
            }
        });
        
        // Wait for script completion and return result
        rx.recv().await.unwrap_or(false)
    }

    pub async fn run_stop_script(&mut self) -> Result<()> {
        // immediately show stopping state
        self.set_message(MessageType::Info, "Stopping print...".to_string());
        
        // use spawn to start command, capture output to display
        let output = match tokio::process::Command::new("sh")
            .current_dir("Gcode-Transmit")
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
}
