use std::{
    collections::BTreeMap,
    sync::Arc,
};
use anyhow::Result;
use sui_sdk::{
    rpc_types::{
        SuiMoveStruct,
        SuiMoveValue,
        SuiObjectDataFilter,
        SuiObjectDataOptions,
        SuiObjectResponseQuery,
        SuiParsedData,
    },
    types::{
        base_types::{ObjectID, SuiAddress},
        Identifier,
    },
    SuiClient,
};
use crate::{
    constants::WALRUS_COIN_TYPE,
    utils::{NetworkState, shorten_id},
};


#[derive(Debug, Clone)]
pub struct SculptItem {
    pub alias: String,
    pub blob_id: String,
    pub printed_count: u64,
}

#[derive(Debug, Clone)]
pub struct PrinterInfo {
    pub id: String,
    pub pool_balance: u128,
}

#[derive(Clone)]
pub struct Wallet {
    client: Arc<SuiClient>,
    address: SuiAddress,
    network_state: NetworkState,
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
            .get_balance(address, Some(WALRUS_COIN_TYPE.to_string()))
            .await?;
        Ok(balance.total_balance)
    }

    // extract printer id from fields
    fn extract_printer_id(&self, fields: &BTreeMap<String, SuiMoveValue>) -> Option<String> {
        fields.get("id").and_then(|id_field| {
            if let SuiMoveValue::UID { id } = id_field {
                Some(shorten_id(&id.to_string()))
            } else {
                None
            }
        })
    }
    
    // get pool balance from move struct
    fn extract_pool_balance(&self, fields: &BTreeMap<String, SuiMoveValue>) -> u128 {
        fields.get("pool")
            .and_then(|pool_field| {
                if let SuiMoveValue::Struct(pool_struct) = pool_field {
                    if let SuiMoveStruct::WithFields(pool_fields) = pool_struct {
                        pool_fields.get("value").and_then(|value| {
                            if let SuiMoveValue::Number(amount) = value {
                                amount.to_string().parse::<u128>().ok()
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }
    
    // get printer info from move struct
    fn extract_printer_from_move_struct(&self, move_struct: &SuiMoveStruct) -> Option<PrinterInfo> {
        if let SuiMoveStruct::WithFields(fields) = move_struct {
            let id = self.extract_printer_id(fields)?;
            let pool_balance = self.extract_pool_balance(fields);
            Some(PrinterInfo { id, pool_balance })
        } else {
            None
        }
    }

    pub async fn get_printer_info(&self, address: SuiAddress) -> Result<PrinterInfo> {
        let package_id: ObjectID = self.network_state.get_current_package_ids().eureka_package_id.parse()?;
        let mut options = SuiObjectDataOptions::new();
        options.show_content = true;
        
        let response = self.client.read_api()
            .get_owned_objects(
                address,
                Some(SuiObjectResponseQuery::new(
                    Some(SuiObjectDataFilter::Package(package_id)),
                    Some(options)
                )),
                None,
                None
            )
            .await?;
            
        // try to get printer info from the first object
        response.data
            .first()
            .and_then(|obj| obj.data.as_ref())
            .and_then(|data| {
                // extract info from content or use object id as backup
                data.content.as_ref()
                    .and_then(|content| {
                        if let SuiParsedData::MoveObject(move_obj) = content {
                            self.extract_printer_from_move_struct(&move_obj.fields)
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        // backup plan: use object id
                        Some(PrinterInfo {
                            id: shorten_id(&data.object_id.to_string()),
                            pool_balance: 0,
                        })
                    })
            })
            .ok_or_else(|| anyhow::anyhow!("No printer found"))
    }

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