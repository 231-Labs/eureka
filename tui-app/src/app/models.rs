use crate::app::core::App;
use crate::constants::AGGREGATOR_URL;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::fs;

impl App {
    pub async fn download_3d_model(&mut self, blob_id: &str) -> Result<()> {
        let url = format!("{}/v1/blobs/{}", AGGREGATOR_URL, blob_id);
        let temp_path = "test.stl";
        let final_path = "Gcode-Transmit/test.stl";
        
        // download to temporary file
        let status = tokio::process::Command::new("curl")
            .arg("-s")
            .arg("-S")
            .arg(&url)
            .arg("-o")
            .arg(temp_path)
            .status()
            .await?;

        if !status.success() {
            self.set_message(crate::app::MessageType::Error, "Failed to download 3D model".to_string());
            return Err(anyhow::anyhow!("Failed to download 3D model"));
        }

        // move file to target directory
        if let Err(e) = fs::rename(temp_path, final_path) {
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
                        {
                            let mut app = app_clone.lock().await;
                            app.print_output.push("[LOG] Preparing to run_print_script".to_string());
                            // 無論打印腳本是否成功執行，都設置為打印狀態
                            app.print_status = crate::app::PrintStatus::Printing;
                        }
                        
                        let print_result = App::run_print_script(Arc::clone(&app_clone)).await;
                        
                        let mut app = app_clone.lock().await;
                        match print_result {
                            Ok(_) => {
                                app.print_output.push("[LOG] run_print_script executed successfully".to_string());
                            },
                            Err(error_msg) => {
                                app.print_output.push(format!("[LOG] run_print_script failed: {}", error_msg));
                                app.set_message(crate::app::MessageType::Error, error_msg);
                                // 即使執行失敗，仍然保持打印狀態
                                app.print_status = crate::app::PrintStatus::Printing;
                            }
                        }
                    }
                }
            }
        });
        Ok(())
    }

    // 處理在線模式下的打印任務
    pub async fn handle_task_print(app: Arc<Mutex<App>>, download_only: bool) -> Result<()> {
        let app_clone = Arc::clone(&app);
        tokio::spawn(async move {
            // 嘗試獲取活動打印任務
            let active_task = {
                let app_guard = app_clone.lock().await;
                app_guard.tasks.iter()
                    .find(|t| matches!(t.status, crate::app::print_job::TaskStatus::Printing))
                    .cloned()
            };

            // 處理活動任務
            if let Some(task) = active_task {
                {
                    let mut app = app_clone.lock().await;
                    app.print_output.push(format!("[LOG] Processing active task: {}", task.name));
                    app.print_output.push(format!("[LOG] Sculpt structure (blob_id): {}", task.sculpt_structure));
                    app.set_message(crate::app::MessageType::Info, format!("Processing print job: {}", task.name));
                }
                
                // 下載模型
                let download_result = {
                    let mut app = app_clone.lock().await;
                    app.download_3d_model(&task.sculpt_structure).await
                };

                // 處理下載結果
                if let Err(e) = download_result {
                    let mut app = app_clone.lock().await;
                    app.set_message(crate::app::MessageType::Error, format!("Failed to download task model: {}", e));
                    return;
                }

                // 執行打印腳本
                if !download_only {
                    {
                        let mut app = app_clone.lock().await;
                        app.print_output.push("[LOG] Preparing to run print script for task".to_string());
                        // 無論打印腳本是否成功執行，都設置為打印狀態
                        app.print_status = crate::app::PrintStatus::Printing;
                    }
                    
                    let print_result = App::run_print_script(Arc::clone(&app_clone)).await;
                    
                    let mut app = app_clone.lock().await;
                    match print_result {
                        Ok(_) => {
                            app.print_output.push("[LOG] Task print script executed successfully".to_string());
                            app.set_message(crate::app::MessageType::Success, "Print job started successfully!".to_string());
                            // 更新區塊鏈上的任務狀態（TODO：實現此功能）
                            // app.update_task_status_on_blockchain(&task.id).await.ok();
                        },
                        Err(error_msg) => {
                            app.print_output.push(format!("[LOG] Task print script failed: {}", error_msg));
                            app.set_message(crate::app::MessageType::Error, format!("Failed to start print job: {}", error_msg));
                            // 即使執行失敗，仍然保持打印狀態
                            app.print_status = crate::app::PrintStatus::Printing;
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
