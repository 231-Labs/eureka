use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use sui_sdk_types::Address;
use sui_rpc::Client as GrpcClient;
use tokio::sync::Mutex;

use crate::constants::{NETWORKS, NETWORK_PACKAGE_IDS, NetworkPackageIds, SUI_DECIMALS};
use crate::wallet::load_active_signer;
use dirs::home_dir;

/// `tui-app/` directory at compile time. Use for `Gcode-Transmit`, `mock_print.stl`, etc., so paths stay
/// correct when the process cwd is `target/debug` or elsewhere.
pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn shorten_id(id: &str) -> String {
    if id.len() > 16 {
        format!("{}...{}", &id[..10], &id[id.len() - 8..])
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
            current_network: 1,
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

pub async fn setup_for_read(
    network_state: &NetworkState,
) -> Result<(Arc<Mutex<GrpcClient>>, Address, sui_crypto::ed25519::Ed25519PrivateKey)> {
    let url = network_state.get_current_rpc();
    let client = GrpcClient::new(url).map_err(|e| anyhow::anyhow!("gRPC client: {}", e))?;

    let config_path = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?
        .join(".sui")
        .join("sui_config")
        .join("client.yaml");

    let (address, signer) = load_active_signer(&config_path)?;

    Ok((Arc::new(Mutex::new(client)), address, signer))
}
