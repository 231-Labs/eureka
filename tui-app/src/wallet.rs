use anyhow::Result;
use sui_sdk::types::base_types::SuiAddress;
use sui_sdk::SuiClient;
use crate::utils::setup_for_read;

pub struct Wallet {
    client: SuiClient,
    address: SuiAddress,
}

impl Wallet {
    pub async fn new() -> Result<Self> {
        let (client, address) = setup_for_read().await?;
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
} 