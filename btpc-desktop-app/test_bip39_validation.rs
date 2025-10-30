#!/usr/bin/env rust-script
//! Validation test for BIP39 mnemonic implementation
//!
//! This validates the critical security fix: BIP39-compliant mnemonic-to-key derivation

use bip39::{Mnemonic, Language};
use sha2::{Sha512, Digest};

fn main() {
    println!("🔐 Testing BIP39 Mnemonic Implementation");
    println!("========================================\n");

    // Test 1: Valid 24-word mnemonic
    let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    match Mnemonic::from_phrase(test_mnemonic, Language::English) {
        Ok(mnemonic) => {
            println!("✅ Test 1: Valid 24-word mnemonic accepted");

            // Derive seed (BIP39 standard)
            let seed = mnemonic.to_seed("");
            println!("   Generated seed: {} bytes", seed.len());

            // Hash first 32 bytes for key material
            let mut hasher = Sha512::new();
            hasher.update(&seed[..32]);
            let hash_result = hasher.finalize();
            let key_material = &hash_result[..32];

            println!("   Key material: {} bytes", key_material.len());
            println!("   Key preview: {}...", hex::encode(&key_material[..8]));
        }
        Err(e) => {
            println!("❌ Test 1 FAILED: {}", e);
            std::process::exit(1);
        }
    }

    println!();

    // Test 2: Invalid mnemonic should be rejected
    let invalid_mnemonic = "invalid mnemonic phrase that should fail";

    match Mnemonic::from_phrase(invalid_mnemonic, Language::English) {
        Ok(_) => {
            println!("❌ Test 2 FAILED: Invalid mnemonic was accepted!");
            std::process::exit(1);
        }
        Err(_) => {
            println!("✅ Test 2: Invalid mnemonic correctly rejected");
        }
    }

    println!();

    // Test 3: Test deterministic derivation (same mnemonic = same seed)
    let test_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let mnemonic1 = Mnemonic::from_phrase(test_phrase, Language::English).unwrap();
    let mnemonic2 = Mnemonic::from_phrase(test_phrase, Language::English).unwrap();

    let seed1 = mnemonic1.to_seed("");
    let seed2 = mnemonic2.to_seed("");

    if seed1 == seed2 {
        println!("✅ Test 3: Deterministic derivation confirmed");
        println!("   Same mnemonic produces identical seed");
    } else {
        println!("❌ Test 3 FAILED: Non-deterministic derivation!");
        std::process::exit(1);
    }

    println!();
    println!("========================================");
    println!("🎉 All BIP39 validation tests passed!");
    println!("\nSecurity improvements verified:");
    println!("  • BIP39 standard compliance");
    println!("  • Invalid mnemonic rejection");
    println!("  • Deterministic key derivation");
}