use crate::app::core::App;
use crate::app::print_job::PrintTask;
use crate::constants::AGGREGATOR_URL;
use crate::utils::crate_root;
use crate::seal::{is_file_encrypted, PrintJobDecryptor};
use crate::app::printer::mock::{run_mock_print_script, MockPrintScriptResult};
use anyhow::Result;
use seal_sdk_rs::native_sui_sdk::sui_types::base_types::ObjectID as SuiObjectID;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::fs;
use std::path::Path;

/// Download plus optional Seal decrypt (only via `eureka::seal_approve` + PrintJob, matching on-chain rules).
async fn download_model_isolated(
    blob_id: &str,
    seal_resource_id: Option<&str>,
    current_rpc: &str,
    eureka_package_id: &str,
    // Required for encrypted models: (printer_object_id, printer_cap_object_id)
    printer_for_seal: Option<(String, String)>,
) -> Result<Vec<String>> {
    let mut log = Vec::new();
    let url = format!("{}/v1/blobs/{}", AGGREGATOR_URL, blob_id);
    let root = crate_root();
    let temp_path = root.join("test.stl");
    let final_path = root.join("Gcode-Transmit").join("test.stl");

    log.push(format!("[LOG] Downloading model from: {}", url));

    let gcode_dir = root.join("Gcode-Transmit");
    if !Path::new(&gcode_dir).exists() {
        log.push(format!("[LOG] Creating directory: {}", gcode_dir.display()));
        fs::create_dir_all(&gcode_dir)?;
    }

    let status = tokio::process::Command::new("curl")
        .arg("-s")
        .arg("-S")
        .arg(&url)
        .arg("-o")
        .arg(&temp_path)
        .status()
        .await?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to download 3D model"));
    }

    if let Some(resource_id_str) = seal_resource_id {
        log.push("[LOG] 🔐 Encrypted model: decrypting via PrintJob + eureka::seal_approve...".to_string());
        log.push(format!("[LOG] 🔐 seal_resource_id: {}", resource_id_str));
        let (printer_id, cap_id) = printer_for_seal.ok_or_else(|| {
            anyhow::anyhow!(
                "Encrypted models need a registered printer and an on-chain PrintJob; create a print job from your selection before downloading."
            )
        })?;
        decrypt_model_with_printjob(
            &temp_path,
            resource_id_str,
            current_rpc,
            eureka_package_id,
            &printer_id,
            &cap_id,
            &mut log,
        )
        .await?;
        log.push("[LOG] ✅ Model decrypted successfully".to_string());
    }

    fs::rename(&temp_path, &final_path).map_err(|e| anyhow::anyhow!("Failed to move 3D model: {}", e))?;
    Ok(log)
}

/// Walrus blob id for the STL and effective Seal id — prefer on-chain `Sculpt.structure` (same source as
/// `PrintJobDecryptor::decrypt_printjob_sculpt`) so online download matches decryption; fall back to
/// `PrintJob.sculpt_structure` only when we cannot query the Sculpt (no printer id).
async fn walrus_blob_and_seal_for_online_task(
    task: &PrintTask,
    rpc: &str,
    eureka_package_id: &str,
    printer_id: &str,
    log: &mut Vec<String>,
) -> Result<(String, Option<String>), anyhow::Error> {
    if printer_id == "No Printer ID" || task.sculpt_blob_id.trim().is_empty() {
        let s = task.sculpt_structure.trim();
        if s.is_empty() {
            return Err(anyhow::anyhow!(
                "PrintJob has no usable Walrus blob id (sculpt_structure empty and no printer to read Sculpt on-chain)"
            ));
        }
        log.push(
            "[LOG] Using PrintJob.sculpt_structure as Walrus blob id (no printer to resolve Sculpt on-chain)"
                .to_string(),
        );
        return Ok((task.sculpt_structure.clone(), task.seal_resource_id.clone()));
    }

    let sculpt_id = SuiObjectID::from_hex_literal(task.sculpt_blob_id.trim())
        .map_err(|e| anyhow::anyhow!("Invalid PrintJob.sculpt_id (sculpt object id): {}", e))?;
    let printer_oid = SuiObjectID::from_hex_literal(printer_id.trim())
        .map_err(|e| anyhow::anyhow!("Invalid printer_id: {}", e))?;

    let decryptor = PrintJobDecryptor::new(rpc.to_string(), eureka_package_id).await?;
    let (structure, seal_on_sculpt, _) = decryptor
        .fetch_sculpt_and_objects(sculpt_id, printer_oid)
        .await?;

    log.push(format!(
        "[LOG] Resolved Walrus STL blob from on-chain Sculpt.structure (prefix {})",
        structure.chars().take(16).collect::<String>()
    ));

    if !task.sculpt_structure.is_empty() && task.sculpt_structure != structure {
        log.push(format!(
            "[LOG] Note: PrintJob.sculpt_structure differs from Sculpt.structure; using chain value for download."
        ));
    }

    let seal = task.seal_resource_id.clone().or(seal_on_sculpt);
    Ok((structure, seal))
}

