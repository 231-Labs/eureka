use anyhow::Result;
use seal_sdk_rs::native_sui_sdk::client::seal_client::SealClient;
use seal_sdk_rs::session_key::SessionKey;
use seal_sdk_rs::native_sui_sdk::sui_sdk::SuiClientBuilder;
use seal_sdk_rs::native_sui_sdk::sui_sdk::wallet_context::WalletContext;
use seal_sdk_rs::native_sui_sdk::sui_types::Identifier;
use seal_sdk_rs::native_sui_sdk::sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use seal_sdk_rs::native_sui_sdk::sui_types::transaction::ProgrammableTransaction;
use seal_sdk_rs::native_sui_sdk::sui_sdk::rpc_types::{SuiObjectDataOptions, SuiMoveValue};
use seal_sdk_rs::generic_types::ObjectID as SealObjectID;
use sui_sdk::types::base_types::ObjectID;
use std::str::FromStr;
use std::path::Path;
use std::collections::BTreeMap;
use bcs;

/// Test decryption for Sculpt from new Archimeters contract
/// 
/// This example dynamically fetches sculpt information from the chain,
/// including seal_resource_id and encrypted STL blob ID.
/// 
/// Usage: cargo run --example test_new_contract -- <sculpt_id>
/// Example: cargo run --example test_new_contract -- 0x2055954dd22165b08f2e59f46b04d99adc5f740ffe82f922f2120723cebc68d5
/// 
/// NOTE: Frontend uses 3 key servers (threshold=1), we must use the same servers for decryption

struct DemoSetup {
    approve_package_id: seal_sdk_rs::generic_types::ObjectID,
    #[allow(dead_code)]
    key_server_ids: Vec<seal_sdk_rs::generic_types::ObjectID>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Get sculpt_id and printer_id from command line arguments
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 3 {
        return Err(anyhow::anyhow!(
            "Usage: cargo run --example test_new_contract -- <sculpt_id> <printer_id>"
        ));
    }
    
    let sculpt_id_str = &args[1];
    let printer_id_str = &args[2];
    
    println!("🔐 Decrypting Sculpt: {}", sculpt_id_str);
    println!("🖨️  Printer ID: {}", printer_id_str);

    // Configuration - New Archimeters package with printer_id parameter
    let package_id_str = "0x9a4790f287d6571f1b984577a4a4bb8bfc656b7126c01aa8a8a8881dca073ba7";
    
    // Key Servers (matching frontend config - all 3 servers with threshold=1)
    let key_server_strs = vec![
        "0x73d05d62c18d9374e3ea529e8e0ed6161da1a141a94d3f76ae3fe4e99356db75", // Mysten Labs 1
        "0xf5d14a81a982144ae441cd7d64b09027f116a468bd36e7eca494f750591623c8", // Mysten Labs 2
        "0x4cded1abeb52a22b6becb42a91d3686a4c901cf52eee16234214d0b5b2da4c46", // Triton One
    ];

    // Parse IDs
    let approve_package_id: SealObjectID = package_id_str.parse()?;
    let sculpt_id: ObjectID = sculpt_id_str.parse()?;
    let printer_id: ObjectID = printer_id_str.parse()?;
    let key_server_ids: Vec<SealObjectID> = key_server_strs
        .iter()
        .map(|s| s.parse())
        .collect::<Result<Vec<_>, _>>()?;

    let setup = DemoSetup {
        approve_package_id,
        key_server_ids,
    };

    // Connect to Sui testnet and fetch sculpt information
    let sui_client = SuiClientBuilder::default()
        .build("https://fullnode.testnet.sui.io:443")
        .await?;
    
    let (encrypted_blob_id, seal_resource_id) = fetch_sculpt_info(&sui_client, sculpt_id).await?;

    // Check if sculpt is encrypted
    let seal_id = match seal_resource_id {
        Some(id) => id,
        None => return Err(anyhow::anyhow!("Sculpt is not encrypted")),
    };

    // Download and parse encrypted data
    println!("📥 Downloading from Walrus...");
    let encrypted_data = download_encrypted_data(&encrypted_blob_id).await?;
    let encrypted_object = parse_encrypted_object(&encrypted_data)?;

    // Decrypt using Seal SDK
    println!("🔓 Decrypting...");
    decrypt_sculpt(&setup, sculpt_id, printer_id, &seal_id, encrypted_object).await?;

    println!("✅ Decryption completed successfully!");
    Ok(())
}

/// Download encrypted data from Walrus
async fn download_encrypted_data(blob_id: &str) -> Result<Vec<u8>> {
    let url = format!("https://aggregator.walrus-testnet.walrus.space/v1/blobs/{}", blob_id);
    let response = reqwest::get(&url).await?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to download: HTTP {}", response.status()));
    }

    Ok(response.bytes().await?.to_vec())
}

/// Parse encrypted data as EncryptedObject
fn parse_encrypted_object(data: &[u8]) -> Result<seal_sdk_rs::crypto::EncryptedObject> {
    // Try to deserialize as BCS
    let encrypted: seal_sdk_rs::crypto::EncryptedObject = bcs::from_bytes(data)
        .map_err(|e| anyhow::anyhow!("BCS deserialization failed: {}", e))?;
    Ok(encrypted)
}

