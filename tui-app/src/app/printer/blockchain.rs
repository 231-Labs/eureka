use crate::app::{App, MessageType};
use crate::app::{PrintStatus, ScriptStatus};
use std::sync::Arc;
use sui_sdk_types::Address;
use tokio::sync::Mutex;

/// Clear stuck `PrintJob` for current offline sculpt selection (`eureka::clear_stuck_print_job*`). Sender must be
/// printer owner **and** sculpt owner (same wallet in typical dev setups).
pub(crate) async fn run_clear_stuck_print_job_from_selection(
    app: Arc<Mutex<App>>,
) -> Result<(), String> {
    let (sculpt_id_str, source_kiosk_id, wallet, sui_rpc, tx_signer, network_state) = {
        let g = app.lock().await;
        let idx = g
            .sculpt_state
            .selected()
            .ok_or_else(|| "No sculpt selected".to_string())?;
        if idx >= g.sculpt_items.len() {
            return Err("Invalid sculpt selection".to_string());
        }
        let item = &g.sculpt_items[idx];
        (
            item.id.clone(),
            item.source_kiosk_id.clone(),
            g.wallet.clone(),
            Arc::clone(&g.sui_rpc),
            g.tx_signer.clone(),
            g.network_state.clone(),
        )
    };
    let address = wallet.address;
    let info = wallet
        .get_printer_info(address)
        .await
        .map_err(|e| format!("Failed to get printer info: {}", e))?;
    let cap_id = wallet
        .get_printer_cap_id(address)
        .await
        .map_err(|e| format!("Failed to get PrinterCap ID: {}", e))?;
    let printer_cap_id = App::parse_object_id(&cap_id, "printer cap ID")?;
    let printer_object_id = App::parse_object_id(&info.id, "printer object ID")?;
    let sculpt_id = App::parse_object_id(&sculpt_id_str, "sculpt ID")?;
    let builder = crate::transactions::TransactionBuilder::new(
        sui_rpc,
        (*tx_signer).clone(),
        address,
        network_state,
    )
    .with_printer_eureka_package(&info.eureka_package_id);
    {
        let mut g = app.lock().await;
        g.set_message(
            MessageType::Info,
            "Clearing stuck PrintJob on-chain...".to_string(),
        );
    }
    let clear_result = if let Some(ref ks) = source_kiosk_id {
        let kiosk_aid: Address = ks
            .parse()
            .map_err(|e| format!("Invalid kiosk object id: {}", e))?;
        let kcap = wallet
            .resolve_kiosk_owner_cap_object_id(address, kiosk_aid)
            .await
            .map_err(|e| format!("Failed to resolve KioskOwnerCap: {}", e))?;
        builder
            .clear_stuck_print_job_from_kiosk(
                printer_cap_id,
                printer_object_id,
                kiosk_aid,
                kcap,
                sculpt_id,
            )
            .await
    } else {
        builder
            .clear_stuck_print_job(printer_cap_id, printer_object_id, sculpt_id)
            .await
    };

    match clear_result {
        Ok(tx_id) => {
            let mut g = app.lock().await;
            g.set_message(
                MessageType::Success,
                format!("Stuck PrintJob cleared (Tx: {})", tx_id),
            );
            g.print_output
                .push("[LOG] Stuck PrintJob cleared on-chain".to_string());
            Ok(())
        }
        Err(e) => {
            let user_friendly_error = parse_blockchain_error(&e.to_string(), "clear stuck print job");
            let mut g = app.lock().await;
            g.print_output.push(format!(
                "[LOG] Failed to clear stuck PrintJob: {}",
                user_friendly_error
            ));
            g.set_message(MessageType::Error, user_friendly_error.clone());
            Err(user_friendly_error)
        }
    }
}

