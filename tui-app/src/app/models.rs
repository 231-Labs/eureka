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
                        }
                        
                        let success = App::run_print_script(Arc::clone(&app_clone)).await;
                        
                        let mut app = app_clone.lock().await;
                        if success {
                            app.print_output.push("[LOG] run_print_script executed successfully".to_string());
                        } else {
                            app.print_output.push("[LOG] run_print_script executed failed".to_string());
                        }
                    }
                }
            }
        });
        Ok(())
    }
}
