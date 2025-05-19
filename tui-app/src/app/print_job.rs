use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub enum TaskStatus {
    Printing,
    Completed,
}

#[derive(Clone, Debug)]
pub struct PrintTask {
    pub id: String,
    pub name: String,
    pub sculpt_blob_id: String,
    pub sculpt_structure: String,
    pub customer: String,
    pub paid_amount: u64,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub status: TaskStatus,
}

impl PrintTask {
    // format end time to display string
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
                        days % 31 + 1,  // simple month and day display
                        days % 12 + 1, 
                        hours,
                        minutes);
                }
            }
        }
        "Pending".to_string()
    }

    // format elapsed time
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

    // format paid amount
    pub fn format_paid_amount(&self) -> String {
        format!("{:.2} SUI", self.paid_amount as f64 / 1_000_000_000.0)
    }

    // get short customer address display
    pub fn get_short_customer(&self) -> String {
        if self.customer.len() > 12 {
            format!("{}...{}", &self.customer[0..6], &self.customer[self.customer.len()-6..])
        } else {
            self.customer.clone()
        }
    }

    // get short sculpt id display
    pub fn get_short_sculpt_id(&self) -> String {
        if self.sculpt_blob_id.len() > 12 {
            format!("{}...{}", &self.sculpt_blob_id[0..6], &self.sculpt_blob_id[self.sculpt_blob_id.len()-6..]) // TODO: change to 1
        } else {
            self.sculpt_blob_id.clone()
        }
    }

    // TODO: calculate elapsed time
    // pub fn get_elapsed_time(&self) -> (u64, u64) {
    //     let current_time = SystemTime::now()
    //         .duration_since(UNIX_EPOCH)
    //         .unwrap_or_default()
    //         .as_secs();

    //     let elapsed_time = self.start_time
    //         .map(|start| current_time.saturating_sub(start))
    //         .unwrap_or(0);
        
    //     let elapsed_hours = elapsed_time / 3600;
    //     let elapsed_minutes = (elapsed_time % 3600) / 60;

    //     (elapsed_hours, elapsed_minutes)
    // }

    // // check if task is printing
    // pub fn is_printing(&self) -> bool {
    //     matches!(self.status, TaskStatus::Printing)
    // }

    // check if task is completed
    pub fn is_completed(&self) -> bool {
        matches!(self.status, TaskStatus::Completed)
    }

    // Create mock tasks for testing UI
    pub fn new_mock_tasks() -> Vec<PrintTask> {
        vec![
            PrintTask {
                id: "0x123456789abcdef123456789abcdef".to_string(),
                name: "Cool Vase".to_string(),
                sculpt_blob_id: "0xabcdef123456789abcdef123456789a".to_string(),
                sculpt_structure: "STL".to_string(),
                customer: "0xabc123def456abc123def456abc123de".to_string(),
                paid_amount: 1500000000, // 1.5 SUI
                start_time: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() - 3600),
                end_time: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() - 600),
                status: TaskStatus::Completed,
            },
            PrintTask {
                id: "0x987654321fedcba987654321fedcba".to_string(),
                name: "Cute Robot".to_string(),
                sculpt_blob_id: "0xfedcba987654321fedcba98765432".to_string(),
                sculpt_structure: "STL".to_string(),
                customer: "0xabc123def456abc123def456abc123de".to_string(),
                paid_amount: 2000000000, // 2.0 SUI
                start_time: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() - 7200),
                end_time: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() - 1800),
                status: TaskStatus::Completed,
            },
        ]
    }
} 