use std::collections::BTreeMap;
use anyhow::{Result, anyhow};
use sui_sdk::{
    rpc_types::{
        SuiMoveStruct,
        SuiMoveValue,
        SuiObjectDataOptions,
        SuiObjectResponseQuery,
        SuiParsedData,
    },
    types::{
        base_types::{ObjectID, SuiAddress},
    },
};
use super::types::PrinterInfo;
use super::client::Wallet;
use super::utils::{extract_id_from_fields, extract_printer_id_from_cap};

impl Wallet {
    // Extract pool balance from move struct
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
    
    // Extract printer info from move struct
    fn extract_printer_from_move_struct(&self, move_struct: &SuiMoveStruct) -> Option<PrinterInfo> {
        if let SuiMoveStruct::WithFields(fields) = move_struct {
            let id = extract_id_from_fields(fields)?;
            let pool_balance = self.extract_pool_balance(fields);
            Some(PrinterInfo { id, pool_balance })
        } else {
            None
        }
    }

    // Get user's PrinterCap and its printer_id
    pub async fn get_printer_cap_info(&self, address: SuiAddress) -> Result<(String, String)> {
        let mut options = SuiObjectDataOptions::new();
        options.show_content = true;
        options.show_owner = true;
        options.show_type = true;
        
        // Get current network's package ID
        let current_package_id = self.network_state.get_current_package_ids().eureka_package_id;
        let printer_cap_type = format!("{}::eureka::PrinterCap", current_package_id);
        
        // Create a filter for the specific PrinterCap type
        let filter = sui_sdk::rpc_types::SuiObjectDataFilter::StructType(
            sui_sdk::types::parse_sui_struct_tag(&printer_cap_type)?
        );
        
        // Query with type filter - this returns only PrinterCap objects from current package
        let response = self.client.read_api()
            .get_owned_objects(
                address,
                Some(SuiObjectResponseQuery::new(
                    Some(filter),
                    Some(options)
                )),
                None,
                None
            )
            .await?;
        
        // Extract info from the first PrinterCap found
        let cap_info = response.data.iter()
            .filter_map(|obj| {
                obj.data.as_ref()
                    .and_then(|data| data.content.as_ref())
                    .and_then(|content| {
                        if let SuiParsedData::MoveObject(move_obj) = content {
                            if let SuiMoveStruct::WithFields(fields) = &move_obj.fields {
                                let cap_id = extract_id_from_fields(fields)?;
                                let printer_id = extract_printer_id_from_cap(fields)?;
                                return Some((cap_id, printer_id));
                            }
                        }
                        None
                    })
            })
            .next();
        
        // If PrinterCap not found, return error
        cap_info.ok_or_else(|| anyhow!("No PrinterCap found for this address. Please register a printer first."))
    }
    
    // Get user's PrinterCap ID
    pub async fn get_printer_cap_id(&self, address: SuiAddress) -> Result<String> {
        let (cap_id, _) = self.get_printer_cap_info(address).await?;
        Ok(cap_id)
    }

    // Get printer information
    pub async fn get_printer_info(&self, address: SuiAddress) -> Result<PrinterInfo> {
        let (_, printer_id) = self.get_printer_cap_info(address).await?;
        
        let mut options = SuiObjectDataOptions::new();
        options.show_content = true;
        options.show_owner = true;
        options.show_type = true;
        
        // Step 2: query shared Printer object using printer_id
        let printer_object_id = ObjectID::from_hex_literal(&printer_id)
            .map_err(|e| anyhow!("Invalid printer ID format: {}", e))?;
            
        let printer_response = self.client.read_api()
            .get_object_with_options(printer_object_id, options)
            .await?;
            
        if let Some(data) = printer_response.data {
            if let Some(content) = data.content {
                if let SuiParsedData::MoveObject(move_obj) = content {
                    if let Some(info) = self.extract_printer_from_move_struct(&move_obj.fields) {
                        return Ok(info);
                    }
                }
            }
        }
        
        // If no corresponding Printer object found, return error
        Err(anyhow!("PrinterCap found but corresponding Printer object not found."))
    }
} 