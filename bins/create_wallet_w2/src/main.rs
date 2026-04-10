use btpc_core::crypto::{PrivateKey, Address};
use btpc_core::Network;
use std::path::PathBuf;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 BTPC Wallet Creator - Creating testingW2");
    println!("============================================\n");

    // Generate new wallet
    println!("🔐 Generating ML-DSA key pair...");
    let private_key = PrivateKey::generate()?;
    let public_key = private_key.public_key();
    let address = Address::from_public_key(&public_key, Network::Regtest);
    println!("✅ Key pair generated");

    // Generate mnemonic (24 words)
    println!("📝 Generating recovery seed phrase...");
    let mut rng = rand::thread_rng();
    let mnemonic = bip39::Mnemonic::generate_in_with(&mut rng, bip39::Language::English, 24)?;
    let seed_phrase = mnemonic.to_string();
    println!("✅ Seed phrase generated");

    // Wallet metadata
    let nickname = "testingW2";
    let description = "Second test wallet created via CLI tool";
    let wallet_dir = PathBuf::from("/home/bob/.btpc/wallets");

    // Ensure wallet directory exists
    std::fs::create_dir_all(&wallet_dir)?;
    println!("📁 Wallet directory: {}", wallet_dir.display());

    // Create wallet file path
    let wallet_id = uuid::Uuid::new_v4().to_string();
    let wallet_file = wallet_dir.join(format!("{}.json", wallet_id));

    // Create wallet JSON structure
    let wallet_data = json!({
        "id": wallet_id,
        "nickname": nickname,
        "address": address.to_string(),
        "encrypted_private_key": hex::encode(private_key.to_bytes()), // WARNING: Not actually encrypted in this simple version!
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
        "source": "cli_tool"
    });

    // Write wallet file
    std::fs::write(&wallet_file, serde_json::to_string_pretty(&wallet_data)?)?;
    println!("✅ Wallet file created: {}", wallet_file.display());

    // Print wallet information
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║              WALLET CREATED SUCCESSFULLY!                      ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("📋 Wallet Details:");
    println!("   ID:       {}", wallet_id);
    println!("   Nickname: {}", nickname);
    println!("   Network:  Regtest");
    println!("   File:     {}", wallet_file.display());

    println!("\n📍 Wallet Address (Base58):");
    println!("   {}", address.to_string());

    println!("\n🔑 Recovery Information:");
    println!("   ⚠️  SAVE THIS INFORMATION IN A SECURE LOCATION!");
    println!("   ⚠️  Anyone with this information can access your funds!\n");
    println!("   Seed Phrase (24 words):");
    let words: Vec<&str> = seed_phrase.split_whitespace().collect();
    for (i, chunk) in words.chunks(6).enumerate() {
        println!("      {:2}. {}", i*6 + 1, chunk.join(" "));
    }

    println!("\n   Private Key (hex):");
    println!("      {}", hex::encode(private_key.to_bytes()));

    // Save address to file for easy reference
    let address_file = PathBuf::from("/home/bob/BTPC/BTPC/testingW2_address.txt");
    std::fs::write(&address_file, format!("{}\n", address.to_string()))?;
    println!("\n💾 Address saved to: {}", address_file.display());

    // Also save complete wallet info for reference
    let info_file = PathBuf::from("/home/bob/BTPC/BTPC/testingW2_info.txt");
    let info_content = format!(
        "BTPC Wallet: testingW2\n\
         =====================\n\n\
         Wallet ID:     {}\n\
         Nickname:      {}\n\
         Network:       Regtest\n\
         Created:       {}\n\n\
         Address (Base58):\n\
         {}\n\n\
         Seed Phrase (24 words):\n\
         {}\n\n\
         Private Key (hex):\n\
         {}\n\n\
         ⚠️  WARNING: Keep this information secure!\n\
         ⚠️  Anyone with access to the private key or seed phrase can access your funds.\n",
        wallet_id,
        nickname,
        chrono::Utc::now().to_rfc2822(),
        address.to_string(),
        seed_phrase,
        hex::encode(private_key.to_bytes())
    );
    std::fs::write(&info_file, info_content)?;
    println!("💾 Complete wallet info saved to: {}", info_file.display());

    println!("\n✅ Done! The wallet should now appear in the desktop app.");
    println!("   Refresh the wallet list in the app to see it.");

    Ok(())
}
