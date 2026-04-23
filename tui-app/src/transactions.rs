use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::Duration;
use sui_crypto::ed25519::Ed25519PrivateKey;
use sui_crypto::SuiSigner;
use sui_rpc::proto::sui::rpc::v2::ExecuteTransactionRequest;
use sui_rpc::proto::sui::rpc::v2::GetObjectRequest;
use sui_rpc::Client as GrpcClient;
use sui_sdk_types::Address;
use sui_sdk_types::Identifier;
use sui_transaction_builder::{
    Argument, Error as TxBuilderError, Function, ObjectInput, TransactionBuilder as TxBuilder,
};
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::constants::{GAS_BUDGET, SUI_CLOCK_OBJECT_ID};
use crate::wallet::read_mask;
use crate::utils::NetworkState;

const TRANSACTION_TIMEOUT: Duration = Duration::from_secs(30);
const OBJECT_FETCH_TIMEOUT: Duration = Duration::from_secs(10);

fn format_tx_builder_error(e: TxBuilderError) -> String {
    match e {
        TxBuilderError::Input(msg) => msg,
        TxBuilderError::SimulationFailure(s) => s.to_string(),
        other => format!("{:?}", other),
    }
}

pub struct TransactionExecutor {
    rpc: Arc<Mutex<GrpcClient>>,
    signer: Ed25519PrivateKey,
    sender: Address,
}

impl TransactionExecutor {
    pub fn new(rpc: Arc<Mutex<GrpcClient>>, signer: Ed25519PrivateKey, sender: Address) -> Self {
        Self {
            rpc,
            signer,
            sender,
        }
    }

    async fn get_shared_object_initial_version(&self, id: Address) -> Result<u64> {
        let fut = async {
            let mut c = self.rpc.lock().await;
            let resp = c
                .ledger_client()
                .get_object(
                    GetObjectRequest::new(&id).with_read_mask(
                        read_mask("owner"),
                    ),
                )
                .await
                .map_err(|e| anyhow!("get_object: {}", e))?
                .into_inner();
            let owner = resp
                .object()
                .owner
                .as_ref()
                .ok_or_else(|| anyhow!("missing owner"))?;
            let sui_owner: sui_sdk_types::Owner = owner
                .try_into()
                .map_err(|e: sui_rpc::proto::TryFromProtoError| anyhow!("owner: {}", e))?;
            match sui_owner {
                sui_sdk_types::Owner::Shared(v) => Ok(v),
                _ => Err(anyhow!("Object is not a shared object")),
            }
        };
        timeout(OBJECT_FETCH_TIMEOUT, fut)
            .await
            .map_err(|_| anyhow!("Timeout fetching shared object version"))?
    }

    async fn owned_object_input(&self, id: Address) -> Result<ObjectInput> {
        let fut = async {
            let mut c = self.rpc.lock().await;
            let resp = c
                .ledger_client()
                .get_object(
                    GetObjectRequest::new(&id).with_read_mask(
                        read_mask("version,digest,owner"),
                    ),
                )
                .await
                .map_err(|e| anyhow!("get_object: {}", e))?
                .into_inner();
            let o = resp.object();
            let version = o.version_opt().ok_or_else(|| anyhow!("missing version"))?;
            let digest_str = o.digest_opt().ok_or_else(|| anyhow!("missing digest"))?;
            let digest = digest_str
                .parse()
                .map_err(|e| anyhow!("digest: {}", e))?;
            Ok(ObjectInput::owned(id, version, digest))
        };
        timeout(OBJECT_FETCH_TIMEOUT, fut)
            .await
            .map_err(|_| anyhow!("Timeout fetching owned object"))?
    }

