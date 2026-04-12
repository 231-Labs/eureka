use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Metadata for a Seal-encrypted resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealResourceMetadata {
    pub package_id: String,
    pub resource_id: String,
    pub is_encrypted: bool,
}

impl SealResourceMetadata {
    #[allow(dead_code)]
    pub fn new(package_id: String, resource_id: String) -> Self {
        Self {
            package_id,
            resource_id,
            is_encrypted: true,
        }
    }

    /// Parse from resource id string (format: "packageId:id")
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

/// Sculpt extension including Seal encryption info
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SculptWithSeal {
    pub alias: String,
    pub blob_id: String,
    pub printed_count: u64,
    pub id: String,
    pub seal_metadata: Option<SealResourceMetadata>,
}

