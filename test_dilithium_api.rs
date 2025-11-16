use crystals_dilithium::dilithium3::{Keypair, PublicKey, SecretKey};

fn main() {
    // Test key generation
    let keypair = Keypair::generate(None);
    
    // Get bytes - need to check what methods are available
    println!("Keypair has: {:?}", std::mem::size_of_val(&keypair));
    
    // Check what fields are available
    // keypair likely has: public and secret fields
}
