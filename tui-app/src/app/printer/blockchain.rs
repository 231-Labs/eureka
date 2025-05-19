use crate::app::core::App;
use crate::app::MessageType;
use anyhow::Result;
use std::sync::Arc;
use sui_sdk::types::base_types::ObjectID;

impl App {
    pub async fn test_start_print_job(&mut self) -> Result<(), String> {
        // 獲取當前選中的 sculpt item
        let selected_index = match self.sculpt_state.selected() {
            Some(index) => index,
            None => {
                return Err("No sculpt selected".to_string());
            }
        };
        
        if selected_index >= self.sculpt_items.len() {
            return Err("Invalid sculpt selection".to_string());
        }
        
        let selected_sculpt = &self.sculpt_items[selected_index];
        let address = match self.wallet.get_active_address().await {
            Ok(addr) => addr,
            Err(e) => {
                return Err(format!("Failed to get active address: {}", e));
            }
        };
        
        // get printer info and printer cap
        match self.wallet.get_printer_info(address).await {
            Ok(info) => {
                match self.wallet.get_printer_cap_id(address).await {
                    Ok(cap_id) => {
                        let builder = crate::transactions::TransactionBuilder::new(
                            Arc::clone(&self.sui_client),
                            ObjectID::from(address),
                            self.network_state.clone()
                        ).await;
                        
                        let printer_cap_id = match ObjectID::from_hex_literal(&cap_id) {
                            Ok(id) => id,
                            Err(e) => {
                                return Err(format!("Invalid printer cap ID: {}", e));
                            }
                        };
                        
                        let printer_object_id = match ObjectID::from_hex_literal(&info.id) {
                            Ok(id) => id,
                            Err(e) => {
                                return Err(format!("Invalid printer object ID: {}", e));
                            }
                        };
                        
                        let sculpt_id = match ObjectID::from_hex_literal(&selected_sculpt.id) {
                            Ok(id) => id,
                            Err(e) => {
                                return Err(format!("Invalid sculpt ID ({}): {}", selected_sculpt.id, e));
                            }
                        };
                        
                        // 發送交易並處理結果
                        self.set_message(MessageType::Info, "Starting print job, waiting for blockchain confirmation...".to_string());
                        
                        match builder.start_print_job(printer_cap_id, printer_object_id, sculpt_id).await {
                            Ok(tx_id) => {
                                self.set_message(
                                    MessageType::Success,
                                    format!("Print job submitted to blockchain (Tx: {})", tx_id)
                                );
                                Ok(())
                            }
                            Err(e) => {
                                let error_msg = e.to_string();
                                
                                // 根據錯誤類型提供用戶友好的錯誤信息
                                let user_friendly_error = if error_msg.contains("TransactionEffects") && error_msg.contains("status") {
                                    if error_msg.contains("dynamic_field") && error_msg.contains("borrow_child_object") {
                                        "Failed to find print job. Please make sure printer is properly registered.".to_string()
                                    } else if error_msg.contains("insufficient gas") {
                                        "Insufficient gas to execute transaction. Please add more SUI to your wallet.".to_string()
                                    } else {
                                        format!("Transaction failed on blockchain: {}", error_msg)
                                    }
                                } else {
                                    format!("Failed to start print job: {}", error_msg)
                                };
                                
                                self.set_message(MessageType::Error, user_friendly_error.clone());
                                Err(user_friendly_error)
                            }
                        }
                    }
                    Err(e) => {
                        Err(format!("Failed to get PrinterCap ID: {}", e))
                    }
                }
            }
            Err(e) => {
                Err(format!("Failed to get printer info: {}", e))
            }
        }
    }

