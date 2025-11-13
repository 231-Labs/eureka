use crate::app::core::App;
use crate::constants::AGGREGATOR_URL;
use crate::seal::{SealDecryptor, PrintJobDecryptor};
use crate::seal::types::SealResourceMetadata;
use crate::app::printer::mock::{run_mock_print_script, MockPrintScriptResult};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::fs;
use std::path::Path;

impl App {
    pub async fn download_3d_model(&mut self, blob_id: &str, seal_resource_id: Option<&str>) -> Result<()> {
        let url = format!("{}/v1/blobs/{}", AGGREGATOR_URL, blob_id);
        let current_dir = std::env::current_dir()?;
        let temp_path = current_dir.join("test.stl");
        let final_path = current_dir.join("Gcode-Transmit").join("test.stl");
        
        self.print_output.push(format!("[LOG] Downloading model from: {}", url));
        
        let gcode_dir = current_dir.join("Gcode-Transmit");
        if !Path::new(&gcode_dir).exists() {
            self.print_output.push(format!("[LOG] Creating directory: {}", gcode_dir.display()));
            fs::create_dir_all(&gcode_dir)?;
        }
        
        let status = tokio::process::Command::new("curl")
            .arg("-s")
            .arg("-S")
            .arg(&url)
            .arg("-o")
            .arg(&temp_path)
            .status()
            .await?;

        if !status.success() {
            self.set_message(crate::app::MessageType::Error, "Failed to download 3D model".to_string());
            return Err(anyhow::anyhow!("Failed to download 3D model"));
        }

        if let Some(resource_id_str) = seal_resource_id {
            self.print_output.push("[LOG] 🔐 Encrypted model detected, attempting to decrypt...".to_string());
            self.print_output.push(format!("[LOG] 🔐 Seal Resource ID: {}", resource_id_str));
            
            match self.decrypt_model_file(&temp_path, resource_id_str).await {
                Ok(_) => {
                    self.print_output.push("[LOG] ✅ Model decrypted successfully".to_string());
                }
                Err(e) => {
                    self.print_output.push(format!("[LOG] ❌ Decryption failed: {}", e));
                    self.set_message(crate::app::MessageType::Error, format!("Failed to decrypt model: {}", e));
                    return Err(anyhow::anyhow!("Failed to decrypt model: {}", e));
                }
            }
        }

        if let Err(e) = fs::rename(&temp_path, &final_path) {
            self.set_message(crate::app::MessageType::Error, format!("Failed to move 3D model: {}", e));
            return Err(anyhow::anyhow!("Failed to move 3D model: {}", e));
        }
        self.set_message(crate::app::MessageType::Success, "3D model downloaded successfully".to_string());
        Ok(())
    }

    async fn decrypt_model_file(&mut self, file_path: &Path, resource_id_str: &str) -> Result<()> {
        let seal_metadata = SealResourceMetadata::from_resource_id_string(resource_id_str)?;
        let encrypted_data = tokio::fs::read(file_path).await?;
        
        if !SealDecryptor::is_file_encrypted(&encrypted_data) {
            self.print_output.push("[LOG] ⚠️  File appears to be unencrypted, skipping decryption".to_string());
            return Ok(());
        }
        
        let rpc_url = self.network_state.get_current_rpc().to_string();
        let wallet_config_path = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?
            .join(".sui")
            .join("sui_config")
            .join("client.yaml");
        
        self.print_output.push("[LOG] 🔐 Initializing Seal decryption service...".to_string());
        let decryptor = SealDecryptor::new(rpc_url, wallet_config_path).await?;
        
        self.print_output.push(format!("[LOG] 🔐 Decrypting with package_id: {}", seal_metadata.package_id));
        self.print_output.push(format!("[LOG] 🔐 Resource ID: {}", seal_metadata.resource_id));
        
        let decrypted_data = decryptor.decrypt_stl(
            encrypted_data,
            &seal_metadata.package_id,
            &seal_metadata.resource_id,
        ).await?;
        
        tokio::fs::write(file_path, decrypted_data).await?;
        
        Ok(())
    }

