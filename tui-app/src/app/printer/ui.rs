use crate::app::core::App;
use crate::app::{MessageType, RegistrationStatus};
use anyhow::Result;
use sui_sdk::types::base_types::ObjectID;
use std::sync::Arc;

impl App {
    pub fn start_toggle_confirm(&mut self) {
        self.is_confirming = true;
    }

    pub async fn confirm_toggle(&mut self) -> Result<()> {
        // track original state
        let original_state = self.is_online;
        
        // first toggle state
        self.is_confirming = false;

        // try to update printer status
        if self.printer_id != "No Printer ID" {
            self.set_message(MessageType::Info, "Sending status update to blockchain...".to_string());
            
            let builder = crate::transactions::TransactionBuilder::new(
                Arc::clone(&self.sui_client),
                ObjectID::from(self.wallet.get_active_address().await?),
                self.network_state.clone()
            ).await;
            
            // get printer info and printer cap
            let address = self.wallet.get_active_address().await?;
            
            match self.wallet.get_printer_info(address).await {
                Ok(info) => {
                    match self.wallet.get_printer_cap_id(address).await {
                        Ok(cap_id) => {
                            
                            let printer_cap_id = ObjectID::from_hex_literal(&cap_id)?;
                            let printer_object_id = ObjectID::from_hex_literal(&info.id)?;
                            
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
                                    
                                    // If switched to online mode, immediately get print tasks
                                    if self.is_online {
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
            
            // if online, update print tasks
            if self.is_online {
                if let Err(e) = self.update_print_tasks().await {
                    self.set_message(MessageType::Error, format!("Failed to get print tasks: {}", e));
                }
            }
        }

        // if offline, update sculpt items
        if !self.is_online {
            match self.wallet.get_user_sculpt(self.wallet.get_active_address().await?).await {
                Ok(items) => {
                    self.sculpt_items = items;
                    // reset selection state
                    if !self.sculpt_items.is_empty() {
                        self.sculpt_state.select(Some(0));
                    }
                }
                Err(e) => {
                    self.set_message(MessageType::Error, format!("Failed to load 3D models: {}", e));
                }
            }
        }
        
        Ok(())
    }

    pub fn cancel_toggle(&mut self) {
        self.is_confirming = false;
    }

    // printer registration
    pub async fn handle_printer_registration_input(&mut self, input: char) -> Result<()> {
        match input {
            '\n' => {
                if !self.printer_alias.is_empty() && self.registration_status == RegistrationStatus::Inputting {
                    self.registration_status = RegistrationStatus::Submitting;
                    self.printer_registration_message = "Sending transaction to network...\nPlease wait...".to_string();
                    
                    let builder = crate::transactions::TransactionBuilder::new(
                        Arc::clone(&self.sui_client),
                        ObjectID::from(self.wallet.get_active_address().await?),
                        self.network_state.clone()
                    ).await;

                    self.printer_registration_message = "Transaction sent. Waiting for confirmation...\nThis may take a few seconds...".to_string();

                    match builder.register_printer(
                        self.network_state.get_current_package_ids().eureka_printer_registry_id.parse()?,
                        &self.printer_alias
                    ).await {
                        Ok(tx_digest) => {
                            self.registration_status = RegistrationStatus::Success(tx_digest.clone());
                            self.printer_registration_message = format!(
                                "Registration Successful!\n\
                                 Printer Name: {}\n\
                                 Transaction ID: {}\n\n\
                                 Press ENTER to continue...",
                                self.printer_alias,
                                tx_digest
                            );
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Registration failed: {}", e));
                            self.registration_status = RegistrationStatus::Failed(e.to_string());
                            self.printer_registration_message = "Registration failed. Press ESC to exit, or try registering again...".to_string();
                        }
                    };
                } else if matches!(self.registration_status, RegistrationStatus::Success(_)) {
                    // immediately exit registration page, let UI continue refreshing
                    self.is_registering_printer = false;
                    
                    // update application state in next loop
                    self.update_basic_info().await?;
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