    async fn sign_and_execute(&self, transaction: sui_sdk_types::Transaction) -> Result<String> {
        let sig = self
            .signer
            .sign_transaction(&transaction)
            .map_err(|e| anyhow!("sign: {}", e))?;

        let fut = async {
            let mut c = self.rpc.lock().await;
            c.execute_transaction_and_wait_for_checkpoint(
                ExecuteTransactionRequest::new(transaction.into())
                    .with_signatures(vec![sig.into()])
                    .with_read_mask(read_mask("*")),
                TRANSACTION_TIMEOUT,
            )
            .await
            .map_err(|e| anyhow!("execute: {}", e))
        };

        let response = timeout(TRANSACTION_TIMEOUT, fut)
            .await
            .map_err(|_| anyhow!("Transaction execution timeout"))??;

        let inner = response.into_inner();
        if !inner.transaction().effects().status().success() {
            let err = inner.transaction().effects().status().error().clone();
            return Err(anyhow!("Transaction failed: {:?}", err));
        }
        inner
            .transaction()
            .digest_opt()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("missing digest in response"))
    }

    async fn run_eureka_ptb(
        &self,
        package: Address,
        function: &str,
        object_inputs: Vec<ObjectInput>,
        pure_args: Vec<Vec<u8>>,
    ) -> Result<String> {
        let args: Vec<EurekaPtbArg> = object_inputs
            .into_iter()
            .map(EurekaPtbArg::Object)
            .chain(pure_args.into_iter().map(EurekaPtbArg::Pure))
            .collect();
        self.run_eureka_ptb_ordered(package, function, args).await
    }

    /// Object and pure arguments in **Move parameter order** (e.g. `sculpt_id` before `clock`).
    async fn run_eureka_ptb_ordered(
        &self,
        package: Address,
        function: &str,
        args: Vec<EurekaPtbArg>,
    ) -> Result<String> {
        let mut tb = TxBuilder::new();
        tb.set_sender(self.sender);
        tb.set_gas_budget(GAS_BUDGET);

        let mut call_args: Vec<Argument> = Vec::with_capacity(args.len());
        for a in args {
            match a {
                EurekaPtbArg::Object(oi) => call_args.push(tb.object(oi)),
                EurekaPtbArg::Pure(bytes) => call_args.push(tb.pure_bytes(bytes)),
            }
        }

        let f = Function::new(
            package,
            Identifier::new("eureka")?,
            Identifier::new(function)?,
        );
        tb.move_call(f, call_args);

        let tx = {
            let mut c = self.rpc.lock().await;
            tb.build(&mut *c)
                .await
                .map_err(|e| anyhow!("PTB build/simulate: {}", format_tx_builder_error(e)))?
        };
        self.sign_and_execute(tx).await
    }
}

enum EurekaPtbArg {
    Object(ObjectInput),
    Pure(Vec<u8>),
}

pub struct TransactionBuilder {
    executor: TransactionExecutor,
    network_state: NetworkState,
    /// When set, used as the package id for `eureka::*` move calls (must match on-chain `Printer` type).
    eureka_package_override: Option<Address>,
}

impl TransactionBuilder {
    pub fn new(
        rpc: Arc<Mutex<GrpcClient>>,
        signer: Ed25519PrivateKey,
        sender: Address,
        network_state: NetworkState,
    ) -> Self {
        let executor = TransactionExecutor::new(rpc, signer, sender);
        Self {
            executor,
            network_state,
            eureka_package_override: None,
        }
    }

    pub fn with_eureka_package(mut self, package_id: Address) -> Self {
        self.eureka_package_override = Some(package_id);
        self
    }

    /// Override from on-chain `Printer` type tag (`original-id`). Ignored for PTB when the network sets
    /// [`NetworkPackageIds::eureka_move_call_package_id`] (upgraded `published-at`).
    pub fn with_printer_eureka_package(self, package_id: &str) -> Self {
        if package_id.is_empty() {
            return self;
        }
        match package_id.parse::<Address>() {
            Ok(p) => self.with_eureka_package(p),
            Err(_) => self,
        }
    }

    fn resolve_eureka_package_id(&self) -> Result<Address> {
        let ids = self.network_state.get_current_package_ids();
        if !ids.eureka_move_call_package_id.is_empty() {
            return ids
                .eureka_move_call_package_id
                .parse()
                .map_err(|e| anyhow!("eureka_move_call_package_id: {}", e));
        }
        if let Some(p) = self.eureka_package_override {
            return Ok(p);
        }
        let s = ids.eureka_package_id;
        if s.is_empty() {
            return Err(anyhow!(
                "Eureka package ID is not set for this network; cannot build eureka transaction"
            ));
        }
        s.parse().map_err(|e| anyhow!("package id: {}", e))
    }

    async fn create_printer_cap_arg(&self, printer_cap_id: Address) -> Result<ObjectInput> {
        let fut = async {
            let mut c = self.executor.rpc.lock().await;
            let resp = c
                .ledger_client()
                .get_object(
                    GetObjectRequest::new(&printer_cap_id).with_read_mask(
                        read_mask("owner,version,digest"),
                    ),
                )
                .await
                .map_err(|e| anyhow!("get_object: {}", e))?
                .into_inner();
            let o = resp.object();
            let owner = o.owner.as_ref().ok_or_else(|| anyhow!("missing owner"))?;
            let sui_owner: sui_sdk_types::Owner = owner
                .try_into()
                .map_err(|e: sui_rpc::proto::TryFromProtoError| anyhow!("owner: {}", e))?;
            if let sui_sdk_types::Owner::Address(addr) = sui_owner {
                if addr != self.executor.sender {
                    return Err(anyhow!("PrinterCap is owned by a different address"));
                }
            } else {
                return Err(anyhow!("PrinterCap has an invalid ownership type"));
            }
            let version = o.version_opt().ok_or_else(|| anyhow!("missing version"))?;
            let digest_str = o.digest_opt().ok_or_else(|| anyhow!("missing digest"))?;
            let digest = digest_str.parse().map_err(|e| anyhow!("digest: {}", e))?;
            Ok(ObjectInput::owned(printer_cap_id, version, digest))
        };
        timeout(OBJECT_FETCH_TIMEOUT, fut)
            .await
            .map_err(|_| anyhow!("Timeout fetching PrinterCap"))?
    }

