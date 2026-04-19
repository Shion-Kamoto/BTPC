//! T064a — Sync throughput benchmark (SC-005).
//!
//! Measures the throughput of the sync subsystem's core scheduling and
//! tracking components: InFlightTracker, BlockRequestScheduler, StallReaper,
//! and handle_received_block. These are the hot-path data structures that
//! run on every block during sync.
//!
//! Target: 10,000 block schedule-complete cycles under 60 seconds.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::net::SocketAddr;
use std::time::Duration;

use btpc_core::network::integrated_sync::{
    handle_received_block, BlockRequestScheduler, BoundedWindowConfig, InFlightTracker, StallReaper,
};

fn peer(n: u8) -> SocketAddr {
    format!("127.0.0.{}:8333", n).parse().unwrap()
}

/// Benchmark: schedule and complete 10,000 blocks across 8 peers through
/// InFlightTracker + BlockRequestScheduler + handle_received_block.
fn bench_10k_block_sync_cycle(c: &mut Criterion) {
    c.bench_function("sync_10k_blocks_schedule_complete", |b| {
        b.iter(|| {
            let cfg = BoundedWindowConfig::default();
            let mut tracker = InFlightTracker::new(cfg);
            let mut scheduler = BlockRequestScheduler::new(BoundedWindowConfig::default());
            let peers: Vec<SocketAddr> = (1..=8).map(peer).collect();

            for height in 0u64..10_000 {
                let p = peers[(height as usize) % peers.len()];

                // Schedule
                let _ = tracker.try_reserve(p, height);
                let _ = scheduler.try_schedule(p, height);

                // Simulate block arrival
                let outcome = handle_received_block(p, true, height);
                black_box(&outcome);

                // Release
                tracker.release(&p, height);
                scheduler.complete(&p, height);
            }
        });
    });
}

/// Benchmark: StallReaper schedule + reap cycle for 1,000 jobs from 8 peers.
fn bench_stall_reaper_throughput(c: &mut Criterion) {
    c.bench_function("stall_reaper_1k_jobs", |b| {
        b.iter(|| {
            let mut reaper = StallReaper::new(Duration::from_secs(30));
            let peers: Vec<SocketAddr> = (1..=8).map(peer).collect();

            // Schedule 1000 jobs
            let mut ids = Vec::with_capacity(1000);
            for height in 0u64..1_000 {
                let p = peers[(height as usize) % peers.len()];
                let id = reaper.schedule(p, height);
                ids.push(id);
            }

            // Complete half normally
            for &id in &ids[..500] {
                reaper.complete(id);
            }

            // Advance past stall window — remaining 500 should be reaped
            reaper.advance_clock(Duration::from_secs(35));
            let stalled = reaper.reap_stalled();
            black_box(stalled.len());
        });
    });
}

/// Benchmark: InFlightTracker per-peer cap enforcement under contention.
fn bench_tracker_cap_enforcement(c: &mut Criterion) {
    c.bench_function("tracker_cap_enforcement_10k", |b| {
        b.iter(|| {
            let cfg = BoundedWindowConfig::default(); // per_peer_cap = 16
            let mut tracker = InFlightTracker::new(cfg);
            let p = peer(1);

            // Fill to cap
            for h in 0..16u64 {
                let _ = tracker.try_reserve(p, h);
            }

            // Attempt 10,000 over-cap reservations (should all fail)
            let mut rejected = 0u64;
            for h in 16..10_016u64 {
                if tracker.try_reserve(p, h).is_err() {
                    rejected += 1;
                }
            }
            black_box(rejected);

            // Release all and refill
            for h in 0..16u64 {
                tracker.release(&p, h);
            }
            for h in 0..16u64 {
                let _ = tracker.try_reserve(p, h);
            }
        });
    });
}

criterion_group!(
    benches,
    bench_10k_block_sync_cycle,
    bench_stall_reaper_throughput,
    bench_tracker_cap_enforcement,
);
criterion_main!(benches);
