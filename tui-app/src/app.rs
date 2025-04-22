use ratatui::widgets::ListState;
use crate::wallet::Wallet;
use crate::utils::{NetworkState, shorten_id};
use crate::constants::NETWORKS;
use anyhow::Result;

#[derive(Clone)]
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

pub struct App {
    pub wallet: Wallet,
    pub wallet_address: String,
    pub printer_id: String,
    pub is_online: bool,
    pub assets: Vec<String>,
    pub assets_state: ListState,
    pub tasks: Vec<PrintTask>,
    pub tasks_state: ListState,
    pub is_confirming: bool,
    pub is_harvesting: bool,
    pub is_switching_network: bool,
    pub harvestable_rewards: String,
    pub sui_balance: u128,
    pub wal_balance: u128,
    pub network_state: NetworkState,
    pub error_message: Option<String>,  // 新增錯誤訊息欄位
    // 新增機器狀態字段
    pub nozzle_temp: f32,      // 噴嘴溫度
    pub bed_temp: f32,         // 加熱板溫度
    // 新增打印機註冊相關狀態
    pub is_registering_printer: bool,
    pub printer_alias: String,
    pub printer_registration_message: String,
}

impl App {
    pub async fn new() -> Result<App> {
        let network_state = NetworkState::new();
        let wallet = Wallet::new(&network_state).await?;
        let wallet_address = shorten_id(&wallet.get_active_address().await?.to_string());
        
        // get balance and printer id
        let sui_balance = wallet.get_sui_balance(wallet.get_active_address().await?).await?;
        let wal_balance = wallet.get_walrus_balance(wallet.get_active_address().await?).await?;
        let printer_id = match wallet.get_user_printer_id(wallet.get_active_address().await?).await {
            Ok(id) => id,
            Err(_e) => {
                "No Printer ID".to_string()
            }
        };
        
        let mut app = App {
            wallet,
            wallet_address,
            printer_id: printer_id.clone(),
            is_online: false,
            assets: vec![
                "3D Model #1 - Cute Cat".to_string(),
                "3D Model #2 - Cool Dragon".to_string(),
                "3D Model #3 - Fancy Vase".to_string(),
                "3D Model #4 - Phone Stand".to_string(),
                "3D Model #5 - Desk Organizer".to_string(),
                "3D Model #6 - Plant Pot".to_string(),
                "3D Model #7 - Jewelry Box".to_string(),
                "3D Model #8 - Toy Car".to_string(),
                "3D Model #9 - Chess Set".to_string(),
                "3D Model #10 - Headphone Stand".to_string(),
                "3D Model #11 - Pencil Holder".to_string(),
                "3D Model #12 - Wall Art".to_string(),
                "3D Model #13 - Lamp Shade".to_string(),
                "3D Model #14 - Tablet Stand".to_string(),
                "3D Model #15 - Key Chain".to_string(),
            ],
            assets_state: ListState::default(),
            tasks: vec![
                PrintTask {
                    id: "#1".to_string(),
                    name: "Cute Cat".to_string(),
                    status: TaskStatus::Printing(75),
                },
                PrintTask {
                    id: "#2".to_string(),
                    name: "Cool Dragon".to_string(),
                    status: TaskStatus::Completed,
                },
                PrintTask {
                    id: "#3".to_string(),
                    name: "Fancy Vase".to_string(),
                    status: TaskStatus::Completed,
                },
                PrintTask {
                    id: "#4".to_string(),
                    name: "Phone Stand".to_string(),
                    status: TaskStatus::Completed,
                },
                PrintTask {
                    id: "#5".to_string(),
                    name: "Desk Organizer".to_string(),
                    status: TaskStatus::Completed,
                },
                PrintTask {
                    id: "#6".to_string(),
                    name: "Plant Pot".to_string(),
                    status: TaskStatus::Completed,
                },
                PrintTask {
                    id: "#7".to_string(),
                    name: "Jewelry Box".to_string(),
                    status: TaskStatus::Completed,
                },
                PrintTask {
                    id: "#8".to_string(),
                    name: "Toy Car".to_string(),
                    status: TaskStatus::Completed,
                },
                PrintTask {
                    id: "#9".to_string(),
                    name: "Chess Set".to_string(),
                    status: TaskStatus::Completed,
                },
                PrintTask {
                    id: "#10".to_string(),
                    name: "Headphone Stand".to_string(),
                    status: TaskStatus::Completed,
                },
                PrintTask {
                    id: "#11".to_string(),
                    name: "Pencil Holder".to_string(),
                    status: TaskStatus::Completed,
                },
                PrintTask {
                    id: "#12".to_string(),
                    name: "Wall Art".to_string(),
                    status: TaskStatus::Completed,
                },
                PrintTask {
                    id: "#13".to_string(),
                    name: "Lamp Shade".to_string(),
                    status: TaskStatus::Completed,
                },
                PrintTask {
                    id: "#14".to_string(),
                    name: "Tablet Stand".to_string(),
                    status: TaskStatus::Completed,
                },
                PrintTask {
                    id: "#15".to_string(),
                    name: "Key Chain".to_string(),
                    status: TaskStatus::Completed,
                },
            ],
            tasks_state: ListState::default(),
            is_confirming: false,
            is_harvesting: false,
            is_switching_network: false,
            harvestable_rewards: "100.0 SUI".to_string(),
            sui_balance,
            wal_balance,
            network_state,
            error_message: None,  // 初始化為 None
            nozzle_temp: 0.0,
            bed_temp: 0.0,
            is_registering_printer: false,
            printer_alias: String::new(),
            printer_registration_message: String::new(),
        };
        
        // 檢查是否需要註冊打印機
        if printer_id == "No Printer ID" {
            app.is_registering_printer = true;
            app.printer_registration_message = "Welcome to Eureka 3D Printing Platform!\n\nNo printer found. Please register your printer to continue.\n\nEnter your printer alias:".to_string();
        }
        
        // 設置初始選中項
        app.assets_state.select(Some(0));
        app.tasks_state.select(Some(0));
        Ok(app)
    }

