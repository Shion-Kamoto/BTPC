//! Address Utilities
//!
//! Provides utility functions for address validation, cleaning, and format conversion.
//! Handles the migration from raw hex public keys to proper Base58 addresses.

use std::path::PathBuf;
use serde_json::Value;

/// Clean and validate a BTPC address, ensuring it's in Base58 format
///
/// This function handles several cases:
/// 1. Strips "Address: " prefix if present
/// 2. Converts raw hex public keys (3584-3904 chars) to Base58 format by looking up wallet files
/// 3. Validates Base58 address format
///
/// Returns the clean Base58 address or an error if invalid
pub fn clean_and_validate_address(address: &str, wallets_dir: Option<&PathBuf>) -> Result<String, String> {
    // Strip "Address: " prefix if present
    let stripped = if address.starts_with("Address: ") {
        address.strip_prefix("Address: ").unwrap_or(address)
    } else {
        address
    };

    // Check if it's a raw hex public key (very long hex string)
    if stripped.len() > 128 && stripped.chars().all(|c| c.is_ascii_hexdigit()) {
        // This is a raw public key - need to convert to Base58
        if let Some(wallets_path) = wallets_dir {
            // Try to find the wallet with this public key and get its Base58 address
            if let Ok(base58_address) = find_address_by_public_key(stripped, wallets_path) {
                return Ok(base58_address);
            } else {
                return Err(format!(
                    "Address is a raw public key ({} chars) with no matching wallet. Cannot convert to Base58.",
                    stripped.len()
                ));
            }
        } else {
            return Err(format!(
                "Address is a raw public key ({} chars) but no wallet directory provided for lookup.",
                stripped.len()
            ));
        }
    }

    // Validate Base58 format using btpc-core
    use btpc_core::crypto::Address;
    match Address::from_string(stripped) {
        Ok(_) => Ok(stripped.to_string()),
        Err(e) => Err(format!("Invalid Base58 address format: {}", e)),
    }
}

/// Find a wallet's Base58 address by searching for a matching public key
fn find_address_by_public_key(public_key_hex: &str, wallets_dir: &PathBuf) -> Result<String, String> {
    // Read all wallet JSON files in the directory
    let wallet_files = std::fs::read_dir(wallets_dir)
        .map_err(|e| format!("Failed to read wallets directory: {}", e))?;

    for entry in wallet_files {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        // Skip non-JSON files and metadata file
        if !path.extension().is_some_and(|ext| ext == "json") {
            continue;
        }
        if path.file_name().is_some_and(|name| name == "wallets_metadata.json") {
            continue;
        }

        // Read wallet file
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(wallet_data) = serde_json::from_str::<Value>(&content) {
                // Check if public_key matches
                if let Some(pk) = wallet_data.get("public_key").and_then(|v| v.as_str()) {
                    if pk == public_key_hex {
                        // Found matching wallet - return its Base58 address
                        if let Some(address) = wallet_data.get("address").and_then(|v| v.as_str()) {
                            println!("✅ Converted {} char public key to Base58: {}", public_key_hex.len(), address);
                            return Ok(address.to_string());
                        }
                    }
                }
            }
        }
    }

    Err(format!("No wallet found with public key: {}...", &public_key_hex[..20]))
}

/// Migrate UTXO addresses from raw public keys to Base58 format
///
/// Reads the UTXO file, converts all raw hex public key addresses to Base58 format
/// by looking them up in wallet files, and writes the corrected UTXO file back.
pub fn migrate_utxo_addresses(
    utxo_file: &PathBuf,
    wallets_dir: &PathBuf,
) -> Result<usize, String> {
    // Read UTXO file
    if !utxo_file.exists() {
        return Err("UTXO file does not exist".to_string());
    }

    let content = std::fs::read_to_string(utxo_file)
        .map_err(|e| format!("Failed to read UTXO file: {}", e))?;

    let mut utxos: Vec<Value> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse UTXO file: {}", e))?;

    let mut migration_count = 0;
    let mut errors = Vec::new();

    // Process each UTXO
    for utxo in utxos.iter_mut() {
        if let Some(address) = utxo.get("address").and_then(|v| v.as_str()) {
            // Clone address to release immutable borrow before mutable borrow
            let address_owned = address.to_string();

            // Check if this is a raw hex public key
            if address_owned.len() > 128 && address_owned.chars().all(|c| c.is_ascii_hexdigit()) {
                match find_address_by_public_key(&address_owned, wallets_dir) {
                    Ok(base58_address) => {
                        // Update the address field (now safe - no immutable borrow)
                        utxo["address"] = Value::String(base58_address.clone());
                        migration_count += 1;
                        println!("Migrated UTXO {} from {} char hex to Base58: {}",
                                 utxo.get("txid").and_then(|v| v.as_str()).unwrap_or("unknown"),
                                 address_owned.len(),
                                 base58_address);
                    }
                    Err(e) => {
                        errors.push(format!("UTXO {}: {}",
                                           utxo.get("txid").and_then(|v| v.as_str()).unwrap_or("unknown"),
                                           e));
                    }
                }
            }
        }
    }

    // Write updated UTXO file
    if migration_count > 0 {
        let updated_content = serde_json::to_string_pretty(&utxos)
            .map_err(|e| format!("Failed to serialize updated UTXOs: {}", e))?;

        // Backup original file
        let backup_file = utxo_file.with_extension("json.backup");
        std::fs::copy(utxo_file, &backup_file)
            .map_err(|e| format!("Failed to create backup: {}", e))?;
        println!("📦 Created backup: {}", backup_file.display());

        // Write updated file
        std::fs::write(utxo_file, updated_content)
            .map_err(|e| format!("Failed to write updated UTXO file: {}", e))?;

        println!("✅ Migrated {} UTXO addresses to Base58 format", migration_count);
        if !errors.is_empty() {
            println!("⚠️ {} UTXOs could not be migrated:", errors.len());
            for error in &errors {
                println!("   - {}", error);
            }
        }
    } else {
        println!("✅ No UTXO addresses need migration - all are already in Base58 format");
    }

    Ok(migration_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_address_with_prefix() {
        let address = "Address: mko1SJtu1c4pVCTCffXoaFSbc64DPZ1Gx3";
        let cleaned = clean_and_validate_address(address, None).unwrap();
        assert_eq!(cleaned, "mko1SJtu1c4pVCTCffXoaFSbc64DPZ1Gx3");
    }

    #[test]
    fn test_clean_address_without_prefix() {
        let address = "mko1SJtu1c4pVCTCffXoaFSbc64DPZ1Gx3";
        let cleaned = clean_and_validate_address(address, None).unwrap();
        assert_eq!(cleaned, "mko1SJtu1c4pVCTCffXoaFSbc64DPZ1Gx3");
    }

    #[test]
    fn test_detect_raw_hex_public_key() {
        let raw_hex = "a".repeat(3904);
        let result = clean_and_validate_address(&raw_hex, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("raw public key"));
    }
}