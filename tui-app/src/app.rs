use ratatui::widgets::ListState;
use crate::wallet::{Wallet, BottegaItem};
use crate::utils::{NetworkState, shorten_id};
use crate::constants::{NETWORKS, AGGREGATOR_URL};
use anyhow::Result;
use crate::transactions::TransactionBuilder;
use sui_sdk::types::base_types::ObjectID;
use std::process::Command;
use futures;
use std::path::PathBuf;

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
}

pub struct App {
    pub wallet: Wallet,
    pub wallet_address: String,
    pub printer_id: String,
    pub is_online: bool,
    pub bottega_state: ListState,
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
    pub nozzle_temp: f32,
    pub bed_temp: f32,
    pub is_registering_printer: bool,
    pub printer_alias: String,
    pub printer_registration_message: String,
    pub registration_status: RegistrationStatus,
    pub bottega_items: Vec<BottegaItem>,
}

impl App {
    pub async fn new() -> Result<App> {
        let network_state = NetworkState::new();
        let wallet = Wallet::new(&network_state).await?;
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
        
        // 獲取 bottega 項目
        let bottega_items = match wallet.get_user_bottega(wallet.get_active_address().await?).await {
            Ok(items) => items,
            Err(_) => vec![BottegaItem {
                name: "Error loading models".to_string(),
                blob_id: String::new(),
                printed_count: 0,
            }]
        };
        
        let mut app = App {
            wallet,
            wallet_address,
            printer_id: printer_id.clone(),
            is_online: false,
            bottega_state: ListState::default(),
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
            nozzle_temp: 0.0,
            bed_temp: 0.0,
            is_registering_printer: false,
            printer_alias: String::new(),
            printer_registration_message: String::new(),
            registration_status: RegistrationStatus::Inputting,
            bottega_items,
        };
        
        // Check if printer registration is needed
        if printer_id == "No Printer ID" {
            app.is_registering_printer = true;
            app.printer_registration_message = "Welcome to Eureka 3D Printing Platform!\n\nNo printer found. Please register your printer to continue.\n\nEnter your printer alias:".to_string();
        }
        
        // Set initial selection
        app.bottega_state.select(Some(0));
        app.tasks_state.select(Some(0));
        Ok(app)
    }

    pub fn start_toggle_confirm(&mut self) {
        self.is_confirming = true;
    }

