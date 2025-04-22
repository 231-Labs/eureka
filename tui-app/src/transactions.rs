#[allow(unused_imports)]
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
use sui_keys::keystore::{AccountKeystore, FileBasedKeystore};
use sui_types::{
    quorum_driver_types::ExecuteTransactionRequestType,
    transaction::{Argument, CallArg, Command},
};
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
        // 獲取可用的代幣
        let coins = self.sui_client
            .coin_read_api()
            .get_coins(self.sender, None, None, None)
            .await?;
        
        let coin = coins.data.into_iter().next()
            .ok_or_else(|| anyhow!("No available coins found"))?;

        // 創建可編程交易構建器
        let mut ptb = ProgrammableTransactionBuilder::new();

        // 使用固定的初始共享版本 18
        ptb.input(CallArg::Object(sui_sdk::types::transaction::ObjectArg::SharedObject {
            id: registry_id,
            initial_shared_version: 18.into(),  // TODO:使用固定的初始共享版本, 後續改為動態
            mutable: true,
        }))?;

        // 準備 printer_name 參數
        let name_bytes = bcs::to_bytes(printer_name)?;
        let name_arg = CallArg::Pure(name_bytes);
        ptb.input(name_arg)?;

        // 添加 move call
        let package = ObjectID::from_hex_literal(crate::constants::EUREKA_DEVNET_PACKAGE_ID)?;
        let module = Identifier::new("eureka")?;
        let function = Identifier::new("register_printer")?;

        ptb.command(Command::move_call(
            package,
            module,
            function,
            vec![],  // 類型參數
            vec![Argument::Input(0), Argument::Input(1)],  // 參數
        ));

        // 完成交易構建
        let builder = ptb.finish();

        // 設置 gas 參數
        let gas_budget = 100_000_000;
        let gas_price = self.sui_client.read_api().get_reference_gas_price().await?;

        // 創建交易數據
        let tx_data = TransactionData::new_programmable(
            self.sender,
            vec![coin.object_ref()],
            builder,
            gas_budget,
            gas_price,
        );

        // 簽名交易
        let keystore_path = PathBuf::from(std::env::var("HOME")?).join(".sui").join("sui_config").join("sui.keystore");
        let keystore = FileBasedKeystore::new(&keystore_path)?;
        let signature = keystore.sign_secure(&self.sender, &tx_data, Intent::sui_transaction())?;

        // 執行交易並等待確認
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
    
    
    
    
    