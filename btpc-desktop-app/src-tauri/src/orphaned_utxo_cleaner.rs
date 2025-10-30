/// Orphaned UTXO Cleaner
///
/// This utility identifies and removes UTXOs that don't belong to any current wallet.
/// Orphaned UTXOs occur when mining was done to addresses/public keys that no longer
/// have corresponding wallet files.
use std::path::PathBuf;
use std::collections::HashSet;
use serde_json::Value;

/// Check for orphaned UTXOs and optionally remove them
pub fn clean_orphaned_utxos(
    utxo_file: &PathBuf,
    wallets_dir: &PathBuf,
    dry_run: bool,
) -> Result<CleanupReport, String> {
    // Read UTXO file
    if !utxo_file.exists() {
        return Err("UTXO file does not exist".to_string());
    }

    let content = std::fs::read_to_string(utxo_file)
        .map_err(|e| format!("Failed to read UTXO file: {}", e))?;

    let utxos: Vec<Value> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse UTXO file: {}", e))?;

    // Build set of wallet public keys and Base58 addresses
    let mut wallet_identifiers = HashSet::new();

    // Read all wallet files
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
                // Add both public_key and address to the set
                if let Some(pk) = wallet_data.get("public_key").and_then(|v| v.as_str()) {
                    wallet_identifiers.insert(pk.to_string());
                }
                if let Some(addr) = wallet_data.get("address").and_then(|v| v.as_str()) {
                    wallet_identifiers.insert(addr.to_string());
                }
            }
        }
    }

    println!("📊 Found {} wallet identifiers (public keys + addresses)", wallet_identifiers.len());

    // Separate UTXOs into owned and orphaned
    let mut owned_utxos = Vec::new();
    let mut orphaned_utxos = Vec::new();
    let mut orphaned_value_credits: u64 = 0;

    for utxo in utxos {
        if let Some(address) = utxo.get("address").and_then(|v| v.as_str()) {
            if wallet_identifiers.contains(address) {
                owned_utxos.push(utxo);
            } else {
                // This is an orphaned UTXO
                let value = utxo.get("value_credits").and_then(|v| v.as_u64()).unwrap_or(0);
                orphaned_value_credits += value;
                orphaned_utxos.push(utxo);
            }
        }
    }

    let report = CleanupReport {
        total_utxos: owned_utxos.len() + orphaned_utxos.len(),
        owned_utxos: owned_utxos.len(),
        orphaned_utxos: orphaned_utxos.len(),
        orphaned_value_credits,
        orphaned_value_btp: orphaned_value_credits as f64 / 100_000_000.0,
    };

    // Print report
    println!("\n📋 Orphaned UTXO Report:");
    println!("  Total UTXOs: {}", report.total_utxos);
    println!("  ✅ Owned UTXOs (belong to current wallets): {}", report.owned_utxos);
    println!("  ❌ Orphaned UTXOs (no matching wallet): {}", report.orphaned_utxos);
    println!("  💰 Orphaned value: {} credits ({:.8} BTP)",
             report.orphaned_value_credits, report.orphaned_value_btp);

    if !dry_run && report.orphaned_utxos > 0 {
        // Backup original file
        let backup_file = utxo_file.with_extension("json.orphan_backup");
        std::fs::copy(utxo_file, &backup_file)
            .map_err(|e| format!("Failed to create backup: {}", e))?;
        println!("\n📦 Created backup: {}", backup_file.display());

        // Write cleaned UTXO file with only owned UTXOs
        let cleaned_content = serde_json::to_string_pretty(&owned_utxos)
            .map_err(|e| format!("Failed to serialize cleaned UTXOs: {}", e))?;

        std::fs::write(utxo_file, cleaned_content)
            .map_err(|e| format!("Failed to write cleaned UTXO file: {}", e))?;

        println!("✅ Removed {} orphaned UTXOs from {}", report.orphaned_utxos, utxo_file.display());
        println!("⚠️  WARNING: Removed {} BTP ({} credits) in orphaned funds",
                 report.orphaned_value_btp, report.orphaned_value_credits);
    } else if report.orphaned_utxos > 0 {
        println!("\n🔍 DRY RUN: Would remove {} orphaned UTXOs", report.orphaned_utxos);
        println!("   Run with dry_run=false to actually remove them");
    } else {
        println!("\n✅ No orphaned UTXOs found - all UTXOs belong to current wallets");
    }

    Ok(report)
}

#[derive(Debug, Clone)]
pub struct CleanupReport {
    pub total_utxos: usize,
    pub owned_utxos: usize,
    pub orphaned_utxos: usize,
    pub orphaned_value_credits: u64,
    pub orphaned_value_btp: f64,
}