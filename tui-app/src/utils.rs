use anyhow::Result;
use sui_sdk::SuiClient;
use sui_sdk::SuiClientBuilder;
use sui_sdk::types::base_types::SuiAddress;
use sui_sdk::wallet_context::WalletContext;

pub async fn setup_for_read() -> Result<(SuiClient, SuiAddress)> {
    let sui = SuiClientBuilder::default()
        .build("https://fullnode.mainnet.sui.io:443")
        .await?;
    
    let config_path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?
        .join(".sui")
        .join("sui_config")
        .join("client.yaml");
        
    let mut context = WalletContext::new(&config_path, None, None)?;
    let active_address = context.active_address()?;
    
    Ok((sui, active_address))
} 