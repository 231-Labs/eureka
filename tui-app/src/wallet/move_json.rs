//! Parse Sui Move object `json` fields from gRPC `prost_types::Value` / `serde_json::Value`.

use anyhow::{anyhow, Result};
use prost_types::value::Kind;
use serde_json::{Map, Number, Value as Json};

pub fn prost_value_to_json(v: &prost_types::Value) -> Json {
    match &v.kind {
        None => Json::Null,
        Some(Kind::NullValue(_)) => Json::Null,
        Some(Kind::NumberValue(n)) => Number::from_f64(*n)
            .map(Json::Number)
            .unwrap_or(Json::Null),
        Some(Kind::StringValue(s)) => Json::String(s.clone()),
        Some(Kind::BoolValue(b)) => Json::Bool(*b),
        Some(Kind::StructValue(s)) => {
            let mut m = Map::new();
            for (k, fv) in &s.fields {
                m.insert(k.clone(), prost_value_to_json(fv));
            }
            Json::Object(m)
        }
        Some(Kind::ListValue(l)) => {
            Json::Array(l.values.iter().map(prost_value_to_json).collect())
        }
    }
}

/// Resolve a Move object's `fields` map from various JSON shapes (JSON-RPC, gRPC `Object.json`, etc.).
pub fn move_fields_map(root: &Json) -> Option<Map<String, Json>> {
    move_fields_map_inner(root, 4)
}

fn move_fields_map_inner(root: &Json, depth: u8) -> Option<Map<String, Json>> {
    if depth == 0 {
        return None;
    }

    let root = match root {
        Json::String(s) => serde_json::from_str::<Json>(s).ok()?,
        _ => root.clone(),
    };

    if let Some(f) = try_extract_fields_from_object(&root) {
        return Some(f);
    }

    let m = root.as_object()?;
    for key in ["object", "value", "data", "result", "response"] {
        if let Some(inner) = m.get(key) {
            if let Some(f) = move_fields_map_inner(inner, depth - 1) {
                return Some(f);
            }
        }
    }

    // Sui gRPC `Object.json` often puts Move struct fields at the top level (e.g. `id`, `printer_id` strings)
    // without JSON-RPC-style `data.content.fields` wrapping; only looking for `fields` used to yield None and miss PrinterCap.
    if m.get("fields").is_none() && m.contains_key("id") && !m.contains_key("dataType") {
        return Some(m.clone());
    }

    None
}

fn try_extract_fields_from_object(root: &Json) -> Option<Map<String, Json>> {
    let m = root.as_object()?;

    if let Some(Json::Object(f)) = m.get("fields") {
        return Some(f.clone());
    }

    if let Some(Json::Object(contents)) = m.get("contents") {
        if let Some(Json::Object(f)) = contents.get("fields") {
            return Some(f.clone());
        }
    }

    // Common in `sui_getObject` / gRPC JSON: `content: { dataType, type, fields }`
    if let Some(Json::Object(content)) = m.get("content") {
        if let Some(Json::Object(f)) = content.get("fields") {
            return Some(f.clone());
        }
    }

    if let Some(Json::Object(data)) = m.get("data") {
        if let Some(Json::Object(content)) = data.get("content") {
            if let Some(Json::Object(f)) = content.get("fields") {
                return Some(f.clone());
            }
        }
        if let Some(Json::Object(f)) = data.get("fields") {
            return Some(f.clone());
        }
    }

    None
}

/// Kiosk `for` / UID / `ID` values in gRPC JSON: string, `{ "id": "0x..." }`, or `{ "bytes": "..." }`.
pub fn json_address_from_move_value(v: &Json) -> Option<sui_sdk_types::Address> {
    use sui_sdk_types::Address;
    match v {
        Json::String(s) => s.parse().ok(),
        Json::Object(o) => {
            if let Some(Json::String(id)) = o.get("id") {
                if let Ok(a) = id.parse::<Address>() {
                    return Some(a);
                }
            }
            if let Some(Json::String(b)) = o.get("bytes") {
                let hx = if b.starts_with("0x") {
                    b.clone()
                } else {
                    format!("0x{}", b)
                };
                return hx.parse().ok();
            }
            None
        }
        _ => None,
    }
}

