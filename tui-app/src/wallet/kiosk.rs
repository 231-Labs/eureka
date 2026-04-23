use anyhow::{anyhow, Result};
use futures::TryStreamExt;
use serde_json::{Map, Value as Json};
use sui_rpc::proto::sui::rpc::v2::GetObjectRequest;
use sui_rpc::proto::sui::rpc::v2::ListDynamicFieldsRequest;
use sui_rpc::proto::sui::rpc::v2::ListOwnedObjectsRequest;
use sui_sdk_types::Address;

use crate::constants::NETWORK_PACKAGE_IDS;
use super::move_json::{json_address_from_move_value, move_fields_map, prost_value_to_json};
use super::read_mask;
use super::utils::{extract_bool_field, extract_string_field};
use super::types::SculptItem;
use super::client::Wallet;

impl Wallet {
    pub async fn get_all_kiosk_sculpts(&self, address: Address) -> Result<Vec<SculptItem>> {
        let mut all_sculpts = Vec::new();
        let kiosk_ids = self.get_owned_kiosk_ids(address).await?;
        for kiosk_id in kiosk_ids {
            if let Ok(sculpts) = self.get_sculpts_from_kiosk(kiosk_id).await {
                all_sculpts.extend(sculpts);
            }
        }
        Ok(all_sculpts)
    }

    /// If `sculpt_id` lives in one of `owner`'s kiosks, return that kiosk's id.
    pub async fn find_kiosk_id_for_sculpt(
        &self,
        owner: Address,
        sculpt_id: Address,
    ) -> Result<Option<Address>> {
        let want = sculpt_id.to_string().to_lowercase();
        for kiosk_id in self.get_owned_kiosk_ids(owner).await? {
            let sculpts = self.get_sculpts_from_kiosk(kiosk_id).await?;
            if sculpts.iter().any(|s| s.id.to_lowercase() == want) {
                return Ok(Some(kiosk_id));
            }
        }
        Ok(None)
    }

    async fn get_owned_kiosk_ids(&self, address: Address) -> Result<Vec<Address>> {
        let mut kiosk_ids = Vec::new();
        let client = self.rpc.lock().await;
        let req = ListOwnedObjectsRequest::default()
            .with_owner(address.to_string())
            .with_page_size(200)
            .with_read_mask(read_mask("object_type,json"));

        let stream = client.list_owned_objects(req);
        tokio::pin!(stream);
        while let Some(obj) = stream.try_next().await? {
            let t = obj.object_type_opt().unwrap_or("");
            if !t.contains("::kiosk::KioskOwnerCap") {
                continue;
            }
            let Some(j) = obj
                .json
                .as_ref()
                .map(|v| prost_value_to_json(v.as_ref()))
            else {
                continue;
            };
            if let Some(fields) = move_fields_map(&j) {
                if let Some(k) = fields.get("for") {
                    if let Some(aid) = json_address_from_move_value(k) {
                        kiosk_ids.push(aid);
                    }
                }
            }
        }
        Ok(kiosk_ids)
    }

    /// Find the owned `KioskOwnerCap` object id whose `for` field matches `kiosk_id`.
    pub async fn resolve_kiosk_owner_cap_object_id(
        &self,
        owner: Address,
        kiosk_id: Address,
    ) -> Result<Address> {
        let client = self.rpc.lock().await;
        let req = ListOwnedObjectsRequest::default()
            .with_owner(owner.to_string())
            .with_page_size(200)
            .with_read_mask(read_mask("object_type,json,object_id"));

        let stream = client.list_owned_objects(req);
        tokio::pin!(stream);
        while let Some(obj) = stream.try_next().await? {
            let t = obj.object_type_opt().unwrap_or("");
            if !t.contains("::kiosk::KioskOwnerCap") {
                continue;
            }
            let Some(j) = obj
                .json
                .as_ref()
                .map(|v| prost_value_to_json(v.as_ref()))
            else {
                continue;
            };
            let Some(fields) = move_fields_map(&j) else {
                continue;
            };
            let Some(for_addr) = fields
                .get("for")
                .and_then(json_address_from_move_value)
            else {
                continue;
            };
            if for_addr != kiosk_id {
                continue;
            }
            let cap_str = obj.object_id_opt().ok_or_else(|| anyhow!("KioskOwnerCap missing object_id"))?;
            return cap_str
                .parse()
                .map_err(|e| anyhow!("KioskOwnerCap id: {}", e));
        }

        Err(anyhow!(
            "No KioskOwnerCap found for kiosk {}; ensure this wallet owns the cap for that kiosk.",
            kiosk_id
        ))
    }

