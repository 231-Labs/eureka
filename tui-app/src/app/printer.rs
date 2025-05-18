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
            
            // get printer info and printer cap
            let address = self.wallet.get_active_address().await?;
            
            match self.wallet.get_printer_info(address).await {
                Ok(info) => {
                    match self.wallet.get_printer_cap_id(address).await {
                        Ok(cap_id) => {
                            // 更新打印機狀態
                            let printer_cap_id = ObjectID::from_hex_literal(&cap_id)?;
                            let printer_object_id = ObjectID::from_hex_literal(&info.id)?;
                            
                            match builder.update_printer_status(printer_cap_id, printer_object_id).await {
                                Ok(tx_id) => {
                                    self.is_online = !original_state;
                                    self.set_message(
                                        MessageType::Success,
                                        format!("Printer status: {} (Digest: {})",
                                            if self.is_online { "ONLINE" } else { "OFFLINE" },
                                            tx_id
                                        )
                                    );
                                    
                                    // 如果切換到 online 模式，立即獲取打印任務
                                    if self.is_online {
                                        if let Err(e) = self.update_print_tasks().await {
                                            self.set_message(MessageType::Error, format!("Failed to get print tasks: {}", e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    self.set_message(MessageType::Error, format!("Failed to update printer status: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            self.set_message(MessageType::Error, format!("Failed to get PrinterCap ID: {}", e));
                            return Ok(());
                        }
                    }
                }
                Err(e) => {
                    self.set_message(MessageType::Error, format!("Failed to get printer info: {}", e));
                    return Ok(());
                }
            }
        } else {
            // if no printer, directly update UI state
            self.is_online = !original_state;
            
            // 如果切換到 online 模式，立即獲取打印任務
            if self.is_online {
                if let Err(e) = self.update_print_tasks().await {
                    self.set_message(MessageType::Error, format!("Failed to get print tasks: {}", e));
                }
            }
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

    pub async fn run_print_script(app: Arc<Mutex<App>>) -> Result<bool, String> {
        {
            let mut app_guard = app.lock().await;
            app_guard.script_status = ScriptStatus::Running;
            app_guard.print_status = PrintStatus::Printing;
            app_guard.clear_print_output();
            app_guard.set_message(MessageType::Info, "Printing...".to_string());
        }
        
        // Use channel to wait for script completion
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<bool, String>>(1);
        let app_clone = Arc::clone(&app);
        
        // Get current directory
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
        
        // 添加調試輸出
        {
            let mut app_locked = app_clone.lock().await;
            app_locked.print_output.push(format!("[DEBUG] Current directory: {}", current_dir.display()));
        }
        
        // Check if Gcode-Transmit directory exists before executing script
        let transmit_dir = current_dir.join("Gcode-Transmit");
        {
            if !transmit_dir.exists() || !transmit_dir.is_dir() {
                let error_msg = format!("Gcode-Transmit directory does not exist at {}", transmit_dir.display());
                let mut app_locked = app_clone.lock().await;
                app_locked.print_output.push(format!("[ERROR] {}", error_msg));
                app_locked.script_status = ScriptStatus::Failed(error_msg.clone());
                // FIXME: Test Only - 正式版本發布前需恢復此行，確保錯誤時正確設置打印狀態
                // 當前忽略錯誤保持打印狀態，方便測試UI動畫效果
                // app_locked.print_status = PrintStatus::Error(error_msg.clone());
                app_locked.set_message(MessageType::Error, error_msg.clone());
                return Err(error_msg);
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
                        let error_msg = format!("Failed to start script: {}", e);
                        let mut app = app_clone.lock().await;
                        app.script_status = ScriptStatus::Failed(error_msg.clone());
                        // FIXME: Test Only - 正式版本發布前需恢復此行，確保腳本啟動失敗時正確設置打印狀態
                        // 當前忽略錯誤保持打印狀態，以便在開發階段觀察UI行為
                        // app.print_status = PrintStatus::Error(error_msg.clone());
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
                    // FIXME: Test Only - 正式版本發布前需恢復此行，確保腳本執行失敗時正確設置打印狀態
                    // 當前保持打印狀態以方便測試UI效果，與在線模式保持一致
                    // app.print_status = PrintStatus::Error(error_msg.clone());
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
                let _ = tx.send(Ok(true)).await;
            } else {
                let error_code = status.code().unwrap_or(-1);
                let error_msg = match error_code {
                    1 => "Printer not connected",
                    2 => "Slicing process failed",
                    3 => "Serial communication failed",
                    _ => "Unknown error",
                };
                let full_error = format!("Script execution failed (Error code: {}): {}", error_code, error_msg);
                app.script_status = ScriptStatus::Failed(full_error.clone());
                // FIXME: Test Only - 正式版本發布前需恢復此行，確保完整反映真實打印狀態
                // 目前無論是否成功都保持打印狀態，統一在線/離線模式行為
                // app.print_status = PrintStatus::Error(full_error.clone());
                app.set_message(MessageType::Error, full_error.clone());
                let _ = tx.send(Err(full_error)).await;
            }
        });
        
        // Wait for script completion and return result
        match rx.recv().await {
            Some(result) => result,
            None => Err("Communication channel with print script was closed unexpectedly".to_string())
        }
    }

    pub async fn run_stop_script(&mut self) -> Result<()> {
        // immediately show stopping state
        self.set_message(MessageType::Info, "Stopping print...".to_string());
        
        // 獲取當前執行目錄
        let current_dir = std::env::current_dir()?;
        let script_dir = current_dir.join("Gcode-Transmit");
        
        // 輸出路徑日誌，以便調試
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
}