async fn decrypt_model_with_printjob(
    file_path: &Path,
    seal_resource_id: &str,
    rpc_url: &str,
    eureka_package_id: &str,
    printer_id: &str,
    printer_cap_id: &str,
    log: &mut Vec<String>,
) -> Result<()> {
    let encrypted_data = tokio::fs::read(file_path).await?;

    if !is_file_encrypted(&encrypted_data) {
        log.push("[LOG] ⚠️  File looks like plaintext STL; skipping decryption".to_string());
        return Ok(());
    }

    log.push("[LOG] 🔐 Initializing PrintJobDecryptor (Seal SDK + JSON-RPC)...".to_string());
    let decryptor = PrintJobDecryptor::new(rpc_url.to_string(), eureka_package_id).await?;

    let printer_oid =
        SuiObjectID::from_hex_literal(printer_id).map_err(|e| anyhow::anyhow!("printer_id: {}", e))?;
    let cap_oid = SuiObjectID::from_hex_literal(printer_cap_id)
        .map_err(|e| anyhow::anyhow!("printer_cap_id: {}", e))?;

    let decrypted = decryptor
        .decrypt_sealed_file_bytes(seal_resource_id, &encrypted_data, printer_oid, cap_oid)
        .await?;

    tokio::fs::write(file_path, decrypted).await?;
    Ok(())
}

