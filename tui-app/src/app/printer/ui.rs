use crate::app::core::App;
use crate::app::{MessageType, RegistrationStatus};
use anyhow::Result;
use std::sync::Arc;
use sui_sdk_types::Address;
use tokio::sync::Mutex;

impl App {
    pub fn start_toggle_confirm(&mut self) {
        self.is_confirming = true;
    }

    pub fn confirm_toggle_immediate(&mut self) {
        // Close the confirmation dialog and mark that we're switching modes
        // Keep the current display mode and animation running while waiting for transaction
        self.is_confirming = false;
        self.is_toggling_mode = true;  // Mark that we're in the process of toggling
    }

    pub async fn confirm_toggle(&mut self) -> Result<()> {
        // track original state
        let original_state = self.is_online;

        // try to update printer status
        if self.printer_id != "No Printer ID" {
            self.set_message(MessageType::Info, "Sending status update to blockchain...".to_string());
            
            let builder = crate::transactions::TransactionBuilder::new(
                Arc::clone(&self.sui_rpc),
                (*self.tx_signer).clone(),
                self.wallet.address,
                self.network_state.clone(),
            );
            
            // get printer info and printer cap
            let address = self.wallet.get_active_address().await?;
            
            match self.wallet.get_printer_info(address).await {
                Ok(info) => {
                    match self.wallet.get_printer_cap_id(address).await {
                        Ok(cap_id) => {
                            
                            let printer_cap_id: Address = cap_id.parse().map_err(|e| anyhow::anyhow!("cap id: {}", e))?;
                            let printer_object_id: Address = info.id.parse().map_err(|e| anyhow::anyhow!("printer id: {}", e))?;
                            
                            match builder.update_printer_status(printer_cap_id, printer_object_id).await {
                                Ok(tx_id) => {
                                    self.is_online = !original_state;
                                    self.set_message(
                                        MessageType::Success,
                                        format!("Printer status: {} (Digest: {})",
                                            if self.is_online { "ONLINE" } else { "OFFLINE" },
                                            tx_id
                                        )
                                    );
                                    
                                    // If switched to online mode, reset loading flag and get print tasks
                                    if self.is_online {
                                        self.is_loading_sculpts = false;  // Stop showing loading state
                                        if let Err(e) = self.update_print_tasks().await {
                                            self.set_message(MessageType::Error, format!("Failed to get print tasks: {}", e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    self.set_message(MessageType::Error, format!("Failed to update printer status: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            self.set_message(MessageType::Error, format!("Failed to get PrinterCap ID: {}", e));
                            return Ok(());
                        }
                    }
                }
                Err(e) => {
                    self.set_message(MessageType::Error, format!("Failed to get printer info: {}", e));
                    return Ok(());
                }
            }
        } else {
            // if no printer, directly update UI state
            self.is_online = !original_state;
            
            // if online, reset loading flag and update print tasks
            if self.is_online {
                self.is_loading_sculpts = false;  // Stop showing loading state
                if let Err(e) = self.update_print_tasks().await {
                    self.set_message(MessageType::Error, format!("Failed to get print tasks: {}", e));
                }
            }
        }

        // if offline, start loading sculpt items in background
        if !self.is_online {
            // Clear existing sculpts to trigger loading
            self.sculpt_items.clear();
            self.is_loading_sculpts = true;
        }
        
        // Mark toggle as complete
        self.is_toggling_mode = false;
        
        Ok(())
    }

    pub fn cancel_toggle(&mut self) {
        self.is_confirming = false;
    }

    // printer registration
    // This method should return quickly to avoid blocking UI event loop
    // Long-running operations are handled in main.rs via spawn
    pub async fn handle_printer_registration_input(&mut self, input: char) -> Result<()> {
        // If registration was successful, any key should exit
        if matches!(self.registration_status, RegistrationStatus::Success(_)) {
            self.is_registering_printer = false;
            self.set_message(MessageType::Success, "Printer registered successfully! Loading...".to_string());
            
            return Ok(());
        }
        
        match input {
            '\n' => {
                if !self.printer_alias.is_empty() && self.registration_status == RegistrationStatus::Inputting {
                    // Just set status to submitting and return immediately
                    // The actual registration will be handled in main.rs via spawn
                    self.registration_status = RegistrationStatus::Submitting;
                    self.printer_registration_message = "Sending transaction to network...\nPlease wait...\n(This may take a few seconds)".to_string();
                    // Return immediately - registration will be handled in background
                    return Ok(());
                }
            }
            '\x08' | '\x7f' => {
                if self.registration_status == RegistrationStatus::Inputting {
                    self.printer_alias.pop();
                }
            }
            c if c.is_ascii() && !c.is_control() => {
                if self.registration_status == RegistrationStatus::Inputting && self.printer_alias.len() < 30 {
                    self.printer_alias.push(c);
                }
            }
            _ => {}
        }
        Ok(())
    }
}

// Start printer registration in background
// This should be called from main.rs after handle_printer_registration_input returns
// This function spawns a background task and returns immediately
pub fn start_printer_registration_background(
    app: Arc<Mutex<crate::app::App>>,
) {
        // Spawn background task to avoid blocking UI event loop
        tokio::spawn(async move {
            // Clone necessary data
            let (sui_rpc, tx_signer, network_state, printer_alias, registry_id, address) = {
                let app_guard = app.lock().await;
                if app_guard.registration_status != RegistrationStatus::Submitting {
                    return; // Not in submitting state, skip
                }
                (
                    Arc::clone(&app_guard.sui_rpc),
                    (*app_guard.tx_signer).clone(),
                    app_guard.network_state.clone(),
                    app_guard.printer_alias.clone(),
                    app_guard.network_state.get_current_package_ids().eureka_printer_registry_id,
                    app_guard.wallet.address,
                )
            };
            
            let app_clone = Arc::clone(&app);
            
            // Update message
            {
                let mut app_guard = app_clone.lock().await;
                app_guard.printer_registration_message = "Transaction sent. Waiting for confirmation...\nThis may take a few seconds...\n(Please wait, do not close the terminal)".to_string();
            }
            
            // Create transaction builder
            let registry: Address = match registry_id.parse() {
                Ok(a) => a,
                Err(e) => {
                    let mut app_guard = app_clone.lock().await;
                    app_guard.error_message = Some(format!("Invalid registry id: {}", e));
                    app_guard.registration_status = RegistrationStatus::Failed(e.to_string());
                    return;
                }
            };

            let builder = crate::transactions::TransactionBuilder::new(
                sui_rpc,
                tx_signer,
                address,
                network_state.clone(),
            );

            match builder.register_printer(registry, &printer_alias).await {
                Ok(tx_digest) => {
                    let mut app_guard = app_clone.lock().await;
                    app_guard.registration_status = RegistrationStatus::Success(tx_digest.clone());
                    app_guard.printer_registration_message = format!(
                        "Registration Successful!\n\
                         Printer Name: {}\n\
                         Transaction ID: {}\n\n\
                         Press ANY KEY to continue...",
                        printer_alias,
                        tx_digest
                    );
                }
                Err(e) => {
                    let mut app_guard = app_clone.lock().await;
                    app_guard.error_message = Some(format!("Registration failed: {}", e));
                    app_guard.registration_status = RegistrationStatus::Failed(e.to_string());
                    app_guard.printer_registration_message = "Registration failed. Press ESC to exit, or try registering again...".to_string();
                }
            }
        });
} 