//! Load active Sui address and Ed25519 signer from `client.yaml` + `sui.keystore`.

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use bech32::{decode, FromBase32, Variant};
use serde::Deserialize;
use std::path::Path;
use sui_crypto::ed25519::Ed25519PrivateKey;
use sui_sdk_types::Address;

const SUI_PRIVKEY_HRP: &str = "suiprivkey";
/// First byte of decoded `suiprivkey` payload: scheme (0 = Ed25519).
const SIGNATURE_SCHEME_ED25519: u8 = 0;

#[derive(Debug, Deserialize)]
struct ClientYaml {
    active_address: String,
    keystore: serde_yaml::Value,
}

fn keystore_file_path(v: &serde_yaml::Value) -> Result<String> {
    v.get("File")
        .or_else(|| v.get("file"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("client.yaml: missing keystore.File path"))
}

/// Decode a Sui CLI / wallet export (`suiprivkey1...` Bech32m or raw base64 key material).
fn decode_sui_key_entry(entry: &str) -> Result<Ed25519PrivateKey> {
    let entry = entry.trim();
    if entry.starts_with("suiprivkey") {
        let (hrp, data, variant) = decode(entry).map_err(|e| anyhow!("bech32 decode: {}", e))?;
        if hrp.as_str() != SUI_PRIVKEY_HRP {
            return Err(anyhow!("unexpected HRP {}", hrp));
        }
        if variant != Variant::Bech32m {
            return Err(anyhow!("suiprivkey must use bech32m checksum"));
        }
        let bytes = Vec::<u8>::from_base32(&data).map_err(|e| anyhow!("from_base32: {}", e))?;
        if bytes.is_empty() {
            return Err(anyhow!("empty key payload"));
        }
        let scheme = bytes[0];
        if scheme != SIGNATURE_SCHEME_ED25519 {
            return Err(anyhow!("unsupported signature scheme flag {}", scheme));
        }
        let sk: [u8; 32] = bytes[1..]
            .try_into()
            .map_err(|_| anyhow!("ed25519 secret must be 32 bytes after scheme flag"))?;
        return Ok(Ed25519PrivateKey::new(sk));
    }

    let raw = base64::engine::general_purpose::STANDARD
        .decode(entry)
        .map_err(|e| anyhow!("base64 decode: {}", e))?;
    let sk_slice = match raw.len() {
        32 => &raw[..],
        33 => {
            if raw[0] != SIGNATURE_SCHEME_ED25519 {
                return Err(anyhow!("unsupported scheme flag {}", raw[0]));
            }
            &raw[1..]
        }
        _ => {
            return Err(anyhow!(
                "expected 32 or 33 byte key after base64, got {}",
                raw.len()
            ));
        }
    };
    let sk: [u8; 32] = sk_slice
        .try_into()
        .map_err(|_| anyhow!("invalid ed25519 secret length"))?;
    Ok(Ed25519PrivateKey::new(sk))
}

fn load_keystore_entries(path: &Path) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read keystore {}", path.display()))?;
    let entries: Vec<String> = serde_json::from_str(&text)
        .with_context(|| format!("parse keystore JSON {}", path.display()))?;
    Ok(entries)
}

/// Resolve `Ed25519PrivateKey` whose Sui address matches `active_address` in `client.yaml`.
pub fn load_active_signer(client_yaml: &Path) -> Result<(Address, Ed25519PrivateKey)> {
    let raw = std::fs::read_to_string(client_yaml)
        .with_context(|| format!("read {}", client_yaml.display()))?;
    let cfg: ClientYaml = serde_yaml::from_str(&raw)
        .map_err(|e| anyhow!("parse client.yaml: {}", e))?;

    let active: Address = cfg
        .active_address
        .parse()
        .map_err(|e| anyhow!("active_address: {}", e))?;

    let ks_path_str = keystore_file_path(&cfg.keystore)?;
    let ks_path = Path::new(&ks_path_str);
    let entries = load_keystore_entries(ks_path)?;

    for entry in &entries {
        let pk = match decode_sui_key_entry(entry) {
            Ok(k) => k,
            Err(_) => continue,
        };
        let addr = pk.public_key().derive_address();
        if addr == active {
            return Ok((active, pk));
        }
    }

    Err(anyhow!(
        "no key in {} matches active_address {}",
        ks_path.display(),
        active
    ))
}