impl App {
    pub async fn handle_model_selection(app: Arc<Mutex<App>>, download_only: bool) -> Result<()> {
        let app_clone = Arc::clone(&app);
        tokio::spawn(async move {
            let selected_item = {
                let app_guard = app_clone.lock().await;
                app_guard.sculpt_state
                    .selected()
                    .and_then(|idx| app_guard.sculpt_items.get(idx).cloned())
            };

            if let Some(item) = selected_item {
                if item.alias != "No printable models found" {
                    {
                        let mut app = app_clone.lock().await;
                        app.print_output.push(format!("[LOG] Selected model: {}", item.alias));
                    }

                    let seal = item.seal_resource_id.as_deref();
                    let (rpc, eureka_pkg) = {
                        let g = app_clone.lock().await;
                        (
                            g.network_state.get_current_rpc().to_string(),
                            g.network_state
                                .get_current_package_ids()
                                .eureka_package_id
                                .to_string(),
                        )
                    };

                    // `eureka::seal_approve` requires a PrintJob on the printer; create it before download/decrypt.
                    let printer_for_seal: Option<(String, String)> = if seal.is_some() {
                        let has_printer = {
                            let g = app_clone.lock().await;
                            g.printer_id != "No Printer ID"
                        };
                        if !has_printer {
                            let mut app = app_clone.lock().await;
                            app.set_message(
                                crate::app::MessageType::Error,
                                "Encrypted models require a registered printer. Complete printer registration first."
                                    .to_string(),
                            );
                            return;
                        }
                        {
                            let mut g = app_clone.lock().await;
                            g.print_output.push(
                                "[LOG] Encrypted model: creating on-chain PrintJob first (required for seal_approve)…"
                                    .to_string(),
                            );
                        }
                        if let Err(e) =
                            crate::app::printer::blockchain::run_create_print_job_from_selection(
                                Arc::clone(&app_clone),
                            )
                            .await
                        {
                            let mut app = app_clone.lock().await;
                            app.set_message(
                                crate::app::MessageType::Error,
                                format!("Could not create PrintJob; cannot decrypt: {}", e),
                            );
                            return;
                        }
                        let wallet_address = { let g = app_clone.lock().await; g.wallet.address };
                        let printer_id = { let g = app_clone.lock().await; g.printer_id.clone() };
                        let cap_id = match app_clone
                            .lock()
                            .await
                            .wallet
                            .get_printer_cap_id(wallet_address)
                            .await
                        {
                            Ok(c) => c,
                            Err(e) => {
                                let mut app = app_clone.lock().await;
                                app.set_message(
                                    crate::app::MessageType::Error,
                                    format!("Could not load PrinterCap: {}", e),
                                );
                                return;
                            }
                        };
                        Some((printer_id, cap_id))
                    } else {
                        None
                    };

                    match download_model_isolated(
                        &item.blob_id,
                        seal,
                        &rpc,
                        &eureka_pkg,
                        printer_for_seal,
                    )
                    .await
                    {
                        Ok(mut lines) => {
                            let mut app = app_clone.lock().await;
                            app.print_output.append(&mut lines);
                            app.set_message(
                                crate::app::MessageType::Success,
                                "3D model downloaded successfully".to_string(),
                            );
                        }
                        Err(e) => {
                            let mut app = app_clone.lock().await;
                            app.set_message(
                                crate::app::MessageType::Error,
                                format!("Failed to download model: {}", e),
                            );
                            return;
                        }
                    }

                    // run print script (not only download)
                    if !download_only {
                        let has_printer = {
                            let g = app_clone.lock().await;
                            g.printer_id != "No Printer ID"
                        };
                        if has_printer && seal.is_none() {
                            {
                                let mut g = app_clone.lock().await;
                                g.print_output
                                    .push("[LOG] Creating print job on blockchain...".to_string());
                            }
                            let _ = crate::app::printer::blockchain::run_create_print_job_from_selection(
                                Arc::clone(&app_clone),
                            )
                            .await;
                        }
                        
                        {
                            let mut app = app_clone.lock().await;
                            app.print_output.push("[LOG] Preparing to run print script".to_string());
                            app.print_status = crate::app::PrintStatus::Printing;
                        }
                        
                        let print_result = App::run_print_script(Arc::clone(&app_clone)).await;
                        
                        let mut app = app_clone.lock().await;
                        match print_result {
                            Ok(_) => {
                                app.print_output.push("[LOG] Print script executed successfully".to_string());
                            },
                            Err(error_msg) => {
                                app.print_output.push(format!("[LOG] Print script failed: {}", error_msg));
                                app.set_message(crate::app::MessageType::Error, error_msg);
                                // Update status to failed when there's an error
                                app.print_status = crate::app::PrintStatus::Idle;
                            }
                        }
                    }
                }
            }
        });
        Ok(())
    }

