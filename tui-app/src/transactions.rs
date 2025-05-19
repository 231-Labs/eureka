use anyhow::{anyhow, Result};
use shared_crypto::intent::Intent;
use std::{
    path::PathBuf,
    sync::Arc,
};
use sui_keys::keystore::{
    AccountKeystore,
    FileBasedKeystore,
};
use sui_sdk::{
    rpc_types::{
        SuiObjectDataOptions,
        SuiObjectRef,
        SuiTransactionBlockResponseOptions,
        SuiTransactionBlockResponse,
        SuiTransactionBlockEffectsAPI,
    },
    types::{
        base_types::{
            ObjectID,
            SuiAddress,
        },
        programmable_transaction_builder::ProgrammableTransactionBuilder,
        transaction::{
            Transaction,
            TransactionData,
        },
        Identifier,
        TypeTag,
    },
    SuiClient,
};
use sui_types::{
    object::Owner,
    quorum_driver_types::ExecuteTransactionRequestType,
    transaction::{
        Argument,
        CallArg,
        Command,
        ObjectArg,
    },
};
use crate::{
    constants::{
        GAS_BUDGET,
        SUI_CLOCK_OBJECT_ID,
    },
    utils::NetworkState,
};

/// Gas configuration for transactions
pub struct GasConfig {
    pub budget: u64,
    pub price: Option<u64>,
}

impl Default for GasConfig {
    fn default() -> Self {
        Self {
            budget: GAS_BUDGET,
            price: None,
        }
    }
}

/// Handles transaction signing and execution
pub struct TransactionExecutor {
    sui_client: Arc<SuiClient>,
    sender: SuiAddress,
}

impl TransactionExecutor {
    /// Create a new transaction executor
    pub fn new(sui_client: Arc<SuiClient>, sender: SuiAddress) -> Self {
        Self {
            sui_client,
            sender,
        }
    }
    
    /// Get a gas coin for transaction
    async fn get_gas_coin(&self) -> Result<SuiObjectRef> {
        let coins = self.sui_client
            .coin_read_api()
            .get_coins(self.sender, None, None, None)
            .await?;
        
        coins.data.into_iter().next()
            .map(|coin| SuiObjectRef {
                object_id: coin.coin_object_id,
                version: coin.version,
                digest: coin.digest
            })
            .ok_or_else(|| anyhow!("No available coins found"))
    }
    
    /// Get the initial shared version of a shared object
    async fn get_shared_object(&self, object_id: ObjectID) -> Result<u64> {
        let object = self.sui_client
            .read_api()
            .get_object_with_options(object_id, SuiObjectDataOptions {
                show_owner: true,
                show_content: true,
                show_display: false,
                show_bcs: false,
                show_storage_rebate: false,
                show_previous_transaction: false,
                show_type: true,
            })
            .await?
            .data
            .ok_or_else(|| anyhow!("Object not found"))?;

        match object.owner {
            Some(Owner::Shared { initial_shared_version }) => {
                Ok(initial_shared_version.value())
            },
            _ => Err(anyhow!("Object is not a shared object")),
        }
    }
    
    /// Build a transaction from a programmable transaction builder
    async fn build_transaction(
        &self, 
        ptb: ProgrammableTransactionBuilder, 
        gas_coin: SuiObjectRef,
        gas_config: GasConfig,
    ) -> Result<TransactionData> {
        // Complete transaction building
        let builder = ptb.finish();

        // Get gas price if not provided
        let gas_price = match gas_config.price {
            Some(price) => price,
            None => self.sui_client.read_api().get_reference_gas_price().await?,
        };

        // Create transaction data
        let tx_data = TransactionData::new_programmable(
            self.sender,
            vec![(gas_coin.object_id, gas_coin.version, gas_coin.digest)],
            builder,
            gas_config.budget,
            gas_price,
        );

        Ok(tx_data)
    }
    
    /// Sign and execute a transaction
    async fn sign_and_execute(&self, tx_data: TransactionData) -> Result<SuiTransactionBlockResponse> {
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

        Ok(transaction_response)
    }
    