/// Fetch sculpt information from chain
async fn fetch_sculpt_info(
    sui_client: &seal_sdk_rs::native_sui_sdk::sui_sdk::SuiClient,
    sculpt_id: ObjectID,
) -> Result<(String, Option<String>)> {
    let mut options = SuiObjectDataOptions::new();
    options.show_content = true;
    options.show_type = true;

    // Convert ObjectID to string and parse as the API expects
    let sculpt_id_str = sculpt_id.to_string();
    let sculpt_id_api: seal_sdk_rs::native_sui_sdk::sui_sdk::types::base_types::ObjectID = 
        sculpt_id_str.parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse ObjectID: {}", e))?;
    
    let object_response = sui_client
        .read_api()
        .get_object_with_options(sculpt_id_api, options)
        .await?;

    let object_data = object_response.data
        .ok_or_else(|| anyhow::anyhow!("Sculpt object not found"))?;

    let content = object_data.content
        .ok_or_else(|| anyhow::anyhow!("Sculpt object has no content"))?;

    let fields = match content {
        seal_sdk_rs::native_sui_sdk::sui_sdk::rpc_types::SuiParsedData::MoveObject(ref obj) => {
            match &obj.fields {
                seal_sdk_rs::native_sui_sdk::sui_sdk::rpc_types::SuiMoveStruct::WithFields(f) => f,
                _ => return Err(anyhow::anyhow!("Sculpt fields are not in WithFields format")),
            }
        }
        _ => return Err(anyhow::anyhow!("Sculpt is not a Move object")),
    };

    // Extract structure (encrypted STL blob ID) - Option<String>
    let structure = extract_option_string_field(fields, "structure")
        .ok_or_else(|| anyhow::anyhow!("Sculpt has no structure field (STL blob ID)"))?;

    // Extract seal_resource_id - Option<String>
    let seal_resource_id = extract_option_string_field(fields, "seal_resource_id");

    Ok((structure, seal_resource_id))
}

/// Extract Option<String> field from Move struct fields
fn extract_option_string_field(
    fields: &BTreeMap<String, SuiMoveValue>,
    field_name: &str,
) -> Option<String> {
    match fields.get(field_name)? {
        SuiMoveValue::String(value) if !value.is_empty() => Some(value.clone()),
        SuiMoveValue::Option(opt) => {
            match opt.as_ref() {
                Some(SuiMoveValue::String(value)) if !value.is_empty() => Some(value.clone()),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Decrypt sculpt using Seal SDK
async fn decrypt_sculpt(
    setup: &DemoSetup,
    _sculpt_id: ObjectID,
    printer_id: ObjectID,
    seal_id: &str,
    encrypted: seal_sdk_rs::crypto::EncryptedObject,
) -> Result<()> {
    // Connect to Sui testnet
    let sui_client = SuiClientBuilder::default()
        .build("https://fullnode.testnet.sui.io:443")
        .await?;

    let client = SealClient::new(sui_client);

    // Load wallet
    let wallet_path = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("Cannot find HOME env var"))?
        + "/.sui/sui_config/client.yaml";

    let mut wallet = WalletContext::new(Path::new(&wallet_path))?;
    
    let session_key = SessionKey::new(
        setup.approve_package_id,
        10,
        &mut wallet,
    )
    .await?;

    // Build approval transaction
    let mut builder = ProgrammableTransactionBuilder::new();

    // seal_id format: "package_id:resource_id"
    // We need to extract only the resource_id part (after the colon)
    let resource_id = if seal_id.contains(':') {
        seal_id.split(':').nth(1)
            .ok_or_else(|| anyhow::anyhow!("Invalid seal_resource_id format"))?
    } else {
        seal_id
    };
    
    // Remove 0x prefix if present
    let id_hex = resource_id.strip_prefix("0x").unwrap_or(resource_id);
    
    // Decode hex string to bytes
    let id_bytes = hex::decode(id_hex)
        .map_err(|e| anyhow::anyhow!("Failed to decode hex ID: {}", e))?;
    
    // Prepare arguments for seal_approve
    let id_arg = builder.pure(id_bytes)?;
    let printer_id_arg = builder.pure(bcs::to_bytes(&printer_id)?)?;

    // Call seal_approve function in the sculpt module
    // Parameters: _id: vector<u8>, _printer_id: ID
    builder.programmable_move_call(
        setup.approve_package_id.into(),
        Identifier::from_str("sculpt")?,
        Identifier::from_str("seal_approve")?,
        vec![],  // No type arguments needed
        vec![id_arg, printer_id_arg],
    );

    let approve_ptb: ProgrammableTransaction = builder.finish();

    // Decrypt
    let plaintext = client
        .decrypt_object_bytes(
            &bcs::to_bytes(&encrypted)?,
            approve_ptb,
            &session_key,
        )
        .await?;

    // Save and verify decrypted STL
    let output_file = "decrypted_sculpt.stl";
    std::fs::write(output_file, &plaintext)?;
    
    let format = if plaintext.starts_with(b"solid") {
        "ASCII STL"
    } else if plaintext.len() > 84 {
        "Binary STL"
    } else {
        "Unknown"
    };
    
    println!("💾 Saved: {} ({}, {} bytes)", output_file, format, plaintext.len());
    Ok(())
}