    pub async fn handle_model_selection(app: Arc<Mutex<App>>, download_only: bool) -> Result<()> {
        let app_clone = Arc::clone(&app);
        tokio::spawn(async move {
            let selected_item = {
                let app_guard = app_clone.lock().await;
                app_guard.sculpt_state
                    .selected()
                    .and_then(|idx| app_guard.sculpt_items.get(idx).cloned())
            };

            if let Some(item) = selected_item {
                if item.alias != "No printable models found" {
                    {
                        let mut app = app_clone.lock().await;
                        app.print_output.push(format!("[LOG] Selected model: {}", item.alias));
                    }
                    
                    // download model
                    let download_result = {
                        let mut app = app_clone.lock().await;
                        app.download_3d_model(&item.blob_id, item.seal_resource_id.as_deref()).await
                    };

                    // process download result
                    if let Err(e) = download_result {
                        let mut app = app_clone.lock().await;
                        app.set_message(crate::app::MessageType::Error, format!("Failed to download model: {}", e));
                        return;
                    }

                    // run print script (not only download)
                    if !download_only {
                        {
                            let mut app = app_clone.lock().await;
                            
                            if app.printer_id != "No Printer ID" {
                                app.print_output.push("[LOG] Creating print job on blockchain...".to_string());
                                
                                match app.test_create_print_job().await {
                                    Ok(_) => {
                                        app.print_output.push("[LOG] Print job created on blockchain successfully".to_string());
                                        app.set_message(crate::app::MessageType::Success, "Print job created on blockchain successfully".to_string());
                                    },
                                    Err(e) => {
                                        if e.contains("A print job already exists") {
                                            app.print_output.push("[LOG] A print job already exists, continuing with printing...".to_string());
                                        } else {
                                            app.print_output.push(format!("[LOG] Failed to create print job on blockchain: {}", e));
                                            app.set_message(crate::app::MessageType::Error, format!("Failed to create print job: {}", e));
                                        }
                                    }
                                }
                            }
                        }
                        
                        {
                            let mut app = app_clone.lock().await;
                            app.print_output.push("[LOG] Preparing to run print script".to_string());
                            app.print_status = crate::app::PrintStatus::Printing;
                        }
                        
                        let print_result = App::run_print_script(Arc::clone(&app_clone)).await;
                        
                        let mut app = app_clone.lock().await;
                        match print_result {
                            Ok(_) => {
                                app.print_output.push("[LOG] Print script executed successfully".to_string());
                            },
                            Err(error_msg) => {
                                app.print_output.push(format!("[LOG] Print script failed: {}", error_msg));
                                app.set_message(crate::app::MessageType::Error, error_msg);
                                // Update status to failed when there's an error
                                app.print_status = crate::app::PrintStatus::Idle;
                            }
                        }
                    }
                }
            }
        });
        Ok(())
    }

    pub async fn handle_task_print(app: Arc<Mutex<App>>, download_only: bool) -> Result<()> {
        let app_clone = Arc::clone(&app);
        tokio::spawn(async move {
            let active_task = {
                let app_guard = app_clone.lock().await;
                app_guard.tasks.iter()
                    .find(|t| matches!(t.status, crate::app::print_job::TaskStatus::Active))
                    .cloned()
            };

            if let Some(task) = active_task {
                {
                    let mut app = app_clone.lock().await;
                    app.print_output.push(format!("[LOG] Processing active task: {}", task.name));
                    app.print_output.push(format!("[LOG] Sculpt structure (blob_id): {}", task.sculpt_structure));
                    app.set_message(crate::app::MessageType::Info, format!("Processing print job: {}", task.name));
                }
                
                let download_result = {
                    let mut app = app_clone.lock().await;
                    app.download_3d_model(&task.sculpt_structure, None).await
                };

                if let Err(e) = download_result {
                    let mut app = app_clone.lock().await;
                    app.set_message(crate::app::MessageType::Error, format!("Failed to download task model: {}", e));
                    return;
                }

                if !download_only {
                    {
                        let mut app = app_clone.lock().await;
                        app.print_output.push("[LOG] Preparing to run print script for task".to_string());
                        app.print_status = crate::app::PrintStatus::Printing;
                    }
                    
                    let print_result = App::run_print_script(Arc::clone(&app_clone)).await;
                    
                    let mut app = app_clone.lock().await;
                    match print_result {
                        Ok(_) => {
                            app.print_output.push("[LOG] Task print script executed successfully".to_string());
                            app.set_message(crate::app::MessageType::Success, "Print job started successfully!".to_string());
                        },
                        Err(error_msg) => {
                            app.print_output.push(format!("[LOG] Task print script failed: {}", error_msg));
                            app.set_message(crate::app::MessageType::Error, format!("Failed to start print job: {}", error_msg));
                            app.print_status = crate::app::PrintStatus::Idle;
                        }
                    }
                }
            } else {
                let mut app = app_clone.lock().await;
                app.set_message(
                    crate::app::MessageType::Info, 
                    "No active print job found. Please wait for new tasks.".to_string()
                );
            }
        });
        Ok(())
    }

