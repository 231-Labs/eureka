use anyhow::Result;
use seal_sdk_rs::native_sui_sdk::client::seal_client::SealClient;
use seal_sdk_rs::native_sui_sdk::sui_sdk::SuiClientBuilder;
use seal_sdk_rs::session_key::SessionKey;
use seal_sdk_rs::native_sui_sdk::sui_sdk::wallet_context::WalletContext;
use seal_sdk_rs::generic_types::ObjectID;
use seal_sdk_rs::native_sui_sdk::sui_types::Identifier;
use seal_sdk_rs::native_sui_sdk::sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use seal_sdk_rs::native_sui_sdk::sui_types::transaction::ProgrammableTransaction;
use std::str::FromStr;
use std::path::PathBuf;

/// Seal decryptor
pub struct SealDecryptor {
    seal_client: SealClient,
    wallet_path: PathBuf,
    #[allow(dead_code)]
    rpc_url: String,
}

impl SealDecryptor {
    /// create new SealDecryptor instance
    pub async fn new(rpc_url: String, wallet_config_path: PathBuf) -> Result<Self> {
        // initialize Sui client
        let sui_client = SuiClientBuilder::default()
            .build(&rpc_url)
            .await?;
        
        let seal_client = SealClient::new(sui_client);

        Ok(Self {
            seal_client,
            wallet_path: wallet_config_path,
            rpc_url,
        })
    }

    /// decrypt STL file
    pub async fn decrypt_stl(
        &self,
        encrypted_data: Vec<u8>,
        package_id: &str,
        resource_id: &str,
    ) -> Result<Vec<u8>> {
        // parse package_id
        let pkg_id: ObjectID = package_id.parse()
            .map_err(|e| anyhow::anyhow!("Invalid package_id: {}", e))?;

        // create wallet context
        let mut wallet = WalletContext::new(self.wallet_path.as_ref())?;

        // create session key (TTL 5 minutes)
        let session_key = SessionKey::new(pkg_id, 5, &mut wallet).await
            .map_err(|e| anyhow::anyhow!("Failed to create session key: {}", e))?;

        // create approval transaction
        let approval_tx = self.create_approval_transaction(pkg_id, resource_id)?;

        // decrypt data
        let decrypted = self.seal_client
            .decrypt_object_bytes(
                &encrypted_data,
                approval_tx,
                &session_key,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        Ok(decrypted)
    }

    /// Create Seal approval transaction
    fn create_approval_transaction(
        &self,
        package_id: ObjectID,
        resource_id: &str,
    ) -> Result<ProgrammableTransaction> {
        let mut builder = ProgrammableTransactionBuilder::new();
        
        // Convert resource_id to bytes
        let id_arg = builder.pure(resource_id.as_bytes().to_vec())
            .map_err(|e| anyhow::anyhow!("Failed to create ID argument: {}", e))?;
        
        // Call seal_approve function
        builder.programmable_move_call(
            package_id.into(),
            Identifier::from_str("sculpt")
                .map_err(|e| anyhow::anyhow!("Invalid module name: {}", e))?,
            Identifier::from_str("seal_approve_printer")
                .map_err(|e| anyhow::anyhow!("Invalid function name: {}", e))?,
            vec![],
            vec![id_arg],
        );

        Ok(builder.finish())
    }

    /// download and decrypt STL file from Walrus
    #[allow(dead_code)]
    pub async fn download_and_decrypt(
        &self,
        blob_id: &str,
        package_id: &str,
        resource_id: &str,
        output_path: PathBuf,
    ) -> Result<()> {
        let encrypted_data = self.download_from_walrus(blob_id).await?;

        // decrypt
        let decrypted_data = self.decrypt_stl(encrypted_data, package_id, resource_id).await?;

        // write to file
        tokio::fs::write(&output_path, decrypted_data).await
            .map_err(|e| anyhow::anyhow!("Failed to write decrypted file: {}", e))?;

        Ok(())
    }

    /// download file from Walrus
    #[allow(dead_code)]
    async fn download_from_walrus(&self, blob_id: &str) -> Result<Vec<u8>> {
        let url = format!(
            "https://aggregator.walrus-testnet.walrus.space/v1/{}",
            blob_id
        );

        let response = reqwest::get(&url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to download from Walrus: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Walrus download failed with status: {}",
                response.status()
            ));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response bytes: {}", e))?;

        Ok(bytes.to_vec())
    }

    /// check if file is encrypted (simple heuristic check)
    pub fn is_file_encrypted(data: &[u8]) -> bool {

        if data.len() < 5 {
            return true; // too small, probably encrypted
        }

        let header = String::from_utf8_lossy(&data[..5]);
        if header.starts_with("solid") {
            return false; // unencrypted ASCII STL
        }

        if data.len() > 84 {
            let triangle_count = u32::from_le_bytes([data[80], data[81], data[82], data[83]]);
            if triangle_count > 0 && triangle_count < 1_000_000 {
                return false; // probably unencrypted binary STL
            }
        }
        
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_file_encrypted_ascii_stl() {
        let stl_data = b"solid cube\n  facet normal 0 0 1\n";
        assert!(!SealDecryptor::is_file_encrypted(stl_data));
    }

    #[test]
    fn test_is_file_encrypted_encrypted_data() {
        let encrypted_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00]; // 隨機數據
        assert!(SealDecryptor::is_file_encrypted(&encrypted_data));
    }
}

