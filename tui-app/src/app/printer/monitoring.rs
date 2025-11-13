use crate::app::core::App;
use crate::constants::{GCODE_CHECK_INTERVAL_MILLIS, GCODE_WAIT_ATTEMPTS};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use std::path::Path;

impl App {
    async fn read_file_chunk(path: &Path, start: usize, end: usize) -> Option<String> {
        let mut file = match tokio::fs::File::open(path).await {
            Ok(file) => file,
            Err(_) => return None,
        };
        
        if let Err(_) = file.seek(SeekFrom::Start(start as u64)).await {
            return None;
        }
        
        let mut buffer = vec![0; end - start];
        match file.read_exact(&mut buffer).await {
            Ok(_) => String::from_utf8(buffer).ok(),
            Err(_) => None,
        }
    }

    pub async fn setup_gcode_monitoring(app: Arc<Mutex<App>>) {
        let app_clone_for_monitor = Arc::clone(&app);
        let _gcode_monitor_handle = tokio::spawn(async move {
            let current_dir = std::env::current_dir().unwrap_or_default();
            let gcode_path = current_dir.join("Gcode-Transmit").join("main").join("test.gcode");
            
            let mut app_lock = app_clone_for_monitor.lock().await;
            app_lock.print_output.push(format!("[GCODE] Monitoring file: {}", gcode_path.display()));
            drop(app_lock);
            
            let mut last_size = 0;
            let mut attempts = 0;
            while !gcode_path.exists() && attempts < GCODE_WAIT_ATTEMPTS {
                tokio::time::sleep(tokio::time::Duration::from_millis(GCODE_CHECK_INTERVAL_MILLIS)).await;
                attempts += 1;
            }
            
            if !gcode_path.exists() {
                let mut app_lock = app_clone_for_monitor.lock().await;
                app_lock.print_output.push("[GCODE] No gcode file found. Cannot display gcode commands.".to_string());
                return;
            }
            
            loop {
                let metadata = match tokio::fs::metadata(&gcode_path).await {
                    Ok(metadata) => metadata,
                    Err(_) => break,
                };
                
                let current_size = metadata.len() as usize;
                
                    if current_size <= last_size {
                        tokio::time::sleep(tokio::time::Duration::from_millis(GCODE_CHECK_INTERVAL_MILLIS)).await;
                        continue;
                    }
                
                if let Some(new_content) = Self::read_file_chunk(&gcode_path, last_size, current_size).await {
                    last_size = current_size;
                    
                    let mut app_lock = app_clone_for_monitor.lock().await;
                    for line in new_content.lines() {
                        if line.starts_with('G') || line.starts_with('M') {
                            app_lock.print_output.push(format!("[GCODE] {}", line));
                        }
                        }
                    }
                    
                    tokio::time::sleep(tokio::time::Duration::from_millis(GCODE_CHECK_INTERVAL_MILLIS)).await;
            }
        });
    }
    
    #[inline]
    pub fn clear_print_log(&mut self) {
        self.print_output.clear();
    }
} 