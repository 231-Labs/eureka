use crate::app::core::App;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use std::path::Path;

impl App {
    // read file chunk
    async fn read_file_chunk(path: &Path, start: usize, end: usize) -> Option<String> {
        let mut file = match tokio::fs::File::open(path).await {
            Ok(file) => file,
            Err(_) => return None,
        };
        
        // move file pointer to start position
        if let Err(_) = file.seek(SeekFrom::Start(start as u64)).await {
            return None;
        }
        
        // create buffer and read content
        let mut buffer = vec![0; end - start];
        match file.read_exact(&mut buffer).await {
            Ok(_) => String::from_utf8(buffer).ok(),
            Err(_) => None,
        }
    }

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
                // try to get file metadata, if error, break loop
                let metadata = match tokio::fs::metadata(&gcode_path).await {
                    Ok(metadata) => metadata,
                    Err(_) => break,
                };
                
                let current_size = metadata.len() as usize;
                
                // if file size is not changed, skip this iteration
                if current_size <= last_size {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    continue;
                }
                
                // read new content
                if let Some(new_content) = Self::read_file_chunk(&gcode_path, last_size, current_size).await {
                    // update recorded file size
                    last_size = current_size;
                    
                    // process and record G-code commands
                    let mut app_lock = app_clone_for_monitor.lock().await;
                    for line in new_content.lines() {
                        if line.starts_with('G') || line.starts_with('M') {
                            app_lock.print_output.push(format!("[GCODE] {}", line));
                        }
                    }
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