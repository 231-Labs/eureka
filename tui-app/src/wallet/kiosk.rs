use anyhow::Result;
use sui_sdk::{
    rpc_types::SuiObjectDataOptions,
    types::base_types::{ObjectID, SuiAddress},
};
use std::collections::BTreeMap;
use sui_sdk::rpc_types::SuiMoveValue;

use crate::constants::NETWORK_PACKAGE_IDS;
use super::utils::{extract_string_field, extract_bool_field};
use super::types::SculptItem;
use super::client::Wallet;

impl Wallet {
    /// Get all Sculpts from all Kiosks owned by the given address
    /// Only returns Sculpts from the current network's package
    pub async fn get_all_kiosk_sculpts(&self, address: SuiAddress) -> Result<Vec<SculptItem>> {
        let mut all_sculpts = Vec::new();
        
        // Get all owned Kiosk owner caps
        let kiosk_ids = self.get_owned_kiosk_ids(address).await?;
        
        // Query each Kiosk for Sculpts
        for kiosk_id in kiosk_ids {
            if let Ok(sculpts) = self.get_sculpts_from_kiosk(kiosk_id).await {
                all_sculpts.extend(sculpts);
            }
        }
        
        Ok(all_sculpts)
    }
    
    /// Get all Kiosk IDs owned by the given address
    async fn get_owned_kiosk_ids(&self, address: SuiAddress) -> Result<Vec<ObjectID>> {
        let mut options = SuiObjectDataOptions::new();
        options.show_content = true;
        options.show_type = true;
        
        let mut kiosk_ids = Vec::new();
        let mut cursor = None;
        
        // Paginate through all owned objects
        loop {
            let response = self.client.read_api()
                .get_owned_objects(
                    address,
                    Some(sui_sdk::rpc_types::SuiObjectResponseQuery::new(None, Some(options.clone()))),
                    cursor,
                    None
                )
                .await?;
            
            // Find all KioskOwnerCaps
            for obj in &response.data {
                if let Some(data) = &obj.data {
                    if let Ok(obj_type) = data.object_type() {
                        let type_str = obj_type.to_string();
                        
                        if type_str.contains("::kiosk::KioskOwnerCap") {
                            // Extract the Kiosk ID from KioskOwnerCap
                            if let Some(kiosk_id) = self.extract_kiosk_id_from_cap(data) {
                                kiosk_ids.push(kiosk_id);
                            }
                        }
                    }
                }
            }
            
            // Check for next page
            if response.has_next_page {
                cursor = response.next_cursor;
            } else {
                break;
            }
        }
        
        Ok(kiosk_ids)
    }
    
    /// Extract Kiosk ID from a KioskOwnerCap object
    fn extract_kiosk_id_from_cap(&self, cap_data: &sui_sdk::rpc_types::SuiObjectData) -> Option<ObjectID> {
        cap_data.content.as_ref()
            .and_then(|content| {
                if let sui_sdk::rpc_types::SuiParsedData::MoveObject(move_obj) = content {
                    if let sui_sdk::rpc_types::SuiMoveStruct::WithFields(fields) = &move_obj.fields {
                        if let Some(kiosk_id_value) = fields.get("for") {
                            return match kiosk_id_value {
                                sui_sdk::rpc_types::SuiMoveValue::Address(addr) => {
                                    Some((*addr).into())
                                }
                                sui_sdk::rpc_types::SuiMoveValue::String(s) => {
                                    ObjectID::from_hex_literal(s).ok()
                                }
                                _ => None,
                            };
                        }
                    }
                }
                None
            })
    }
    