/// Create print job from current offline sculpt selection — **no** `App` mutex held across network I/O.
pub(crate) async fn run_create_print_job_from_selection(app: Arc<Mutex<App>>) -> Result<(), String> {
    let (sculpt_id_str, source_kiosk_id, wallet, sui_rpc, tx_signer, network_state) = {
        let g = app.lock().await;
        let idx = g
            .sculpt_state
            .selected()
            .ok_or_else(|| "No sculpt selected".to_string())?;
        if idx >= g.sculpt_items.len() {
            return Err("Invalid sculpt selection".to_string());
        }
        let item = &g.sculpt_items[idx];
        (
            item.id.clone(),
            item.source_kiosk_id.clone(),
            g.wallet.clone(),
            Arc::clone(&g.sui_rpc),
            g.tx_signer.clone(),
            g.network_state.clone(),
        )
    };
    let address = wallet.address;
    let info = wallet
        .get_printer_info(address)
        .await
        .map_err(|e| format!("Failed to get printer info: {}", e))?;
    let printer_object_id = App::parse_object_id(&info.id, "printer object ID")?;
    let sculpt_id = App::parse_object_id(&sculpt_id_str, "sculpt ID")?;
    let builder = crate::transactions::TransactionBuilder::new(
        sui_rpc,
        (*tx_signer).clone(),
        address,
        network_state,
    )
    .with_printer_eureka_package(&info.eureka_package_id);
    {
        let mut g = app.lock().await;
        g.set_message(
            MessageType::Info,
            "Creating print job, waiting for blockchain confirmation...".to_string(),
        );
    }
    let create_result = if let Some(ref ks) = source_kiosk_id {
        let kiosk_aid: Address = ks
            .parse()
            .map_err(|e| format!("Invalid kiosk object id: {}", e))?;
        let cap_id = wallet
            .resolve_kiosk_owner_cap_object_id(address, kiosk_aid)
            .await
            .map_err(|e| format!("Failed to resolve KioskOwnerCap: {}", e))?;
        builder
            .create_print_job_from_kiosk_free(printer_object_id, kiosk_aid, cap_id, sculpt_id)
            .await
    } else {
        builder
            .create_and_assign_print_job_free(printer_object_id, sculpt_id)
            .await
    };

    match create_result {
        Ok(tx_id) => {
            let mut g = app.lock().await;
            g.set_message(
                MessageType::Success,
                format!("Print job created successfully on blockchain (Tx: {})", tx_id),
            );
            g.print_output
                .push("[LOG] Print job created on blockchain successfully".to_string());
            Ok(())
        }
        Err(e) => {
            let user_friendly_error = parse_blockchain_error(&e.to_string(), "create print job");
            let mut g = app.lock().await;
            if user_friendly_error.contains("A print job already exists") {
                g.print_output.push(
                    "[LOG] A print job already exists, continuing with printing...".to_string(),
                );
                Ok(())
            } else {
                g.print_output.push(format!(
                    "[LOG] Failed to create print job on blockchain: {}",
                    user_friendly_error
                ));
                g.set_message(MessageType::Error, user_friendly_error.clone());
                Err(user_friendly_error)
            }
        }
    }
}

/// Start print job from current sculpt selection — **no** `App` mutex held across network I/O.
pub(crate) async fn run_start_print_job_from_selection(app: Arc<Mutex<App>>) -> Result<(), String> {
    let (sculpt_id_str, source_kiosk_id, wallet, sui_rpc, tx_signer, network_state) = {
        let g = app.lock().await;
        let idx = g
            .sculpt_state
            .selected()
            .ok_or_else(|| "No sculpt selected".to_string())?;
        if idx >= g.sculpt_items.len() {
            return Err("Invalid sculpt selection".to_string());
        }
        let item = &g.sculpt_items[idx];
        (
            item.id.clone(),
            item.source_kiosk_id.clone(),
            g.wallet.clone(),
            Arc::clone(&g.sui_rpc),
            g.tx_signer.clone(),
            g.network_state.clone(),
        )
    };
    let address = wallet.address;
    let info = wallet
        .get_printer_info(address)
        .await
        .map_err(|e| format!("Failed to get printer info: {}", e))?;
    let cap_id = wallet
        .get_printer_cap_id(address)
        .await
        .map_err(|e| format!("Failed to get PrinterCap ID: {}", e))?;
    let printer_cap_id = App::parse_object_id(&cap_id, "printer cap ID")?;
    let printer_object_id = App::parse_object_id(&info.id, "printer object ID")?;
    let sculpt_id = App::parse_object_id(&sculpt_id_str, "sculpt ID")?;
    let builder = crate::transactions::TransactionBuilder::new(
        sui_rpc,
        (*tx_signer).clone(),
        address,
        network_state,
    )
    .with_printer_eureka_package(&info.eureka_package_id);
    {
        let mut g = app.lock().await;
        g.set_message(
            MessageType::Info,
            "Starting print job, waiting for blockchain confirmation...".to_string(),
        );
    }
    let start_result = if let Some(ref ks) = source_kiosk_id {
        let kiosk_aid: Address = ks
            .parse()
            .map_err(|e| format!("Invalid kiosk object id: {}", e))?;
        let kcap = wallet
            .resolve_kiosk_owner_cap_object_id(address, kiosk_aid)
            .await
            .map_err(|e| format!("Failed to resolve KioskOwnerCap: {}", e))?;
        builder
            .start_print_job_from_kiosk(
                printer_cap_id,
                printer_object_id,
                kiosk_aid,
                kcap,
                sculpt_id,
            )
            .await
    } else {
        builder
            .start_print_job(printer_cap_id, printer_object_id, sculpt_id)
            .await
    };

    match start_result {
        Ok(tx_id) => {
            let mut g = app.lock().await;
            g.set_message(
                MessageType::Success,
                format!("Print job submitted to blockchain (Tx: {})", tx_id),
            );
            Ok(())
        }
        Err(e) => {
            let user_friendly_error = parse_blockchain_error(&e.to_string(), "start print job");
            let mut g = app.lock().await;
            g.set_message(MessageType::Error, user_friendly_error.clone());
            Err(user_friendly_error)
        }
    }
}

