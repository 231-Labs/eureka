//! Verify `ListDynamicFields` `read_mask` paths (relative to `DynamicField`, not `dynamic_fields.*`).
//! Run: `cargo run -p tui-app --example grpc_inspect_dynamic_fields -- <PARENT_OBJECT_ID>`

use anyhow::Result;
use futures::TryStreamExt;
use sui_rpc::field::FieldMaskUtil;
use sui_rpc::proto::sui::rpc::v2::ListDynamicFieldsRequest;
use sui_rpc::Client;

fn read_mask(paths: &str) -> prost_types::FieldMask {
    <prost_types::FieldMask as FieldMaskUtil>::from_str(paths)
}

#[tokio::main]
async fn main() -> Result<()> {
    let parent = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "0xb6703e859ec0f6be599d48d080fe8d9adddce22a6d7a7cf2bc5a2798b7a4deb5".to_string());

    let client = Client::new(Client::TESTNET_FULLNODE)?;

    for (label, mask) in [
        ("BAD dynamic_fields.child_id", "dynamic_fields.child_id"),
        ("GOOD child_id", "child_id"),
        ("GOOD child_id,field_object.object_id", "child_id,field_object.object_id"),
    ] {
        print!("{label}: ");
        let req = ListDynamicFieldsRequest::default()
            .with_parent(parent.clone())
            .with_page_size(3)
            .with_read_mask(read_mask(mask));

        let stream = client.list_dynamic_fields(req);
        tokio::pin!(stream);
        match stream.try_next().await {
            Ok(Some(df)) => {
                println!(
                    "ok — child_id={:?} field_object.object_id={:?}",
                    df.child_id.as_deref(),
                    df.field_object.as_ref().and_then(|o| o.object_id_opt())
                );
            }
            Ok(None) => println!("empty page"),
            Err(e) => println!("ERR {e}"),
        }
    }

    Ok(())
}
