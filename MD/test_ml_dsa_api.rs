// Test file to understand pqc_dilithium API
use pqc_dilithium::*;

fn main() {
    println!("Testing pqc_dilithium API for ML-DSA-65 (Dilithium3)");

    // Test key generation
    let keys = Keypair::generate();

    println!("Public key size: {}", keys.public.len());
    println!("Private key size: {}", keys.expose_secret().len());

    // Test signing
    let message = b"Test message for ML-DSA";
    let signature = keys.sign(message);

    println!("Signature size: {}", signature.len());

    // Test verification
    let is_valid = verify(&signature, message, &keys.public);
    println!("Signature valid: {}", is_valid.is_ok());

    // Test with wrong message
    let is_invalid = verify(&signature, b"Wrong message", &keys.public);
    println!("Wrong message valid: {}", is_invalid.is_ok());
}
