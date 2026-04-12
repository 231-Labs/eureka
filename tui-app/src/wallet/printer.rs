use anyhow::{anyhow, Result};
use futures::TryStreamExt;
use serde_json::{Map, Value as Json};
use sui_rpc::proto::sui::rpc::v2::GetObjectRequest;
use sui_rpc::proto::sui::rpc::v2::ListOwnedObjectsRequest;
use sui_sdk_types::Address;

use super::move_json::{move_fields_map, prost_value_to_json};
use super::read_mask;
use super::types::PrinterInfo;
use super::utils::{extract_id_from_fields, extract_printer_id_from_cap};
use super::client::Wallet;

impl Wallet {
    fn extract_pool_balance(fields: &Map<String, Json>) -> u128 {
        fields
            .get("pool")
            .and_then(|pool_field| {
                let obj = pool_field.as_object()?;
                let inner = obj.get("fields")?;
                let value = inner.get("value")?;
                json_u128(value)
            })
            .unwrap_or(0)
    }

    fn extract_printer_from_json(&self, root: &Json) -> Option<PrinterInfo> {
        let fields = move_fields_map(root)?;
        let id = extract_id_from_fields(&fields)?;
        let pool_balance = Self::extract_pool_balance(&fields);
        Some(PrinterInfo { id, pool_balance })
    }

    pub async fn get_printer_cap_info(&self, address: Address) -> Result<(String, String)> {
        let current_package_id = self.network_state.get_current_package_ids().eureka_package_id;
        if current_package_id.is_empty() {
            return Err(anyhow!(
                "Eureka package ID is not set for this network (e.g. mainnet). Switch to devnet/testnet in the network menu or add an ID in constants."
            ));
        }

        // Align object_type filter with canonical on-chain address strings so gRPC filtering matches constants.
        let printer_cap_type = current_package_id
            .parse::<Address>()
            .map(|a| format!("{}::eureka::PrinterCap", a))
            .unwrap_or_else(|_| format!("{}::eureka::PrinterCap", current_package_id));

        let owner = address.to_string();

        {
            let client = self.rpc.lock().await;
            // page_size is per RPC page; list_owned_objects stream follows page_token until all matching objects are read.
            let req = ListOwnedObjectsRequest::default()
                .with_owner(owner.clone())
                .with_object_type(printer_cap_type)
                .with_page_size(50)
                .with_read_mask(read_mask("json,object_id"));

            let stream = client.list_owned_objects(req);
            tokio::pin!(stream);
            while let Some(obj) = stream.try_next().await? {
                let Some(pv) = obj.json.as_ref() else {
                    continue;
                };
                let j = prost_value_to_json(pv.as_ref());
                let Some(fields) = move_fields_map(&j) else {
                    continue;
                };
                if let (Some(cap_id), Some(printer_id)) = (
                    extract_id_from_fields(&fields),
                    extract_printer_id_from_cap(&fields),
                ) {
                    return Ok((cap_id, printer_id));
                }
            }
        }

        // Fallback: list all owned objects (still paginated), filter by type string containing PrinterCap.
        {
            let client = self.rpc.lock().await;
            let req = ListOwnedObjectsRequest::default()
                .with_owner(owner)
                .with_page_size(200)
                .with_read_mask(read_mask("json,object_type,object_id"));

            let stream = client.list_owned_objects(req);
            tokio::pin!(stream);
            while let Some(obj) = stream.try_next().await? {
                let t = obj.object_type_opt().unwrap_or("");
                if !t.contains("::eureka::PrinterCap") {
                    continue;
                }
                let Some(pv) = obj.json.as_ref() else {
                    continue;
                };
                let j = prost_value_to_json(pv.as_ref());
                let Some(fields) = move_fields_map(&j) else {
                    continue;
                };
                if let (Some(cap_id), Some(printer_id)) = (
                    extract_id_from_fields(&fields),
                    extract_printer_id_from_cap(&fields),
                ) {
                    return Ok((cap_id, printer_id));
                }
            }
        }

        Err(anyhow!(
            "No PrinterCap for this address. Register a printer first and ensure the selected network matches your wallet/deployment."
        ))
    }

    pub async fn get_printer_cap_id(&self, address: Address) -> Result<String> {
        let (cap_id, _) = self.get_printer_cap_info(address).await?;
        Ok(cap_id)
    }

    pub async fn get_printer_info(&self, address: Address) -> Result<PrinterInfo> {
        let (_, printer_id) = self.get_printer_cap_info(address).await?;
        let printer_aid: Address = printer_id
            .parse()
            .map_err(|e| anyhow!("Invalid printer ID format: {}", e))?;

        let mut client = self.rpc.lock().await;
        let resp = client
            .ledger_client()
            .get_object(
                GetObjectRequest::new(&printer_aid).with_read_mask(read_mask("json")),
            )
            .await?
            .into_inner();

        if let Some(j) = resp.object().json.as_ref() {
            let json = prost_value_to_json(j);
            if let Some(info) = self.extract_printer_from_json(&json) {
                return Ok(info);
            }
        }

        Err(anyhow!(
            "PrinterCap found but corresponding Printer object not found."
        ))
    }
}

fn json_u128(v: &Json) -> Option<u128> {
    match v {
        Json::String(s) => s.parse().ok(),
        Json::Number(n) => n.as_u64().map(|x| x as u128),
        _ => None,
    }
}
