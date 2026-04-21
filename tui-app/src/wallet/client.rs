use std::sync::Arc;
use anyhow::{anyhow, Result};
use futures::TryStreamExt;
use sui_rpc::proto::sui::rpc::v2::{Balance, ListBalancesRequest};
use sui_sdk_types::{Address, TypeTag};
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
        let want: TypeTag = coin_type
            .parse()
            .map_err(|e| anyhow!("invalid coin type string {coin_type:?}: {e}"))?;

        let client = self.rpc.lock().await;
        let req = ListBalancesRequest::default()
            .with_owner(address.to_string())
            .with_page_size(200);

        let mut total: u128 = 0;
        let stream = client.list_balances(req);
        tokio::pin!(stream);
        while let Some(bal) = stream.try_next().await? {
            let Some(ct_str) = bal.coin_type_opt() else {
                continue;
            };
            let Ok(ct) = ct_str.parse::<TypeTag>() else {
                continue;
            };
            if ct != want {
                continue;
            }
            total += Self::balance_from_list_row(&bal);
        }
        Ok(total)
    }

    /// `Balance.balance` is the preferred total; some RPCs only fill `address_balance` / `coin_balance`.
    fn balance_from_list_row(bal: &Balance) -> u128 {
        if let Some(b) = bal.balance_opt() {
            return b as u128;
        }
        bal.address_balance_opt().unwrap_or(0) as u128 + bal.coin_balance_opt().unwrap_or(0) as u128
    }
}
