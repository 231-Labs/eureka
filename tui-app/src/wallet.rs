use anyhow::Result;
use sui_sdk::types::base_types::SuiAddress;
use sui_sdk::SuiClient;
use crate::utils::{setup_for_read, NetworkState};
use crate::constants::WALRUS_COIN_TYPE;

pub struct Wallet {
    client: SuiClient,
    address: SuiAddress,
}

impl Wallet {
    pub async fn new(network_state: &NetworkState) -> Result<Self> {
        let (client, address) = setup_for_read(network_state).await?;
        Ok(Wallet { client, address })
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
            .get_balance(address, Some(WALRUS_COIN_TYPE.to_string()))
            .await?;
        Ok(balance.total_balance)
    }
} 