    /// Get all Sculpts from a Kiosk's dynamic fields
    /// Filters to only include Sculpts from the current network's package
    async fn get_sculpts_from_kiosk(&self, kiosk_id: ObjectID) -> Result<Vec<SculptItem>> {
        let mut sculpt_items = Vec::new();
        let mut cursor = None;
        
        // Get current network's package ID for filtering
        let network = self.network_state.current_network;
        let current_package_id = NETWORK_PACKAGE_IDS[network as usize].bottega_package_id;
        
        // Iterate through all dynamic fields in the Kiosk
        loop {
            let response = self.client.read_api()
                .get_dynamic_fields(kiosk_id, cursor, None)
                .await?;
            
            // Collect all field object IDs to fetch in batch
            let field_ids: Vec<_> = response.data.iter().map(|f| f.object_id).collect();
            
            if !field_ids.is_empty() {
                // Batch fetch all objects at once instead of one by one
                let options = SuiObjectDataOptions {
                    show_type: true,
                    show_content: true,
                    show_owner: true,
                    ..Default::default()
                };
                
                match self.client.read_api()
                    .multi_get_object_with_options(field_ids, options)
                    .await {
                    Ok(objects) => {
                        for obj_response in objects {
                            if let Some(data) = obj_response.data {
                                if let Ok(obj_type) = data.object_type() {
                                    let type_str = obj_type.to_string();
                                    
                                    // Only accept Sculpts from the current package
                                    if type_str.contains("::sculpt::Sculpt") && type_str.contains(current_package_id) {
                                        if let Some(item) = self.parse_sculpt_from_kiosk_field(&data) {
                                            sculpt_items.push(item);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(_e) => {
                        // If batch fetch fails, fall back to individual fetches with delays
                        for field in &response.data {
                            let field_object = self.client.read_api()
                                .get_object_with_options(
                                    field.object_id,
                                    SuiObjectDataOptions {
                                        show_type: true,
                                        show_content: true,
                                        show_owner: true,
                                        ..Default::default()
                                    }
                                )
                                .await?;
                            
                            if let Some(data) = &field_object.data {
                                if let Ok(obj_type) = data.object_type() {
                                    let type_str = obj_type.to_string();
                                    
                                    if type_str.contains("::sculpt::Sculpt") && type_str.contains(current_package_id) {
                                        if let Some(item) = self.parse_sculpt_from_kiosk_field(data) {
                                            sculpt_items.push(item);
                                        }
                                    }
                                }
                            }
                            
                            // Add small delay to avoid rate limiting
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                        }
                    }
                }
            }
            
            if response.has_next_page {
                cursor = response.next_cursor;
            } else {
                break;
            }
        }
        
        Ok(sculpt_items)
    }
    
    /// Parse Sculpt from a Kiosk dynamic field
    fn parse_sculpt_from_kiosk_field(&self, obj: &sui_sdk::rpc_types::SuiObjectData) -> Option<SculptItem> {
        let object_id = obj.object_id.to_string();
        
        obj.content.as_ref()
            .and_then(|content| match content {
                sui_sdk::rpc_types::SuiParsedData::MoveObject(move_obj) => {
                    if let sui_sdk::rpc_types::SuiMoveStruct::WithFields(fields) = &move_obj.fields {
                        // Try parsing Sculpt directly from fields
                        if let Some(item) = self.parse_sculpt_fields(fields, object_id.clone()) {
                            return Some(item);
                        }
                        
                        // If that fails, try looking for it wrapped in a "value" field
                        if let Some(sui_sdk::rpc_types::SuiMoveValue::Struct(value_struct)) = fields.get("value") {
                            if let sui_sdk::rpc_types::SuiMoveStruct::WithFields(sculpt_fields) = value_struct {
                                if let Some(item) = self.parse_sculpt_fields(sculpt_fields, object_id.clone()) {
                                    return Some(item);
                                }
                            }
                        }
                    }
                    None
                },
                _ => None,
            })
    }
    
    /// Parse Sculpt fields from a Move struct
    fn parse_sculpt_fields(
        &self,
        fields: &BTreeMap<String, SuiMoveValue>,
        object_id: String
    ) -> Option<SculptItem> {
        // Only require alias and printed - these should always be present
        let alias = match fields.get("alias") {
            Some(SuiMoveValue::String(s)) => s.clone(),
            _ => return None,
        };
        
        let printed = match fields.get("printed") {
            Some(SuiMoveValue::Number(n)) => {
                n.to_string().parse::<u64>().unwrap_or(0)
            }
            Some(SuiMoveValue::String(s)) => {
                s.parse::<u64>().unwrap_or(0)
            }
            _ => return None,
        };
        
        // All other fields are optional and use defaults if not present
        let encrypted = extract_bool_field(fields, "encrypted").unwrap_or(false);
        let structure_value = extract_string_field(fields, "structure").unwrap_or_default();
        let seal_resource_id = extract_string_field(fields, "seal_resource_id");
        
        Some(SculptItem {
            alias,
            blob_id: structure_value,
            printed_count: printed,
            id: object_id,
            is_encrypted: encrypted,
            seal_resource_id,
        })
    }
}

