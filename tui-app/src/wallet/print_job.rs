use std::collections::BTreeMap;
use anyhow::{Result, anyhow};
use sui_sdk::{
    rpc_types::{
        SuiMoveStruct,
        SuiMoveValue,
        SuiParsedData,
    },
    types::{
        base_types::ObjectID,
        dynamic_field::DynamicFieldName,
        TypeTag,
    },
};
use serde_json;
use super::client::Wallet;
use super::utils::{extract_id_from_fields, extract_string_field, extract_address_field, extract_bool_field, extract_optional_u64_field};
use crate::app::print_job::{PrintTask, TaskStatus};

// Extract print task from blockchain data
pub fn extract_print_task(fields: &BTreeMap<String, SuiMoveValue>) -> Result<PrintTask> {
    let id = extract_id_from_fields(fields)
        .ok_or_else(|| anyhow!("Failed to extract job ID"))?;
    let sculpt_alias = extract_string_field(fields, "sculpt_alias")
        .ok_or_else(|| anyhow!("Failed to extract sculpt_alias field"))?;
    let sculpt_id = extract_address_field(fields, "sculpt_id")
        .ok_or_else(|| anyhow!("Failed to extract sculpt_id field"))?;
    let sculpt_structure = extract_string_field(fields, "sculpt_structure")
        .ok_or_else(|| anyhow!("Failed to extract sculpt_structure field"))?;
    let customer = extract_address_field(fields, "customer")
        .ok_or_else(|| anyhow!("Failed to extract customer field"))?;
    let paid_amount = extract_balance_value(fields)
        .ok_or_else(|| anyhow!("Failed to extract paid_amount field"))?;
    let is_completed = extract_bool_field(fields, "is_completed")
        .ok_or_else(|| anyhow!("Failed to extract is_completed field"))?;
    let start_time = extract_optional_u64_field(fields, "start_time");
    let end_time = extract_optional_u64_field(fields, "end_time");

    Ok(PrintTask {
        id,
        name: sculpt_alias,
        sculpt_id,
        sculpt_structure,
        customer,
        paid_amount,
        start_time,
        end_time,
        status: if is_completed { TaskStatus::Completed } else { TaskStatus::Printing },
    })
}

// Helper method: extract balance value
fn extract_balance_value(fields: &BTreeMap<String, SuiMoveValue>) -> Option<u64> {
    if let Some(SuiMoveValue::String(value)) = fields.get("paid_amount") {
        return value.parse::<u64>().ok();
    }
    None
}

impl Wallet {
    // Get active print job from the blockchain for a printer
    pub async fn get_active_print_job(&self, printer_id: &str) -> Result<Option<PrintTask>> {
        let printer_object_id = ObjectID::from_hex_literal(printer_id)
            .map_err(|e| anyhow!("Invalid printer ID format: {}", e))?;

        // Use get_dynamic_field_object to get print_job
        let response = self.client.read_api()
            .get_dynamic_field_object(
                printer_object_id,
                DynamicFieldName {
                    type_: TypeTag::Vector(Box::new(TypeTag::U8)),
                    value: serde_json::Value::String("print_job".to_string()),
                },
            )
            .await?;

        if let Some(data) = response.data {
            if let Some(content) = data.content {
                if let SuiParsedData::MoveObject(move_obj) = content {
                    if let SuiMoveStruct::WithFields(fields) = &move_obj.fields {
                        let task = extract_print_task(fields)?;
                        return Ok(Some(task));
                    }
                }
            }
        }
        Ok(None)
    }
} 