    pub async fn test_create_print_job(&mut self) -> Result<(), String> {
        // 獲取當前選中的 sculpt item
        let selected_index = match self.sculpt_state.selected() {
            Some(index) => index,
            None => {
                return Err("No sculpt selected".to_string());
            }
        };
        
        if selected_index >= self.sculpt_items.len() {
            return Err("Invalid sculpt selection".to_string());
        }
        
        let selected_sculpt = &self.sculpt_items[selected_index];
        let address = match self.wallet.get_active_address().await {
            Ok(addr) => addr,
            Err(e) => {
                return Err(format!("Failed to get active address: {}", e));
            }
        };
        
        // get printer info
        match self.wallet.get_printer_info(address).await {
            Ok(info) => {
                let builder = crate::transactions::TransactionBuilder::new(
                    Arc::clone(&self.sui_client),
                    ObjectID::from(address),
                    self.network_state.clone()
                ).await;
                
                let printer_object_id = match ObjectID::from_hex_literal(&info.id) {
                    Ok(id) => id,
                    Err(e) => {
                        return Err(format!("Invalid printer object ID: {}", e));
                    }
                };
                
                let sculpt_id = match ObjectID::from_hex_literal(&selected_sculpt.id) {
                    Ok(id) => id,
                    Err(e) => {
                        return Err(format!("Invalid sculpt ID ({}): {}", selected_sculpt.id, e));
                    }
                };
                
                // 發送交易並處理結果
                self.set_message(MessageType::Info, "Creating print job, waiting for blockchain confirmation...".to_string());
                
                match builder.create_and_assign_print_job_free(printer_object_id, sculpt_id).await {
                    Ok(tx_id) => {
                        self.set_message(
                            MessageType::Success,
                            format!("Print job created successfully on blockchain (Tx: {})", tx_id)
                        );
                        Ok(())
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        
                        // 根據錯誤類型提供用戶友好的錯誤信息
                        let user_friendly_error = if error_msg.contains("TransactionEffects") && error_msg.contains("status") {
                            if error_msg.contains("EPrintJobExists") {
                                "A print job already exists for this printer.".to_string()
                            } else if error_msg.contains("insufficient gas") {
                                "Insufficient gas to execute transaction. Please add more SUI to your wallet.".to_string()
                            } else {
                                format!("Transaction failed on blockchain: {}", error_msg)
                            }
                        } else {
                            format!("Failed to create print job: {}", error_msg)
                        };
                        
                        self.set_message(MessageType::Error, user_friendly_error.clone());
                        Err(user_friendly_error)
                    }
                }
            }
            Err(e) => {
                Err(format!("Failed to get printer info: {}", e))
            }
        }
    }

    pub async fn test_complete_print_job(&mut self) -> Result<(), String> {
        let selected_index = match self.sculpt_state.selected() {
            Some(index) => index,
            None => {
                return Err("No sculpt selected".to_string());
            }
        };
        
        if selected_index >= self.sculpt_items.len() {
            return Err("Invalid sculpt selection".to_string());
        }
        
        let selected_sculpt = &self.sculpt_items[selected_index];
        let address = match self.wallet.get_active_address().await {
            Ok(addr) => addr,
            Err(e) => {
                return Err(format!("Failed to get active address: {}", e));
            }
        };
        
        // get printer info and printer cap
        match self.wallet.get_printer_info(address).await {
            Ok(info) => {
                match self.wallet.get_printer_cap_id(address).await {
                    Ok(cap_id) => {
                        let builder = crate::transactions::TransactionBuilder::new(
                            Arc::clone(&self.sui_client),
                            ObjectID::from(address),
                            self.network_state.clone()
                        ).await;
                        
                        let printer_cap_id = match ObjectID::from_hex_literal(&cap_id) {
                            Ok(id) => id,
                            Err(e) => {
                                return Err(format!("Invalid printer cap ID: {}", e));
                            }
                        };
                        
                        let printer_object_id = match ObjectID::from_hex_literal(&info.id) {
                            Ok(id) => id,
                            Err(e) => {
                                return Err(format!("Invalid printer object ID: {}", e));
                            }
                        };
                        
                        let sculpt_id = match ObjectID::from_hex_literal(&selected_sculpt.id) {
                            Ok(id) => id,
                            Err(e) => {
                                return Err(format!("Invalid sculpt ID ({}): {}", selected_sculpt.id, e));
                            }
                        };
                        
                        self.set_message(MessageType::Info, "Completing print job, waiting for blockchain confirmation...".to_string());
                        
                        match builder.complete_print_job(printer_cap_id, printer_object_id, sculpt_id).await {
                            Ok(tx_id) => {
                                self.set_message(
                                    MessageType::Success,
                                    format!("Print job completed successfully on blockchain (Tx: {})", tx_id)
                                );
                                Ok(())
                            }
                            Err(e) => {
                                let error_msg = e.to_string();
                                
                                // 根據錯誤類型提供用戶友好的錯誤信息
                                let user_friendly_error = if error_msg.contains("TransactionEffects") && error_msg.contains("status") {
                                    if error_msg.contains("EPrintJobNotStarted") {
                                        "Print job has not been started yet. Please start the print job first.".to_string()
                                    } else if error_msg.contains("EPrintJobCompleted") {
                                        "Print job has already been completed.".to_string()
                                    } else if error_msg.contains("ENotAuthorized") {
                                        "Not authorized to complete this print job.".to_string()
                                    } else if error_msg.contains("insufficient gas") {
                                        "Insufficient gas to execute transaction. Please add more SUI to your wallet.".to_string()
                                    } else {
                                        format!("Transaction failed on blockchain: {}", error_msg)
                                    }
                                } else {
                                    format!("Failed to complete print job: {}", error_msg)
                                };
                                
                                self.set_message(MessageType::Error, user_friendly_error.clone());
                                Err(user_friendly_error)
                            }
                        }
                    }
                    Err(e) => {
                        Err(format!("Failed to get PrinterCap ID: {}", e))
                    }
                }
            }
            Err(e) => {
                Err(format!("Failed to get printer info: {}", e))
            }
        }
    }
} 