    async fn get_sculpts_from_kiosk(&self, kiosk_id: Address) -> Result<Vec<SculptItem>> {
        let mut sculpt_items = Vec::new();
        let network = self.network_state.current_network;
        let current_package_id = NETWORK_PACKAGE_IDS[network as usize].bottega_package_id;
        let sculpt_pkg_canonical: Option<String> = current_package_id
            .parse::<Address>()
            .ok()
            .map(|a| a.to_string());

        let mut client = self.rpc.lock().await;
        let list_req = ListDynamicFieldsRequest::default()
            .with_parent(kiosk_id.to_string())
            .with_page_size(200)
            // `ListDynamicFieldsRequest.read_mask` paths are relative to `DynamicField` (see proto; default parent,field_id).
            .with_read_mask(read_mask("child_id,field_object.object_id"));

        let stream = client.list_dynamic_fields(list_req);
        tokio::pin!(stream);
        while let Some(df) = stream.try_next().await? {
            let child_id = df
                .child_id
                .clone()
                .or_else(|| df.field_object.as_ref().and_then(|o| o.object_id.clone()));
            let Some(cid) = child_id else { continue };
            let Ok(child_aid) = cid.parse::<Address>() else { continue };

            let resp = match client
                .ledger_client()
                .get_object(
                    GetObjectRequest::new(&child_aid)
                        .with_read_mask(read_mask("json,object_type,object_id")),
                )
                .await
            {
                Ok(r) => r.into_inner(),
                Err(_) => continue,
            };

            let obj = resp.object();
            let t = obj.object_type_opt().unwrap_or("");
            if !t.contains("::sculpt::Sculpt") {
                continue;
            }
            if current_package_id.is_empty() {
                continue;
            }
            let matches_package = t.contains(current_package_id)
                || sculpt_pkg_canonical
                    .as_ref()
                    .is_some_and(|c| !c.is_empty() && t.contains(c.as_str()));
            if !matches_package {
                continue;
            }
            let oid = obj.object_id_opt().unwrap_or_default().to_string();
            if let Some(j) = obj
                .json
                .as_ref()
                .map(|v| prost_value_to_json(v.as_ref()))
            {
                if let Some(item) = self.parse_sculpt_from_kiosk_json(&j, oid, kiosk_id) {
                    sculpt_items.push(item);
                }
            }
        }

        Ok(sculpt_items)
    }

    fn parse_sculpt_from_kiosk_json(
        &self,
        root: &Json,
        object_id: String,
        source_kiosk: Address,
    ) -> Option<SculptItem> {
        let fields = move_fields_map(root)?;
        let mut item = if let Some(i) = self.parse_sculpt_fields_json(&fields, object_id.clone()) {
            i
        } else {
            let value = fields.get("value")?;
            let inner = move_fields_map(value)?;
            self.parse_sculpt_fields_json(&inner, object_id)?
        };
        item.source_kiosk_id = Some(source_kiosk.to_string());
        Some(item)
    }

    fn parse_sculpt_fields_json(
        &self,
        fields: &Map<String, Json>,
        object_id: String,
    ) -> Option<SculptItem> {
        let alias = match fields.get("alias") {
            Some(Json::String(s)) => s.clone(),
            _ => return None,
        };
        let printed = match fields.get("printed") {
            Some(Json::Number(n)) => n.as_u64().unwrap_or(0),
            Some(Json::String(s)) => s.parse().unwrap_or(0),
            _ => return None,
        };
        let encrypted = extract_bool_field(fields, "encrypted").unwrap_or(false);
        let structure_value = extract_string_field(fields, "structure").unwrap_or_default();
        let seal_resource_id = extract_string_field(fields, "seal_resource_id");

        Some(SculptItem {
            alias,
            blob_id: structure_value,
            printed_count: printed,
            id: object_id,
            source_kiosk_id: None,
            is_encrypted: encrypted,
            seal_resource_id,
        })
    }
}
