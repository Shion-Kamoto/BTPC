use pqc_dilithium::*;

fn main() {
    println!("pqc_dilithium default mode (mode3 = ML-DSA-65):");
    println!("PUBLICKEYBYTES: {}", PUBLICKEYBYTES);
    println!("SECRETKEYBYTES: {}", SECRETKEYBYTES);
    println!("SIGNBYTES: {}", SIGNBYTES);
    println!();
    
    // Calculate expected values for mode3 (K=6, L=5)
    let calc_pub = 32 + 6 * 320; // SEEDBYTES + K * POLYT1_PACKEDBYTES
    let calc_sec = 3 * 32 + 5 * 128 + 6 * 128 + 6 * 416; // 3*SEEDBYTES + L*POLYETA + K*POLYETA + K*POLYT0
    let calc_sig = 32 + 5 * 640 + (55 + 6); // SEEDBYTES + L*POLYZ + (OMEGA + K)
    
    println!("Calculated public: {}", calc_pub);
    println!("Calculated secret: {}", calc_sec);
    println!("Calculated sig: {}", calc_sig);
}
