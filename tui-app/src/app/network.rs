use crate::app::core::App;
use crate::constants::NETWORKS;
use anyhow::Result;
use futures;
use std::sync::Arc;
use crate::utils::setup_for_read;

impl App {
    #[allow(dead_code)]
    pub fn switch_network(&mut self) {
        self.network_state.next_network();
        if let Err(e) = futures::executor::block_on(self.update_network()) {
            self.error_message = Some(e.to_string());
        }
    }

    pub async fn update_network(&mut self) -> Result<()> {
        self.error_message = None;

        match self.do_update_network().await {
            Ok(_) => {
                if !self.sculpt_items.is_empty() {
                    self.sculpt_state.select(Some(0));
                }
                Ok(())
            },
            Err(e) => {
                self.error_message = Some(e.to_string());
                Ok(())
            }
        }
    }

    async fn do_update_network(&mut self) -> Result<()> {
        // get new SuiClient and address
        let (client, address) = setup_for_read(&self.network_state).await?;
        self.sui_client = Arc::new(client);
        
        // update Wallet
        self.wallet = crate::wallet::Wallet::new(&self.network_state, Arc::clone(&self.sui_client), address).await;
        self.wallet_address = crate::utils::shorten_id(&self.wallet.get_active_address().await?.to_string());
        self.sui_balance = self.wallet.get_sui_balance(self.wallet.get_active_address().await?).await?;
        self.wal_balance = self.wallet.get_walrus_balance(self.wallet.get_active_address().await?).await?;
        
        // update printer info and reward balance
        let printer_info = match self.wallet.get_printer_info(self.wallet.get_active_address().await?).await {
            Ok(info) => info,
            Err(_) => {
                crate::wallet::PrinterInfo {
                    id: "No Printer ID".to_string(),
                    pool_balance: 0,
                }
            }
        };
        self.printer_id = printer_info.id;
        
        // format pool balance to SUI
        if printer_info.pool_balance > 0 {
            self.harvestable_rewards = format!("{:.2} SUI", printer_info.pool_balance as f64 / 1_000_000_000.0);
        } else {
            self.harvestable_rewards = "0.00 SUI".to_string();
        }

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
}