/// Complete print job using selected sculpt — **no** `App` mutex held across the transaction.
pub(crate) async fn run_complete_print_job_from_sculpt_selection(
    app: Arc<Mutex<App>>,
) -> Result<(), String> {
    let (sculpt_id_str, wallet, sui_rpc, tx_signer, network_state) = {
        let g = app.lock().await;
        let idx = g
            .sculpt_state
            .selected()
            .ok_or_else(|| "No sculpt selected".to_string())?;
        if idx >= g.sculpt_items.len() {
            return Err("Invalid sculpt selection".to_string());
        }
        (
            g.sculpt_items[idx].id.clone(),
            g.wallet.clone(),
            Arc::clone(&g.sui_rpc),
            g.tx_signer.clone(),
            g.network_state.clone(),
        )
    };
    let address = wallet.address;
    let info = wallet
        .get_printer_info(address)
        .await
        .map_err(|e| format!("Failed to get printer info: {}", e))?;
    let cap_id = wallet
        .get_printer_cap_id(address)
        .await
        .map_err(|e| format!("Failed to get PrinterCap ID: {}", e))?;
    let printer_cap_id = App::parse_object_id(&cap_id, "printer cap ID")?;
    let printer_object_id = App::parse_object_id(&info.id, "printer object ID")?;
    let sculpt_id = App::parse_object_id(&sculpt_id_str, "sculpt ID")?;
    let builder = crate::transactions::TransactionBuilder::new(
        sui_rpc,
        (*tx_signer).clone(),
        address,
        network_state,
    )
    .with_printer_eureka_package(&info.eureka_package_id);
    {
        let mut g = app.lock().await;
        g.set_message(
            MessageType::Info,
            "Completing print job, waiting for blockchain confirmation...".to_string(),
        );
    }
    match builder
        .complete_print_job(printer_cap_id, printer_object_id, sculpt_id)
        .await
    {
        Ok(tx_id) => {
            {
                let mut g = app.lock().await;
                g.set_message(
                    MessageType::Success,
                    format!("Print job completed successfully on blockchain (Tx: {})", tx_id),
                );
                g.tasks.clear();
                g.print_status = PrintStatus::Idle;
                g.script_status = ScriptStatus::Idle;
            }
            {
                let mut g = app.lock().await;
                if let Err(e) = g.update_print_tasks().await {
                    g.set_message(
                        MessageType::Error,
                        format!("Failed to update print tasks: {}", e),
                    );
                    return Err(format!("Failed to update print tasks: {}", e));
                }
                g.set_message(
                    MessageType::Success,
                    "Print job completed and tasks updated successfully".to_string(),
                );
                g.clamp_tasks_list_state();
            }
            Ok(())
        }
        Err(e) => {
            let user_friendly_error = parse_blockchain_error(&e.to_string(), "complete print job");
            let mut g = app.lock().await;
            g.set_message(MessageType::Error, user_friendly_error.clone());
            Err(user_friendly_error)
        }
    }
}

