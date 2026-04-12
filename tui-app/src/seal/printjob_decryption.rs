use anyhow::Result;
use seal_sdk_rs::native_sui_sdk::client::seal_client::SealClient;
use seal_sdk_rs::session_key::SessionKey;
use seal_sdk_rs::native_sui_sdk::sui_sdk::SuiClientBuilder;
use seal_sdk_rs::native_sui_sdk::sui_sdk::wallet_context::WalletContext;
use seal_sdk_rs::native_sui_sdk::sui_types::Identifier;
use seal_sdk_rs::native_sui_sdk::sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use seal_sdk_rs::native_sui_sdk::sui_types::transaction::{ProgrammableTransaction, ObjectArg};
use seal_sdk_rs::native_sui_sdk::sui_sdk::rpc_types::{SuiObjectDataOptions, SuiMoveValue};
use seal_sdk_rs::generic_types::ObjectID as SealObjectID;
use seal_sdk_rs::native_sui_sdk::sui_types::base_types::ObjectID as SuiObjectID;
use seal_sdk_rs::native_sui_sdk::sui_types::object::Owner;
use std::str::FromStr;
use std::path::Path;
use std::collections::BTreeMap;
use bcs;
/// PrintJob-based decryption aligned with on-chain `eureka::seal_approve` (requires PrintJob); SessionKey namespace is the **Eureka package**.
pub struct PrintJobDecryptor {
    eureka_package_id: SealObjectID,
    sui_client: seal_sdk_rs::native_sui_sdk::sui_sdk::SuiClient,
    seal_client: SealClient,
}

impl PrintJobDecryptor {
    /// `rpc_url`: same network as the TUI (Seal SDK uses JSON-RPC). `eureka_package_id`: Eureka package for that network.
    pub async fn new(rpc_url: String, eureka_package_id: &str) -> Result<Self> {
        if eureka_package_id.is_empty() {
            return Err(anyhow::anyhow!(
                "Eureka package ID is not configured for this network; PrintJob decryption is unavailable."
            ));
        }
        let eureka_package_id: SealObjectID = eureka_package_id
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid eureka package id: {}", e))?;

        let sui_client = SuiClientBuilder::default()
            .build(&rpc_url)
            .await?;

        let seal_client = SealClient::new(sui_client.clone());

        Ok(Self {
            eureka_package_id,
            sui_client,
            seal_client,
        })
    }

    pub async fn fetch_printer_shared_version(&self, printer_id: SuiObjectID) -> Result<u64> {
        let mut options = SuiObjectDataOptions::new();
        options.show_owner = true;
        let printer_response = self
            .sui_client
            .read_api()
            .get_object_with_options(printer_id, options)
            .await?;
        let printer_data = printer_response
            .data
            .ok_or_else(|| anyhow::anyhow!("Printer not found"))?;
        match printer_data.owner {
            Some(Owner::Shared { initial_shared_version }) => Ok(initial_shared_version.value()),
            _ => Err(anyhow::anyhow!("Printer is not a shared object")),
        }
    }

