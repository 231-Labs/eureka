use anyhow::Result;
use seal_sdk_rs::native_sui_sdk::client::seal_client::SealClient;
use seal_sdk_rs::session_key::SessionKey;
use seal_sdk_rs::native_sui_sdk::sui_sdk::SuiClientBuilder;
use seal_sdk_rs::native_sui_sdk::sui_sdk::wallet_context::WalletContext;
use seal_sdk_rs::native_sui_sdk::sui_types::Identifier;
use seal_sdk_rs::native_sui_sdk::sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use seal_sdk_rs::native_sui_sdk::sui_types::transaction::ProgrammableTransaction;
use seal_sdk_rs::native_sui_sdk::sui_types::TypeTag;
use seal_sdk_rs::generic_types::ObjectID;
use std::str::FromStr;
use std::path::Path;
use bcs;

/// DemoSetup structure matching official SDK documentation exactly
struct DemoSetup {
    approve_package_id: ObjectID,
    key_server_id: ObjectID,
}

/// Complete encrypt and decrypt test (following official demo)
#[tokio::main]
async fn main() -> Result<()> {
    println!("🔐 Complete Seal Encrypt/Decrypt Test");
    println!("================================\n");

    // Configuration matching official docs - Using old package (only _id parameter)
    let package_id_str = "0x927efc566998883385df85bf7ff45da1c9b1c897fc5be48f3d81df1d2f3774b1";
    let key_server_id_str = "0x73d05d62c18d9374e3ea529e8e0ed6161da1a141a94d3f76ae3fe4e99356db75"; // Mysten Labs 1

    // Parse IDs
    let approve_package_id: ObjectID = package_id_str.parse()?;
    let key_server_id: ObjectID = key_server_id_str.parse()?;

    // Create DemoSetup (exactly matching official docs)
    let setup = DemoSetup {
        approve_package_id,
        key_server_id,
    };

    println!("📋 Configuration:");
    println!("  Package ID: {}", package_id_str);
    println!("  Key Server ID: {}", key_server_id_str);
    println!();

    // Test data
    let test_data = b"Hello from Seal SDK test! This is a test message for encryption.";

    // Step 1: Encrypt the data (following official demo)
    println!("🔒 Step 1: Encrypting data...");
    let encrypted = encrypt_message(&setup, test_data).await?;
    println!("   ✅ Encryption successful!");
    println!("   Original size: {} bytes", test_data.len());
    println!("   Encrypted size: {} bytes", encrypted.len());
    println!();

    // Step 2: Decrypt the data (following official demo)
    println!("🔓 Step 2: Decrypting data...");
    let decrypted = decrypt_message(&setup, &encrypted).await?;
    println!("   ✅ Decryption successful!");
    println!("   Decrypted size: {} bytes", decrypted.len());

    // Step 3: Verify the result
    println!("✅ Step 3: Verifying result...");
    if decrypted == test_data {
        println!("   ✅ SUCCESS: Decrypted data matches original!");
        println!("   Original: {}", String::from_utf8_lossy(test_data));
        println!("   Decrypted: {}", String::from_utf8_lossy(&decrypted));
    } else {
        println!("   ❌ FAILURE: Decrypted data does not match original!");
        println!("   Original: {}", String::from_utf8_lossy(test_data));
        println!("   Decrypted: {}", String::from_utf8_lossy(&decrypted));
        return Err(anyhow::anyhow!("Decryption verification failed"));
    }

    println!();
    println!("🎉 Complete test PASSED!");
    Ok(())
}

/// Encrypt message function (exactly matching official demo)
async fn encrypt_message(setup: &DemoSetup, data: &[u8]) -> Result<Vec<u8>> {
    let sui_client = SuiClientBuilder::default()
        .build("https://fullnode.testnet.sui.io:443")
        .await?;

    let client = SealClient::new(sui_client);

    // Load wallet
    #[allow(unused_variables)]
    let wallet_path = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("Cannot find HOME env var"))?
        + "/.sui/sui_config/client.yaml";

    // let wallet = WalletContext::new(Path::new(&wallet_path))?;

    // let session_key = SessionKey::new(
    //     setup.approve_package_id,
    //     5,
    //     &mut wallet,
    // )
    // .await?;

    // Encrypt using Seal SDK (matching official demo)
    let test_id = b"test_id".to_vec();
    let (encrypted, _recovery_key) = client
        .encrypt_bytes(
            setup.approve_package_id,
            test_id,                 // ID parameter - must match decrypt ID
            1, // threshold
            vec![setup.key_server_id],
            data.to_vec(),
        )
        .await?;

    // Return BCS serialized encrypted data
    Ok(bcs::to_bytes(&encrypted)?)
}

/// Decrypt message function (exactly matching official demo)
async fn decrypt_message(setup: &DemoSetup, encrypted_data: &[u8]) -> Result<Vec<u8>> {
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
        5,
        &mut wallet,
    )
    .await?;

    let mut builder = ProgrammableTransactionBuilder::new();

    // Use the same test_id as in encryption (must match!)
    let test_id = b"test_id".to_vec();
    let id_arg = builder.pure(test_id)?;
    
    // Type argument (not a value argument!)
    let type_arg = TypeTag::from_str("0x927efc566998883385df85bf7ff45da1c9b1c897fc5be48f3d81df1d2f3774b1::atelier::ATELIER")?;

    // Note: This simplified test won't work with the new seal_approve signature
    // which requires: sculpt_id, kiosk, kiosk_cap, printer, printer_cap
    // Use test_seal_approve_kiosk.rs for full testing
    
    // Call our seal_approve function in the sculpt module
    builder.programmable_move_call(
        setup.approve_package_id.into(),
        Identifier::from_str("eureka")?,
        Identifier::from_str("seal_approve")?,
        vec![type_arg],
        vec![id_arg],  // Only _id parameter - incomplete for new signature!
    );

    let approve_ptb: ProgrammableTransaction = builder.finish();

    // Deserialize encrypted data
    let encrypted: seal_sdk_rs::crypto::EncryptedObject = bcs::from_bytes(encrypted_data)?;

    let plaintext = client
        .decrypt_object_bytes(
            &bcs::to_bytes(&encrypted)?,
            approve_ptb,
            &session_key,
        )
        .await?;

    Ok(plaintext)
}
