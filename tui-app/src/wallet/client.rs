use std::sync::Arc;
use anyhow::Result;
use futures::TryStreamExt;
use sui_rpc::proto::sui::rpc::v2::ListBalancesRequest;
use sui_sdk_types::Address;
use tokio::sync::Mutex;

use crate::constants::WALRUS_COIN_TYPE;
use crate::utils::NetworkState;

#[derive(Clone)]
pub struct Wallet {
    pub rpc: Arc<Mutex<sui_rpc::Client>>,
    pub address: Address,
    #[allow(dead_code)]
    pub network_state: NetworkState,
}

impl Wallet {
    pub async fn new(
        network_state: &NetworkState,
        rpc: Arc<Mutex<sui_rpc::Client>>,
        address: Address,
    ) -> Self {
        Wallet {
            rpc,
            address,
            network_state: network_state.clone(),
        }
    }

    pub async fn get_active_address(&self) -> Result<Address> {
        Ok(self.address)
    }

    pub async fn get_sui_balance(&self, address: Address) -> Result<u128> {
        self.get_coin_balance(address, "0x2::sui::SUI").await
    }

    pub async fn get_walrus_balance(&self, address: Address) -> Result<u128> {
        self.get_coin_balance(address, WALRUS_COIN_TYPE).await
    }

    async fn get_coin_balance(&self, address: Address, coin_type: &str) -> Result<u128> {
        let client = self.rpc.lock().await;
        let req = ListBalancesRequest::default()
            .with_owner(address.to_string())
            .with_page_size(200);

        let mut total: u128 = 0;
        let stream = client.list_balances(req);
        tokio::pin!(stream);
        while let Some(bal) = stream.try_next().await? {
            if bal.coin_type_opt() == Some(coin_type) {
                total += bal.balance_opt().unwrap_or(0) as u128;
            }
        }
        Ok(total)
    }
}
