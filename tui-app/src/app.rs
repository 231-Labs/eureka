use ratatui::widgets::ListState;
use crate::wallet::{Wallet, SculptItem};
use crate::utils::{setup_for_read, shorten_id, NetworkState};
use crate::constants::{NETWORKS, AGGREGATOR_URL};
use anyhow::Result;
use crate::transactions::TransactionBuilder;
use sui_sdk::types::base_types::ObjectID;
use sui_sdk::SuiClient;
use futures;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{BufReader, AsyncBufReadExt};
use std::fs;

#[derive(Clone)]
#[allow(dead_code)]
pub enum TaskStatus {
    Printing(u8),
    Completed,
}

#[derive(Clone)]
pub struct PrintTask {
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
}

#[derive(Clone, PartialEq)]
pub enum RegistrationStatus {
    Inputting,
    Submitting,
    Success(String),  // Contains printer_id
    Failed(String),   // Contains error message
}

#[derive(Clone, PartialEq)]
pub enum MessageType {
    Error,
    Info,
    Success,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptStatus {
    Idle,
    Running,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrintStatus {
    Idle,
    // Printing(u8),  // Progress percentage
    Completed,
    // Paused,
    Error(String),
}

#[derive(Clone)]
pub struct App {
    pub sui_client: Arc<SuiClient>,
    pub wallet: Wallet,
    pub wallet_address: String,
    pub printer_id: String,
    pub is_online: bool,
    pub sculpt_state: ListState,
    pub tasks: Vec<PrintTask>,
    pub tasks_state: ListState,
    pub is_confirming: bool,
    pub is_harvesting: bool,
    pub is_switching_network: bool,
    pub harvestable_rewards: String,
    pub sui_balance: u128,
    pub wal_balance: u128,
    pub network_state: NetworkState,
    pub error_message: Option<String>,
    pub message_type: MessageType,
    pub is_registering_printer: bool,
    pub printer_alias: String,
    pub printer_registration_message: String,
    pub registration_status: RegistrationStatus,
    pub sculpt_items: Vec<SculptItem>,
    pub script_status: ScriptStatus,
    pub print_status: PrintStatus,
    pub success_message: Option<String>,
    pub print_output: Vec<String>,  // 新增：存儲列印輸出
}

impl App {
    pub async fn new() -> Result<App> {
        let network_state = NetworkState::new();
        
        // Initialize SuiClient
        let (client, address) = setup_for_read(&network_state).await?;
        let sui_client = Arc::new(client);
        
        // Initialize Wallet
        let wallet = Wallet::new(&network_state, Arc::clone(&sui_client), address).await;
        let wallet_address = shorten_id(&wallet.get_active_address().await?.to_string());
        
        // Get balance and printer id
        let sui_balance = wallet.get_sui_balance(wallet.get_active_address().await?).await?;
        let wal_balance = wallet.get_walrus_balance(wallet.get_active_address().await?).await?;
        let printer_id = match wallet.get_user_printer_id(wallet.get_active_address().await?).await {
            Ok(id) => id,
            Err(_e) => {
                "No Printer ID".to_string()
            }
        };
        
        // Get Sculpt items
        let sculpt_items = match wallet.get_user_sculpt(wallet.get_active_address().await?).await {
            Ok(items) => items,
            Err(_) => vec![SculptItem {
                alias: "Error loading models".to_string(),
                blob_id: String::new(),
                printed_count: 0,
            }]
        };
        
        let mut app = App {
            sui_client,
            wallet,
            wallet_address,
            printer_id: printer_id.clone(),
            is_online: false,
            sculpt_state: ListState::default(),
            tasks: Vec::new(),
            tasks_state: ListState::default(),
            is_confirming: false,
            is_harvesting: false,
            is_switching_network: false,
            harvestable_rewards: "100.0 SUI".to_string(),
            sui_balance,
            wal_balance,
            network_state,
            error_message: None,
            message_type: MessageType::Info,
            is_registering_printer: false,
            printer_alias: String::new(),
            printer_registration_message: String::new(),
            registration_status: RegistrationStatus::Inputting,
            sculpt_items,
            script_status: ScriptStatus::Idle,
            print_status: PrintStatus::Idle,
            success_message: None,
            print_output: Vec::new(),  // 初始化輸出列表
        };
        
        // Check if printer registration is needed
        if printer_id == "No Printer ID" {
            app.is_registering_printer = true;
            app.printer_registration_message = "Welcome to Eureka 3D Printing Platform!\n\nNo printer found. Please register your printer to continue.\n\nEnter your printer alias:".to_string();
        }
        
        // Set initial selection
        app.sculpt_state.select(Some(0));
        app.tasks_state.select(Some(0));
        Ok(app)
    }

    pub fn start_toggle_confirm(&mut self) {
        self.is_confirming = true;
    }

    pub async fn confirm_toggle(&mut self) -> Result<()> {
        self.is_online = !self.is_online;
        self.is_confirming = false;

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

    pub fn start_harvest_confirm(&mut self) {
        self.is_harvesting = true;
    }

    pub fn confirm_harvest(&mut self) {
        self.is_harvesting = false;
        // TODO: 實際執行 harvest 邏輯
        self.success_message = Some("Harvest completed successfully!".to_string());
    }

    pub fn cancel_harvest(&mut self) {
        self.is_harvesting = false;
    }

    pub fn next_item(&mut self) {
        let items_len = if self.is_online {
            self.tasks.len()
        } else {
            self.sculpt_items.len()
        };

        if items_len == 0 {
            return;
        }

        if self.is_online {
            let i = match self.tasks_state.selected() {
                Some(i) => {
                    if i >= items_len - 1 {
                        i
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.tasks_state.select(Some(i));
        } else {
            let i = match self.sculpt_state.selected() {
                Some(i) => {
                    if i >= items_len - 1 {
                        i
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.sculpt_state.select(Some(i));
        }
    }

    pub fn previous_item(&mut self) {
        let items_len = if self.is_online {
            self.tasks.len()
        } else {
            self.sculpt_items.len()
        };

        if items_len == 0 {
            return;
        }

        if self.is_online {
            let i = match self.tasks_state.selected() {
                Some(i) => {
                    if i == 0 {
                        0
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.tasks_state.select(Some(i));
        } else {
            let i = match self.sculpt_state.selected() {
                Some(i) => {
                    if i == 0 {
                        0
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.sculpt_state.select(Some(i));
        }
    }

    #[allow(dead_code)]
    pub fn switch_network(&mut self) {
        self.network_state.next_network();
        // 觸發網路更新
        if let Err(e) = futures::executor::block_on(self.update_network()) {
            self.error_message = Some(e.to_string());
        }
    }

    pub async fn update_network(&mut self) -> Result<()> {
        self.error_message = None;  // Clear previous error message
        
        match self.do_update_network().await {
            Ok(_) => {
                // 重置選擇狀態
                if !self.sculpt_items.is_empty() {
                    self.sculpt_state.select(Some(0));
                }
                Ok(())
            },
            Err(e) => {
                self.error_message = Some(e.to_string());  // Store error message
                Ok(())  // Don't interrupt program execution
            }
        }
    }

    async fn do_update_network(&mut self) -> Result<()> {
        // 獲取新的 SuiClient 和地址
        let (client, address) = setup_for_read(&self.network_state).await?;
        self.sui_client = Arc::new(client);
        
        // 更新 Wallet
        self.wallet = Wallet::new(&self.network_state, Arc::clone(&self.sui_client), address).await;
        self.wallet_address = shorten_id(&self.wallet.get_active_address().await?.to_string());
        self.sui_balance = self.wallet.get_sui_balance(self.wallet.get_active_address().await?).await?;
        self.wal_balance = self.wallet.get_walrus_balance(self.wallet.get_active_address().await?).await?;
        self.printer_id = self.wallet.get_user_printer_id(self.wallet.get_active_address().await?).await?;
        self.sculpt_items = self.wallet.get_user_sculpt(self.wallet.get_active_address().await?).await?;
        
        Ok(())
    }

    pub fn start_network_switch(&mut self) {
        self.is_switching_network = true;
    }

    pub fn cancel_network_switch(&mut self) {
        self.is_switching_network = false;
    }

    pub fn switch_to_network(&mut self, network_index: usize) {
        if network_index < NETWORKS.len() {
            self.network_state.current_network = network_index;
        }
        self.is_switching_network = false;
    }

    pub fn get_network_options(&self) -> String {
        format!("1) {}  2) {}  3) {}", 
            NETWORKS[2].0.to_uppercase(),  // MAINNET
            NETWORKS[0].0.to_uppercase(),  // TESTNET
            NETWORKS[1].0.to_uppercase()   // DEVNET
        )
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
        self.success_message = None;
    }

    // 設置訊息的方法
    pub fn set_message(&mut self, message_type: MessageType, message: String) {
        self.message_type = message_type.clone();
        match message_type {
            MessageType::Error => {
                self.error_message = Some(message);
                self.success_message = None;
            }
            MessageType::Success => {
                self.success_message = Some(message);
                self.error_message = None;
            }
            MessageType::Info => {
                self.error_message = Some(message);
                self.success_message = None;
            }
        }
    }

    // printer registration
    pub async fn handle_printer_registration_input(&mut self, input: char) -> Result<()> {
        match input {
            '\n' => {
                if !self.printer_alias.is_empty() && self.registration_status == RegistrationStatus::Inputting {
                    self.registration_status = RegistrationStatus::Submitting;
                    self.printer_registration_message = "Sending transaction to network...\nPlease wait...".to_string();
                    
                    let builder = TransactionBuilder::new(
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
                            self.printer_id = tx_digest.clone();
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
                    // Only exit registration page when Enter is pressed in success state
                    self.is_registering_printer = false;
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

    pub async fn download_3d_model(&mut self, blob_id: &str) -> Result<()> {
        let url = format!("{}/v1/blobs/{}", AGGREGATOR_URL, blob_id);
        let temp_path = "test.stl";
        let final_path = "Gcode-Transmit/test.stl";
        
        // 先下載到臨時文件
        let status = tokio::process::Command::new("curl")
            .arg("-s")
            .arg("-S")
            .arg(&url)
            .arg("-o")
            .arg(temp_path)
            .status()
            .await?;

        if !status.success() {
            self.set_message(MessageType::Error, "Failed to download 3D model".to_string());
            return Err(anyhow::anyhow!("Failed to download 3D model"));
        }

        // 移動文件到目標目錄
        if let Err(e) = fs::rename(temp_path, final_path) {
            self.set_message(MessageType::Error, format!("Failed to move 3D model: {}", e));
            return Err(anyhow::anyhow!("Failed to move 3D model: {}", e));
        }
        self.set_message(MessageType::Success, "3D model downloaded successfully".to_string());
        Ok(())
    }

    pub fn get_tech_animation(&self) -> String {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let frame = (time % 3) as usize;

        // 只顯示一種狀態：優先顯示 print_status
        match &self.print_status {
            PrintStatus::Idle => {
                // 檢查是否在停止過程中
                if matches!(self.message_type, MessageType::Info) && 
                   self.error_message.as_ref().map_or(false, |msg| msg.contains("Stopping print")) {
                    return "║▒▓░ STOPPING PRINT... ░▓▒║".to_string();
                }

                // 沒有列印時才顯示 script_status
                match self.script_status {
                    ScriptStatus::Idle => {
                        match frame {
                            0 => "║▓▒░ SYS IDLE ░▒▓║".to_string(),
                            1 => "║▒▓░ SYS IDLE ░▓▒║".to_string(),
                            _ => "║░▓▒ SYS IDLE ▒▓░║".to_string(),
                        }
                    },
                    ScriptStatus::Running => "║▒▓░ SCRIPT RUNNING ░▓▒║".to_string(),
                    ScriptStatus::Completed => "║▓▒░ SCRIPT COMPLETE ░▒▓║".to_string(),
                    ScriptStatus::Failed(_) => "║▒▓░ SCRIPT ERROR ░▓▒║".to_string(),
                }
            }
            // PrintStatus::Printing(progress) => format!("║▒▓░ PRINTING {}% ░▓▒║", progress),
            PrintStatus::Completed => "║▓▒░ PRINT COMPLETE ░▒▓║".to_string(),
            PrintStatus::Error(_) => "║▒▓░ PRINTER ERROR ░▓▒║".to_string(),
        }
    }

    pub fn clear_print_output(&mut self) {
        self.print_output.clear();
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
        // 立即顯示停止狀態
        self.set_message(MessageType::Info, "Stopping print...".to_string());
        
        // 使用 spawn 啟動命令，捕捉輸出以便顯示
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

        // 處理輸出
        if output.status.success() {
            // 從輸出中獲取訊息
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // 重置狀態
            self.script_status = ScriptStatus::Idle;
            self.print_status = PrintStatus::Idle;
            self.set_message(MessageType::Success, 
                if stdout.is_empty() { "Print stopped successfully".to_string() } else { stdout }
            );
        } else {
            // 處理錯誤
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

    pub async fn handle_model_selection(app: Arc<Mutex<App>>, download_only: bool) -> Result<()> {
        let app_clone = Arc::clone(&app);
        tokio::spawn(async move {
            // 獲取選擇的模型
            let selected_item = {
                let app_guard = app_clone.lock().await;
                app_guard.sculpt_state
                    .selected()
                    .and_then(|idx| app_guard.sculpt_items.get(idx).cloned())
            };

            // 處理選擇的模型
            if let Some(item) = selected_item {
                if item.alias != "No printable models found" {
                    {
                        let mut app = app_clone.lock().await;
                        app.print_output.push(format!("[LOG] Selected model: {}", item.alias));
                    }
                    
                    // 下載模型
                    let download_result = {
                        let mut app = app_clone.lock().await;
                        app.download_3d_model(&item.blob_id).await
                    };

                    // 處理下載結果
                    if let Err(e) = download_result {
                        let mut app = app_clone.lock().await;
                        app.set_message(MessageType::Error, format!("Failed to download model: {}", e));
                        return;
                    }

                    // 執行列印腳本（如果不是只下載）
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

    // pub async fn get_wallet_balance(&self) -> Result<u128> {
    //     if let Some(wallet) = &self.wallet {
    //         let address = wallet.get_active_address().await?;
    //         wallet.get_sui_balance(address).await
    //     } else {
    //         Err(anyhow::anyhow!("Wallet not initialized"))
    //     }
    // }

    // pub async fn update_wallet_address(&mut self) -> Result<()> {
    //     if let Some(wallet) = &self.wallet {
    //         self.wallet_address = wallet.get_active_address().await?.to_string();
    //         Ok(())
    //     } else {
    //         Err(anyhow::anyhow!("Wallet not initialized"))
    //     }
    // }
} 