pub fn json_string(v: &Json) -> Option<String> {
    match v {
        Json::String(s) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

pub fn json_u64_string(v: &Json) -> Option<u64> {
    match v {
        Json::String(s) => s.parse().ok(),
        Json::Number(n) => n.as_u64(),
        _ => None,
    }
}

pub fn json_bool(v: &Json) -> Option<bool> {
    v.as_bool()
}

/// `0x2::object::UID` often appears as `{ "id": "0x..." }` or nested JSON with `type` / `fields`.
pub fn extract_id_from_fields(fields: &Map<String, Json>) -> Option<String> {
    let idv = fields.get("id")?;
    json_object_id_value(idv, 8)
}

fn normalize_hex_id(s: &str) -> Option<String> {
    if s.is_empty() {
        return None;
    }
    if s.starts_with("0x") {
        Some(s.to_string())
    } else {
        Some(format!("0x{}", s))
    }
}

/// gRPC/indexers often serialize `ID`/`UID` as `{ "fields": { "bytes" | "id": ... } }` or nested `id` objects; this resolves a 0x address string.
fn json_object_id_value(v: &Json, depth: u8) -> Option<String> {
    if depth == 0 {
        return None;
    }
    match v {
        Json::String(s) => normalize_hex_id(s),
        Json::Object(o) => {
            if let Some(inner) = o.get("id") {
                if let Some(s) = json_object_id_value(inner, depth - 1) {
                    return Some(s);
                }
            }
            if let Some(b) = o.get("bytes") {
                if let Json::String(s) = b {
                    if let Some(out) = normalize_hex_id(s) {
                        return Some(out);
                    }
                } else if let Some(s) = json_object_id_value(b, depth - 1) {
                    return Some(s);
                }
            }
            if let Some(Json::Object(f)) = o.get("fields") {
                for key in ["id", "bytes"] {
                    if let Some(inner) = f.get(key) {
                        if let Some(s) = json_object_id_value(inner, depth - 1) {
                            return Some(s);
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

pub fn extract_printer_id_from_cap(fields: &Map<String, Json>) -> Option<String> {
    let v = fields.get("printer_id")?;
    json_object_id_value(v, 8)
}

pub fn extract_string_field(fields: &Map<String, Json>, name: &str) -> Option<String> {
    let v = fields.get(name)?;
    match v {
        Json::String(_) => json_string(v),
        Json::Null => None,
        Json::Object(o) => {
            if let Some(Json::String(s)) = o.get("vec") {
                return Some(s.clone());
            }
            if let Some(Json::Array(_)) = o.get("vec") {
                return None;
            }
            None
        }
        _ => None,
    }
}

pub fn extract_address_field(fields: &Map<String, Json>, name: &str) -> Option<String> {
    let v = fields.get(name)?;
    json_object_id_value(v, 8)
}

pub fn extract_bool_field(fields: &Map<String, Json>, name: &str) -> Option<bool> {
    fields.get(name).and_then(json_bool)
}

/// Move `option::Option<String>` JSON shapes: `null`, `{ "Some": "..." }`, or a plain string.
pub fn extract_optional_string_field(fields: &Map<String, Json>, name: &str) -> Option<String> {
    let v = fields.get(name)?;
    match v {
        Json::Null => None,
        Json::String(s) if !s.is_empty() => Some(s.clone()),
        Json::Object(o) => match o.get("Some") {
            Some(Json::String(s)) if !s.is_empty() => Some(s.clone()),
            _ => None,
        },
        _ => None,
    }
}

pub fn extract_optional_u64_field(fields: &Map<String, Json>, name: &str) -> Option<u64> {
    match fields.get(name)? {
        Json::Null => None,
        Json::Object(o) => {
            if let Some(inner) = o.get("Some") {
                return json_u64_string(inner);
            }
            None
        }
        other => json_u64_string(other),
    }
}

pub fn extract_print_task_from_object_json(root: &Json) -> Result<crate::app::print_job::PrintTask> {
    use crate::app::print_job::{PrintTask, TaskStatus};

    let fields = move_fields_map(root).ok_or_else(|| anyhow!("missing Move fields in json"))?;

    let id = extract_id_from_fields(&fields).ok_or_else(|| anyhow!("job id"))?;
    let name = extract_string_field(&fields, "sculpt_alias").ok_or_else(|| anyhow!("sculpt_alias"))?;
    let sculpt_id = extract_address_field(&fields, "sculpt_id").ok_or_else(|| anyhow!("sculpt_id"))?;
    let sculpt_structure =
        extract_string_field(&fields, "sculpt_structure").ok_or_else(|| anyhow!("sculpt_structure"))?;
    let customer = extract_address_field(&fields, "customer").ok_or_else(|| anyhow!("customer"))?;
    let paid_amount = extract_balance_value(&fields).ok_or_else(|| anyhow!("paid_amount"))?;
    let is_completed =
        extract_bool_field(&fields, "is_completed").ok_or_else(|| anyhow!("is_completed"))?;
    let start_time = extract_optional_u64_field(&fields, "start_time");
    let end_time = extract_optional_u64_field(&fields, "end_time");
    let seal_resource_id = extract_optional_string_field(&fields, "seal_resource_id");

    Ok(PrintTask {
        id,
        name,
        sculpt_blob_id: sculpt_id,
        sculpt_structure,
        customer,
        paid_amount,
        start_time,
        end_time,
        seal_resource_id,
        status: if is_completed {
            TaskStatus::Completed
        } else {
            TaskStatus::Active
        },
    })
}

fn extract_balance_value(fields: &Map<String, Json>) -> Option<u64> {
    match fields.get("paid_amount")? {
        Json::String(s) => s.parse().ok(),
        Json::Number(n) => n.as_u64(),
        _ => None,
    }
}
