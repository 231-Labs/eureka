use anyhow::Result;
use sui_sdk::SuiClient;
use sui_sdk::SuiClientBuilder;
use sui_sdk::types::base_types::SuiAddress;
use sui_sdk::wallet_context::WalletContext;
use crate::constants::{NETWORKS, NETWORK_PACKAGE_IDS, NetworkPackageIds, SUI_DECIMALS};
use dirs::home_dir;

pub fn shorten_id(id: &str) -> String {
    if id.len() > 16 {
        // For addresses like 0x598928d17a9a5dadfaffdaca2e5d2315bd2e9387d73c8a63488a1a0f4d73ffbd
        // Show: 0x598928...4d73ffbd (first 8 chars including 0x, last 8 chars)
        format!("{}...{}", &id[..10], &id[id.len()-8..])
    } else {
        id.to_string()
    }
}

pub fn format_sui_balance(amount: u128) -> String {
    format!("{:.2} SUI", amount as f64 / SUI_DECIMALS)
}

#[allow(dead_code)]
pub fn format_sui_amount(amount: u128, decimals: u64) -> String {
    format!("{:.2}", amount as f64 / 10_f64.powi(decimals as i32))
}

#[derive(Clone)]
pub struct NetworkState {
    pub current_network: usize,
}

impl NetworkState {
    pub fn new() -> Self {
        NetworkState {
            current_network: 1  // Default to testnet
        }
    }

    #[allow(dead_code)]
    pub fn next_network(&mut self) {
        self.current_network = (self.current_network + 1) % NETWORKS.len();
    }

    pub fn get_current_network(&self) -> &str {
        NETWORKS[self.current_network].0
    }

    pub fn get_current_rpc(&self) -> &str {
        NETWORKS[self.current_network].1
    }

    pub fn get_current_package_ids(&self) -> &NetworkPackageIds {
        &NETWORK_PACKAGE_IDS[self.current_network]
    }
}

pub async fn setup_for_read(network_state: &NetworkState) -> Result<(SuiClient, SuiAddress)> {
    let sui = SuiClientBuilder::default()
        .build(network_state.get_current_rpc())
        .await?;
    
    let config_path = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?
        .join(".sui")
        .join("sui_config")
        .join("client.yaml");
        
    let mut context = WalletContext::new(&config_path)?;
    let active_address = context.active_address()?;
    
    Ok((sui, active_address))
} 