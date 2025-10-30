//! BTPC Cryptographic Operations Benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use btpc_core::crypto::{Hash, PrivateKey};

fn bench_sha512_hashing(c: &mut Criterion) {
    let data = b"BTPC transaction data for hashing benchmark";

    c.bench_function("sha512_hash", |b| {
        b.iter(|| {
            let _hash = Hash::hash(black_box(data));
        });
    });
}

fn bench_ml_dsa_key_generation(c: &mut Criterion) {
    c.bench_function("ml_dsa_keygen", |b| {
        b.iter(|| {
            let _keypair = PrivateKey::generate_ml_dsa().unwrap();
        });
    });
}

fn bench_ml_dsa_signing(c: &mut Criterion) {
    let keypair = PrivateKey::generate_ml_dsa().unwrap();
    let message = b"Transaction data to sign";

    c.bench_function("ml_dsa_sign", |b| {
        b.iter(|| {
            let _sig = keypair.sign(black_box(message)).unwrap();
        });
    });
}

fn bench_ml_dsa_verification(c: &mut Criterion) {
    let keypair = PrivateKey::generate_ml_dsa().unwrap();
    let public_key = keypair.public_key();
    let message = b"Transaction data to verify";
    let signature = keypair.sign(message).unwrap();

    c.bench_function("ml_dsa_verify", |b| {
        b.iter(|| {
            let _valid = public_key.verify(black_box(message), black_box(&signature));
        });
    });
}

criterion_group!(
    benches,
    bench_sha512_hashing,
    bench_ml_dsa_key_generation,
    bench_ml_dsa_signing,
    bench_ml_dsa_verification
);
criterion_main!(benches);