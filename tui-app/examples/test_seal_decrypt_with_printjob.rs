use anyhow::Result;
use seal_sdk_rs::native_sui_sdk::client::seal_client::SealClient;
use seal_sdk_rs::session_key::SessionKey;
use seal_sdk_rs::native_sui_sdk::sui_sdk::SuiClientBuilder;
use seal_sdk_rs::native_sui_sdk::sui_sdk::wallet_context::WalletContext;
use seal_sdk_rs::native_sui_sdk::sui_types::Identifier;
use seal_sdk_rs::native_sui_sdk::sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use seal_sdk_rs::native_sui_sdk::sui_types::transaction::{ProgrammableTransaction, ObjectArg};
use seal_sdk_rs::native_sui_sdk::sui_sdk::rpc_types::{SuiObjectDataOptions, SuiMoveValue};
use seal_sdk_rs::generic_types::ObjectID as SealObjectID;
use seal_sdk_rs::native_sui_sdk::sui_types::base_types::ObjectID as SuiObjectID;
use seal_sdk_rs::native_sui_sdk::sui_types::object::Owner;
use std::str::FromStr;
use std::path::Path;
use std::collections::BTreeMap;
use bcs;

/// E2E test of Seal SDK decryption with PrintJob-based authorization
/// 
/// This test demonstrates the complete decryption flow:
/// 1. Fetch encrypted sculpt data from chain
/// 2. Download encrypted STL from Walrus
/// 3. Build seal_approve transaction with PrintJob authorization
/// 4. Decrypt using Seal SDK
/// 
/// New simplified seal_approve signature:
/// - _id: seal resource ID
/// - printer: shared object reference
/// - printer_cap: owned object reference
/// 
/// Usage: cargo run --example test_seal_decrypt_with_printjob -- <printer_id> <printer_cap_id>
/// 
/// Example: 
/// cargo run --example test_seal_decrypt_with_printjob -- \
///   0xabc...printer \
///   0xdef...printer_cap
/// 
/// Note: A PrintJob must exist for the printer before running this test!
/// The sculpt_id will be automatically fetched from the PrintJob.

struct DemoSetup {
    approve_package_id: seal_sdk_rs::generic_types::ObjectID,
    #[allow(dead_code)]
    key_server_ids: Vec<seal_sdk_rs::generic_types::ObjectID>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Get arguments from command line
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 3 {
        eprintln!("Usage: {} <printer_id> <printer_cap_id>", args[0]);
        eprintln!("\nNote: Make sure a PrintJob exists for this printer!");
        eprintln!("      The sculpt_id will be fetched from PrintJob automatically.");
        std::process::exit(1);
    }
    
    let printer_id_str = &args[1];
    let printer_cap_id_str = &args[2];
    
    println!("🔐 E2E Seal Decryption Test (PrintJob-based Auth)");
    println!("==================================================");
    println!("   Printer: {}", printer_id_str);
    println!();

    // Configuration - Updated package IDs
    let eureka_package_id_str = "0x4e43c7642828f9d8c410a47d7ed80b3df7711e49662c4704549dc05b23076bec";
    
    // Key Servers (matching frontend config)
    let key_server_strs = vec![
        "0x73d05d62c18d9374e3ea529e8e0ed6161da1a141a94d3f76ae3fe4e99356db75", // Mysten Labs 1
        "0xf5d14a81a982144ae441cd7d64b09027f116a468bd36e7eca494f750591623c8", // Mysten Labs 2
        "0x4cded1abeb52a22b6becb42a91d3686a4c901cf52eee16234214d0b5b2da4c46", // Triton One
    ];

    // Parse IDs
    let approve_package_id: SealObjectID = eureka_package_id_str.parse()?;
    let printer_id: SuiObjectID = SuiObjectID::from_hex_literal(printer_id_str)?;
    let printer_cap_id: SuiObjectID = SuiObjectID::from_hex_literal(printer_cap_id_str)?;
    let key_server_ids: Vec<SealObjectID> = key_server_strs
        .iter()
        .map(|s| s.parse())
        .collect::<Result<Vec<_>, _>>()?;

    let setup = DemoSetup {
        approve_package_id,
        key_server_ids,
    };

    // Connect to Sui testnet
    let sui_client = SuiClientBuilder::default()
        .build("https://fullnode.testnet.sui.io:443")
        .await?;
    
    // Fetch sculpt_id from PrintJob
    println!("🔍 Fetching PrintJob from printer...");
    let sculpt_id = fetch_sculpt_id_from_printjob(&sui_client, printer_id).await?;
    println!("   ✅ Sculpt ID: {}", sculpt_id);
    println!();
    
