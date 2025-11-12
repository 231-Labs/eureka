#[derive(Debug, Clone)]
pub struct SculptItem {
    pub alias: String,
    pub blob_id: String,
    pub printed_count: u64,
    pub id: String,
    #[allow(dead_code)]
    pub is_encrypted: bool,
    pub seal_resource_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PrinterInfo {
    pub id: String,
    pub pool_balance: u128,
} 