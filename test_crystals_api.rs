// Test file to understand crystals-dilithium API
use crystals_dilithium::dilithium3::{Keypair, PublicKey};

fn test_verify() {
    // Generate a keypair
    let keypair = Keypair::generate();

    // Get public key bytes
    let public_key_bytes = keypair.public_key_bytes();

    // Create message to sign
    let message = b"Hello, World!";

    // Sign the message
    let signature = keypair.sign(message);

    // Create public key from bytes
    let public_key = PublicKey::from_bytes(&public_key_bytes).unwrap();

    // Verify signature
    let is_valid = public_key.verify(&signature, message);

    println!("Signature valid: {}", is_valid);
}

fn main() {
    test_verify();
}