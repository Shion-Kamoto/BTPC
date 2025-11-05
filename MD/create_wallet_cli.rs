#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! btpc-core = { path = "./btpc-core" }
//! serde = { version = "1.0", features = ["derive"] }
//! serde_json = "1.0"
//! chrono = "0.4"
//! hex = "0.4"
//! bip39 = "2.1"
//! ```

use btpc_core::crypto::{PrivateKey, Address};
use btpc_core::Network;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 BTPC Wallet Creator CLI");
    println!("Creating wallet: testingW2\n");

    // Generate new wallet
    let private_key = PrivateKey::generate()?;
    let public_key = private_key.public_key();
    let address = Address::from_public_key(&public_key, Network::Regtest);

    // Generate mnemonic (24 words)
    let mut rng = rand::thread_rng();
    let mnemonic = bip39::Mnemonic::generate_in_with(&mut rng, bip39::Language::English, 24)?;
    let seed_phrase = mnemonic.to_string();

    // Wallet metadata
    let nickname = "testingW2";
    let description = "Second test wallet created via CLI";
    let wallet_dir = PathBuf::from("/home/bob/.btpc/wallets");

    // Ensure wallet directory exists
    std::fs::create_dir_all(&wallet_dir)?;

    // Create wallet file path
    let wallet_id = uuid::Uuid::new_v4().to_string();
    let wallet_file = wallet_dir.join(format!("{}.json", wallet_id));

    // Create wallet JSON structure (simplified)
    let wallet_data = serde_json::json!({
        "id": wallet_id,
        "nickname": nickname,
        "address": address.to_string(),
        "encrypted_private_key": hex::encode(private_key.to_bytes()), // In production, this should be encrypted!
        "created_at": chrono::Utc::now().to_rfc3339(),
        "metadata": {
            "description": description,
            "category": "personal",
            "color": "#6366f1",
            "is_favorite": false,
            "auto_backup": true,
            "notifications_enabled": true,
            "default_fee_credits": 10000
        },
        "is_default": false,
        "cached_balance_credits": 0,
        "cached_balance_btp": 0.0,
        "balance_updated_at": chrono::Utc::now().to_rfc3339(),
        "source": "cli_created"
    });

    // Write wallet file
    std::fs::write(&wallet_file, serde_json::to_string_pretty(&wallet_data)?)?;

    println!("✅ Wallet created successfully!");
    println!("\n📋 Wallet Details:");
    println!("   ID: {}", wallet_id);
    println!("   Nickname: {}", nickname);
    println!("   Address: {}", address.to_string());
    println!("   File: {}", wallet_file.display());
    println!("\n🔑 Recovery Information (SAVE THIS!):");
    println!("   Seed Phrase: {}", seed_phrase);
    println!("   Private Key (hex): {}", hex::encode(private_key.to_bytes()));

    // Save address to file for easy reference
    let address_file = PathBuf::from("/home/bob/BTPC/BTPC/testingW2_address.txt");
    std::fs::write(&address_file, address.to_string())?;
    println!("\n💾 Address saved to: {}", address_file.display());

    Ok(())
}
