use crate::app::core::App;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};

impl App {
    // Gcode file monitoring implementation
    pub async fn setup_gcode_monitoring(app: Arc<Mutex<App>>) {
        let app_clone_for_monitor = Arc::clone(&app);
        let _gcode_monitor_handle = tokio::spawn(async move {
            // According to the Gcode-Send.sh script, the Gcode file is generated in the main directory
            let current_dir = std::env::current_dir().unwrap_or_default();
            let gcode_path = current_dir.join("Gcode-Transmit").join("main").join("test.gcode");
            
            let mut app_lock = app_clone_for_monitor.lock().await;
            app_lock.print_output.push(format!("[GCODE] Monitoring file: {}", gcode_path.display()));
            drop(app_lock); // Release lock to avoid long-term holding
            
            // Check the file every 500 milliseconds
            let mut last_size = 0;
            let mut attempts = 0;
            while !gcode_path.exists() && attempts < 40 {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                attempts += 1;
            }
            
            if !gcode_path.exists() {
                let mut app_lock = app_clone_for_monitor.lock().await;
                app_lock.print_output.push("[GCODE] No gcode file found. Cannot display gcode commands.".to_string());
                return;
            }
            
            // Read and display the incremental content of the Gcode file
            loop {
                match tokio::fs::metadata(&gcode_path).await {
                    Ok(metadata) => {
                        let current_size = metadata.len() as usize;
                        if current_size > last_size {
                            // Open the file and read the incremental part
                            if let Ok(file) = tokio::fs::File::open(&gcode_path).await {
                                let mut reader = tokio::io::BufReader::new(file);
                                reader.seek(SeekFrom::Start(last_size as u64)).await.ok();
                                
                                let mut buffer = String::new();
                                if reader.read_to_string(&mut buffer).await.is_ok() && !buffer.is_empty() {
                                    // Output the Gcode commands line by line to the log
                                    let mut app_lock = app_clone_for_monitor.lock().await;
                                    for line in buffer.lines() {
                                        if line.starts_with('G') || line.starts_with('M') {
                                            app_lock.print_output.push(format!("[GCODE] {}", line));
                                        }
                                    }
                                }
                            }
                            last_size = current_size;
                        }
                    }
                    Err(_) => break,
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });
    }
    
    // clear print log
    #[inline]
    pub fn clear_print_log(&mut self) {
        self.print_output.clear();
    }
} 