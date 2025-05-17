use anyhow::Result;
use sui_sdk::{
    rpc_types::{
        SuiObjectDataFilter,
        SuiObjectDataOptions,
        SuiObjectResponseQuery
    },
    types::{
        base_types::{ObjectID, SuiAddress},
        Identifier,
    },
};
use super::client::Wallet;
use super::types::SculptItem;

impl Wallet {
    // Get user's sculpt (3D model) items
    pub async fn get_user_sculpt(&self, address: SuiAddress) -> Result<Vec<SculptItem>> {
        let package_id: ObjectID = self.network_state.get_current_package_ids().bottega_package_id.parse()?;
        let mut options = SuiObjectDataOptions::new();
        options.show_content = true;

        let filter = SuiObjectDataFilter::MoveModule {
            package: package_id,
            module: Identifier::new("sculpt".to_string())?,
        };

        let response = self.client.read_api()
            .get_owned_objects(
                address,
                Some(SuiObjectResponseQuery::new(Some(filter), Some(options))),
                None,
                None
            )
            .await?;

        let sculpt_items: Vec<SculptItem> = response.data.iter()
            .filter_map(|obj| self.parse_sculpt_object(obj))
            .collect();

        Ok(if sculpt_items.is_empty() {
            vec![SculptItem {
                alias: "No printable models found".to_string(),
                blob_id: String::new(),
                printed_count: 0,
            }]
        } else {
            let mut items = sculpt_items;
            items.sort_by(|a, b| a.alias.cmp(&b.alias));
            items
        })
    }

    // Parse sculpt object from response
    fn parse_sculpt_object(&self, obj: &sui_sdk::rpc_types::SuiObjectResponse) -> Option<SculptItem> {
        obj.data.as_ref()
            .and_then(|data| data.content.as_ref())
            .and_then(|content| match content {
                sui_sdk::rpc_types::SuiParsedData::MoveObject(move_obj) => {
                    if let sui_sdk::rpc_types::SuiMoveStruct::WithFields(fields) = &move_obj.fields {
                        Some(fields)
                    } else {
                        None
                    }
                },
                _ => None,
            })
            .and_then(|fields| {
                let structure = fields.get("structure")?;
                let printed = fields.get("printed")?;
                let alias = fields.get("alias")?;

                match (structure, printed, alias) {
                    (
                        sui_sdk::rpc_types::SuiMoveValue::String(structure_id),
                        sui_sdk::rpc_types::SuiMoveValue::String(printed_str),
                        sui_sdk::rpc_types::SuiMoveValue::String(alias_str)
                    ) => {
                        Some(SculptItem {
                            alias: alias_str.clone(),
                            blob_id: structure_id.clone(),
                            printed_count: printed_str.parse::<u64>().unwrap_or(0),
                        })
                    },
                    _ => None,
                }
            })
    }
} 