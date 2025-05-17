use std::sync::Arc;
use anyhow::Result;
use sui_sdk::{
    types::base_types::SuiAddress,
    SuiClient,
};
use crate::utils::NetworkState;

#[derive(Clone)]
pub struct Wallet {
    pub client: Arc<SuiClient>,
    pub address: SuiAddress,
    pub network_state: NetworkState,
}

impl Wallet {
    pub async fn new(network_state: &NetworkState, client: Arc<SuiClient>, address: SuiAddress) -> Self {
        Wallet { 
            client, 
            address,
            network_state: network_state.clone(),
        }
    }

    pub async fn get_active_address(&self) -> Result<SuiAddress> {
        Ok(self.address)
    }

    pub async fn get_sui_balance(&self, address: SuiAddress) -> Result<u128> {
        let balance = self.client.coin_read_api()
            .get_balance(address, None)
            .await?;
        Ok(balance.total_balance)
    }

    pub async fn get_walrus_balance(&self, address: SuiAddress) -> Result<u128> {
        let balance = self.client.coin_read_api()
            .get_balance(address, Some(crate::constants::WALRUS_COIN_TYPE.to_string()))
            .await?;
        Ok(balance.total_balance)
    }
} 