    pub async fn confirm_toggle(&mut self) -> Result<()> {
        self.is_online = !self.is_online;
        self.is_confirming = false;

        // 如果切換到離線狀態，更新 bottega 列表
        if !self.is_online {
            match self.wallet.get_user_bottega(self.wallet.get_active_address().await?).await {
                Ok(items) => {
                    self.bottega_items = items;
                    self.message_type = MessageType::Info;
                    self.error_message = Some("Successfully loaded 3D models".to_string());
                    // 重置選擇狀態
                    if !self.bottega_items.is_empty() {
                        self.bottega_state.select(Some(0));
                    }
                }
                Err(e) => {
                    self.message_type = MessageType::Error;
                    self.error_message = Some(format!("Failed to load 3D models: {}", e));
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
    }

    pub fn cancel_harvest(&mut self) {
        self.is_harvesting = false;
    }

    pub fn next_item(&mut self) {
        let items_len = if self.is_online {
            self.tasks.len()
        } else {
            self.bottega_items.len()
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
            let i = match self.bottega_state.selected() {
                Some(i) => {
                    if i >= items_len - 1 {
                        i
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.bottega_state.select(Some(i));
        }
    }

    pub fn previous_item(&mut self) {
        let items_len = if self.is_online {
            self.tasks.len()
        } else {
            self.bottega_items.len()
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
            let i = match self.bottega_state.selected() {
                Some(i) => {
                    if i == 0 {
                        0
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.bottega_state.select(Some(i));
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
                if !self.bottega_items.is_empty() {
                    self.bottega_state.select(Some(0));
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
        self.wallet = Wallet::new(&self.network_state).await?;
        self.wallet_address = shorten_id(&self.wallet.get_active_address().await?.to_string());
        self.sui_balance = self.wallet.get_sui_balance(self.wallet.get_active_address().await?).await?;
        self.wal_balance = self.wallet.get_walrus_balance(self.wallet.get_active_address().await?).await?;
        self.printer_id = self.wallet.get_user_printer_id(self.wallet.get_active_address().await?).await?;
        
        // 更新 bottega 項目
        self.bottega_items = self.wallet.get_user_bottega(self.wallet.get_active_address().await?).await?;
        
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
    }

    // printer registration
    pub async fn handle_printer_registration_input(&mut self, input: char) -> Result<()> {
        match input {
            '\n' => {
                if !self.printer_alias.is_empty() && self.registration_status == RegistrationStatus::Inputting {
                    self.registration_status = RegistrationStatus::Submitting;
                    self.printer_registration_message = "Sending transaction to network...\nPlease wait...".to_string();
                    
                    let builder = TransactionBuilder::new(
                        self.wallet.get_client().clone(),
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
        let aggregator_url = AGGREGATOR_URL;
        let output_path = PathBuf::from("Gcode-Transmit/test.stl");
        
        // 使用 curl 下載文件，添加 -s (silent) 和 -S (show error) 參數
        let status = Command::new("curl")
            .arg("-s")
            .arg("-S")
            .arg(format!("{}/v1/blobs/{}", aggregator_url, blob_id))
            .arg("-o")
            .arg(&output_path)
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to download 3D model"));
        }

        self.message_type = MessageType::Info;
        self.error_message = Some("3D model downloaded successfully".to_string());
        Ok(())
    }

    pub fn run_print_script(&mut self) {
        match Command::new("sh")
            .current_dir("Gcode-Transmit")
            .arg("Gcode-Send.sh")
            .output() {
                Ok(output) => {
                    if !output.status.success() {
                        if let Ok(error) = String::from_utf8(output.stderr) {
                            self.message_type = MessageType::Error;
                            self.error_message = Some(format!("Script failed: {}", error));
                        } else {
                            self.message_type = MessageType::Error;
                            self.error_message = Some("Script failed with non-utf8 error".to_string());
                        }
                    } else {
                        self.message_type = MessageType::Info;
                        self.error_message = Some("Printing...".to_string());
                    }
                }
                Err(e) => {
                    self.message_type = MessageType::Error;
                    self.error_message = Some(format!("Failed to execute script: {}", e));
                }
            }
    }
    pub async fn run_stop_script(&mut self) {
         match Command::new("sh")
             .current_dir("Gcode-Transmit")
             .arg("Gcode-Stop.sh")
             .output() {
                 Ok(output) => {
                     if !output.status.success() {
                         if let Ok(error) = String::from_utf8(output.stderr) {
                             self.message_type = MessageType::Error;
                             self.error_message = Some(format!("Script failed: {}", error));
                         } else {
                             self.message_type = MessageType::Error;
                             self.error_message = Some("Script failed with non-utf8 error".to_string());
                         }
                     } else {
                         self.message_type = MessageType::Info;
                         self.error_message = Some("Printing...".to_string());
                     }
                 }
                 Err(e) => {
                     self.message_type = MessageType::Error;
                     self.error_message = Some(format!("Failed to execute script: {}", e));
                 }
             }
     }

    pub async fn handle_model_selection(&mut self, download_only: bool) -> Result<()> {
        if let Some(selected) = self.bottega_state.selected() {
            if let Some(item) = self.bottega_items.get(selected) {
                if item.name != "No printable models found" {
                    let blob_id = item.blob_id.clone();
                    // 下載檔案
                    self.download_3d_model(&blob_id).await?;
                    
                    // 如果不是只下載，則執行列印腳本
                    if !download_only {
                        self.run_print_script();
                    }
                }
            }
        }
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
