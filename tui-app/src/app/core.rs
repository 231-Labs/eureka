use ratatui::widgets::ListState;
use crate::wallet::{Wallet, SculptItem, PrinterInfo};
use crate::utils::{setup_for_read, shorten_id, NetworkState};
use anyhow::Result;
use sui_sdk::SuiClient;
use std::sync::Arc;
use std::vec::Vec;
use super::print_job::{PrintTask};

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
    Printing,
    Completed,
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
    pub print_output: Vec<String>,  // 存儲列印輸出
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
        let printer_info = match wallet.get_printer_info(wallet.get_active_address().await?).await {
            Ok(info) => {
                info
            },
            Err(_e  ) => {
                PrinterInfo {
                    id: "No Printer ID".to_string(),
                    pool_balance: 0,
                }
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
        
        // format pool balance to SUI
        let pool_balance_formatted = if printer_info.pool_balance > 0 {
            format!("{:.2} SUI", printer_info.pool_balance as f64 / 1_000_000_000.0)
        } else {
            "0.00 SUI".to_string()
        };
        
        let mut app = App {
            sui_client,
            wallet,
            wallet_address,
            printer_id: printer_info.id.clone(),
            is_online: false,
            sculpt_state: ListState::default(),
            tasks: PrintTask::new_mock_tasks(),
            tasks_state: ListState::default(),
            is_confirming: false,
            is_harvesting: false,
            is_switching_network: false,
            harvestable_rewards: pool_balance_formatted,
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
            print_output: Vec::new(),  // initialize output list
        };
        
        // Check if printer registration is needed
        if printer_info.id == "No Printer ID" {
            app.is_registering_printer = true;
            app.printer_registration_message = "Welcome to Eureka 3D Printing Platform!\n\nNo printer found. Please register your printer to continue.\n\nEnter your printer alias:".to_string();
        }
        
        // Set initial selection
        app.sculpt_state.select(Some(0));
        app.tasks_state.select(Some(0));
        Ok(app)
    }

    // clear error and success message
    pub fn clear_error(&mut self) {
        self.error_message = None;
        self.success_message = None;
    }

    // set message method
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

    pub fn clear_print_output(&mut self) {
        self.print_output.clear();
    }

    pub async fn update_basic_info(&mut self) -> Result<()> {
        // try to get latest info from blockchain
        let address = self.wallet.get_active_address().await?;
        
        // get basic balance info
        self.sui_balance = self.wallet.get_sui_balance(address).await?;
        self.wal_balance = self.wallet.get_walrus_balance(address).await?;
        
        // get printer info
        match self.wallet.get_printer_info(address).await {
            Ok(info) => {
                // println!("Successfully got printer ID: {}", info.id);
                self.printer_id = info.id.clone();

                if info.pool_balance > 0 {
                    self.harvestable_rewards = format!("{:.2} SUI", info.pool_balance as f64 / 1_000_000_000.0);
                } else {
                    self.harvestable_rewards = "0.00 SUI".to_string();
                }
            }
            Err(e) => {
                // println!("Failed to get printer ID: {}", e);
                self.set_message(MessageType::Error, format!("Failed to get printer ID: {}", e));
            }
        }
        
        // get available models
        match self.wallet.get_user_sculpt(address).await {
            Ok(items) => {
                self.sculpt_items = items;
                if !self.sculpt_items.is_empty() {
                    self.sculpt_state.select(Some(0));
                }
            }
            Err(e) => {
                // println!("Failed to load 3D models: {}", e);
                self.set_message(MessageType::Error, format!("Failed to load 3D models: {}", e));
            }
        }
        
        Ok(())
    }

    pub async fn update_print_tasks(&mut self) -> Result<()> {
        if self.is_online && self.printer_id != "No Printer ID" {
            // 獲取當前活動的打印任務
            match self.wallet.get_active_print_job(&self.printer_id).await {
                Ok(Some(task)) => {
                    // 檢查是否已經有這個任務
                    let task_exists = self.tasks.iter().any(|t| t.id == task.id);
                    
                    if !task_exists {
                        // 如果是新任務，添加到任務列表的開頭
                        self.tasks.insert(0, task.clone());
                        // 確保選中最新的任務
                        self.tasks_state.select(Some(0));
                        // 新任務默認為待機狀態
                        self.print_status = PrintStatus::Idle;
                        self.script_status = ScriptStatus::Idle;
                    } else {
                        // 如果任務已存在，更新其狀態
                        if let Some(existing_task) = self.tasks.iter_mut().find(|t| t.id == task.id) {
                            *existing_task = task.clone();
                            // 只有在腳本正在運行時才設置為打印狀態
                            if matches!(self.script_status, ScriptStatus::Running) {
                                self.print_status = PrintStatus::Printing;
                            }
                        }
                    }
                }
                Ok(None) => {
                    // 如果沒有活動任務，設置打印機為空閒狀態
                    self.print_status = PrintStatus::Idle;
                    self.script_status = ScriptStatus::Idle;
                }
                Err(e) => {
                    println!("獲取打印任務時出錯：{:?}", e);
                    self.set_message(MessageType::Error, format!("獲取打印任務失敗：{}", e));
                }
            }
        }
        Ok(())
    }
}