    // Fetch sculpt information
    let (encrypted_blob_id, seal_resource_id, printer_version) = 
        fetch_sculpt_and_objects(&sui_client, sculpt_id, printer_id).await?;

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
    println!("🔓 Decrypting with PrintJob-based authorization...");
    decrypt_sculpt(
        &setup, 
        &seal_id, 
        encrypted_object,
        printer_id,
        printer_cap_id,
        printer_version,
    ).await?;

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
    let encrypted: seal_sdk_rs::crypto::EncryptedObject = bcs::from_bytes(data)
        .map_err(|e| anyhow::anyhow!("BCS deserialization failed: {}", e))?;
    Ok(encrypted)
}

/// Fetch sculpt_id from printer's PrintJob dynamic field
async fn fetch_sculpt_id_from_printjob(
    sui_client: &seal_sdk_rs::native_sui_sdk::sui_sdk::SuiClient,
    printer_id: SuiObjectID,
) -> Result<SuiObjectID> {
    // Get dynamic fields
    let dynamic_fields = sui_client
        .read_api()
        .get_dynamic_fields(printer_id, None, None)
        .await?;

    // Find print_job field (name.value is a vector<u8>)
    let print_job_field = dynamic_fields
        .data
        .iter()
        .find(|field| {
            // The field name is stored as bytes ("print_job" as vector<u8>)
            if let Some(name_bytes) = field.name.value.as_array() {
                let bytes: Vec<u8> = name_bytes.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                    .collect();
                bytes == b"print_job"
            } else {
                false
            }
        })
        .ok_or_else(|| anyhow::anyhow!("No PrintJob found for this printer. Make sure to create a PrintJob first!"))?;

    // Fetch the PrintJob object
    let mut options = SuiObjectDataOptions::new();
    options.show_content = true;
    
    let print_job_obj = sui_client
        .read_api()
        .get_object_with_options(print_job_field.object_id, options)
        .await?;

    let print_job_data = print_job_obj.data
        .ok_or_else(|| anyhow::anyhow!("PrintJob data not found"))?;

    let content = print_job_data.content
        .ok_or_else(|| anyhow::anyhow!("PrintJob has no content"))?;

    let fields = match content {
        seal_sdk_rs::native_sui_sdk::sui_sdk::rpc_types::SuiParsedData::MoveObject(ref obj) => {
            match &obj.fields {
                seal_sdk_rs::native_sui_sdk::sui_sdk::rpc_types::SuiMoveStruct::WithFields(f) => f,
                _ => return Err(anyhow::anyhow!("PrintJob fields are not in WithFields format")),
            }
        }
        _ => return Err(anyhow::anyhow!("PrintJob is not a Move object")),
    };

    // Extract sculpt_id
    let sculpt_id_str = match fields.get("sculpt_id") {
        Some(SuiMoveValue::String(id)) => id,
        _ => return Err(anyhow::anyhow!("sculpt_id field not found in PrintJob")),
    };

    SuiObjectID::from_hex_literal(sculpt_id_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse sculpt_id: {}", e))
}

/// Fetch sculpt information and printer version from chain
async fn fetch_sculpt_and_objects(
    sui_client: &seal_sdk_rs::native_sui_sdk::sui_sdk::SuiClient,
    sculpt_id: SuiObjectID,
    printer_id: SuiObjectID,
) -> Result<(String, Option<String>, u64)> {
    let mut options = SuiObjectDataOptions::new();
    options.show_content = true;
    options.show_type = true;
    options.show_owner = true;

    // Fetch sculpt to get encrypted blob ID and seal_resource_id
    let object_response = sui_client
        .read_api()
        .get_object_with_options(sculpt_id, options.clone())
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

    // Extract structure (encrypted STL blob ID)
    let structure = extract_option_string_field(fields, "structure")
        .ok_or_else(|| anyhow::anyhow!("Sculpt has no structure field (STL blob ID)"))?;

    // Extract seal_resource_id
    let seal_resource_id = extract_option_string_field(fields, "seal_resource_id");

    // Fetch printer to get its shared version
    let printer_response = sui_client
        .read_api()
        .get_object_with_options(printer_id, options)
        .await?;
    let printer_data = printer_response.data
        .ok_or_else(|| anyhow::anyhow!("Printer not found"))?;
    let printer_version = match printer_data.owner {
        Some(Owner::Shared { initial_shared_version }) => initial_shared_version.value(),
        _ => return Err(anyhow::anyhow!("Printer is not a shared object")),
    };

    Ok((structure, seal_resource_id, printer_version))
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

/// Decrypt sculpt using Seal SDK with PrintJob-based authorization
async fn decrypt_sculpt(
    setup: &DemoSetup,
    seal_id: &str,
    encrypted: seal_sdk_rs::crypto::EncryptedObject,
    printer_id: SuiObjectID,
    printer_cap_id: SuiObjectID,
    printer_version: u64,
) -> Result<()> {
    // Connect to Sui testnet
    let sui_client = SuiClientBuilder::default()
        .build("https://fullnode.testnet.sui.io:443")
        .await?;

    let client = SealClient::new(sui_client.clone());

    // Load wallet
    let wallet_path = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("Cannot find HOME env var"))?
        + "/.sui/sui_config/client.yaml";

    let mut wallet = WalletContext::new(Path::new(&wallet_path))?;
    
    // Print current wallet address for debugging
    let current_address = wallet.active_address()?;
    println!("\n👛 Current Wallet:");
    println!("   Address: {}", current_address);
    println!("   Note: This must match the printer owner!");
    
    let session_key = SessionKey::new(
        setup.approve_package_id,
        10,
        &mut wallet,
    )
    .await?;

    // Build approval transaction for simplified seal_approve
    println!("\n🔨 Building approval transaction (PrintJob-based)...");
    let mut builder = ProgrammableTransactionBuilder::new();

    // Extract resource_id from seal_id
    let resource_id = if seal_id.contains(':') {
        seal_id.split(':').nth(1)
            .ok_or_else(|| anyhow::anyhow!("Invalid seal_resource_id format"))?
    } else {
        seal_id
    };
    
    let id_hex = resource_id.strip_prefix("0x").unwrap_or(resource_id);
    let id_bytes = hex::decode(id_hex)
        .map_err(|e| anyhow::anyhow!("Failed to decode hex ID: {}", e))?;
    
    println!("   ✅ Arg 0 (_id): {} bytes", id_bytes.len());
    
    // Argument 0: _id (vector<u8>)
    let id_arg = builder.pure(id_bytes)?;
    
    // Argument 1: printer (shared object)
    let printer_arg = builder.obj(ObjectArg::SharedObject {
        id: printer_id,
        initial_shared_version: printer_version.into(),
        mutable: false,
    })?;
    println!("   ✅ Arg 1 (printer): {} (shared, v{})", printer_id, printer_version);
    
    // Argument 2: printer_cap (owned object)
    let printer_cap_obj = sui_client
        .read_api()
        .get_object_with_options(printer_cap_id, SuiObjectDataOptions::bcs_lossless())
        .await?
        .data
        .ok_or_else(|| anyhow::anyhow!("PrinterCap not found"))?;
    let printer_cap_arg = builder.obj(ObjectArg::ImmOrOwnedObject((
        printer_cap_id,
        printer_cap_obj.version,
        printer_cap_obj.digest,
    )))?;
    println!("   ✅ Arg 2 (printer_cap): {} (owned, v{})", printer_cap_id, printer_cap_obj.version);

    // Call seal_approve in eureka module (no type arguments!)
    // entry fun seal_approve(_id, printer, printer_cap, ctx)
    builder.programmable_move_call(
        setup.approve_package_id.into(),
        Identifier::from_str("eureka")?,
        Identifier::from_str("seal_approve")?,
        vec![], // No type arguments needed!
        vec![
            id_arg,          // _id: vector<u8>
            printer_arg,     // printer: &Printer
            printer_cap_arg, // printer_cap: &PrinterCap
        ],
    );

    let approve_ptb: ProgrammableTransaction = builder.finish();

    // Debug: Print transaction details
    println!("\n📋 Transaction Details:");
    println!("   Package: {}", setup.approve_package_id);
    println!("   Module: eureka");
    println!("   Function: seal_approve");
    println!("   Arguments: 3 (_id, printer, printer_cap)");
    println!("   Authorization: PrintJob existence = approval");
    println!();

    // Decrypt with detailed error handling
    println!("🔑 Requesting decryption from Seal SDK...");
    let plaintext = match client
        .decrypt_object_bytes(
            &bcs::to_bytes(&encrypted)?,
            approve_ptb,
            &session_key,
        )
        .await
    {
        Ok(data) => {
            println!("   ✅ Seal SDK decryption successful!");
            data
        },
        Err(e) => {
            eprintln!("\n❌ Seal SDK Error:");
            eprintln!("   {}", e);
            eprintln!("\n🔍 Possible causes:");
            eprintln!("   1. ENotPrinterOwner (code 5): Caller is not the printer owner");
            eprintln!("   2. EInvalidPrinterCap (code 6): PrinterCap doesn't match this printer");
            eprintln!("   3. EPrintJobNotFound (code 7): No PrintJob exists for this printer");
            eprintln!("   4. EPrinterIdMismatch (code 8): PrintJob's printer_id mismatch");
            eprintln!("   5. Object not found: Invalid object IDs");
            eprintln!("\n💡 Debug tips:");
            eprintln!("   - Check printer owner: sui client object {}", printer_id);
            eprintln!("   - Check PrinterCap: sui client object {}", printer_cap_id);
            eprintln!("   - Verify active wallet: sui client active-address");
            eprintln!("   - Ensure PrintJob exists: Create via frontend Print button");
            return Err(e.into());
        }
    };

    // Save and verify decrypted STL
    let output_file = "decrypted_sculpt_printjob.stl";
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

