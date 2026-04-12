use anyhow::{anyhow, Result};
use futures::TryStreamExt;
use sui_rpc::proto::sui::rpc::v2::GetObjectRequest;
use sui_rpc::proto::sui::rpc::v2::ListDynamicFieldsRequest;
use sui_sdk_types::Address;

use super::move_json::{extract_print_task_from_object_json, prost_value_to_json};
use super::read_mask;
use super::client::Wallet;
use crate::app::print_job::PrintTask;

impl Wallet {
    pub async fn get_active_print_job(&self, printer_id: &str) -> Result<Option<PrintTask>> {
        let printer_aid: Address = printer_id
            .parse()
            .map_err(|e| anyhow!("Invalid printer ID format: {}", e))?;

        let mut client = self.rpc.lock().await;
        let list_req = ListDynamicFieldsRequest::default()
            .with_parent(printer_aid.to_string())
            .with_page_size(200)
            // Paths are relative to `DynamicField` (not `dynamic_fields.*`). Include field_object.object_id as kiosk-style fallback.
            .with_read_mask(read_mask("child_id,field_object.object_id"));

        let stream = client.list_dynamic_fields(list_req);
        tokio::pin!(stream);
        while let Some(df) = stream.try_next().await? {
            let Some(cid) = df.child_id.clone() else { continue };
            let Ok(child_aid) = cid.parse::<Address>() else { continue };

            let resp = match client
                .ledger_client()
                .get_object(
                    GetObjectRequest::new(&child_aid).with_read_mask(read_mask("json")),
                )
                .await
            {
                Ok(r) => r.into_inner(),
                Err(_) => continue,
            };

            let obj = resp.object();
            if let Some(j) = obj
                .json
                .as_ref()
                .map(|v| prost_value_to_json(v.as_ref()))
            {
                if let Ok(task) = extract_print_task_from_object_json(&j) {
                    return Ok(Some(task));
                }
            }
        }

        Ok(None)
    }
}
