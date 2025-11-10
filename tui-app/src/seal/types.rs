use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Seal 加密資源的元數據
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealResourceMetadata {
    pub package_id: String,
    pub resource_id: String,
    pub is_encrypted: bool,
}

impl SealResourceMetadata {
    pub fn new(package_id: String, resource_id: String) -> Self {
        Self {
            package_id,
            resource_id,
            is_encrypted: true,
        }
    }

    /// 從 resource_id 字符串解析 (格式: "packageId:id")
    pub fn from_resource_id_string(resource_id_str: &str) -> Result<Self> {
        let parts: Vec<&str> = resource_id_str.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid resource ID format. Expected 'packageId:id'"));
        }

        Ok(Self {
            package_id: parts[0].to_string(),
            resource_id: parts[1].to_string(),
            is_encrypted: true,
        })
    }
}

/// Sculpt 擴展，包含 Seal 加密信息
#[derive(Debug, Clone)]
pub struct SculptWithSeal {
    pub alias: String,
    pub blob_id: String,
    pub printed_count: u64,
    pub id: String,
    pub seal_metadata: Option<SealResourceMetadata>,
}