/// Transfer completed print job (mock / task flow) — **no** `App` mutex held across the transaction.
pub(crate) async fn run_transfer_completed_print_job(app: Arc<Mutex<App>>) -> Result<(), String> {
    let (wallet, sui_rpc, tx_signer, network_state) = {
        let g = app.lock().await;
        (
            g.wallet.clone(),
            Arc::clone(&g.sui_rpc),
            g.tx_signer.clone(),
            g.network_state.clone(),
        )
    };
    let address = wallet.address;
    let info = wallet
        .get_printer_info(address)
        .await
        .map_err(|e| format!("Failed to get printer info: {}", e))?;
    let cap_id = wallet
        .get_printer_cap_id(address)
        .await
        .map_err(|e| format!("Failed to get PrinterCap ID: {}", e))?;
    let printer_cap_id = App::parse_object_id(&cap_id, "printer cap ID")?;
    let printer_object_id = App::parse_object_id(&info.id, "printer object ID")?;
    let builder = crate::transactions::TransactionBuilder::new(
        sui_rpc,
        (*tx_signer).clone(),
        address,
        network_state,
    )
    .with_printer_eureka_package(&info.eureka_package_id);
    {
        let mut g = app.lock().await;
        g.set_message(
            MessageType::Info,
            "Transferring completed PrintJob to printer owner wallet...".to_string(),
        );
    }
    match builder
        .transfer_completed_print_job(printer_cap_id, printer_object_id)
        .await
    {
        Ok(tx_id) => {
            {
                let mut g = app.lock().await;
                g.set_message(
                    MessageType::Success,
                    format!(
                        "PrintJob transferred to printer owner wallet successfully (Tx: {})",
                        tx_id
                    ),
                );
                g.tasks.clear();
                g.print_status = PrintStatus::Idle;
                g.script_status = ScriptStatus::Idle;
            }
            {
                let mut g = app.lock().await;
                if let Err(e) = g.update_print_tasks().await {
                    g.set_message(
                        MessageType::Error,
                        format!("Failed to update print tasks: {}", e),
                    );
                    return Err(format!("Failed to update print tasks: {}", e));
                }
                g.set_message(
                    MessageType::Success,
                    "PrintJob transferred and printer is ready for next job".to_string(),
                );
                g.clamp_tasks_list_state();
            }
            Ok(())
        }
        Err(e) => {
            let user_friendly_error =
                parse_blockchain_error(&e.to_string(), "transfer completed PrintJob");
            let mut g = app.lock().await;
            g.set_message(MessageType::Error, user_friendly_error.clone());
            Err(user_friendly_error)
        }
    }
}

fn parse_blockchain_error(error_msg: &str, context: &str) -> String {
    // PTB simulation uses MoveAbort(MoveLocation { ... function: ... }, <code>) — no "EPrintJobExists" string.
    if error_msg.contains("MoveAbort") || error_msg.contains("MOVE_ABORT") {
        if error_msg.contains("create_and_assign_print_job_internal")
            || error_msg.contains("create_print_job_from_kiosk")
        {
            if error_msg.contains(", 2)") {
                return "A print job already exists for this printer.".to_string();
            }
        }
    }

    if error_msg.contains("TransactionEffects") && error_msg.contains("status") {
        if error_msg.contains("EPrintJobExists") {
            "A print job already exists for this printer.".to_string()
        } else if error_msg.contains("EPrintJobNotStarted") {
            "Print job has not been started yet. Please start the print job first.".to_string()
        } else if error_msg.contains("EPrintJobCompleted") {
            "Print job has already been completed.".to_string()
        } else if error_msg.contains("ENotAuthorized") {
            "Not authorized to complete this print job.".to_string()
        } else if error_msg.contains("dynamic_field") && error_msg.contains("borrow_child_object") {
            "Failed to find print job. Please make sure printer is properly registered.".to_string()
        } else if error_msg.contains("insufficient gas") {
            "Insufficient gas to execute transaction. Please add more SUI to your wallet.".to_string()
        } else {
            format!("Transaction failed on blockchain: {}", error_msg)
        }
    } else {
        format!("Failed to {}: {}", context, error_msg)
    }
}

impl App {
    pub(crate) fn parse_object_id(id_str: &str, context: &str) -> Result<Address, String> {
        id_str
            .parse()
            .map_err(|e| format!("Invalid {} ({}): {}", context, id_str, e))
    }
}