    /// Execute a move call
    pub async fn execute_move_call(
        &self,
        package_id: ObjectID,
        module: &str,
        function: &str,
        type_args: Vec<TypeTag>,
        args: Vec<CallArg>,
        gas_config: Option<GasConfig>,
    ) -> Result<String> {
        // Get gas configuration
        let gas_config = gas_config.unwrap_or_default();
        
        // Get coin for gas
        let coin = self.get_gas_coin().await?;
        
        // Build programmable transaction
        let mut ptb = ProgrammableTransactionBuilder::new();
        
        // Add inputs
        for arg in &args {
            ptb.input(arg.clone())?;
        }
        
        // Create argument indices
        let args_len = args.len();
        let arg_indices: Vec<Argument> = (0..args_len).map(|i| Argument::Input(i as u16)).collect();
        
        // Add move call
        let module = Identifier::new(module)?;
        let function = Identifier::new(function)?;
        
        ptb.command(Command::move_call(
            package_id,
            module,
            function,
            type_args,
            arg_indices,
        ));
        
        // Build transaction
        let tx_data = self.build_transaction(ptb, coin, gas_config).await?;
        
        // Sign and execute
        let tx_response = self.sign_and_execute(tx_data).await?;
        
        // 檢查交易是否成功
        if let Some(effects) = &tx_response.effects {
            if !effects.status().is_ok() {
                let error_detail = format!("{:?}", effects.status());
                return Err(anyhow!("Transaction failed: {}", error_detail));
            }
        }
        
        Ok(tx_response.digest.base58_encode())
    }
}

/// Main transaction builder for business logic
pub struct TransactionBuilder {
    executor: TransactionExecutor,
    network_state: NetworkState,
}

impl TransactionBuilder {
    /// Create a new transaction builder
    pub async fn new(sui_client: Arc<SuiClient>, sender: ObjectID, network_state: NetworkState) -> Self {
        let sender_address = SuiAddress::from(sender);
        let executor = TransactionExecutor::new(sui_client, sender_address);
        
        Self {
            executor,
            network_state,
        }
    }

    /// Register a printer with the given name
    pub async fn register_printer(
        &self,
        registry_id: ObjectID,
        printer_name: &str,
    ) -> Result<String> {
        // Get shared object information
        let registry_version = self.executor.get_shared_object(registry_id).await?;
        
        // Create shared object argument
        let registry_arg = CallArg::Object(ObjectArg::SharedObject {
            id: registry_id,
            initial_shared_version: registry_version.into(),
            mutable: true,
        });
        
        // Create printer name argument
        let name_bytes = bcs::to_bytes(printer_name)?;
        let name_arg = CallArg::Pure(name_bytes);
        
        // Get package ID
        let package_id = ObjectID::from_hex_literal(&self.network_state.get_current_package_ids().eureka_package_id)?;
        
        // Execute the move call
        self.executor.execute_move_call(
            package_id,
            "eureka",
            "register_printer",
            vec![],
            vec![registry_arg, name_arg],
            None,
        ).await
    }
    
    /// validate and create PrinterCap argument
    async fn create_printer_cap_arg(&self, printer_cap_id: ObjectID) -> Result<CallArg> {
        // get PrinterCap object information
        let cap_response = self.executor.sui_client
            .read_api()
            .get_object_with_options(printer_cap_id, SuiObjectDataOptions {
                show_owner: true,
                show_content: false,
                show_display: false,
                show_bcs: false,
                show_storage_rebate: false,
                show_previous_transaction: false,
                show_type: true,
            })
            .await?;
        
        let cap_data = cap_response.data
            .ok_or_else(|| anyhow!("PrinterCap object not found"))?;
            
        // check if PrinterCap is owned by the sender
        if let Some(Owner::AddressOwner(addr)) = cap_data.owner {
            if addr != self.executor.sender {
                return Err(anyhow!("PrinterCap is owned by a different address"));
            }
        } else {
            return Err(anyhow!("PrinterCap has an invalid ownership type"));
        }
        
        // create PrinterCap argument
        Ok(CallArg::Object(ObjectArg::ImmOrOwnedObject((
            printer_cap_id,
            cap_data.version,
            cap_data.digest,
        ))))
    }

    /// create shared object argument
    async fn create_shared_object_arg(&self, object_id: ObjectID, mutable: bool) -> Result<CallArg> {
        let version = self.executor.get_shared_object(object_id).await?;
        
        Ok(CallArg::Object(ObjectArg::SharedObject {
            id: object_id,
            initial_shared_version: version.into(),
            mutable,
        }))
    }

    /// create owned object argument
    async fn create_owned_object_arg(&self, object_id: ObjectID) -> Result<CallArg> {
        // get object information
        let object_response = self.executor.sui_client
            .read_api()
            .get_object_with_options(object_id, SuiObjectDataOptions {
                show_owner: true,
                show_content: false,
                show_display: false,
                show_bcs: false,
                show_storage_rebate: false,
                show_previous_transaction: false,
                show_type: true,
            })
            .await?;
        
        let object_data = object_response.data
            .ok_or_else(|| anyhow!("Object not found"))?;
            
        // create owned object argument
        Ok(CallArg::Object(ObjectArg::ImmOrOwnedObject((
            object_id,
            object_data.version,
            object_data.digest,
        ))))
    }