    pub fn start_toggle_confirm(&mut self) {
        self.is_confirming = true;
    }

    pub fn confirm_toggle(&mut self) {
        self.is_online = !self.is_online;
        self.is_confirming = false;
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
            self.assets.len()
        };

        if items_len == 0 {
            return;
        }

        if self.is_online {
            let i = match self.tasks_state.selected() {
                Some(i) => {
                    if i >= items_len - 1 {
                        i  // 已經到底部，保持當前位置
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.tasks_state.select(Some(i));
        } else {
            let i = match self.assets_state.selected() {
                Some(i) => {
                    if i >= items_len - 1 {
                        i  // 已經到底部，保持當前位置
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.assets_state.select(Some(i));
        }
    }

    pub fn previous_item(&mut self) {
        let items_len = if self.is_online {
            self.tasks.len()
        } else {
            self.assets.len()
        };

        if items_len == 0 {
            return;
        }

        if self.is_online {
            let i = match self.tasks_state.selected() {
                Some(i) => {
                    if i == 0 {
                        0  // 已經到頂部，保持當前位置
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.tasks_state.select(Some(i));
        } else {
            let i = match self.assets_state.selected() {
                Some(i) => {
                    if i == 0 {
                        0  // 已經到頂部，保持當前位置
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.assets_state.select(Some(i));
        }
    }

    pub fn switch_network(&mut self) {
        self.network_state.next_network();
    }

    pub async fn update_network(&mut self) -> Result<()> {
        self.switch_network();
        self.error_message = None;  // 清除之前的錯誤訊息
        
        match self.do_update_network().await {
            Ok(_) => Ok(()),
            Err(e) => {
                self.error_message = Some(e.to_string());  // 儲存錯誤訊息
                Ok(())  // 不中斷程序執行
            }
        }
    }

    async fn do_update_network(&mut self) -> Result<()> {
        self.wallet = Wallet::new(&self.network_state).await?;
        self.wallet_address = shorten_id(&self.wallet.get_active_address().await?.to_string());
        self.sui_balance = self.wallet.get_sui_balance(self.wallet.get_active_address().await?).await?;
        self.wal_balance = self.wallet.get_walrus_balance(self.wallet.get_active_address().await?).await?;
        self.printer_id = self.wallet.get_user_printer_id(self.wallet.get_active_address().await?).await?;
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

    // 新增打印機註冊相關方法
    pub fn handle_printer_registration_input(&mut self, input: char) {
        match input {
            '\n' => {
                if !self.printer_alias.is_empty() {
                    // TODO: 調用智能合約鑄造 Printer Object
                    self.printer_registration_message = format!("Registering printer with alias: {}\n\nPlease wait...", self.printer_alias);
                    // 模擬註冊成功
                    self.printer_id = "0x64a8982336ac12bd081ac9f7c646e5bf88523839fd66fb38e2f92884bfcd1999".to_string();
                    self.is_registering_printer = false;
                }
            }
            '\x08' | '\x7f' => {
                self.printer_alias.pop();
            }
            c if c.is_ascii() && !c.is_control() => {
                self.printer_alias.push(c);
            }
            _ => {}
        }
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