    pub async fn handle_task_print(app: Arc<Mutex<App>>, download_only: bool) -> Result<()> {
        let app_clone = Arc::clone(&app);
        tokio::spawn(async move {
            let active_task = {
                let app_guard = app_clone.lock().await;
                app_guard.tasks.iter()
                    .find(|t| matches!(t.status, crate::app::print_job::TaskStatus::Active))
                    .cloned()
            };

            if let Some(task) = active_task {
                {
                    let mut app = app_clone.lock().await;
                    app.print_output.push(format!("[LOG] Processing active task: {}", task.name));
                    app.print_output.push(format!("[LOG] Sculpt structure (blob_id): {}", task.sculpt_structure));
                    app.set_message(crate::app::MessageType::Info, format!("Processing print job: {}", task.name));
                }
                
                let (rpc, eureka_pkg) = {
                    let g = app_clone.lock().await;
                    (
                        g.network_state.get_current_rpc().to_string(),
                        g.network_state
                            .get_current_package_ids()
                            .eureka_package_id
                            .to_string(),
                    )
                };

                let printer_id = {
                    let g = app_clone.lock().await;
                    g.printer_id.clone()
                };

                let mut resolve_logs = Vec::new();
                let (walrus_blob_id, seal_effective) =
                    match walrus_blob_and_seal_for_online_task(
                        &task,
                        &rpc,
                        &eureka_pkg,
                        &printer_id,
                        &mut resolve_logs,
                    )
                    .await
                    {
                        Ok(x) => x,
                        Err(e) => {
                            let mut app = app_clone.lock().await;
                            app.print_output.append(&mut resolve_logs);
                            app.set_message(
                                crate::app::MessageType::Error,
                                format!("Could not resolve model blob from chain: {}", e),
                            );
                            return;
                        }
                    };

                let seal_for_download = seal_effective.as_deref();

                let printer_for_seal: Option<(String, String)> = if seal_for_download.is_some() {
                    if printer_id == "No Printer ID" {
                        let mut app = app_clone.lock().await;
                        app.print_output.append(&mut resolve_logs);
                        app.set_message(
                            crate::app::MessageType::Error,
                            "This task uses an encrypted model; a connected printer is required.".to_string(),
                        );
                        return;
                    }
                    let wallet_address = { let g = app_clone.lock().await; g.wallet.address };
                    let cap_id = match app_clone
                        .lock()
                        .await
                        .wallet
                        .get_printer_cap_id(wallet_address)
                        .await
                    {
                        Ok(c) => c,
                        Err(e) => {
                            let mut app = app_clone.lock().await;
                            app.print_output.append(&mut resolve_logs);
                            app.set_message(
                                crate::app::MessageType::Error,
                                format!("Could not load PrinterCap: {}", e),
                            );
                            return;
                        }
                    };
                    Some((printer_id.clone(), cap_id))
                } else {
                    None
                };

                match download_model_isolated(
                    &walrus_blob_id,
                    seal_for_download,
                    &rpc,
                    &eureka_pkg,
                    printer_for_seal,
                )
                .await
                {
                    Ok(mut lines) => {
                        let mut app = app_clone.lock().await;
                        app.print_output.append(&mut resolve_logs);
                        app.print_output.append(&mut lines);
                        app.set_message(
                            crate::app::MessageType::Success,
                            "3D model downloaded successfully".to_string(),
                        );
                    }
                    Err(e) => {
                        let mut app = app_clone.lock().await;
                        app.print_output.append(&mut resolve_logs);
                        app.set_message(
                            crate::app::MessageType::Error,
                            format!("Failed to download task model: {}", e),
                        );
                        return;
                    }
                }

                if !download_only {
                    {
                        let mut app = app_clone.lock().await;
                        app.print_output.push("[LOG] Preparing to run print script for task".to_string());
                        app.print_status = crate::app::PrintStatus::Printing;
                    }
                    
                    let print_result = App::run_print_script(Arc::clone(&app_clone)).await;
                    
                    let mut app = app_clone.lock().await;
                    match print_result {
                        Ok(_) => {
                            app.print_output.push("[LOG] Task print script executed successfully".to_string());
                            app.set_message(crate::app::MessageType::Success, "Print job started successfully!".to_string());
                        },
                        Err(error_msg) => {
                            app.print_output.push(format!("[LOG] Task print script failed: {}", error_msg));
                            app.set_message(crate::app::MessageType::Error, format!("Failed to start print job: {}", error_msg));
                            app.print_status = crate::app::PrintStatus::Idle;
                        }
                    }
                }
            } else {
                let mut app = app_clone.lock().await;
                app.set_message(
                    crate::app::MessageType::Info, 
                    "No active print job found. Please wait for new tasks.".to_string()
                );
            }
        });
        Ok(())
    }

