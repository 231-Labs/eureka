use anyhow::Result;
use sui_sdk::types::base_types::{SuiAddress, ObjectID};
use sui_sdk::SuiClient;
use sui_sdk::rpc_types::{SuiObjectDataFilter, SuiObjectResponseQuery, SuiObjectDataOptions};
use sui_sdk::types::Identifier;
use crate::utils::{setup_for_read, NetworkState, shorten_id};
use crate::constants::WALRUS_COIN_TYPE;


#[derive(Debug, Clone)]
pub struct BottegaItem {
    pub alias: String,
    pub blob_id: String,
    pub printed_count: u64,
}

#[derive(Clone)]
pub struct Wallet {
    client: SuiClient,
    address: SuiAddress,
    network_state: NetworkState,
}

impl Wallet {
    pub async fn new(network_state: &NetworkState) -> Result<Self> {
        let (client, address) = setup_for_read(network_state).await?;
        Ok(Wallet { 
            client, 
            address,
            network_state: network_state.clone(),
        })
    }

    pub fn get_client(&self) -> &SuiClient {
        &self.client
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

    pub async fn get_user_printer_id(&self, address: SuiAddress) -> Result<String> {
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
            
        let printer_id = response.data
            .first()
            .and_then(|obj| obj.data.as_ref())
            .and_then(|data| data.content.as_ref())
            .and_then(|content| match content {
                sui_sdk::rpc_types::SuiParsedData::MoveObject(move_obj) => {
                    if let sui_sdk::rpc_types::SuiMoveStruct::WithFields(fields) = &move_obj.fields {
                        fields.get("printer_id")
                            .and_then(|id| if let sui_sdk::rpc_types::SuiMoveValue::Address(addr) = id {
                                Some(addr.to_string())
                            } else {
                                None
                            })
                    } else {
                        None
                    }
                },
                _ => None
            })
            .or_else(|| {
                response.data
                    .first()
                    .and_then(|obj| obj.data.as_ref())
                    .map(|data| data.object_id.to_string())
            })
            .map(|id| shorten_id(&id))
            .ok_or_else(|| anyhow::anyhow!("No printer found"))?;

        Ok(printer_id)
    }

    pub async fn get_user_bottega(&self, address: SuiAddress) -> Result<Vec<BottegaItem>> {
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

        let bottega_items: Vec<BottegaItem> = response.data.iter()
            .filter_map(|obj| self.parse_bottega_object(obj))
            .collect();

        Ok(if bottega_items.is_empty() {
            vec![BottegaItem {
                alias: "No printable models found".to_string(),
                blob_id: String::new(),
                printed_count: 0,
            }]
        } else {
            let mut items = bottega_items;
            items.sort_by(|a, b| a.alias.cmp(&b.alias));
            items
        })
    }

    fn parse_bottega_object(&self, obj: &sui_sdk::rpc_types::SuiObjectResponse) -> Option<BottegaItem> {
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
                        Some(BottegaItem {
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