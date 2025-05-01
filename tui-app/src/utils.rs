use anyhow::Result;
use sui_sdk::SuiClient;
use sui_sdk::SuiClientBuilder;
use sui_sdk::types::base_types::SuiAddress;
use sui_sdk::wallet_context::WalletContext;
use crate::constants::{NETWORKS, NETWORK_PACKAGE_IDS, NetworkPackageIds};
use dirs::home_dir;

pub fn shorten_id(id: &str) -> String {
    if id.len() > 10 {
        format!("{}..{}", &id[..6], &id[id.len()-6..])
    } else {
        id.to_string()
    }
}

#[derive(Clone)]
pub struct NetworkState {
    pub current_network: usize,
}

impl NetworkState {
    pub fn new() -> Self {
        NetworkState {
            current_network: 1  // 默認為 testnet
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
        
    let mut context = WalletContext::new(&config_path, None, None)?;
    let active_address = context.active_address()?;
    
    Ok((sui, active_address))
} 