    async fn create_shared_object_arg(&self, object_id: Address, mutable: bool) -> Result<ObjectInput> {
        let v = self.executor.get_shared_object_initial_version(object_id).await?;
        Ok(ObjectInput::shared(object_id, v, mutable))
    }

    async fn create_owned_object_arg(&self, object_id: Address) -> Result<ObjectInput> {
        self.executor.owned_object_input(object_id).await
    }

    async fn create_clock_arg(&self) -> Result<ObjectInput> {
        let clock_id: Address = SUI_CLOCK_OBJECT_ID
            .parse()
            .map_err(|e| anyhow!("clock id: {}", e))?;
        self.create_shared_object_arg(clock_id, false).await
    }

    async fn execute_eureka_call(
        &self,
        function: &str,
        object_inputs: Vec<ObjectInput>,
        pure_args: Vec<Vec<u8>>,
    ) -> Result<String> {
        let package = self.resolve_eureka_package_id()?;
        self.executor
            .run_eureka_ptb(package, function, object_inputs, pure_args)
            .await
    }

    async fn execute_eureka_call_ordered(
        &self,
        function: &str,
        args: Vec<EurekaPtbArg>,
    ) -> Result<String> {
        let package = self.resolve_eureka_package_id()?;
        self.executor
            .run_eureka_ptb_ordered(package, function, args)
            .await
    }

