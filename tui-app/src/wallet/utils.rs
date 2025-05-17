use std::collections::BTreeMap;
use sui_sdk::rpc_types::SuiMoveValue;

// Helper method: extract ID from fields
pub fn extract_id_from_fields(fields: &BTreeMap<String, SuiMoveValue>) -> Option<String> {
    fields.get("id").and_then(|id_field| {
        if let SuiMoveValue::UID { id } = id_field {
            // Ensure ID has 0x prefix
            let id_str = id.to_string();
            let formatted_id = if !id_str.starts_with("0x") {
                format!("0x{}", id_str)
            } else {
                id_str
            };
            Some(formatted_id)
        } else {
            None
        }
    })
}

// Helper method: extract printer_id from PrinterCap
pub fn extract_printer_id_from_cap(fields: &BTreeMap<String, SuiMoveValue>) -> Option<String> {
    fields.get("printer_id").and_then(|id_field| {
        if let SuiMoveValue::Address(id) = id_field {
            // Ensure ID has 0x prefix
            let id_str = id.to_string();
            let formatted_id = if !id_str.starts_with("0x") {
                format!("0x{}", id_str)
            } else {
                id_str
            };
            Some(formatted_id)
        } else {
            None
        }
    })
}

// Helper method: extract string field
pub fn extract_string_field(fields: &BTreeMap<String, SuiMoveValue>, field_name: &str) -> Option<String> {
    if let Some(SuiMoveValue::String(value)) = fields.get(field_name) {
        Some(value.clone())
    } else {
        None
    }
}

// Helper method: extract address field
pub fn extract_address_field(fields: &BTreeMap<String, SuiMoveValue>, field_name: &str) -> Option<String> {
    if let Some(SuiMoveValue::Address(address)) = fields.get(field_name) {
        Some(format!("0x{}", address))
    } else {
        None
    }
}

// Helper method: extract boolean field
pub fn extract_bool_field(fields: &BTreeMap<String, SuiMoveValue>, field_name: &str) -> Option<bool> {
    if let Some(SuiMoveValue::Bool(value)) = fields.get(field_name) {
        Some(*value)
    } else {
        None
    }
}

// Helper method: extract optional u64 field
pub fn extract_optional_u64_field(fields: &BTreeMap<String, SuiMoveValue>, field_name: &str) -> Option<u64> {
    fields.get(field_name).and_then(|value| {
        if let SuiMoveValue::Option(inner) = value {
            if let Some(inner_value) = inner.as_ref() {
                if let SuiMoveValue::Number(num) = inner_value {
                    num.to_string().parse::<u64>().ok()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    })
} 