    /// Handle mock print with PrintJob-based decryption (T key handler)
    pub async fn handle_mock_print_with_printjob(app: Arc<Mutex<App>>) -> Result<()> {
        let app_clone = Arc::clone(&app);
        tokio::spawn(async move {
            // Get active print job
            let active_task = {
                let app_guard = app_clone.lock().await;
                app_guard.tasks.iter()
                    .find(|t| matches!(t.status, crate::app::print_job::TaskStatus::Active))
                    .cloned()
            };

            if let Some(task) = active_task {
                {
                    let mut app = app_clone.lock().await;
                    app.print_output.push(format!("=== MOCK PRINT MODE: {} ===", task.name));
                    app.print_output.push("[MOCK] Starting PrintJob-based decryption...".to_string());
                    app.set_message(crate::app::MessageType::Info, format!("Mock printing: {}", task.name));
                }

                // Get printer information from app state
                let (printer_id_str, wallet_address) = {
                    let app_guard = app_clone.lock().await;
                    let wallet_addr = app_guard.wallet.get_active_address().await.ok();
                    (app_guard.printer_id.clone(), wallet_addr)
                };

                if printer_id_str == "No Printer ID" {
                    let mut app = app_clone.lock().await;
                    app.set_message(crate::app::MessageType::Error, "Printer ID not available".to_string());
                    return;
                }

                let wallet_address = match wallet_address {
                    Some(addr) => addr,
                    None => {
                        let mut app = app_clone.lock().await;
                        app.set_message(crate::app::MessageType::Error, "Failed to get wallet address".to_string());
                        return;
                    }
                };

                // Get printer cap ID from wallet
                let printer_cap_id_str = {
                    let app_guard = app_clone.lock().await;
                    match app_guard.wallet.get_printer_cap_id(wallet_address).await {
                        Ok(cap_id) => cap_id,
                        Err(e) => {
                            drop(app_guard);
                            let mut app = app_clone.lock().await;
                            app.set_message(crate::app::MessageType::Error, format!("Failed to get PrinterCap ID: {}", e));
                            return;
                        }
                    }
                };

                // Parse printer IDs
                let printer_id = match seal_sdk_rs::native_sui_sdk::sui_types::base_types::ObjectID::from_hex_literal(&printer_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        let mut app = app_clone.lock().await;
                        app.set_message(crate::app::MessageType::Error, format!("Invalid printer ID: {}", e));
                        return;
                    }
                };

                let printer_cap_id = match seal_sdk_rs::native_sui_sdk::sui_types::base_types::ObjectID::from_hex_literal(&printer_cap_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        let mut app = app_clone.lock().await;
                        app.set_message(crate::app::MessageType::Error, format!("Invalid printer cap ID: {}", e));
                        return;
                    }
                };

                // Create PrintJob decryptor and perform decryption
                let decryption_result = match PrintJobDecryptor::new().await {
                    Ok(decryptor) => {
                        {
                            let mut app = app_clone.lock().await;
                            app.print_output.push("[MOCK] PrintJob decryptor initialized".to_string());
                            app.print_output.push("[MOCK] Starting decryption with PrintJob authorization...".to_string());
                        }

                        decryptor.decrypt_printjob_sculpt(printer_id, printer_cap_id).await
                    },
                    Err(e) => {
                        let mut app = app_clone.lock().await;
                        app.set_message(crate::app::MessageType::Error, format!("Failed to create decryptor: {}", e));
                        return;
                    }
                };

                match decryption_result {
                    Ok(plaintext) => {
                        {
                            let mut app = app_clone.lock().await;
                            app.print_output.push("[MOCK] ✅ PrintJob-based decryption successful!".to_string());
                            
                            let format = if plaintext.starts_with(b"solid") {
                                "ASCII STL"
                            } else if plaintext.len() > 84 {
                                "Binary STL"
                            } else {
                                "Unknown"
                            };
                            
                            app.print_output.push(format!("[MOCK] Decrypted STL: {} ({} bytes)", format, plaintext.len()));
                            app.print_output.push("[MOCK] Starting mock print process...".to_string());
                        }

                        // Save decrypted file for mock printing and testing
                        let current_dir = match std::env::current_dir() {
                            Ok(dir) => dir,
                            Err(e) => {
                                let mut app = app_clone.lock().await;
                                app.set_message(crate::app::MessageType::Error, format!("Failed to get current directory: {}", e));
                                return;
                            }
                        };

                        // Save to mock_print.stl for mock printing
                        let mock_stl_path = current_dir.join("mock_print.stl");
                        if let Err(e) = std::fs::write(&mock_stl_path, &plaintext) {
                            let mut app = app_clone.lock().await;
                            app.set_message(crate::app::MessageType::Error, format!("Failed to save decrypted STL: {}", e));
                            return;
                        }

                        {
                            let mut app = app_clone.lock().await;
                            app.print_output.push(format!("[MOCK] Saved decrypted STL to: {}", mock_stl_path.display()));
                            app.print_output.push("[MOCK] Starting slicing test...".to_string());
                        }

                        // Run slicing test (optional - won't fail if PrusaSlicer not available)
                        let slice_result = App::run_slice_test(Arc::clone(&app_clone)).await;
                        match slice_result {
                            Ok(_) => {
                                let mut app = app_clone.lock().await;
                                app.print_output.push("[MOCK] ✅ Slicing test completed successfully".to_string());
                            },
                            Err(e) => {
                                let mut app = app_clone.lock().await;
                                app.print_output.push(format!("[MOCK] ⚠️ Slicing test failed (optional): {}", e));
                                // Don't return here - continue with mock printing even if slicing fails
                            }
                        }

                        // Run mock print script (5 seconds, success result)
                        let mock_result = run_mock_print_script(
                            Arc::clone(&app_clone),
                            MockPrintScriptResult::Success,
                            5, // 5 seconds
                            true // generate logs
                        ).await;

                        match mock_result {
                            Ok(_) => {
                                {
                                    let mut app = app_clone.lock().await;
                                    app.print_output.push("[MOCK] ✅ Mock print completed successfully!".to_string());
                                    app.print_output.push("[MOCK] Marking PrintJob as completed on blockchain...".to_string());
                                    app.set_message(crate::app::MessageType::Success, "Mock print job completed successfully!".to_string());
                                }

                                // Directly call PrintJob completion using PrintJob context (not sculpt selection)
                                let completion_result = {
                                    let mut app = app_clone.lock().await;
                                    app.test_complete_print_job_from_task(&task).await
                                };

                                match completion_result {
                                    Ok(_) => {
                                        let mut app = app_clone.lock().await;
                                        app.print_output.push("[MOCK] ✅ PrintJob marked as completed on blockchain!".to_string());
                                        app.set_message(crate::app::MessageType::Success, "Mock print and PrintJob completion successful!".to_string());
                                    },
                                    Err(e) => {
                                        let mut app = app_clone.lock().await;
                                        app.print_output.push(format!("[MOCK] ⚠️ Print completed but failed to mark PrintJob as completed: {}", e));
                                        app.set_message(crate::app::MessageType::Success, "Mock print completed, but blockchain update failed".to_string());
                                    }
                                }
                            },
                            Err(e) => {
                                let mut app = app_clone.lock().await;
                                app.print_output.push(format!("[MOCK] ❌ Mock print failed: {}", e));
                                app.set_message(crate::app::MessageType::Error, format!("Mock print failed: {}", e));
                            }
                        }
                    },
                    Err(e) => {
                        let mut app = app_clone.lock().await;
                        app.print_output.push(format!("[MOCK] ❌ Decryption failed: {}", e));
                        app.set_message(crate::app::MessageType::Error, format!("PrintJob decryption failed: {}", e));
                        
                        // Add debug information
                        app.print_output.push("[MOCK] 🔍 Possible causes:".to_string());
                        app.print_output.push("[MOCK]   1. ENotPrinterOwner: Caller is not the printer owner".to_string());
                        app.print_output.push("[MOCK]   2. EInvalidPrinterCap: PrinterCap doesn't match this printer".to_string());
                        app.print_output.push("[MOCK]   3. EPrintJobNotFound: No PrintJob exists for this printer".to_string());
                        app.print_output.push("[MOCK]   4. EPrinterIdMismatch: PrintJob's printer_id mismatch".to_string());
                    }
                }
            } else {
                let mut app = app_clone.lock().await;
                app.set_message(
                    crate::app::MessageType::Info, 
                    "No active PrintJob found. Please wait for new tasks or create a PrintJob.".to_string()
                );
                app.print_output.push("[MOCK] ❌ No active PrintJob found for mock printing".to_string());
            }
        });
        Ok(())
    }
}
