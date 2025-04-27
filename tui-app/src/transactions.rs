use sui_sdk::{
    types::{
        base_types::{ObjectID, SuiAddress},
        transaction::{Transaction, TransactionData},
        programmable_transaction_builder::ProgrammableTransactionBuilder,
        Identifier,
    },
    rpc_types::{SuiObjectDataOptions, SuiTransactionBlockResponseOptions},
    SuiClient,
};
use sui_types::{
    object::Owner,
    quorum_driver_types::ExecuteTransactionRequestType,
    transaction::{Argument, CallArg, Command}
};
use sui_keys::keystore::{AccountKeystore, FileBasedKeystore};
use shared_crypto::intent::Intent;
use std::path::PathBuf;
use anyhow::{anyhow, Result};

pub struct TransactionBuilder {
    sui_client: SuiClient,
    sender: SuiAddress,
}

impl TransactionBuilder {
    pub async fn new(sui_client: SuiClient, sender: ObjectID) -> Self {
        Self {
            sui_client,
            sender: SuiAddress::from(sender),
        }
    }

    pub async fn register_printer(
        &self,
        registry_id: ObjectID,
        printer_name: &str,
    ) -> Result<String> {
        // Get available coins
        let coins = self.sui_client
            .coin_read_api()
            .get_coins(self.sender, None, None, None)
            .await?;
        
        let coin = coins.data.into_iter().next()
            .ok_or_else(|| anyhow!("No available coins found"))?;

        let registry = self.sui_client
            .read_api()
            .get_object_with_options(registry_id, SuiObjectDataOptions {
                show_owner: true,
                ..Default::default()
            })
            .await?
            .data
            .ok_or_else(|| anyhow!("Registry object initial_shared_version not found"))?;

        // Create programmable transaction builder
        let mut ptb = ProgrammableTransactionBuilder::new();

        // Get initial shared version
        let initial_shared_version = match registry.owner {
            Some(Owner::Shared { initial_shared_version }) => initial_shared_version,
            _ => return Err(anyhow!("Registry is not a shared object")),
        };

        ptb.input(CallArg::Object(sui_sdk::types::transaction::ObjectArg::SharedObject {
            id: registry_id,
            initial_shared_version: initial_shared_version.into(),
            mutable: true,
        }))?;

        // Prepare printer_name parameter
        let name_bytes = bcs::to_bytes(printer_name)?;
        let name_arg = CallArg::Pure(name_bytes);
        ptb.input(name_arg)?;

        // Add move call
        let package = ObjectID::from_hex_literal(crate::constants::EUREKA_DEVNET_PACKAGE_ID)?;
        let module = Identifier::new("eureka")?;
        let function = Identifier::new("register_printer")?;

        ptb.command(Command::move_call(
            package,
            module,
            function,
            vec![],  // Type parameters
            vec![Argument::Input(0), Argument::Input(1)],  // Parameters
        ));

        // Complete transaction building
        let builder = ptb.finish();

        // Set gas parameters
        let gas_budget = 100_000_000;
        let gas_price = self.sui_client.read_api().get_reference_gas_price().await?;

        // Create transaction data
        let tx_data = TransactionData::new_programmable(
            self.sender,
            vec![coin.object_ref()],
            builder,
            gas_budget,
            gas_price,
        );

        // Sign transaction
        let keystore_path = PathBuf::from(std::env::var("HOME")?).join(".sui").join("sui_config").join("sui.keystore");
        let keystore = FileBasedKeystore::new(&keystore_path)?;
        let signature = keystore.sign_secure(&self.sender, &tx_data, Intent::sui_transaction())?;

        // Execute transaction and wait for confirmation
        let transaction_response = self.sui_client
            .quorum_driver_api()
            .execute_transaction_block(
                Transaction::from_data(tx_data, vec![signature]),
                SuiTransactionBlockResponseOptions::full_content(),
                Some(ExecuteTransactionRequestType::WaitForLocalExecution),
            )
            .await?;

        Ok(transaction_response.digest.base58_encode())
    }
}
    
    
    
    
    