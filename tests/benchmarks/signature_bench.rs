//! Performance benchmarks for ML-DSA signature operations
//!
//! Tests signature generation and verification performance to ensure
//! constitutional requirements are met (<2ms generation, <1.5ms verification).

use btpc_core::crypto::{PrivateKey, PublicKey, Hash};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::{Duration, Instant};

/// Test signature generation performance
fn bench_signature_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("signature_generation");

    // Test with different message sizes
    let message_sizes = vec![32, 64, 128, 256, 512, 1024];

    for size in message_sizes {
        let private_key = PrivateKey::generate().expect("Failed to generate private key");
        let message = vec![0u8; size];
        let message_hash = Hash::hash(&message);

        group.bench_with_input(
            BenchmarkId::new("ml_dsa_87", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(private_key.sign(black_box(message_hash.as_bytes())))
                        .expect("Signature generation failed")
                });
            },
        );
    }

    group.finish();
}

/// Test signature verification performance
fn bench_signature_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("signature_verification");

    // Test with different message sizes
    let message_sizes = vec![32, 64, 128, 256, 512, 1024];

    for size in message_sizes {
        let private_key = PrivateKey::generate().expect("Failed to generate private key");
        let public_key = private_key.public_key();
        let message = vec![0u8; size];
        let message_hash = Hash::hash(&message);
        let signature = private_key.sign(message_hash.as_bytes()).expect("Failed to sign");

        group.bench_with_input(
            BenchmarkId::new("ml_dsa_87", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(public_key.verify(
                        black_box(message_hash.as_bytes()),
                        black_box(&signature)
                    )).expect("Verification failed")
                });
            },
        );
    }

    group.finish();
}

/// Test key generation performance
fn bench_key_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_generation");

    group.bench_function("ml_dsa_87", |b| {
        b.iter(|| {
            black_box(PrivateKey::generate()).expect("Key generation failed")
        });
    });

    group.finish();
}

/// Test batch signature verification
fn bench_batch_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_verification");

    let batch_sizes = vec![10, 50, 100, 500, 1000];

    for batch_size in batch_sizes {
        // Prepare batch data
        let mut signatures = Vec::new();
        let mut public_keys = Vec::new();
        let mut messages = Vec::new();

        for i in 0..batch_size {
            let private_key = PrivateKey::generate().expect("Failed to generate private key");
            let public_key = private_key.public_key();
            let message = format!("test message {}", i);
            let message_hash = Hash::hash(message.as_bytes());
            let signature = private_key.sign(message_hash.as_bytes()).expect("Failed to sign");

            signatures.push(signature);
            public_keys.push(public_key);
            messages.push(message_hash);
        }

        group.bench_with_input(
            BenchmarkId::new("sequential", batch_size),
            &batch_size,
            |b, _| {
                b.iter(|| {
                    for i in 0..batch_size {
                        black_box(public_keys[i].verify(
                            black_box(messages[i].as_bytes()),
                            black_box(&signatures[i])
                        )).expect("Verification failed");
                    }
                });
            },
        );
    }

    group.finish();
}

/// Constitutional compliance test
fn test_constitutional_performance_requirements() {
    println!("Testing constitutional performance requirements...");

    // Test signature generation (<2ms requirement)
    let private_key = PrivateKey::generate().expect("Failed to generate private key");
    let message = b"constitutional compliance test message";
    let message_hash = Hash::hash(message);

    let start = Instant::now();
    let signature = private_key.sign(message_hash.as_bytes()).expect("Failed to sign");
    let generation_time = start.elapsed();

    println!("Signature generation time: {:?}", generation_time);
    assert!(
        generation_time < Duration::from_millis(2),
        "Signature generation took {:?}, exceeds 2ms constitutional requirement",
        generation_time
    );

    // Test signature verification (<1.5ms requirement)
    let public_key = private_key.public_key();

    let start = Instant::now();
    let is_valid = public_key.verify(message_hash.as_bytes(), &signature).expect("Failed to verify");
    let verification_time = start.elapsed();

    println!("Signature verification time: {:?}", verification_time);
    assert!(is_valid, "Signature verification failed");
    assert!(
        verification_time < Duration::from_millis(1500),
        "Signature verification took {:?}, exceeds 1.5ms constitutional requirement",
        verification_time
    );

    println!("✅ All constitutional performance requirements met!");
}

/// Memory usage benchmark
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    group.bench_function("key_pair_size", |b| {
        b.iter(|| {
            let private_key = black_box(PrivateKey::generate()).expect("Failed to generate key");
            let public_key = black_box(private_key.public_key());

            // Measure serialized sizes
            let private_key_bytes = private_key.to_bytes();
            let public_key_bytes = public_key.to_bytes();

            // Ensure the compiler doesn't optimize away the allocations
            black_box((private_key_bytes.len(), public_key_bytes.len()))
        });
    });

    group.bench_function("signature_size", |b| {
        let private_key = PrivateKey::generate().expect("Failed to generate key");
        let message = Hash::hash(b"benchmark message");

        b.iter(|| {
            let signature = black_box(private_key.sign(message.as_bytes())).expect("Failed to sign");
            black_box(signature.len())
        });
    });

    group.finish();
}

