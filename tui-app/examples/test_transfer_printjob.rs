use anyhow::Result;
use sui_sdk::SuiClientBuilder;
use sui_sdk::wallet_context::WalletContext;
use sui_sdk::types::base_types::ObjectID;
use sui_sdk::types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use sui_sdk::types::transaction::TransactionData;
use sui_sdk::types::{Identifier, TypeTag};
use sui_sdk::types::transaction::{ObjectArg, SharedObjectMutability};
use sui_sdk::rpc_types::SuiObjectDataOptions;
use std::str::FromStr;
use std::path::Path;

/// Test completed_and_detach_print_job function directly using Sui SDK
/// 
/// This test calls the completed_and_detach_print_job function which returns a PrintJob,
/// then uses Sui's built-in transfer function to send it to the caller's wallet.
/// 
/// Usage: cargo run --example test_transfer_printjob -- <printer_id> <printer_cap_id>
/// 
/// Example:
/// cargo run --example test_transfer_printjob -- \
///   0xabc...printer \
///   0xdef...printer_cap

#[tokio::main]
async fn main() -> Result<()> {
    // Get arguments from command line
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 3 {
        eprintln!("Usage: {} <printer_id> <printer_cap_id>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} 0xabc...printer 0xdef...printer_cap", args[0]);
        eprintln!("\nNote: This uses the new transfer_completed_print_job function");
        eprintln!("which returns a PrintJob and then transfers it using Sui's transfer function.");
        std::process::exit(1);
    }
    
    let printer_id_str = &args[1];
    let printer_cap_id_str = &args[2];
    
    println!("🔧 Testing completed_and_detach_print_job function");
    println!("=================================================");
    println!("   Printer ID: {}", printer_id_str);
    println!("   PrinterCap ID: {}", printer_cap_id_str);
    println!();

    // Configuration - Fresh deployment 2025-11-13
    let eureka_package_id_str = "0x8852004ffc677790d0ee729aa386286cbcbc7f4f1b4aa87c50213d2acb5d678f";
    
    // Parse IDs
    let eureka_package_id = ObjectID::from_hex_literal(eureka_package_id_str)?;
    let printer_id = ObjectID::from_hex_literal(printer_id_str)?;
    let printer_cap_id = ObjectID::from_hex_literal(printer_cap_id_str)?;

    // Connect to Sui testnet
    let sui_client = SuiClientBuilder::default()
        .build("https://fullnode.testnet.sui.io:443")
        .await?;
    
    // Load wallet
    let wallet_path = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("Cannot find HOME env var"))?
        + "/.sui/sui_config/client.yaml";

    let mut wallet = WalletContext::new(Path::new(&wallet_path))?;
    let sender = wallet.active_address()?;
    
    println!("👛 Wallet Address: {}", sender);
    println!();

    // Step 1: Check if objects exist and get their details
    println!("🔍 Step 1: Checking object existence...");
    
    let mut options = SuiObjectDataOptions::new();
    options.show_content = true;
    options.show_owner = true;
    options.show_type = true;

    // Check printer
    println!("   Checking printer object...");
    let printer_obj = sui_client
        .read_api()
        .get_object_with_options(printer_id, options.clone())
        .await?;
    
    if let Some(data) = &printer_obj.data {
        println!("   ✅ Printer found: {}", data.object_id);
        if let Some(owner) = &data.owner {
            println!("      Owner: {:?}", owner);
        }
        if let Some(obj_type) = &data.type_ {
            println!("      Type: {}", obj_type);
        }
    } else {
        println!("   ❌ Printer not found!");
        return Err(anyhow::anyhow!("Printer object not found"));
    }

    // Check printer cap
    println!("   Checking printer cap object...");
    let printer_cap_obj = sui_client
        .read_api()
        .get_object_with_options(printer_cap_id, options.clone())
        .await?;
    
    if let Some(data) = &printer_cap_obj.data {
        println!("   ✅ PrinterCap found: {}", data.object_id);
        if let Some(owner) = &data.owner {
            println!("      Owner: {:?}", owner);
        }
    } else {
        println!("   ❌ PrinterCap not found!");
        return Err(anyhow::anyhow!("PrinterCap object not found"));
    }

    println!();

    // Step 2: Check if PrintJob exists on the printer
    println!("🔍 Step 2: Checking PrintJob existence...");
    let dynamic_fields = sui_client
        .read_api()
        .get_dynamic_fields(printer_id, None, None)
        .await?;

    let print_job_exists = dynamic_fields
        .data
        .iter()
        .any(|field| {
            if let Some(name_bytes) = field.name.value.as_array() {
                let bytes: Vec<u8> = name_bytes.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                    .collect();
                bytes == b"print_job"
            } else {
                false
            }
        });

    if print_job_exists {
        println!("   ✅ PrintJob found on printer");
    } else {
        println!("   ❌ PrintJob not found on printer!");
        println!("   Available dynamic fields:");
        for field in &dynamic_fields.data {
            println!("      - {:?}", field.name);
        }
        return Err(anyhow::anyhow!("PrintJob not found on printer"));
    }

    println!();

    // Step 3: Build and execute the completed_and_detach_print_job transaction
    println!("🔨 Step 3: Building completed_and_detach_print_job transaction...");
    
    let mut ptb = ProgrammableTransactionBuilder::new();
    
    // Get the printer object for shared reference
    let printer_data = printer_obj.data.unwrap();
    let printer_version = match printer_data.owner.unwrap() {
        sui_sdk::types::object::Owner::Shared { initial_shared_version } => initial_shared_version,
        _ => return Err(anyhow::anyhow!("Printer is not a shared object")),
    };

    // Arguments for completed_and_detach_print_job
    let printer_cap_data = printer_cap_obj.data.unwrap();
    let printer_cap_arg = ptb.obj(ObjectArg::ImmOrOwnedObject((
        printer_cap_id,
        printer_cap_data.version,
        printer_cap_data.digest,
    )))?;

    let printer_arg = ptb.obj(ObjectArg::SharedObject {
        id: printer_id,
        initial_shared_version: printer_version,
        mutability: SharedObjectMutability::Mutable,
    })?;

    // Get clock object (0x6 is the clock object ID)
    let clock_id = ObjectID::from_hex_literal("0x0000000000000000000000000000000000000000000000000000000000000006")?;
    let clock_arg = ptb.obj(ObjectArg::SharedObject {
        id: clock_id,
        initial_shared_version: sui_sdk::types::base_types::SequenceNumber::from_u64(1),
        mutability: SharedObjectMutability::Immutable,
    })?;

    // Call completed_and_detach_print_job function - this returns a PrintJob
    let print_job_result = ptb.programmable_move_call(
        eureka_package_id,
        Identifier::from_str("eureka")?,
        Identifier::from_str("completed_and_detach_print_job")?,
        vec![], // No type arguments
        vec![
            printer_cap_arg,  // printer_cap: &PrinterCap
            printer_arg,      // printer: &mut Printer
            clock_arg,        // clock: &Clock
        ],
    );

    // Transfer the returned PrintJob to the sender using Sui's transfer function
    let sender_arg = ptb.pure(sender)?;
    
    // Create the PrintJob type tag using string parsing
    let print_job_type_str = format!("{}::print_job::PrintJob", eureka_package_id_str);
    let print_job_type = TypeTag::from_str(&print_job_type_str)?;
    
    ptb.programmable_move_call(
        ObjectID::from_hex_literal("0x0000000000000000000000000000000000000000000000000000000000000002")?, // Sui framework
        Identifier::from_str("transfer")?,
        Identifier::from_str("public_transfer")?,
        vec![print_job_type], // Type argument: the PrintJob type
        vec![
            print_job_result,  // The PrintJob returned from transfer_completed_print_job
            sender_arg,        // Recipient address
        ],
    );

    let pt = ptb.finish();
    
    println!("   ✅ Transaction built successfully");
    println!("   Function 1: {}::eureka::completed_and_detach_print_job", eureka_package_id_str);
    println!("   Function 2: 0x2::transfer::public_transfer<PrintJob>");
    println!("   Arguments: 3 (printer_cap, printer, clock) + transfer");
    println!();

    // Step 4: Execute the transaction
    println!("🚀 Step 4: Executing transaction...");
    
    let gas_budget = 10_000_000; // 0.01 SUI
    let gas_price = sui_client.read_api().get_reference_gas_price().await?;
    
    // Get a gas coin for the transaction
    let coins = sui_client
        .coin_read_api()
        .get_coins(sender, None, None, None)
        .await?;
    
    let gas_coin = coins.data.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!("No available coins found for gas payment"))?;
    
    let tx_data = TransactionData::new_programmable(
        sender,
        vec![(gas_coin.coin_object_id, gas_coin.version, gas_coin.digest)],
        pt,
        gas_budget,
        gas_price,
    );

    // Sign and execute the transaction
    let signed_tx = wallet.sign_transaction(&tx_data).await;
    let result = wallet.execute_transaction_may_fail(signed_tx).await;

    println!("   Gas budget: {} MIST", gas_budget);
    println!("   Gas price: {}", gas_price);
    println!();

    // Transaction already executed by wallet
    match result {
        Ok(response) => {
            println!("✅ Transaction executed successfully!");
            println!("   Transaction digest: {}", response.digest);
            
            if let Some(_effects) = &response.effects {
                println!("   Status: Success");
                println!("   PrintJob has been transferred to your wallet!");
            }

            if let Some(events) = &response.events {
                println!("   Events emitted: {}", events.data.len());
                for (i, event) in events.data.iter().enumerate() {
                    println!("     Event {}: {}", i + 1, event.type_);
                }
            }

            if let Some(object_changes) = &response.object_changes {
                println!("   Object changes: {}", object_changes.len());
                for change in object_changes {
                    println!("     {:?}", change);
                }
            }
        }
        Err(e) => {
            println!("❌ Transaction failed!");
            println!("   Error: {}", e);
            
            // Try to parse the error for more details
            let error_str = e.to_string();
            if error_str.contains("EPrintJobCompleted") {
                println!("   → PrintJob is already completed");
            } else if error_str.contains("EPrintJobNotStarted") {
                println!("   → PrintJob has not been started yet");
            } else if error_str.contains("ENotAuthorized") {
                println!("   → Not authorized (wrong PrinterCap or not printer owner)");
            } else if error_str.contains("dynamic_field") {
                println!("   → PrintJob dynamic field issue");
            } else if error_str.contains("insufficient") {
                println!("   → Insufficient gas or balance");
            }
            
            return Err(e.into());
        }
    }

    println!();
    println!("🎉 PrintJob transfer completed successfully!");
    println!("   The PrintJob has been removed from the printer and transferred to your wallet.");
    println!("   The printer is now ready to accept new jobs.");
    
    Ok(())
}
