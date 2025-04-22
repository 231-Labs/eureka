use anyhow::Result;
use sui_sdk::types::base_types::{SuiAddress, ObjectID};
use sui_sdk::SuiClient;
use sui_sdk::rpc_types::{SuiObjectDataFilter, SuiObjectResponseQuery, SuiObjectDataOptions};
use crate::utils::{setup_for_read, NetworkState, shorten_id};
use crate::constants::{WALRUS_COIN_TYPE, EUREKA_DEVNET_PACKAGE_ID};

pub struct Wallet {
    client: SuiClient,
    address: SuiAddress,
}

impl Wallet {
    pub async fn new(network_state: &NetworkState) -> Result<Self> {
        let (client, address) = setup_for_read(network_state).await?;
        Ok(Wallet { client, address })
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
        let package_id: ObjectID = EUREKA_DEVNET_PACKAGE_ID.parse()?;
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
} 