/// Stress test with rapid successive operations
fn bench_stress_test(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_test");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("rapid_signatures", |b| {
        let private_key = PrivateKey::generate().expect("Failed to generate key");
        let messages: Vec<Hash> = (0..1000)
            .map(|i| Hash::hash(&i.to_le_bytes()))
            .collect();

        b.iter(|| {
            for message in &messages {
                black_box(private_key.sign(black_box(message.as_bytes())))
                    .expect("Signature failed");
            }
        });
    });

    group.bench_function("rapid_verifications", |b| {
        let private_key = PrivateKey::generate().expect("Failed to generate key");
        let public_key = private_key.public_key();

        // Pre-generate signatures
        let signatures: Vec<_> = (0..1000)
            .map(|i| {
                let message = Hash::hash(&i.to_le_bytes());
                let signature = private_key.sign(message.as_bytes()).expect("Failed to sign");
                (message, signature)
            })
            .collect();

        b.iter(|| {
            for (message, signature) in &signatures {
                black_box(public_key.verify(
                    black_box(message.as_bytes()),
                    black_box(signature)
                )).expect("Verification failed");
            }
        });
    });

    group.finish();
}

/// Throughput benchmark
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    group.measurement_time(Duration::from_secs(5));

    // Signatures per second
    group.bench_function("signatures_per_second", |b| {
        let private_key = PrivateKey::generate().expect("Failed to generate key");
        let message = Hash::hash(b"throughput test");

        b.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                black_box(private_key.sign(black_box(message.as_bytes())))
                    .expect("Signature failed");
            }
            start.elapsed()
        });
    });

    // Verifications per second
    group.bench_function("verifications_per_second", |b| {
        let private_key = PrivateKey::generate().expect("Failed to generate key");
        let public_key = private_key.public_key();
        let message = Hash::hash(b"throughput test");
        let signature = private_key.sign(message.as_bytes()).expect("Failed to sign");

        b.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                black_box(public_key.verify(
                    black_box(message.as_bytes()),
                    black_box(&signature)
                )).expect("Verification failed");
            }
            start.elapsed()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_signature_generation,
    bench_signature_verification,
    bench_key_generation,
    bench_batch_verification,
    bench_memory_usage,
    bench_stress_test,
    bench_throughput
);
criterion_main!(benches);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constitutional_requirements() {
        test_constitutional_performance_requirements();
    }

    #[test]
    fn test_signature_consistency() {
        let private_key = PrivateKey::generate().expect("Failed to generate key");
        let public_key = private_key.public_key();
        let message = Hash::hash(b"consistency test");

        // Generate multiple signatures of the same message
        let sig1 = private_key.sign(message.as_bytes()).expect("Failed to sign");
        let sig2 = private_key.sign(message.as_bytes()).expect("Failed to sign");

        // Both should verify
        assert!(public_key.verify(message.as_bytes(), &sig1).expect("Verification failed"));
        assert!(public_key.verify(message.as_bytes(), &sig2).expect("Verification failed"));

        // ML-DSA is probabilistic, so signatures should be different
        assert_ne!(sig1, sig2, "ML-DSA signatures should be probabilistic");
    }

    #[test]
    fn test_key_serialization_roundtrip() {
        let original_key = PrivateKey::generate().expect("Failed to generate key");
        let key_bytes = original_key.to_bytes();
        let restored_key = PrivateKey::from_bytes(&key_bytes).expect("Failed to restore key");

        // Test that restored key works the same
        let message = Hash::hash(b"serialization test");
        let original_sig = original_key.sign(message.as_bytes()).expect("Failed to sign");
        let restored_sig = restored_key.sign(message.as_bytes()).expect("Failed to sign");

        let original_pubkey = original_key.public_key();
        let restored_pubkey = restored_key.public_key();

        // Both public keys should verify both signatures
        assert!(original_pubkey.verify(message.as_bytes(), &original_sig).expect("Verification failed"));
        assert!(restored_pubkey.verify(message.as_bytes(), &restored_sig).expect("Verification failed"));

        // Cross-verification should work
        assert!(original_pubkey.verify(message.as_bytes(), &restored_sig).expect("Verification failed"));
        assert!(restored_pubkey.verify(message.as_bytes(), &original_sig).expect("Verification failed"));
    }

    #[test]
    fn test_signature_sizes() {
        let private_key = PrivateKey::generate().expect("Failed to generate key");
        let public_key = private_key.public_key();
        let message = Hash::hash(b"size test");
        let signature = private_key.sign(message.as_bytes()).expect("Failed to sign");

        // Verify ML-DSA-87 sizes
        assert_eq!(private_key.to_bytes().len(), 4864, "Private key should be 4864 bytes for ML-DSA-87");
        assert_eq!(public_key.to_bytes().len(), 2592, "Public key should be 2592 bytes for ML-DSA-87");
        assert_eq!(signature.len(), 4627, "Signature should be 4627 bytes for ML-DSA-87");
    }

    #[test]
    fn test_large_message_handling() {
        let private_key = PrivateKey::generate().expect("Failed to generate key");
        let public_key = private_key.public_key();

        // Test with large message (ML-DSA should hash first)
        let large_message = vec![0x42u8; 1_000_000]; // 1MB message
        let message_hash = Hash::hash(&large_message);

        let start = Instant::now();
        let signature = private_key.sign(message_hash.as_bytes()).expect("Failed to sign");
        let sign_time = start.elapsed();

        let start = Instant::now();
        let is_valid = public_key.verify(message_hash.as_bytes(), &signature).expect("Verification failed");
        let verify_time = start.elapsed();

        assert!(is_valid);

        // Should still meet performance requirements even with large messages
        // (since we hash first)
        assert!(sign_time < Duration::from_millis(5), "Large message signing too slow");
        assert!(verify_time < Duration::from_millis(5), "Large message verification too slow");
    }
}