    /// Fetch sculpt_id from printer's PrintJob dynamic field
    pub async fn fetch_sculpt_id_from_printjob(
        &self,
        printer_id: SuiObjectID,
    ) -> Result<SuiObjectID> {
        // Get dynamic fields
        let dynamic_fields = self.sui_client
            .read_api()
            .get_dynamic_fields(printer_id, None, None)
            .await?;

        // Find print_job field (name.value is a vector<u8>)
        let print_job_field = dynamic_fields
            .data
            .iter()
            .find(|field| {
                // The field name is stored as bytes ("print_job" as vector<u8>)
                if let Some(name_bytes) = field.name.value.as_array() {
                    let bytes: Vec<u8> = name_bytes.iter()
                        .filter_map(|v| v.as_u64().map(|n| n as u8))
                        .collect();
                    bytes == b"print_job"
                } else {
                    false
                }
            })
            .ok_or_else(|| anyhow::anyhow!("No PrintJob found for this printer. Make sure to create a PrintJob first!"))?;

        // Fetch the PrintJob object
        let mut options = SuiObjectDataOptions::new();
        options.show_content = true;
        
        let print_job_obj = self.sui_client
            .read_api()
            .get_object_with_options(print_job_field.object_id, options)
            .await?;

        let print_job_data = print_job_obj.data
            .ok_or_else(|| anyhow::anyhow!("PrintJob data not found"))?;

        let content = print_job_data.content
            .ok_or_else(|| anyhow::anyhow!("PrintJob has no content"))?;

        let fields = match content {
            seal_sdk_rs::native_sui_sdk::sui_sdk::rpc_types::SuiParsedData::MoveObject(ref obj) => {
                match &obj.fields {
                    seal_sdk_rs::native_sui_sdk::sui_sdk::rpc_types::SuiMoveStruct::WithFields(f) => f,
                    _ => return Err(anyhow::anyhow!("PrintJob fields are not in WithFields format")),
                }
            }
            _ => return Err(anyhow::anyhow!("PrintJob is not a Move object")),
        };

        // Extract sculpt_id (ID type is usually represented as a String in Sui JSON)
        let sculpt_id_str = match fields.get("sculpt_id") {
            Some(SuiMoveValue::String(id)) => id.clone(),
            Some(SuiMoveValue::Address(addr)) => format!("0x{}", addr),
            Some(other) => {
                return Err(anyhow::anyhow!("sculpt_id has unexpected format: {:?}", other));
            }
            None => {
                return Err(anyhow::anyhow!("sculpt_id field not found in PrintJob"));
            }
        };

        SuiObjectID::from_hex_literal(&sculpt_id_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse sculpt_id '{}': {}", sculpt_id_str, e))
    }

    /// Fetch sculpt information and printer version from chain
    pub async fn fetch_sculpt_and_objects(
        &self,
        sculpt_id: SuiObjectID,
        printer_id: SuiObjectID,
    ) -> Result<(String, Option<String>, u64)> {
        let mut options = SuiObjectDataOptions::new();
        options.show_content = true;
        options.show_type = true;
        options.show_owner = true;

        // Fetch sculpt to get encrypted blob ID and seal_resource_id
        let object_response = self.sui_client
            .read_api()
            .get_object_with_options(sculpt_id, options.clone())
            .await?;

        let object_data = object_response.data
            .ok_or_else(|| anyhow::anyhow!("Sculpt object not found"))?;

        let content = object_data.content
            .ok_or_else(|| anyhow::anyhow!("Sculpt object has no content"))?;

        let fields = match content {
            seal_sdk_rs::native_sui_sdk::sui_sdk::rpc_types::SuiParsedData::MoveObject(ref obj) => {
                match &obj.fields {
                    seal_sdk_rs::native_sui_sdk::sui_sdk::rpc_types::SuiMoveStruct::WithFields(f) => f,
                    _ => return Err(anyhow::anyhow!("Sculpt fields are not in WithFields format")),
                }
            }
            _ => return Err(anyhow::anyhow!("Sculpt is not a Move object")),
        };

        // Extract structure (encrypted STL blob ID)
        let structure = self.extract_option_string_field(fields, "structure")
            .ok_or_else(|| anyhow::anyhow!("Sculpt has no structure field (STL blob ID)"))?;

        // Extract seal_resource_id
        let seal_resource_id = self.extract_option_string_field(fields, "seal_resource_id");

        // Fetch printer to get its shared version
        let printer_response = self.sui_client
            .read_api()
            .get_object_with_options(printer_id, options)
            .await?;
        let printer_data = printer_response.data
            .ok_or_else(|| anyhow::anyhow!("Printer not found"))?;
        let printer_version = match printer_data.owner {
            Some(Owner::Shared { initial_shared_version }) => initial_shared_version.value(),
            _ => return Err(anyhow::anyhow!("Printer is not a shared object")),
        };

        Ok((structure, seal_resource_id, printer_version))
    }

    /// Extract Option<String> field from Move struct fields
    fn extract_option_string_field(
        &self,
        fields: &BTreeMap<String, SuiMoveValue>,
        field_name: &str,
    ) -> Option<String> {
        match fields.get(field_name)? {
            SuiMoveValue::String(value) if !value.is_empty() => Some(value.clone()),
            SuiMoveValue::Option(opt) => {
                match opt.as_ref() {
                    Some(SuiMoveValue::String(value)) if !value.is_empty() => Some(value.clone()),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Download encrypted blob from Walrus (same aggregator base URL as TUI model download).
    pub async fn download_encrypted_data(&self, blob_id: &str) -> Result<Vec<u8>> {
        let base = crate::constants::AGGREGATOR_URL.trim_end_matches('/');
        let url = format!("{}/v1/blobs/{}", base, blob_id);
        let response = reqwest::get(&url).await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to download: HTTP {}", response.status()));
        }

        Ok(response.bytes().await?.to_vec())
    }

    /// Parse encrypted data as EncryptedObject
    pub fn parse_encrypted_object(&self, data: &[u8]) -> Result<seal_sdk_rs::crypto::EncryptedObject> {
        let encrypted: seal_sdk_rs::crypto::EncryptedObject = bcs::from_bytes(data)
            .map_err(|e| anyhow::anyhow!("BCS deserialization failed: {}", e))?;
        Ok(encrypted)
    }

    /// Decrypt sculpt using Seal SDK with PrintJob-based authorization
    pub async fn decrypt_sculpt(
        &self,
        seal_id: &str,
        encrypted: seal_sdk_rs::crypto::EncryptedObject,
        printer_id: SuiObjectID,
        printer_cap_id: SuiObjectID,
        printer_version: u64,
    ) -> Result<Vec<u8>> {
        // Seal ciphertext embeds the Eureka `package_id` used at encrypt time; it must match the
        // app-configured package *and* the on-chain Printer/PrintJob deployment, or key fetch fails
        // with e.g. "No keys available for object … full_id …".
        let seal_package_id = encrypted.package_id;
        if seal_package_id != self.eureka_package_id {
            return Err(anyhow::anyhow!(
                "Seal ciphertext is bound to Eureka package {}, but this app is configured for {}.\n\
                 After redeploying Eureka you must re-encrypt and re-upload to Walrus with the new package; re-registering the printer alone cannot decrypt ciphertext from an older package.",
                seal_package_id,
                self.eureka_package_id
            ));
        }

        // Load wallet
        let wallet_path = std::env::var("HOME")
            .map_err(|_| anyhow::anyhow!("Cannot find HOME env var"))?
            + "/.sui/sui_config/client.yaml";

        let mut wallet = WalletContext::new(Path::new(&wallet_path))?;
        
        // SessionKey IBE namespace must be the same `package_id` stored in the ciphertext.
        let session_key = SessionKey::new(
            seal_package_id,
            10,
            &mut wallet,
        )
        .await?;

        // Build approval transaction for simplified seal_approve
        let mut builder = ProgrammableTransactionBuilder::new();

        // Extract resource_id from seal_id
        let resource_id = if seal_id.contains(':') {
            seal_id.split(':').nth(1)
                .ok_or_else(|| anyhow::anyhow!("Invalid seal_resource_id format"))?
        } else {
            seal_id
        };
        
        let id_hex = resource_id.strip_prefix("0x").unwrap_or(resource_id);
        let id_bytes = hex::decode(id_hex)
            .map_err(|e| anyhow::anyhow!("Failed to decode hex ID '{}': {}", id_hex, e))?;
        
        // Argument 0: _id (vector<u8>)
        let id_arg = builder.pure(id_bytes)?;
        
        // Argument 1: printer (shared object)
        let printer_arg = builder.obj(ObjectArg::SharedObject {
            id: printer_id,
            initial_shared_version: printer_version.into(),
            mutable: false,
        })?;
        
        // Argument 2: printer_cap (owned object)
        let printer_cap_obj = self.sui_client
            .read_api()
            .get_object_with_options(printer_cap_id, SuiObjectDataOptions::bcs_lossless())
            .await?
            .data
            .ok_or_else(|| anyhow::anyhow!("PrinterCap not found"))?;
        let printer_cap_arg = builder.obj(ObjectArg::ImmOrOwnedObject((
            printer_cap_id,
            printer_cap_obj.version,
            printer_cap_obj.digest,
        )))?;

        // Call seal_approve in eureka module (no type arguments!)
        // entry fun seal_approve(_id, printer, printer_cap, ctx)
        builder.programmable_move_call(
            seal_package_id.into(),
            Identifier::from_str("eureka")?,
            Identifier::from_str("seal_approve")?,
            vec![], // No type arguments needed!
            vec![
                id_arg,          // _id: vector<u8>
                printer_arg,     // printer: &Printer
                printer_cap_arg, // printer_cap: &PrinterCap
            ],
        );

        let approve_ptb: ProgrammableTransaction = builder.finish();

        // Decrypt with detailed error handling
        let plaintext = self.seal_client
            .decrypt_object_bytes(
                &bcs::to_bytes(&encrypted)?,
                approve_ptb,
                &session_key,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Seal SDK decryption failed: {}", e))?;

        Ok(plaintext)
    }

    /// Decrypt already-downloaded Walrus ciphertext using printer + PrinterCap + on-chain PrintJob (`eureka::seal_approve`).
    pub async fn decrypt_sealed_file_bytes(
        &self,
        seal_resource_id: &str,
        encrypted_file_bytes: &[u8],
        printer_id: SuiObjectID,
        printer_cap_id: SuiObjectID,
    ) -> Result<Vec<u8>> {
        let printer_version = self.fetch_printer_shared_version(printer_id).await?;
        let encrypted_object = self.parse_encrypted_object(encrypted_file_bytes)?;
        self.decrypt_sculpt(
            seal_resource_id,
            encrypted_object,
            printer_id,
            printer_cap_id,
            printer_version,
        )
        .await
    }

    /// Complete PrintJob-based decryption flow
    pub async fn decrypt_printjob_sculpt(
        &self,
        printer_id: SuiObjectID,
        printer_cap_id: SuiObjectID,
    ) -> Result<Vec<u8>> {
        // Step 1: Fetch sculpt_id from PrintJob
        let sculpt_id = self.fetch_sculpt_id_from_printjob(printer_id).await?;
        
        // Step 2: Fetch sculpt information
        let (encrypted_blob_id, seal_resource_id, printer_version) = 
            self.fetch_sculpt_and_objects(sculpt_id, printer_id).await?;

        // Step 3: Check if sculpt is encrypted
        let seal_id = match seal_resource_id {
            Some(id) => id,
            None => return Err(anyhow::anyhow!("Sculpt is not encrypted")),
        };

        // Step 4: Download and parse encrypted data
        let encrypted_data = self.download_encrypted_data(&encrypted_blob_id).await?;
        let encrypted_object = self.parse_encrypted_object(&encrypted_data)?;

        // Step 5: Decrypt using Seal SDK
        let plaintext = self.decrypt_sculpt(
            &seal_id, 
            encrypted_object,
            printer_id,
            printer_cap_id,
            printer_version,
        ).await?;

        Ok(plaintext)
    }
}
