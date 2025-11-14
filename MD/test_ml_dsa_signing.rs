//! Test ML-DSA Transaction Signing
//!
//! This program tests that ML-DSA signature generation works correctly
//! for transaction signing in BTPC.

use btpc_core::crypto::{PrivateKey, PublicKey};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== BTPC ML-DSA Transaction Signing Test ===\n");

    // Test 1: Generate ML-DSA key pair
    println!("1. Generating ML-DSA (Dilithium5) key pair...");
    let seed: [u8; 32] = [42u8; 32]; // Deterministic seed for testing
    let private_key = PrivateKey::from_seed(&seed)?;
    let public_key = private_key.public_key();
    println!("   ✅ Key pair generated successfully");
    println!("   Private key size: {} bytes", private_key.to_bytes().len());
    println!("   Public key size: {} bytes\n", public_key.to_bytes().len());

    // Test 2: Sign a transaction message with ML-DSA
    println!("2. Signing transaction message with ML-DSA...");
    let tx_message = b"transaction_id_12345:input_index_0";
    println!("   Message: {}", String::from_utf8_lossy(tx_message));

    let signature = private_key.sign(tx_message)?;
    let signature_bytes = signature.to_bytes();
    println!("   ✅ Signature generated successfully");
    println!("   Signature size: {} bytes\n", signature_bytes.len());

    // Test 3: Verify ML-DSA signature
    println!("3. Verifying ML-DSA signature...");
    let is_valid = public_key.verify(tx_message, &signature)?;

    if is_valid {
        println!("   ✅ Signature verification PASSED\n");
    } else {
        println!("   ❌ Signature verification FAILED\n");
        return Err("Signature verification failed".into());
    }

    // Test 4: Verify signature with wrong message fails
    println!("4. Testing signature rejection with wrong message...");
    let wrong_message = b"wrong_transaction_id:wrong_input";
    let wrong_sig_result = public_key.verify(wrong_message, &signature);

    match wrong_sig_result {
        Ok(false) | Err(_) => {
            println!("   ✅ Correctly rejected invalid signature\n");
        }
        Ok(true) => {
            println!("   ❌ ERROR: Accepted invalid signature!\n");
            return Err("Invalid signature was accepted".into());
        }
    }

    // Test 5: Multiple signatures (simulating multi-input transaction)
    println!("5. Testing multiple input signing (multi-UTXO transaction)...");
    let inputs = vec![
        "txid_abc123:0",
        "txid_def456:1",
        "txid_ghi789:2",
    ];

    let mut signatures = Vec::new();
    for (i, input) in inputs.iter().enumerate() {
        let message = input.as_bytes();
        let sig = private_key.sign(message)?;
        signatures.push(sig);
        println!("   Input {}: Signed successfully ({} bytes)",
                 i, sig.to_bytes().len());
    }
    println!("   ✅ All {} inputs signed successfully\n", inputs.len());

    // Summary
    println!("=== Test Summary ===");
    println!("✅ ML-DSA key generation: PASS");
    println!("✅ ML-DSA signature generation: PASS");
    println!("✅ ML-DSA signature verification: PASS");
    println!("✅ Invalid signature rejection: PASS");
    println!("✅ Multi-input signing: PASS");
    println!("\n🎉 All ML-DSA transaction signing tests PASSED!");
    println!("\nML-DSA (Dilithium5) is working correctly for BTPC transactions.");

    Ok(())
}
