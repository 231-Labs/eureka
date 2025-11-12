use anyhow::Result;
use sui_sdk::types::base_types::SuiAddress;

use super::client::Wallet;
use super::types::SculptItem;

impl Wallet {
    /// Get user's sculpt (3D model) items from all Kiosks
    pub async fn get_user_sculpt(&self, address: SuiAddress) -> Result<Vec<SculptItem>> {
        let sculpts = self.get_all_kiosk_sculpts(address).await?;

        Ok(if sculpts.is_empty() {
            vec![SculptItem {
                alias: "No printable models found".to_string(),
                blob_id: String::new(),
                printed_count: 0,
                id: String::new(),
                is_encrypted: false,
                seal_resource_id: None,
            }]
        } else {
            let mut items = sculpts;
            items.sort_by(|a, b| a.alias.cmp(&b.alias));
            items
        })
    }
}
