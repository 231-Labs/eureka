use crate::app::core::App;
use crate::constants::AGGREGATOR_URL;
use crate::seal::SealDecryptor;
use crate::seal::types::SealResourceMetadata;
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

    /// Download and decrypt model to test_decryption folder
    pub async fn download_3d_model_test(&mut self, blob_id: &str, seal_resource_id: Option<&str>) -> Result<()> {
        let url = format!("{}/v1/blobs/{}", AGGREGATOR_URL, blob_id);
        let current_dir = std::env::current_dir()?;
        
        // Create test_decryption folder
        let test_dir = current_dir.join("test_decryption");
        if !Path::new(&test_dir).exists() {
            self.print_output.push(format!("[TEST] Creating test directory: {}", test_dir.display()));
            fs::create_dir_all(&test_dir)?;
        }
        
        let temp_path = test_dir.join("encrypted.stl");
        let final_path = test_dir.join("decrypted.stl");
        
        self.print_output.push(format!("[TEST] Downloading model from: {}", url));
        
        let status = tokio::process::Command::new("curl")
            .arg("-s")
            .arg("-S")
            .arg(&url)
            .arg("-o")
            .arg(&temp_path)
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to download test model"));
        }
        
        // Check file size
        let file_size = tokio::fs::metadata(&temp_path).await?.len();
        self.print_output.push(format!("[TEST] Downloaded to: {} ({} bytes)", temp_path.display(), file_size));
        
        if file_size == 0 {
            return Err(anyhow::anyhow!("Downloaded file is empty"));
        }

        if let Some(resource_id_str) = seal_resource_id {
            self.print_output.push("[TEST] 🔐 Encrypted model detected, decrypting...".to_string());
            self.print_output.push(format!("[TEST] 🔐 Seal Resource ID: {}", resource_id_str));
            
            // Check if file is actually encrypted
            let file_data = tokio::fs::read(&temp_path).await?;
            let is_encrypted = SealDecryptor::is_file_encrypted(&file_data);
            self.print_output.push(format!("[TEST] 🔍 File encryption check: {}", if is_encrypted { "ENCRYPTED" } else { "NOT ENCRYPTED" }));
            
            if !is_encrypted {
                self.print_output.push("[TEST] ⚠️  File appears to be unencrypted, skipping decryption".to_string());
                // Just rename the file
                if let Err(e) = fs::rename(&temp_path, &final_path) {
                    return Err(anyhow::anyhow!("Failed to rename file: {}", e));
                }
                self.print_output.push(format!("[TEST] File saved to: {}", final_path.display()));
                return Ok(());
            }
            
            match self.decrypt_model_file(&temp_path, resource_id_str).await {
                Ok(_) => {
                    self.print_output.push("[TEST] ✅ Model decrypted successfully".to_string());
                    // Rename decrypted file
                    if let Err(e) = fs::rename(&temp_path, &final_path) {
                        return Err(anyhow::anyhow!("Failed to rename decrypted file: {}", e));
                    }
                    self.print_output.push(format!("[TEST] Decrypted file saved to: {}", final_path.display()));
                }
                Err(e) => {
                    self.print_output.push(format!("[TEST] ❌ Decryption failed: {}", e));
                    // Save error details
                    self.print_output.push(format!("[TEST] Error details: {:?}", e));
                    return Err(anyhow::anyhow!("Failed to decrypt test model: {}", e));
                }
            }
        } else {
            // No encryption, just rename
            if let Err(e) = fs::rename(&temp_path, &final_path) {
                return Err(anyhow::anyhow!("Failed to rename file: {}", e));
            }
            self.print_output.push(format!("[TEST] File saved to: {}", final_path.display()));
        }

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

    /// Test decryption and whitelist removal without printer connection
    pub async fn handle_test_decryption(app: Arc<Mutex<App>>) -> Result<()> {
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
                        app.print_output.push(format!("=== TEST MODE: {} ===", item.alias));
                        app.print_output.push("[TEST] Starting decryption test...".to_string());
                    }
                    
                    // Download and decrypt to test folder
                    let download_result = {
                        let mut app = app_clone.lock().await;
                        app.download_3d_model_test(&item.blob_id, item.seal_resource_id.as_deref()).await
                    };

                    if let Err(e) = download_result {
                        let mut app = app_clone.lock().await;
                        app.set_message(crate::app::MessageType::Error, format!("Test download failed: {}", e));
                        return;
                    }
                    
                    {
                        let mut app = app_clone.lock().await;
                        app.print_output.push("[TEST] Model downloaded and decrypted successfully".to_string());
                        app.print_output.push("[TEST] Starting slicing test...".to_string());
                    }
                    
                    // Run slicing test
                    let slice_result = App::run_slice_test(Arc::clone(&app_clone)).await;
                    
                    match slice_result {
                        Ok(_) => {
                            let mut app = app_clone.lock().await;
                            app.print_output.push("[TEST] ✅ Slicing completed successfully".to_string());
                            app.print_output.push("[TEST] Starting whitelist removal test...".to_string());
                        }
                        Err(e) => {
                            let mut app = app_clone.lock().await;
                            app.print_output.push(format!("[TEST] ❌ Slicing failed: {}", e));
                            app.set_message(crate::app::MessageType::Error, format!("Test slicing failed: {}", e));
                            return;
                        }
                    }
                    
                    // Test whitelist removal
                    {
                        let mut app = app_clone.lock().await;
                        match app.test_complete_print_job().await {
                            Ok(_) => {
                                app.print_output.push("[TEST] ✅ Whitelist removal called successfully".to_string());
                                app.print_output.push("[TEST] === Test completed ===".to_string());
                                app.set_message(crate::app::MessageType::Success, "Test completed successfully".to_string());
                            }
                            Err(e) => {
                                app.print_output.push(format!("[TEST] ❌ Whitelist removal failed: {}", e));
                                app.set_message(crate::app::MessageType::Error, format!("Test whitelist removal failed: {}", e));
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
}
