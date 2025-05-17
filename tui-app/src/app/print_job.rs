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
    pub sculpt_id: String,
    pub sculpt_structure: String,
    pub customer: String,
    pub paid_amount: u64,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub status: TaskStatus,
}

impl PrintTask {
    pub fn new_mock_tasks() -> Vec<PrintTask> {
        vec![
            PrintTask {
                id: "task_001".to_string(),
                name: "Benchy 3D".to_string(),
                sculpt_id: "0x123...abc".to_string(),
                sculpt_structure: "Standard".to_string(),
                customer: "0x598...fbd".to_string(),
                paid_amount: 1_000_000_000, // 1 SUI
                start_time: Some(1709856000), // Unix timestamp
                end_time: Some(1709859600), // 完成時間設為開始時間 + 1 小時
                status: TaskStatus::Completed,
            },
            PrintTask {
                id: "task_002".to_string(),
                name: "Calibration Cube".to_string(),
                sculpt_id: "0x456...def".to_string(),
                sculpt_structure: "Basic".to_string(),
                customer: "0x598...fbd".to_string(),
                paid_amount: 500_000_000, // 0.5 SUI
                start_time: Some(1709852400),
                end_time: Some(1709856000),
                status: TaskStatus::Completed,
            },
        ]
    }

    // 格式化結束時間為顯示字符串
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
                        days % 31 + 1,  // 簡單的月日顯示
                        days % 12 + 1, 
                        hours,
                        minutes);
                }
            }
        }
        "Pending".to_string()
    }

    // 格式化已運行時間
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

    // 格式化支付金額
    pub fn format_paid_amount(&self) -> String {
        format!("{:.2} SUI", self.paid_amount as f64 / 1_000_000_000.0)
    }

    // 獲取簡短的客戶地址顯示
    pub fn get_short_customer(&self) -> String {
        if self.customer.len() > 12 {
            format!("{}...{}", &self.customer[0..6], &self.customer[self.customer.len()-6..])
        } else {
            self.customer.clone()
        }
    }

    // 獲取簡短的模型 ID 顯示
    pub fn get_short_sculpt_id(&self) -> String {
        if self.sculpt_id.len() > 12 {
            format!("{}...{}", &self.sculpt_id[0..6], &self.sculpt_id[self.sculpt_id.len()-6..])
        } else {
            self.sculpt_id.clone()
        }
    }

    // 計算任務已經過的時間
    pub fn get_elapsed_time(&self) -> (u64, u64) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let elapsed_time = self.start_time
            .map(|start| current_time.saturating_sub(start))
            .unwrap_or(0);
        
        let elapsed_hours = elapsed_time / 3600;
        let elapsed_minutes = (elapsed_time % 3600) / 60;

        (elapsed_hours, elapsed_minutes)
    }

    // 檢查任務是否正在進行中
    pub fn is_printing(&self) -> bool {
        matches!(self.status, TaskStatus::Printing)
    }

    // 檢查任務是否已完成
    pub fn is_completed(&self) -> bool {
        matches!(self.status, TaskStatus::Completed)
    }
} 