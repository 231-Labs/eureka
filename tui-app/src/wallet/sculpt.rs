use anyhow::Result;
use sui_sdk_types::Address;

use super::client::Wallet;
use super::types::SculptItem;

impl Wallet {
    pub async fn get_user_sculpt(&self, address: Address) -> Result<Vec<SculptItem>> {
        let sculpts = self.get_all_kiosk_sculpts(address).await?;

        Ok(if sculpts.is_empty() {
            vec![SculptItem {
                alias: "No printable models found".to_string(),
                blob_id: String::new(),
                printed_count: 0,
                id: String::new(),
                source_kiosk_id: None,
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
