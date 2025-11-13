use anyhow::Result;
use sui_sdk::{
    SuiClientBuilder,
    rpc_types::{
        SuiObjectDataOptions, 
        SuiTransactionBlockResponseOptions,
        SuiTransactionBlockEffectsAPI,
    },
    types::{
        base_types::{ObjectID, SuiAddress},
        programmable_transaction_builder::ProgrammableTransactionBuilder,
        transaction::{Transaction, TransactionData, ObjectArg},
        Identifier,
    },
};
use sui_keys::keystore::{AccountKeystore, FileBasedKeystore};
use shared_crypto::intent::Intent;
use std::path::PathBuf;
use std::str::FromStr;

/// Test seal_approve using pure Sui SDK
/// Signature: seal_approve(_id, printer, printer_cap, ctx)
/// 
/// Usage: cargo run --example test_seal_approve -- <printer_id> <printer_cap_id>

#[tokio::main]
async fn main() -> Result<()> {
    // Get arguments
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 3 {
        eprintln!("Usage: {} <printer_id> <printer_cap_id>", args[0]);
        eprintln!("\nNote: Make sure a PrintJob exists for this printer before testing!");
        std::process::exit(1);
    }
    
    let printer_id_str = &args[1];
    let printer_cap_id_str = &args[2];
    
    println!("🧪 Seal Approve Test");
    println!("====================");
    println!("Signature: seal_approve(_id, printer, printer_cap, ctx)\n");
    
    // Parse IDs
    let printer_id = ObjectID::from_hex_literal(printer_id_str)?;
    let printer_cap_id = ObjectID::from_hex_literal(printer_cap_id_str)?;
    
    println!("📋 Input IDs:");
    println!("   Printer: {}", printer_id);
    println!("   PrinterCap: {}", printer_cap_id);
    println!();
    
    // Package ID (fresh deployment 2025-11-13)
    let eureka_package_id = ObjectID::from_hex_literal(
        "0x8852004ffc677790d0ee729aa386286cbcbc7f4f1b4aa87c50213d2acb5d678f"
    )?;
    
    println!("📦 Package ID:");
    println!("   Eureka: {}", eureka_package_id);
    println!();
    
    // Connect to Sui
    let sui_client = SuiClientBuilder::default()
        .build("https://fullnode.testnet.sui.io:443")
        .await?;
    
    // Load keystore
    let keystore_path = PathBuf::from(std::env::var("HOME")?)
        .join(".sui")
        .join("sui_config")
        .join("sui.keystore");
    let keystore = FileBasedKeystore::load_or_create(&keystore_path)?;
    
    // Read active address from client.yaml config
    let config_path = PathBuf::from(std::env::var("HOME")?)
        .join(".sui")
        .join("sui_config")
        .join("client.yaml");
    
    let config_content = std::fs::read_to_string(config_path)?;
    
    // Parse active_address from YAML (simple string parsing)
    let active_address_str = config_content
        .lines()
        .find(|line| line.trim().starts_with("active_address:"))
        .and_then(|line| line.split(':').nth(1))
        .map(|s| s.trim().trim_matches('"'))
        .ok_or_else(|| anyhow::anyhow!("Could not find active_address in client.yaml"))?;
    
    let active_address = SuiAddress::from_str(active_address_str)?;
    
    println!("👛 Wallet Info:");
    println!("   Active Address: {}", active_address);
    println!("   🔑 This address will sign the transaction");
    println!();
    
    // Fetch object information
    println!("🔍 Fetching object information...");
    
    let printer_obj = sui_client.read_api()
        .get_object_with_options(printer_id, SuiObjectDataOptions::new().with_owner())
        .await?
        .data
        .ok_or_else(|| anyhow::anyhow!("Printer not found"))?;
    
    let printer_version = match printer_obj.owner {
        Some(sui_types::object::Owner::Shared { initial_shared_version }) => {
            initial_shared_version.value()
        },
        _ => return Err(anyhow::anyhow!("Printer is not shared")),
    };
    println!("   ✅ Printer: shared v{}", printer_version);
    
    let printer_cap_obj = sui_client.read_api()
        .get_object_with_options(printer_cap_id, SuiObjectDataOptions::bcs_lossless())
        .await?
        .data
        .ok_or_else(|| anyhow::anyhow!("PrinterCap not found"))?;
    println!("   ✅ PrinterCap: owned v{}", printer_cap_obj.version);
    println!();
    
    // Build transaction
    println!("🔨 Building transaction...");
    let mut ptb = ProgrammableTransactionBuilder::new();
    
    // Argument 0: _id (dummy value for testing)
    let test_id = vec![0u8; 32]; // 32 bytes dummy ID
    let id_arg = ptb.pure(test_id)?;
    println!("   ✅ Arg 0 (_id): 32 bytes (dummy)");
    
    // Argument 1: printer (shared object)
    let printer_arg = ptb.obj(ObjectArg::SharedObject {
        id: printer_id,
        initial_shared_version: printer_version.into(),
        mutability: sui_types::transaction::SharedObjectMutability::Immutable,
    })?;
    println!("   ✅ Arg 1 (printer): shared v{}", printer_version);
    
    // Argument 2: printer_cap (owned object)
    let printer_cap_arg = ptb.obj(ObjectArg::ImmOrOwnedObject((
        printer_cap_id,
        printer_cap_obj.version,
        printer_cap_obj.digest,
    )))?;
    println!("   ✅ Arg 2 (printer_cap): owned v{}", printer_cap_obj.version);
    println!();
    
    // Call seal_approve (no type arguments needed!)
    println!("📞 Calling seal_approve...");
    println!("   Package: {}", eureka_package_id);
    println!("   Module: eureka");
    println!("   Function: seal_approve");
    println!("   Arguments: 3 (_id, printer, printer_cap)");
    println!();
    
    ptb.command(sui_types::transaction::Command::move_call(
        eureka_package_id,
        Identifier::new("eureka")?,
        Identifier::new("seal_approve")?,
        vec![], // No type arguments!
        vec![id_arg, printer_arg, printer_cap_arg],
    ));
    
    // Get gas coin
    let coins = sui_client.coin_read_api()
        .get_coins(active_address, None, None, None)
        .await?;
    
    let gas_coin = coins.data.first()
        .ok_or_else(|| anyhow::anyhow!("No gas coins available"))?;
    
    let gas_budget = 100_000_000; // 0.1 SUI (restored to original)
    let gas_price = sui_client.read_api().get_reference_gas_price().await?;
    
    println!("⛽ Gas Info:");
    println!("   Budget: {} MIST", gas_budget);
    println!("   Price: {} MIST", gas_price);
    println!("   Coin: {} v{}", gas_coin.coin_object_id, gas_coin.version);
    println!();
    
    // Build final transaction
    let pt = ptb.finish();
    let tx_data = TransactionData::new_programmable(
        active_address,
        vec![gas_coin.object_ref()],
        pt,
        gas_budget,
        gas_price,
    );
    
    // Sign transaction
    println!("✍️  Signing transaction...");
    println!("   Signer: {}", active_address);
    println!("   Signing with keystore at: ~/.sui/sui_config/sui.keystore");
    let signature = keystore.sign_secure(&active_address, &tx_data, Intent::sui_transaction()).await?;
    println!("   ✅ Signature created");
    
    // Execute transaction
    println!("🚀 Executing transaction...");
    let response = sui_client
        .quorum_driver_api()
        .execute_transaction_block(
            Transaction::from_data(tx_data, vec![signature]),
            SuiTransactionBlockResponseOptions::new()
                .with_effects()
                .with_events(),
            Some(sui_types::quorum_driver_types::ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await?;
    
    println!();
    println!("📊 Transaction Result:");
    println!("   Digest: {}", response.digest);
    
    if let Some(effects) = &response.effects {
        let status = effects.status();
        if status.is_ok() {
            println!("   ✅ Status: SUCCESS");
        } else {
            println!("   ❌ Status: FAILED");
            println!("   Full Error: {:?}", status);
            
            // Try to parse error code from MoveAbort
            let error_str = format!("{:?}", status);
            
            // Extract error code number from pattern like "}, 7)"
            let error_code = if let Some(pos) = error_str.rfind("}, ") {
                let after = &error_str[pos+3..];
                if let Some(end) = after.find(")") {
                    after[..end].trim().parse::<u64>().ok()
                } else {
                    None
                }
            } else {
                None
            };
            
            println!("\n🔍 Parsed Error Code: {:?}", error_code);
            
            match error_code {
                Some(5) => {
                    println!("\n❌ Error Code 5 (ENotPrinterOwner):");
                    println!("   The transaction sender is not the printer owner.");
                    println!("   Current sender: {}", active_address);
                    println!("   Check printer: sui client object {}", printer_id);
                },
                Some(6) => {
                    println!("\n❌ Error Code 6 (EInvalidPrinterCap):");
                    println!("   The PrinterCap does not match this printer.");
                    println!("   Check PrinterCap: sui client object {}", printer_cap_id);
                },
                Some(7) => {
                    println!("\n❌ Error Code 7 (EPrintJobNotFound):");
                    println!("   🚨 No PrintJob exists for this printer!");
                    println!("   Authorization requires an active PrintJob.");
                    println!("\n💡 Solution:");
                    println!("   1. Go to frontend: Vault > Sculpt > Select a sculpt");
                    println!("   2. Click 'Print' button and select this printer");
                    println!("   3. Confirm transaction to create PrintJob");
                    println!("   4. Then retry this test");
                },
                Some(8) => {
                    println!("\n❌ Error Code 8 (EPrinterIdMismatch):");
                    println!("   PrintJob's printer_id doesn't match the Printer.");
                },
                _ => {
                    println!("\n❓ Unknown error code or parse failed");
                }
            }
        }
        
        println!("   Gas Used: {} MIST", effects.gas_cost_summary().net_gas_usage());
    }
    
    if let Some(events) = &response.events {
        if !events.data.is_empty() {
            println!("\n📢 Events:");
            for event in &events.data {
                println!("   - {}", event.type_);
            }
        }
    }
    
    println!();
    println!("🎉 Test completed!");
    
    Ok(())
}

