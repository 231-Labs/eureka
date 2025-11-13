use crate::app::core::App;
use crate::app::MessageType;
use anyhow::Result;
use std::sync::Arc;
use sui_sdk::types::base_types::ObjectID;

fn parse_blockchain_error(error_msg: &str, context: &str) -> String {
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

struct SelectedSculptContext {
    sculpt_id: String,
    printer_info_id: String,
    cap_id: String,
}

impl App {
    async fn get_selected_sculpt_context(&self) -> Result<SelectedSculptContext, String> {
        let selected_index = self.sculpt_state.selected()
            .ok_or_else(|| "No sculpt selected".to_string())?;
        
        if selected_index >= self.sculpt_items.len() {
            return Err("Invalid sculpt selection".to_string());
        }
        
        let selected_sculpt = &self.sculpt_items[selected_index];
        let address = self.wallet.get_active_address().await
            .map_err(|e| format!("Failed to get active address: {}", e))?;
        
        let info = self.wallet.get_printer_info(address).await
            .map_err(|e| format!("Failed to get printer info: {}", e))?;
        
        let cap_id = self.wallet.get_printer_cap_id(address).await
            .map_err(|e| format!("Failed to get PrinterCap ID: {}", e))?;
        
        Ok(SelectedSculptContext {
            sculpt_id: selected_sculpt.id.clone(),
            printer_info_id: info.id,
            cap_id,
        })
    }
    
    fn parse_object_id(id_str: &str, context: &str) -> Result<ObjectID, String> {
        ObjectID::from_hex_literal(id_str)
            .map_err(|e| format!("Invalid {} ({}): {}", context, id_str, e))
    }
    
    async fn create_transaction_builder(&self) -> Result<crate::transactions::TransactionBuilder, String> {
        let address = self.wallet.get_active_address().await
            .map_err(|e| format!("Failed to get active address: {}", e))?;
        
        Ok(crate::transactions::TransactionBuilder::new(
            Arc::clone(&self.sui_client),
            ObjectID::from(address),
            self.network_state.clone()
        ).await)
    }
    
    pub async fn test_start_print_job(&mut self) -> Result<(), String> {
        let ctx = self.get_selected_sculpt_context().await?;
        let builder = self.create_transaction_builder().await?;
        
        let printer_cap_id = Self::parse_object_id(&ctx.cap_id, "printer cap ID")?;
        let printer_object_id = Self::parse_object_id(&ctx.printer_info_id, "printer object ID")?;
        let sculpt_id = Self::parse_object_id(&ctx.sculpt_id, "sculpt ID")?;
        
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
                let user_friendly_error = parse_blockchain_error(&e.to_string(), "start print job");
                self.set_message(MessageType::Error, user_friendly_error.clone());
                Err(user_friendly_error)
            }
        }
    }

    pub async fn test_create_print_job(&mut self) -> Result<(), String> {
        let ctx = self.get_selected_sculpt_context().await?;
        let builder = self.create_transaction_builder().await?;
        
        let printer_object_id = Self::parse_object_id(&ctx.printer_info_id, "printer object ID")?;
        let sculpt_id = Self::parse_object_id(&ctx.sculpt_id, "sculpt ID")?;
        
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
                let user_friendly_error = parse_blockchain_error(&e.to_string(), "create print job");
                self.set_message(MessageType::Error, user_friendly_error.clone());
                Err(user_friendly_error)
            }
        }
    }

    pub async fn test_complete_print_job(&mut self) -> Result<(), String> {
        let ctx = self.get_selected_sculpt_context().await?;
        let builder = self.create_transaction_builder().await?;
        
        let printer_cap_id = Self::parse_object_id(&ctx.cap_id, "printer cap ID")?;
        let printer_object_id = Self::parse_object_id(&ctx.printer_info_id, "printer object ID")?;
        let sculpt_id = Self::parse_object_id(&ctx.sculpt_id, "sculpt ID")?;
        
        self.set_message(MessageType::Info, "Completing print job, waiting for blockchain confirmation...".to_string());
        
        match builder.complete_print_job(printer_cap_id, printer_object_id, sculpt_id).await {
            Ok(tx_id) => {
                self.set_message(
                    MessageType::Success,
                    format!("Print job completed successfully on blockchain (Tx: {})", tx_id)
                );
                
                self.tasks.clear();
                self.print_status = crate::app::PrintStatus::Idle;
                self.script_status = crate::app::ScriptStatus::Idle;
                
                if let Err(e) = self.update_print_tasks().await {
                    self.set_message(MessageType::Error, format!("Failed to update print tasks: {}", e));
                } else {
                    self.set_message(MessageType::Success, "Print job completed and tasks updated successfully".to_string());
                }
                
                Ok(())
            }
            Err(e) => {
                let user_friendly_error = parse_blockchain_error(&e.to_string(), "complete print job");
                self.set_message(MessageType::Error, user_friendly_error.clone());
                Err(user_friendly_error)
            }
        }
    }

    /// Complete print job using PrintJob task context instead of sculpt selection
    pub async fn test_complete_print_job_from_task(&mut self, _task: &crate::app::print_job::PrintTask) -> Result<(), String> {
        let builder = self.create_transaction_builder().await?;
        
        // Get printer info from current app state
        let address = self.wallet.get_active_address().await
            .map_err(|e| format!("Failed to get active address: {}", e))?;
        
        let info = self.wallet.get_printer_info(address).await
            .map_err(|e| format!("Failed to get printer info: {}", e))?;
        
        let cap_id = self.wallet.get_printer_cap_id(address).await
            .map_err(|e| format!("Failed to get PrinterCap ID: {}", e))?;
        
        // Parse required object IDs
        let printer_cap_id = Self::parse_object_id(&cap_id, "printer cap ID")?;
        let printer_object_id = Self::parse_object_id(&info.id, "printer object ID")?;
        
        self.set_message(MessageType::Info, "Transferring completed PrintJob to printer owner wallet...".to_string());
        
        // Use the simplified transfer_completed_print_job function
        // This removes the PrintJob from the printer and transfers it to the printer owner's wallet
        // allowing the printer to accept new jobs immediately
        match builder.transfer_completed_print_job(printer_cap_id, printer_object_id).await {
            Ok(tx_id) => {
                self.set_message(
                    MessageType::Success,
                    format!("PrintJob transferred to printer owner wallet successfully (Tx: {})", tx_id)
                );
                
                // Clear tasks and reset status
                self.tasks.clear();
                self.print_status = crate::app::PrintStatus::Idle;
                self.script_status = crate::app::ScriptStatus::Idle;
                
                // Update print tasks to refresh the list
                if let Err(e) = self.update_print_tasks().await {
                    self.set_message(MessageType::Error, format!("Failed to update print tasks: {}", e));
                } else {
                    self.set_message(MessageType::Success, "PrintJob transferred and printer is ready for next job".to_string());
                }
                
                Ok(())
            }
            Err(e) => {
                let user_friendly_error = parse_blockchain_error(&e.to_string(), "transfer completed PrintJob");
                self.set_message(MessageType::Error, user_friendly_error.clone());
                Err(user_friendly_error)
            }
        }
    }
}