    /// Handle mock print with PrintJob-based decryption (T key handler)
    pub async fn handle_mock_print_with_printjob(app: Arc<Mutex<App>>) -> Result<()> {
        let app_clone = Arc::clone(&app);
        tokio::spawn(async move {
            // Get active print job
            let active_task = {
                let app_guard = app_clone.lock().await;
                app_guard.tasks.iter()
                    .find(|t| matches!(t.status, crate::app::print_job::TaskStatus::Active))
                    .cloned()
            };

            if let Some(task) = active_task {
                {
                    let mut app = app_clone.lock().await;
                    app.print_output.push(format!("=== MOCK PRINT MODE: {} ===", task.name));
                    app.print_output.push("[MOCK] Starting PrintJob-based decryption...".to_string());
                    app.set_message(crate::app::MessageType::Info, format!("Mock printing: {}", task.name));
                }

                // Get printer information from app state
                let (printer_id_str, wallet_address) = {
                    let app_guard = app_clone.lock().await;
                    (app_guard.printer_id.clone(), app_guard.wallet.address)
                };

                if printer_id_str.as_str() == "No Printer ID" {
                    let mut app = app_clone.lock().await;
                    app.set_message(crate::app::MessageType::Error, "Printer ID not available".to_string());
                    return;
                }

                // Get printer cap ID from wallet
                let printer_cap_id_str = {
                    let app_guard = app_clone.lock().await;
                    match app_guard.wallet.get_printer_cap_id(wallet_address).await {
                        Ok(cap_id) => cap_id,
                        Err(e) => {
                            drop(app_guard);
                            let mut app = app_clone.lock().await;
                            app.set_message(crate::app::MessageType::Error, format!("Failed to get PrinterCap ID: {}", e));
                            return;
                        }
                    }
                };

                // Parse printer IDs
                let printer_id = match seal_sdk_rs::native_sui_sdk::sui_types::base_types::ObjectID::from_hex_literal(&printer_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        let mut app = app_clone.lock().await;
                        app.set_message(crate::app::MessageType::Error, format!("Invalid printer ID: {}", e));
                        return;
                    }
                };

                let printer_cap_id = match seal_sdk_rs::native_sui_sdk::sui_types::base_types::ObjectID::from_hex_literal(&printer_cap_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        let mut app = app_clone.lock().await;
                        app.set_message(crate::app::MessageType::Error, format!("Invalid printer cap ID: {}", e));
                        return;
                    }
                };

                let (rpc, eureka_pkg) = {
                    let app_guard = app_clone.lock().await;
                    (
                        app_guard.network_state.get_current_rpc().to_string(),
                        app_guard
                            .network_state
                            .get_current_package_ids()
                            .eureka_package_id
                            .to_string(),
                    )
                };

                // Create PrintJob decryptor and perform decryption
                let decryption_result = match PrintJobDecryptor::new(rpc, &eureka_pkg).await {
                    Ok(decryptor) => {
                        {
                            let mut app = app_clone.lock().await;
                            app.print_output.push("[MOCK] PrintJob decryptor initialized".to_string());
                            app.print_output.push("[MOCK] Starting decryption with PrintJob authorization...".to_string());
                        }

                        decryptor.decrypt_printjob_sculpt(printer_id, printer_cap_id).await
                    },
                    Err(e) => {
                        let mut app = app_clone.lock().await;
                        app.set_message(crate::app::MessageType::Error, format!("Failed to create decryptor: {}", e));
                        return;
                    }
                };

                match decryption_result {
                    Ok(plaintext) => {
                        {
                            let mut app = app_clone.lock().await;
                            app.print_output.push("[MOCK] ✅ PrintJob-based decryption successful!".to_string());
                            
                            let format = if plaintext.starts_with(b"solid") {
                                "ASCII STL"
                            } else if plaintext.len() > 84 {
                                "Binary STL"
                            } else {
                                "Unknown"
                            };
                            
                            app.print_output.push(format!("[MOCK] Decrypted STL: {} ({} bytes)", format, plaintext.len()));
                            app.print_output.push("[MOCK] Starting mock print process...".to_string());
                        }

                        // Save decrypted file for mock printing and testing
                        // Save to mock_print.stl for mock printing
                        let mock_stl_path = crate_root().join("mock_print.stl");
                        if let Err(e) = std::fs::write(&mock_stl_path, &plaintext) {
                            let mut app = app_clone.lock().await;
                            app.set_message(crate::app::MessageType::Error, format!("Failed to save decrypted STL: {}", e));
                            return;
                        }

                        {
                            let mut app = app_clone.lock().await;
                            app.print_output.push(format!("[MOCK] Saved decrypted STL to: {}", mock_stl_path.display()));
                            app.print_output.push("[MOCK] Starting slicing test...".to_string());
                        }

                        // Run slicing test (optional - won't fail if PrusaSlicer not available)
                        let slice_result = App::run_slice_test(Arc::clone(&app_clone)).await;
                        match slice_result {
                            Ok(_) => {
                                let mut app = app_clone.lock().await;
                                app.print_output.push("[MOCK] ✅ Slicing test completed successfully".to_string());
                            },
                            Err(e) => {
                                let mut app = app_clone.lock().await;
                                app.print_output.push(format!("[MOCK] ⚠️ Slicing test failed (optional): {}", e));
                                // Don't return here - continue with mock printing even if slicing fails
                            }
                        }

                        // Run mock print script (5 seconds, success result)
                        let mock_result = run_mock_print_script(
                            Arc::clone(&app_clone),
                            MockPrintScriptResult::Success,
                            5, // 5 seconds
                            true // generate logs
                        ).await;

                        match mock_result {
                            Ok(_) => {
                                {
                                    let mut app = app_clone.lock().await;
                                    app.print_output.push("[MOCK] ✅ Mock print completed successfully!".to_string());
                                    app.print_output.push(
                                        "[MOCK] Submitting start_print_job (required before transfer_completed)…"
                                            .to_string(),
                                    );
                                    app.set_message(
                                        crate::app::MessageType::Info,
                                        "Mock print done; submitting on-chain start_print_job…"
                                            .to_string(),
                                    );
                                }

                                if let Err(e) = crate::app::printer::blockchain::run_start_print_job_for_active_task(
                                    Arc::clone(&app_clone),
                                    &task,
                                )
                                .await
                                {
                                    let mut app = app_clone.lock().await;
                                    app.print_output.push(format!(
                                        "[MOCK] ⚠️ Cannot complete on-chain without start: {}",
                                        e
                                    ));
                                    app.set_message(
                                        crate::app::MessageType::Error,
                                        format!("Mock print ok, but start_print_job failed: {}", e),
                                    );
                                    return;
                                }

                                {
                                    let mut app = app_clone.lock().await;
                                    app.print_output.push("[MOCK] Marking PrintJob as completed on blockchain...".to_string());
                                }

                                // `transfer_completed_print_job` requires `start_time > 0` on the PrintJob.
                                let completion_result = crate::app::printer::blockchain::run_transfer_completed_print_job(
                                    Arc::clone(&app_clone),
                                )
                                .await;

                                match completion_result {
                                    Ok(_) => {
                                        let mut app = app_clone.lock().await;
                                        app.print_output.push("[MOCK] ✅ PrintJob marked as completed on blockchain!".to_string());
                                        // Success banner comes from `run_transfer_completed_print_job` (tx + task refresh).
                                    },
                                    Err(e) => {
                                        let mut app = app_clone.lock().await;
                                        app.print_output.push(format!("[MOCK] ⚠️ Print completed but failed to mark PrintJob as completed: {}", e));
                                        app.set_message(crate::app::MessageType::Success, "Mock print completed, but blockchain update failed".to_string());
                                    }
                                }
                            },
                            Err(e) => {
                                let mut app = app_clone.lock().await;
                                app.print_output.push(format!("[MOCK] ❌ Mock print failed: {}", e));
                                app.set_message(crate::app::MessageType::Error, format!("Mock print failed: {}", e));
                            }
                        }
                    },
                    Err(e) => {
                        let mut app = app_clone.lock().await;
                        app.print_output.push(format!("[MOCK] ❌ Decryption failed: {}", e));
                        app.set_message(crate::app::MessageType::Error, format!("PrintJob decryption failed: {}", e));
                        
                        // Add debug information
                        app.print_output.push("[MOCK] 🔍 Possible causes:".to_string());
                        app.print_output.push("[MOCK]   1. ENotPrinterOwner: Caller is not the printer owner".to_string());
                        app.print_output.push("[MOCK]   2. EInvalidPrinterCap: PrinterCap doesn't match this printer".to_string());
                        app.print_output.push("[MOCK]   3. EPrintJobNotFound: No PrintJob exists for this printer".to_string());
                        app.print_output.push("[MOCK]   4. EPrinterIdMismatch: PrintJob's printer_id mismatch".to_string());
                    }
                }
            } else {
                let mut app = app_clone.lock().await;
                app.set_message(
                    crate::app::MessageType::Info, 
                    "No active PrintJob found. Please wait for new tasks or create a PrintJob.".to_string()
                );
                app.print_output.push("[MOCK] ❌ No active PrintJob found for mock printing".to_string());
            }
        });
        Ok(())
    }
}
