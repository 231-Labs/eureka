#[derive(Debug, Clone)]
pub struct SculptItem {
    pub alias: String,
    pub blob_id: String,
    pub printed_count: u64,
    pub id: String,
    /// When listed from a Kiosk, set so print-job creation uses `create_print_job_from_kiosk_*`.
    pub source_kiosk_id: Option<String>,
    #[allow(dead_code)]
    pub is_encrypted: bool,
    pub seal_resource_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PrinterInfo {
    pub id: String,
    pub pool_balance: u128,
    /// Package id parsed from on-chain `0x…::eureka::Printer` (falls back to network constants when empty).
    pub eureka_package_id: String,
} 