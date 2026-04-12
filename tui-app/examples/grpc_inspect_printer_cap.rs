//! Call testnet gRPC `ListOwnedObjects` and print real `Object.json` for PrinterCap (debug).
//!
//! Run: `cargo run -p tui-app --example grpc_inspect_printer_cap -- [OWNER_HEX]`

use anyhow::Result;
use futures::TryStreamExt;
use prost_types::value::Kind;
use serde_json::{Map, Number, Value as Json};
use sui_rpc::field::FieldMaskUtil;
use sui_rpc::proto::sui::rpc::v2::ListOwnedObjectsRequest;
use sui_rpc::Client;
use sui_sdk_types::Address;

fn read_mask(paths: &str) -> prost_types::FieldMask {
    <prost_types::FieldMask as FieldMaskUtil>::from_str(paths)
}

fn prost_to_json(v: &prost_types::Value) -> Json {
    match &v.kind {
        None | Some(Kind::NullValue(_)) => Json::Null,
        Some(Kind::NumberValue(n)) => Number::from_f64(*n)
            .map(Json::Number)
            .unwrap_or(Json::Null),
        Some(Kind::StringValue(s)) => Json::String(s.clone()),
        Some(Kind::BoolValue(b)) => Json::Bool(*b),
        Some(Kind::StructValue(s)) => {
            let mut m = Map::new();
            for (k, fv) in &s.fields {
                m.insert(k.clone(), prost_to_json(fv));
            }
            Json::Object(m)
        }
        Some(Kind::ListValue(l)) => Json::Array(l.values.iter().map(prost_to_json).collect()),
    }
}

const DEFAULT_OWNER: &str = "0x006d980cadd43c778e628201b45cfd3ba6e1047c65f67648a88f635108ffd6eb";
const EUREKA_TESTNET_PKG: &str = "0x8852004ffc677790d0ee729aa386286cbcbc7f4f1b4aa87c50213d2acb5d678f";

#[tokio::main]
async fn main() -> Result<()> {
    let owner = std::env::args().nth(1).unwrap_or_else(|| DEFAULT_OWNER.to_string());
    let client = Client::new(Client::TESTNET_FULLNODE)?;

    let pkg: Address = EUREKA_TESTNET_PKG.parse()?;
    let cap_type = format!("{}::eureka::PrinterCap", pkg);

    println!("owner={owner}\nobject_type filter={cap_type}\n");

    println!("=== A) read_mask \"json,object_id\" (same as printer.rs) ===\n");
    let req = ListOwnedObjectsRequest::default()
        .with_owner(owner.clone())
        .with_object_type(cap_type.clone())
        .with_page_size(5)
        .with_read_mask(read_mask("json,object_id"));

    let stream = client.list_owned_objects(req);
    tokio::pin!(stream);
    let mut n = 0;
    while let Some(obj) = stream.try_next().await? {
        n += 1;
        println!("--- #{n} ---");
        println!("object_id: {:?}", obj.object_id_opt());
        println!("object_type: {:?}", obj.object_type_opt());
        match obj.json.as_deref() {
            None => println!("json: <MISSING / unset>"),
            Some(jv) => {
                let j = prost_to_json(jv);
                println!("{}", serde_json::to_string_pretty(&j)?);
            }
        }
    }
    if n == 0 {
        println!("(0 objects for this filter — object_type may not match node index)\n");
    }

    println!("\n=== B) No read_mask (see node default fields) ===\n");
    let req2 = ListOwnedObjectsRequest::default()
        .with_owner(owner.clone())
        .with_object_type(cap_type)
        .with_page_size(5);

    let stream2 = client.list_owned_objects(req2);
    tokio::pin!(stream2);
    if let Some(obj) = stream2.try_next().await? {
        println!("object_id: {:?}", obj.object_id_opt());
        println!("object_type: {:?}", obj.object_type_opt());
        match obj.json.as_deref() {
            None => println!("json: <MISSING>"),
            Some(jv) => {
                let j = prost_to_json(jv);
                println!("top-level keys: {:?}", j.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()));
                println!("{}", serde_json::to_string_pretty(&j)?);
            }
        }
    } else {
        println!("(still 0 objects)");
    }

    Ok(())
}
