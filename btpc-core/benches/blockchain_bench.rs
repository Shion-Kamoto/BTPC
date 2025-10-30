//! BTPC Blockchain Operations Benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use btpc_core::{
    consensus::{Difficulty, DifficultyTarget},
    crypto::Hash,
};

fn bench_difficulty_from_bits(c: &mut Criterion) {
    c.bench_function("difficulty_from_bits", |b| {
        b.iter(|| {
            let _diff = Difficulty::from_bits(black_box(545259519));
        });
    });
}

fn bench_pow_check(c: &mut Criterion) {
    let difficulty = Difficulty::from_bits(545259519);
    let target = DifficultyTarget::from_bits(difficulty.bits());
    let target_bytes = target.as_bytes();
    let hash = Hash::hash(b"test block hash");

    c.bench_function("pow_check", |b| {
        b.iter(|| {
            let _valid = black_box(&hash).meets_target(black_box(target_bytes));
        });
    });
}

fn bench_hash_comparison(c: &mut Criterion) {
    let hash1 = Hash::hash(b"block1");
    let hash2 = Hash::hash(b"block2");

    c.bench_function("hash_compare", |b| {
        b.iter(|| {
            let _equal = black_box(&hash1) == black_box(&hash2);
        });
    });
}

criterion_group!(
    benches,
    bench_difficulty_from_bits,
    bench_pow_check,
    bench_hash_comparison
);
criterion_main!(benches);