    pub async fn register_printer(&self, registry_id: Address, printer_name: &str) -> Result<String> {
        const MAX_RETRIES: u32 = 3;
        let mut last_error = None;
        let name_bytes = bcs::to_bytes(printer_name)?;

        for attempt in 0..MAX_RETRIES {
            let registry_version = match self
                .executor
                .get_shared_object_initial_version(registry_id)
                .await
            {
                Ok(version) => version,
                Err(e) => {
                    last_error = Some(e);
                    if attempt < MAX_RETRIES - 1 {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                    return Err(last_error.unwrap());
                }
            };

            let registry_obj = ObjectInput::shared(registry_id, registry_version, true);

            match self
                .execute_eureka_call(
                    "register_printer_and_transfer",
                    vec![registry_obj],
                    vec![name_bytes.clone()],
                )
                .await
            {
                Ok(digest) => return Ok(digest),
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("version")
                        || error_msg.contains("stale")
                        || error_msg.contains("SharedObjectSequenceNumberMismatch")
                    {
                        last_error = Some(e);
                        if attempt < MAX_RETRIES - 1 {
                            tokio::time::sleep(Duration::from_millis(1000)).await;
                            continue;
                        }
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow!("Failed to register printer after {} attempts", MAX_RETRIES)
        }))
    }

    pub async fn update_printer_status(
        &self,
        printer_cap_id: Address,
        printer_id: Address,
    ) -> Result<String> {
        let cap_arg = self.create_printer_cap_arg(printer_cap_id).await?;
        let printer_arg = self.create_shared_object_arg(printer_id, true).await?;
        self.execute_eureka_call("update_printer_status", vec![cap_arg, printer_arg], vec![])
            .await
    }

    pub async fn start_print_job(
        &self,
        printer_cap_id: Address,
        printer_id: Address,
        sculpt_id: Address,
    ) -> Result<String> {
        let cap_arg = self.create_printer_cap_arg(printer_cap_id).await?;
        let printer_arg = self.create_shared_object_arg(printer_id, true).await?;
        let sculpt_arg = self.create_owned_object_arg(sculpt_id).await?;
        let clock_arg = self.create_clock_arg().await?;
        self.execute_eureka_call(
            "start_print_job",
            vec![cap_arg, printer_arg, sculpt_arg, clock_arg],
            vec![],
        )
        .await
    }

    pub async fn start_print_job_from_kiosk(
        &self,
        printer_cap_id: Address,
        printer_id: Address,
        kiosk_id: Address,
        kiosk_cap_id: Address,
        sculpt_id: Address,
    ) -> Result<String> {
        let cap_arg = self.create_printer_cap_arg(printer_cap_id).await?;
        let printer_arg = self.create_shared_object_arg(printer_id, true).await?;
        let kiosk_arg = self.create_shared_object_arg(kiosk_id, true).await?;
        let kiosk_cap_arg = self.create_owned_object_arg(kiosk_cap_id).await?;
        let clock_arg = self.create_clock_arg().await?;
        let id_bytes = bcs::to_bytes(&sculpt_id).map_err(|e| anyhow!("bcs sculpt ID: {}", e))?;
        self.execute_eureka_call_ordered(
            "start_print_job_from_kiosk",
            vec![
                EurekaPtbArg::Object(cap_arg),
                EurekaPtbArg::Object(printer_arg),
                EurekaPtbArg::Object(kiosk_arg),
                EurekaPtbArg::Object(kiosk_cap_arg),
                EurekaPtbArg::Pure(id_bytes),
                EurekaPtbArg::Object(clock_arg),
            ],
        )
        .await
    }

    pub async fn complete_print_job(
        &self,
        printer_cap_id: Address,
        printer_id: Address,
        sculpt_id: Address,
    ) -> Result<String> {
        let cap_arg = self.create_printer_cap_arg(printer_cap_id).await?;
        let printer_arg = self.create_shared_object_arg(printer_id, true).await?;
        let sculpt_arg = self.create_owned_object_arg(sculpt_id).await?;
        let clock_arg = self.create_clock_arg().await?;
        self.execute_eureka_call(
            "complete_print_job",
            vec![cap_arg, printer_arg, sculpt_arg, clock_arg],
            vec![],
        )
        .await
    }

    pub async fn transfer_completed_print_job(
        &self,
        printer_cap_id: Address,
        printer_id: Address,
    ) -> Result<String> {
        let cap_arg = self.create_printer_cap_arg(printer_cap_id).await?;
        let printer_arg = self.create_shared_object_arg(printer_id, true).await?;
        let clock_arg = self.create_clock_arg().await?;
        self.execute_eureka_call(
            "transfer_completed_print_job",
            vec![cap_arg, printer_arg, clock_arg],
            vec![],
        )
        .await
    }

    pub async fn create_and_assign_print_job_free(
        &self,
        printer_id: Address,
        sculpt_id: Address,
    ) -> Result<String> {
        let printer_arg = self.create_shared_object_arg(printer_id, true).await?;
        let sculpt_arg = self.create_owned_object_arg(sculpt_id).await?;
        self.execute_eureka_call(
            "create_and_assign_print_job_free",
            vec![printer_arg, sculpt_arg],
            vec![],
        )
        .await
    }

    /// Print job for a `Sculpt` listed from a Kiosk (uses `kiosk::borrow_mut` on-chain).
    pub async fn create_print_job_from_kiosk_free(
        &self,
        printer_id: Address,
        kiosk_id: Address,
        kiosk_cap_id: Address,
        sculpt_id: Address,
    ) -> Result<String> {
        let printer_arg = self.create_shared_object_arg(printer_id, true).await?;
        let kiosk_arg = self.create_shared_object_arg(kiosk_id, true).await?;
        let cap_arg = self.create_owned_object_arg(kiosk_cap_id).await?;
        let id_bytes = bcs::to_bytes(&sculpt_id).map_err(|e| anyhow!("bcs sculpt ID: {}", e))?;
        self.execute_eureka_call(
            "create_print_job_from_kiosk_free",
            vec![printer_arg, kiosk_arg, cap_arg],
            vec![id_bytes],
        )
        .await
    }

    /// Dev / recovery: `eureka::clear_stuck_print_job` (owned sculpt).
    pub async fn clear_stuck_print_job(
        &self,
        printer_cap_id: Address,
        printer_id: Address,
        sculpt_id: Address,
    ) -> Result<String> {
        let cap_arg = self.create_printer_cap_arg(printer_cap_id).await?;
        let printer_arg = self.create_shared_object_arg(printer_id, true).await?;
        let sculpt_arg = self.create_owned_object_arg(sculpt_id).await?;
        self.execute_eureka_call(
            "clear_stuck_print_job",
            vec![cap_arg, printer_arg, sculpt_arg],
            vec![],
        )
        .await
    }

    /// Dev / recovery: `eureka::clear_stuck_print_job_from_kiosk`.
    pub async fn clear_stuck_print_job_from_kiosk(
        &self,
        printer_cap_id: Address,
        printer_id: Address,
        kiosk_id: Address,
        kiosk_cap_id: Address,
        sculpt_id: Address,
    ) -> Result<String> {
        let cap_arg = self.create_printer_cap_arg(printer_cap_id).await?;
        let printer_arg = self.create_shared_object_arg(printer_id, true).await?;
        let kiosk_arg = self.create_shared_object_arg(kiosk_id, true).await?;
        let kiosk_cap_arg = self.create_owned_object_arg(kiosk_cap_id).await?;
        let id_bytes = bcs::to_bytes(&sculpt_id).map_err(|e| anyhow!("bcs sculpt ID: {}", e))?;
        self.execute_eureka_call(
            "clear_stuck_print_job_from_kiosk",
            vec![cap_arg, printer_arg, kiosk_arg, kiosk_cap_arg],
            vec![id_bytes],
        )
        .await
    }
}