    /// execute Move call
    async fn execute_eureka_call(
        &self,
        function: &str,
        args: Vec<CallArg>,
    ) -> Result<String> {
        let package_id = ObjectID::from_hex_literal(&self.network_state.get_current_package_ids().eureka_package_id)?;
        
        self.executor.execute_move_call(
            package_id,
            "eureka",
            function,
            vec![],
            args,
            None,
        ).await
    }

    /// Update printer status
    pub async fn update_printer_status(
        &self,
        printer_cap_id: ObjectID,
        printer_id: ObjectID,
    ) -> Result<String> {
        let cap_arg = self.create_printer_cap_arg(printer_cap_id).await?;
        let printer_arg = self.create_shared_object_arg(printer_id, true).await?;
        
        self.execute_eureka_call(
            "update_printer_status",
            vec![cap_arg, printer_arg],
        ).await
    }

    /// create clock object argument
    async fn create_clock_arg(&self) -> Result<CallArg> {
        let clock_id = ObjectID::from_hex_literal(SUI_CLOCK_OBJECT_ID)?;
        self.create_shared_object_arg(clock_id, false).await
    }

    pub async fn start_print_job(
        &self,
        printer_cap_id: ObjectID,
        printer_id: ObjectID,
        sculpt_id: ObjectID,
    ) -> Result<String> {
        let cap_arg = self.create_printer_cap_arg(printer_cap_id).await?;
        let printer_arg = self.create_shared_object_arg(printer_id, true).await?;
        let sculpt_arg = self.create_owned_object_arg(sculpt_id).await?;
        let clock_arg = self.create_clock_arg().await?;
        
        self.execute_eureka_call(
            "start_print_job",
            vec![cap_arg, printer_arg, sculpt_arg, clock_arg],
        ).await
    }

    pub async fn complete_print_job(
        &self,
        printer_cap_id: ObjectID,
        printer_id: ObjectID,
        sculpt_id: ObjectID,
    ) -> Result<String> {
        let cap_arg = self.create_printer_cap_arg(printer_cap_id).await?;
        let printer_arg = self.create_shared_object_arg(printer_id, true).await?;
        let sculpt_arg = self.create_owned_object_arg(sculpt_id).await?;
        let clock_arg = self.create_clock_arg().await?;
        
        self.execute_eureka_call(
            "complete_print_job",
            vec![cap_arg, printer_arg, sculpt_arg, clock_arg],
        ).await
    }

    /// Create and assign a free print job
    pub async fn create_and_assign_print_job_free(
        &self,
        printer_id: ObjectID,
        sculpt_id: ObjectID,
    ) -> Result<String> {
        let printer_arg = self.create_shared_object_arg(printer_id, true).await?;
        let sculpt_arg = self.create_owned_object_arg(sculpt_id).await?;
        
        self.execute_eureka_call(
            "create_and_assign_print_job_free",
            vec![printer_arg, sculpt_arg],
        ).await
    }
    
    // add new transaction methods, for example:
    /*
    pub async fn print_sculpt(
        &self,
        printer_id: ObjectID,
        sculpt_id: ObjectID,
        payment_amount: u64,
    ) -> Result<String> {
        // 獲取共享對象版本
        let printer_version = self.executor.get_shared_object(printer_id).await?;
        let sculpt_version = self.executor.get_shared_object(sculpt_id).await?;
        
        // 構建參數列表
        let args = vec![
            // 創建共享對象參數
            CallArg::Object(ObjectArg::SharedObject {
                id: printer_id,
                initial_shared_version: printer_version.into(),
                mutable: true,
            }),
            CallArg::Object(ObjectArg::SharedObject {
                id: sculpt_id,
                initial_shared_version: sculpt_version.into(),
                mutable: false,
            }),
            // 創建支付金額參數
            CallArg::Pure(bcs::to_bytes(&payment_amount)?),
        ];
        
        // 獲取包 ID
        let package_id = ObjectID::from_hex_literal(&self.network_state.get_current_package_ids().eureka_package_id)?;
        
        // 執行 Move 調用
        self.executor.execute_move_call(
            package_id,
            "eureka",
            "print_sculpt",
            vec![],
            args,
            None,
        ).await
    }
    */
}
    
    
    
    
    