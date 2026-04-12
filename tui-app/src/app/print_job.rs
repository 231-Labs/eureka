use std::time::{SystemTime, UNIX_EPOCH};
use crate::utils::format_sui_balance;

#[derive(Clone, Debug)]
pub enum TaskStatus {
    Active,
    Completed,
}

#[derive(Clone, Debug)]
pub struct PrintTask {
    pub id: String,
    pub name: String,
    pub sculpt_blob_id: String,
    pub sculpt_structure: String,
    /// Seal resource id (matches on-chain `PrintJob.seal_resource_id`); required to decrypt encrypted models.
    pub seal_resource_id: Option<String>,
    pub customer: String,
    pub paid_amount: u64,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub status: TaskStatus,
}

impl PrintTask {
    pub fn format_end_time(&self) -> String {
        if let Some(end_time) = self.end_time {
            if end_time > 0 {
                let dt = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(end_time);
                if let Ok(datetime) = dt.duration_since(UNIX_EPOCH) {
                    let total_seconds = datetime.as_secs();
                    let hours = (total_seconds / 3600) % 24;
                    let minutes = (total_seconds / 60) % 60;
                    let days = total_seconds / (24 * 3600);
                    
                    return format!("{:02}-{:02} {:02}:{:02}", 
                        days % 31 + 1,
                        days % 12 + 1, 
                        hours,
                        minutes);
                }
            }
        }
        "Pending".to_string()
    }

    pub fn format_elapsed_time(&self) -> String {
        let elapsed = if let Some(start) = self.start_time {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            now.saturating_sub(start)
        } else {
            0
        };
        
        let hours = elapsed / 3600;
        let minutes = (elapsed % 3600) / 60;
        let seconds = elapsed % 60;
        
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }

    pub fn format_paid_amount(&self) -> String {
        format_sui_balance(self.paid_amount as u128)
    }

    pub fn get_short_customer(&self) -> String {
        if self.customer.len() > 12 {
            format!("{}...{}", &self.customer[0..6], &self.customer[self.customer.len()-6..])
        } else {
            self.customer.clone()
        }
    }

    pub fn get_short_sculpt_id(&self) -> String {
        if self.sculpt_blob_id.len() > 12 {
            format!("{}...{}", &self.sculpt_blob_id[0..6], &self.sculpt_blob_id[self.sculpt_blob_id.len()-6..])
        } else {
            self.sculpt_blob_id.clone()
        }
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.status, TaskStatus::Completed)
    }
} 