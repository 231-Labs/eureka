use ratatui::widgets::ListState;
use crate::wallet::Wallet;
use anyhow::Result;

#[derive(Clone)]
pub enum TaskStatus {
    Printing(u8),  // 進度百分比
    Completed,
}

#[derive(Clone)]
pub struct PrintTask {
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
}

pub struct App {
    pub wallet_address: String,
    pub printer_id: String,
    pub is_online: bool,
    pub assets: Vec<String>,
    pub assets_state: ListState,
    pub tasks: Vec<PrintTask>,
    pub tasks_state: ListState,
    pub is_confirming: bool,
    pub is_harvesting: bool,
    pub harvestable_rewards: String,
    pub sui_balance: u128,
    pub wal_balance: u128,
}

impl App {
    pub async fn new() -> Result<App> {
        let wallet = Wallet::new().await?;
        let wallet_address = wallet.get_active_address().await?.to_string();
        
        // 獲取餘額
        let sui_balance = wallet.get_sui_balance(wallet.get_active_address().await?).await?;
        let wal_balance = 0; // 初始 WAL 餘額為 0
        
        let mut app = App {
            wallet_address,
            printer_id: "Not Set".to_string(),
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
            harvestable_rewards: "100.0 SUI".to_string(),
            sui_balance,
            wal_balance,
        };
        
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