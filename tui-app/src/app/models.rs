use crate::app::core::App;
use crate::constants::AGGREGATOR_URL;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::fs;
use std::path::Path;

impl App {
    pub async fn download_3d_model(&mut self, blob_id: &str) -> Result<()> {
        let url = format!("{}/v1/blobs/{}", AGGREGATOR_URL, blob_id);
        
        // Get current directory
        let current_dir = std::env::current_dir()?;
        let temp_path = current_dir.join("test.stl");
        let final_path = current_dir.join("Gcode-Transmit").join("test.stl");
        
        self.print_output.push(format!("[LOG] Downloading model from: {}", url));
        
        // Ensure target directory exists
        let gcode_dir = current_dir.join("Gcode-Transmit");
        if !Path::new(&gcode_dir).exists() {
            self.print_output.push(format!("[LOG] Creating directory: {}", gcode_dir.display()));
            fs::create_dir_all(&gcode_dir)?;
        }
        
        // download to temporary file
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

        // move file to target directory
        if let Err(e) = fs::rename(&temp_path, &final_path) {
            self.set_message(crate::app::MessageType::Error, format!("Failed to move 3D model: {}", e));
            return Err(anyhow::anyhow!("Failed to move 3D model: {}", e));
        }
        self.set_message(crate::app::MessageType::Success, "3D model downloaded successfully".to_string());
        Ok(())
    }

    pub async fn handle_model_selection(app: Arc<Mutex<App>>, download_only: bool) -> Result<()> {
        let app_clone = Arc::clone(&app);
        tokio::spawn(async move {
            // get selected model
            let selected_item = {
                let app_guard = app_clone.lock().await;
                app_guard.sculpt_state
                    .selected()
                    .and_then(|idx| app_guard.sculpt_items.get(idx).cloned())
            };

            // process selected model
            if let Some(item) = selected_item {
                if item.alias != "No printable models found" {
                    {
                        let mut app = app_clone.lock().await;
                        app.print_output.push(format!("[LOG] Selected model: {}", item.alias));
                    }
                    
                    // download model
                    let download_result = {
                        let mut app = app_clone.lock().await;
                        app.download_3d_model(&item.blob_id).await
                    };

                    // process download result
                    if let Err(e) = download_result {
                        let mut app = app_clone.lock().await;
                        app.set_message(crate::app::MessageType::Error, format!("Failed to download model: {}", e));
                        return;
                    }

                    // run print script (not only download)
                    if !download_only {
                        // create print job on blockchain
                        {
                            let mut app = app_clone.lock().await;
                            
                            // only create print job on blockchain if printer_id is not "No Printer ID"
                            if app.printer_id != "No Printer ID" {
                                app.print_output.push("[LOG] Creating print job on blockchain...".to_string());
                                
                                match app.test_create_print_job().await {
                                    Ok(_) => {
                                        app.print_output.push("[LOG] Print job created on blockchain successfully".to_string());
                                        app
                                        .set_message(crate::app::MessageType::Success, "Print job created on blockchain successfully"
                                        .to_string());
                                    },
                                    Err(e) => {
                                        // if error is because print job already exists, continue printing, otherwise report error
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

    // Handle print tasks in online mode
    pub async fn handle_task_print(app: Arc<Mutex<App>>, download_only: bool) -> Result<()> {
        let app_clone = Arc::clone(&app);
        tokio::spawn(async move {
            // Try to get active print task
            let active_task = {
                let app_guard = app_clone.lock().await;
                app_guard.tasks.iter()
                    .find(|t| matches!(t.status, crate::app::print_job::TaskStatus::Active))
                    .cloned()
            };

            // Process active task
            if let Some(task) = active_task {
                {
                    let mut app = app_clone.lock().await;
                    app.print_output.push(format!("[LOG] Processing active task: {}", task.name));
                    app.print_output.push(format!("[LOG] Sculpt structure (blob_id): {}", task.sculpt_structure));
                    app.set_message(crate::app::MessageType::Info, format!("Processing print job: {}", task.name));
                }
                
                // Download model
                let download_result = {
                    let mut app = app_clone.lock().await;
                    app.download_3d_model(&task.sculpt_structure).await
                };

                // Handle download result
                if let Err(e) = download_result {
                    let mut app = app_clone.lock().await;
                    app.set_message(crate::app::MessageType::Error, format!("Failed to download task model: {}", e));
                    return;
                }

                // Run print script
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
