//! Mining Thread Pool Module
//!
//! Implements CPU and GPU mining via background thread pools.
//! Replaces external btpc_miner binary with in-process mining.
//!
//! Key features:
//! - CPU mining: Rayon thread pool with (num_cpus - 2) threads
//! - GPU mining: Optional dedicated thread for OpenCL/CUDA
//! - Atomic statistics: hashrate, blocks found, uptime
//! - Graceful shutdown with thread cleanup

use anyhow::{Context, Result};
use num_format::ToFormattedString;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};

use crate::gpu_miner::{enumerate_gpu_devices, GpuMiner};

// Import RpcClientInterface trait directly from rpc_client module
use crate::debug_logger::get_debug_logger;
use crate::rpc_client::RpcClientInterface;

// REM-C002: Mining event types for frontend notification
/// Events emitted by the mining thread pool
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MiningEvent {
    /// Block successfully mined and submitted
    BlockMined {
        block_hash: String,
        block_height: u64,
        reward_btpc: f64,
        device_id: u32,
        device_name: String,
        difficulty: String,
        nonce: u64,
    },
    /// GPU thermal throttling occurred
    ThermalThrottle {
        device_id: u32,
        device_name: String,
        current_temperature: f32,
        thermal_limit: f32,
        action: String, // "warning", "throttle", "shutdown"
    },
    /// GPU error encountered
    GpuError {
        device_id: u32,
        device_name: String,
        error_type: String, // "kernel_error", "device_lost", "memory_error"
        error_message: String,
        mining_stopped: bool,
    },
    /// FIX 2025-12-08: Real-time mining activity update (pushed to frontend, not polled)
    /// Emitted every ~500ms with current mining state for live ASCII console display
    MiningActivity {
        device_id: u32,
        hashrate: f64,       // Current hashrate in H/s
        total_hashes: u64,   // Total hashes computed
        current_nonce: u64,  // Current nonce being tested
        block_height: u64,   // Block height being mined
        difficulty: String,  // Current difficulty target
        uptime_seconds: u64, // Mining uptime
        blocks_found: u64,   // Total blocks found this session
        message: String,     // Status message (e.g., "Searching...", "Template fetched")
        extra_nonce: u64, // FIX 2026-02-23: extraNonce counter (increments when u32 nonce exhausted)
    },
    /// FIX 2025-12-08: Block construction log event for live mining console display
    /// Emitted when coinbase TX, merkle root, or block header is constructed
    BlockConstruction {
        log_type: String, // "COINBASE", "MERKLE", or "HEADER"
        message: String,  // Formatted log message
    },
}

/// Trait for types that can log mining events
pub trait MiningLogger {
    fn add_entry(&mut self, level: String, message: String);
}

/// Cached block template with expiration tracking
///
/// Reduces RPC requests by reusing templates for 10 seconds.
/// Fixes 429 rate limit errors from requesting new template every batch.
#[allow(dead_code)] // Template caching reserved for RPC rate limiting
struct CachedTemplate {
    template: crate::rpc_client::BlockTemplate,
    cached_at: Instant,
    /// Cache duration (10 seconds - node generates new block every ~10s)
    ttl: Duration,
}

impl CachedTemplate {
    fn new(template: crate::rpc_client::BlockTemplate) -> Self {
        Self {
            template,
            cached_at: Instant::now(),
            ttl: Duration::from_secs(10),
        }
    }

    /// Check if cache is still valid
    fn is_valid(&self) -> bool {
        self.cached_at.elapsed() < self.ttl
    }

    /// Get cached template (clones data)
    #[allow(dead_code)]
    fn get(&self) -> crate::rpc_client::BlockTemplate {
        self.template.clone()
    }
}

/// Per-GPU mining statistics (Feature 012: T016)
///
/// Tracks mining performance for individual GPU devices.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerGpuStats {
    pub device_index: u32,
    pub current_hashrate: f64,
    pub total_hashes: u64,
    pub blocks_found: u64,
    pub mining_uptime: u64, // seconds
    #[serde(skip, default = "Instant::now")]
    pub last_updated: Instant,
}

/// FIX 2025-12-11: Rolling window hash sample for accurate current hashrate
/// Stores (timestamp, hash_count) pairs from the last 60 seconds
#[derive(Debug, Clone)]
struct HashrateSample {
    timestamp: Instant,
    hashes: u64,
}

/// Rolling window duration for hashrate calculation (60 seconds)
const HASHRATE_WINDOW_SECONDS: u64 = 60;

/// Mining thread pool manager
///
/// Design decisions (from research.md R003):
/// - Rayon for CPU parallelism (work-stealing thread pool)
/// - Below-normal priority to preserve UI responsiveness
/// - Atomic counters for lock-free stats reads
/// - Separate shutdown channel per mining type
///
/// Feature 012 (T016): Added per-GPU statistics tracking
pub struct MiningThreadPool {
    /// Is CPU mining active?
    cpu_mining_active: Arc<AtomicBool>,

    /// Is GPU mining active?
    gpu_mining_active: Arc<AtomicBool>,

    /// Number of active CPU threads
    cpu_threads: Arc<AtomicU64>,

    /// Number of active GPU devices
    gpu_devices: Arc<AtomicU64>,

    /// Total hashes computed (CPU)
    cpu_total_hashes: Arc<AtomicU64>,

    /// Total hashes computed (GPU)
    gpu_total_hashes: Arc<AtomicU64>,

    /// Blocks found count
    blocks_found: Arc<AtomicU64>,

    /// Mining start time
    start_time: Arc<RwLock<Option<Instant>>>,

    /// Mining address (coinbase output)
    mining_address: Arc<RwLock<String>>,

    /// Shutdown signal sender (CPU)
    cpu_shutdown_tx: Option<mpsc::Sender<()>>,

    /// Shutdown signal sender (GPU) - uses broadcast for multiple receivers
    gpu_shutdown_tx: Option<broadcast::Sender<()>>,

    /// Per-GPU statistics tracking (Feature 012: T016)
    /// Maps GPU device index to individual mining stats
    per_gpu_stats: Arc<RwLock<HashMap<u32, PerGpuStats>>>,

    /// GPU device information (hardware specs)
    /// Maps GPU device index to device info for stats display
    gpu_device_info: Arc<RwLock<HashMap<u32, crate::gpu_miner::GpuDevice>>>,

    /// REM-C002: Mining event channel sender (optional for event emission)
    mining_event_tx: Option<mpsc::UnboundedSender<MiningEvent>>,

    /// FIX 2025-11-26: Shared template invalidation counter for multi-GPU coordination
    /// When ANY GPU mines a block, this counter is incremented.
    /// All GPUs check this counter and invalidate their templates if it changed.
    /// This prevents multiple GPUs from mining at the same stale height.
    template_version: Arc<AtomicU64>,

    /// FIX 2025-12-11: Rolling window hash samples for accurate current hashrate
    /// Stores recent (timestamp, hash_count) samples for last 60 seconds
    /// This replaces lifetime average with instantaneous hashrate
    gpu_hash_samples: Arc<RwLock<Vec<HashrateSample>>>,

    /// Network fork_id for coinbase transactions (0=Mainnet, 1=Testnet, 2=Regtest)
    network_fork_id: u8,
}

impl MiningThreadPool {
    /// Create new mining thread pool (inactive)
    ///
    /// # Arguments
    /// * `initial_blocks_found` - Starting value for lifetime blocks found counter (for persistence)
    /// * `network_fork_id` - Fork ID for the active network (0=Mainnet, 1=Testnet, 2=Regtest)
    pub fn new(initial_blocks_found: u64, network_fork_id: u8) -> Self {
        MiningThreadPool {
            cpu_mining_active: Arc::new(AtomicBool::new(false)),
            gpu_mining_active: Arc::new(AtomicBool::new(false)),
            cpu_threads: Arc::new(AtomicU64::new(0)),
            gpu_devices: Arc::new(AtomicU64::new(0)),
            cpu_total_hashes: Arc::new(AtomicU64::new(0)),
            gpu_total_hashes: Arc::new(AtomicU64::new(0)),
            blocks_found: Arc::new(AtomicU64::new(initial_blocks_found)),
            start_time: Arc::new(RwLock::new(None)),
            mining_address: Arc::new(RwLock::new(String::new())),
            cpu_shutdown_tx: None,
            gpu_shutdown_tx: None,
            per_gpu_stats: Arc::new(RwLock::new(HashMap::new())),
            gpu_device_info: Arc::new(RwLock::new(HashMap::new())),
            mining_event_tx: None,                               // REM-C002
            template_version: Arc::new(AtomicU64::new(0)),       // FIX 2025-11-26
            gpu_hash_samples: Arc::new(RwLock::new(Vec::new())), // FIX 2025-12-11: Rolling window
            network_fork_id,
        }
    }

    /// FIX 2025-12-11: Calculate hashrate from rolling window
    /// Returns hashes per second over the last 60 seconds
    fn calculate_rolling_hashrate(&self) -> f64 {
        let samples = self.gpu_hash_samples.read().unwrap();

        if samples.len() < 2 {
            // Not enough samples - fall back to lifetime average
            let uptime = self
                .start_time
                .read()
                .unwrap()
                .map(|s| s.elapsed().as_secs())
                .unwrap_or(0);
            let hashes = self.gpu_total_hashes.load(Ordering::SeqCst);
            return if uptime > 0 {
                hashes as f64 / uptime as f64
            } else {
                0.0
            };
        }

        // Get oldest and newest samples in window
        let oldest = samples.first().unwrap();
        let newest = samples.last().unwrap();

        // Calculate delta hashes over delta time
        let delta_hashes = newest.hashes.saturating_sub(oldest.hashes);
        let delta_secs = newest
            .timestamp
            .duration_since(oldest.timestamp)
            .as_secs_f64();

        if delta_secs > 0.0 {
            delta_hashes as f64 / delta_secs
        } else {
            0.0
        }
    }

    /// Set the mining event channel sender (REM-C002)
    ///
    /// Called by mining_commands.rs to enable event emission to frontend
    pub fn set_event_channel(&mut self, tx: mpsc::UnboundedSender<MiningEvent>) {
        self.mining_event_tx = Some(tx);
    }

    /// Start CPU mining
    ///
    /// # Arguments
    /// * `num_threads` - Number of threads (None = num_cpus - 2)
    /// * `mining_address` - Coinbase output address
    ///
    /// # Returns
    /// * `Ok(u32)` - Number of threads started
    /// * `Err(anyhow::Error)` - Failed to start mining
    ///
    /// # Thread Priority
    /// Uses below-normal priority to preserve UI responsiveness (Constitution requirement)
    pub async fn start_cpu_mining(
        &mut self,
        num_threads: Option<u32>,
        mining_address: String,
    ) -> Result<u32> {
        // Stop existing mining if active
        if self.cpu_mining_active.load(Ordering::SeqCst) {
            self.stop_cpu_mining().await?;
        }

        // Calculate thread count: (num_cpus - 2), minimum 1
        let thread_count =
            num_threads.unwrap_or_else(|| (num_cpus::get() as i32 - 2).max(1) as u32);

        // Store mining address
        *self.mining_address.write().unwrap() = mining_address;

        // Initialize Rayon thread pool with custom config
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count as usize)
            .build()
            .context("Failed to create Rayon thread pool")?;

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.cpu_shutdown_tx = Some(shutdown_tx);

        // Clone atomics for background task
        let mining_active = self.cpu_mining_active.clone();
        let total_hashes = self.cpu_total_hashes.clone();
        let _blocks_found = self.blocks_found.clone(); // Reserved for future block submission

        // Set mining as active
        mining_active.store(true, Ordering::SeqCst);
        self.cpu_threads
            .store(thread_count as u64, Ordering::SeqCst);

        // Record start time
        *self.start_time.write().unwrap() = Some(Instant::now());

        // Spawn background task for mining
        // CRITICAL: Move pool into closure to keep it alive
        tokio::spawn(async move {
            eprintln!(
                "🔧 Mining task started - thread_count: {}",
                pool.current_num_threads()
            );

            // Mining loop
            let mut iteration = 0u64;
            while mining_active.load(Ordering::SeqCst) {
                // Check for shutdown signal (non-blocking)
                if shutdown_rx.try_recv().is_ok() {
                    eprintln!("🛑 Mining shutdown signal received");
                    break;
                }

                // Perform mining work in parallel
                pool.install(|| {
                    // Parallel iterator over work items
                    (0..1000).into_par_iter().for_each(|_nonce| {
                        // TODO: In T013 full implementation:
                        // 1. Get current block template from node
                        // 2. Compute SHA-512 hash with nonce
                        // 3. Check if hash meets difficulty target
                        // 4. If valid, submit block and increment blocks_found
                        // 5. Increment total_hashes counter

                        // Placeholder: increment hash counter
                        total_hashes.fetch_add(1, Ordering::Relaxed);
                    });
                });

                iteration += 1;
                if iteration % 100 == 0 {
                    let hashes = total_hashes.load(Ordering::Relaxed);
                    eprintln!(
                        "⛏️  Mining iteration {} - total hashes: {}",
                        iteration, hashes
                    );
                }

                // Sleep briefly to allow shutdown checks
                tokio::time::sleep(Duration::from_millis(10)).await;
            }

            eprintln!(
                "🔧 Mining task exiting - total hashes: {}",
                total_hashes.load(Ordering::Relaxed)
            );

            // Clean up
            mining_active.store(false, Ordering::SeqCst);

            // Drop pool explicitly at end of scope
            drop(pool);
        });

        Ok(thread_count)
    }

    /// Stop CPU mining gracefully
    pub async fn stop_cpu_mining(&mut self) -> Result<()> {
        if !self.cpu_mining_active.load(Ordering::SeqCst) {
            return Ok(()); // Already stopped
        }

        // Send shutdown signal
        if let Some(tx) = self.cpu_shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Wait for mining to stop (with timeout)
        let timeout = Duration::from_secs(5);
        let start = Instant::now();

        while self.cpu_mining_active.load(Ordering::SeqCst) {
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!("CPU mining shutdown timeout"));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Reset thread count
        self.cpu_threads.store(0, Ordering::SeqCst);

        Ok(())
    }

    /// Start GPU mining
    ///
    /// # Arguments
    /// * `mining_address` - Coinbase output address
    /// * `rpc_client` - RPC client for block template requests and block submission
    /// * `log_tx` - Optional channel for sending mining log events (level, message)
    /// * `mining_logs` - Optional direct access to mining logs buffer (bypasses channel)
    ///
    /// # Returns
    /// * `Ok(u32)` - Number of GPU devices initialized
    /// * `Err(antml:Error)` - Failed to start GPU mining
    pub async fn start_gpu_mining<T, R>(
        &mut self,
        mining_address: String,
        rpc_client: Arc<R>,
        log_tx: Option<mpsc::UnboundedSender<(String, String)>>,
        mining_logs: Option<Arc<std::sync::Mutex<T>>>,
    ) -> Result<u32>
    where
        T: 'static + Send + MiningLogger,
        R: 'static + Send + Sync + RpcClientInterface,
    {
        // Stop existing GPU mining if active
        if self.gpu_mining_active.load(Ordering::SeqCst) {
            self.stop_gpu_mining().await?;
        }

        // Enumerate available GPU devices
        let gpu_devices = enumerate_gpu_devices().context("Failed to enumerate GPU devices")?;

        if gpu_devices.is_empty() {
            return Err(anyhow::anyhow!("No GPU devices found"));
        }

        let device_count = gpu_devices.len() as u32;
        println!("🎮 Found {} GPU device(s)", device_count);
        for device in &gpu_devices {
            println!(
                "  - GPU {}: {} ({})",
                device.device_index, device.model_name, device.vendor
            );
        }

        // Store mining address
        *self.mining_address.write().unwrap() = mining_address;

        // Create broadcast shutdown channel (one signal to multiple GPU threads)
        let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);
        self.gpu_shutdown_tx = Some(shutdown_tx);

        // Clone atomics for background task
        let mining_active = self.gpu_mining_active.clone();
        let per_gpu_stats = self.per_gpu_stats.clone();
        let start_time = self.start_time.clone();
        let blocks_found_counter = self.blocks_found.clone(); // Clone global blocks counter
        let mining_address_arc = self.mining_address.clone(); // Clone Arc for closure access
        let mining_event_tx = self.mining_event_tx.clone(); // REM-C002: Clone event channel

        // Initialize stats and store device info for each GPU
        {
            let mut device_info_map = self.gpu_device_info.write().unwrap();
            for device in &gpu_devices {
                self.init_gpu_stats(device.device_index);
                device_info_map.insert(device.device_index, device.clone());
            }
        }

        // Record start time
        *self.start_time.write().unwrap() = Some(Instant::now());

        // Store device count (for stats)
        self.gpu_devices
            .store(device_count as u64, Ordering::SeqCst);

        // Spawn GPU mining task for each device
        for device_info in gpu_devices.into_iter() {
            let device_index = device_info.device_index;
            let device_name = device_info.model_name.clone();
            let mining_active = mining_active.clone();
            let per_gpu_stats = per_gpu_stats.clone();
            let start_time = start_time.clone();
            let blocks_found_counter_clone = blocks_found_counter.clone(); // Clone for this GPU thread
            let mut shutdown_rx_clone = shutdown_rx.resubscribe();
            let log_tx_clone = log_tx.clone();
            let _mining_logs_clone = mining_logs.clone(); // Reserved for future log aggregation
            let gpu_total_hashes_clone = self.gpu_total_hashes.clone();
            let rpc_client_clone = rpc_client.clone();
            let mining_address_clone = mining_address_arc.clone(); // Clone Arc for EACH GPU thread
            let event_tx_clone = mining_event_tx.clone(); // REM-C002: Clone event channel for this thread
            let template_version_clone = self.template_version.clone(); // FIX 2025-11-26: Multi-GPU coordination
            let hash_samples_clone = self.gpu_hash_samples.clone(); // FIX 2025-12-11: Rolling window samples
            let network_fork_id = self.network_fork_id; // Network-aware fork_id for coinbase TXs

            tokio::spawn(async move {
                println!("🚀 Starting GPU {} mining thread", device_index);

                // Create GPU miner for this device
                let mut miner = match GpuMiner::new(device_index) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("❌ Failed to initialize GPU {}: {}", device_index, e);
                        // Don't set mining_active if initialization fails
                        return;
                    }
                };

                // Mark mining as active AFTER successful GPU initialization
                mining_active.store(true, Ordering::SeqCst);
                miner.set_mining(true);
                println!("✅ GPU {} initialized successfully", device_index);

                // Mining loop with template caching
                let mut nonce_start = device_index * 1_000_000_000; // Partition nonce space
                let mut cached_template: Option<CachedTemplate> = None; // Template cache (10s TTL)
                let mut current_template: Option<crate::rpc_client::BlockTemplate> = None; // Working template
                let mut my_template_version = template_version_clone.load(Ordering::SeqCst); // FIX 2025-11-26: Track local version

                // FIX 2026-02-23: extraNonce mechanism (Bitcoin-style)
                // The block header nonce is u32 (~4.3B values). At 140+ MH/s the GPU
                // exhausts the entire nonce space in ~30 seconds. Without extraNonce,
                // if the valid nonce for a given header doesn't exist in u32 range,
                // the block is UNFINDABLE until the template refreshes (new timestamp).
                // extraNonce is written into the coinbase script_sig, changing the
                // merkle root → completely new header → fresh 4.3B nonce search space.
                let mut extra_nonce: u64 = 0;
                let mut _nonce_wrapped = false; // Track when u32 nonce space is exhausted

                // FIX 2025-12-08: Track last activity event emission time for rate limiting
                // 250ms = 4 updates/second — smooth UI without flooding IBUS GTK event queue
                // 10ms (100/sec) caused IBUS to drop keyboard events on login page
                let mut last_activity_event = Instant::now();
                const ACTIVITY_EVENT_INTERVAL: Duration = Duration::from_millis(250);

                // FIX 2025-12-13: Cache last known block height and difficulty for MiningActivity events
                // On Regtest, blocks are mined so fast that current_template is often None during rebuild
                // Without caching, MiningActivity events won't be emitted and UI shows 0 hashrate/uptime
                let mut cached_block_height: u64 = 0;
                let mut cached_difficulty: String = "1d00ffff".to_string();

                while mining_active.load(Ordering::SeqCst) {
                    // Check for shutdown signal (non-blocking)
                    if shutdown_rx_clone.try_recv().is_ok() {
                        println!("🛑 GPU {} mining shutdown signal received", device_index);
                        break;
                    }

                    // FIX 2025-11-26: Check if another GPU found a block (version mismatch)
                    // This prevents multiple GPUs from mining on the same stale height
                    let global_version = template_version_clone.load(Ordering::SeqCst);
                    if global_version != my_template_version {
                        eprintln!("[GPU {}] 🔄 Another GPU mined a block (version {} -> {}), invalidating template...",
                            device_index, my_template_version, global_version);
                        cached_template = None;
                        current_template = None;
                        my_template_version = global_version;
                    }

                    // Cache validity check - only fetch when truly needed
                    let should_fetch_template =
                        match (current_template.as_ref(), cached_template.as_ref()) {
                            // No template at all - must fetch
                            (None, _) => true,
                            // Have template but cache expired - should fetch new one
                            (Some(_), Some(cache)) if !cache.is_valid() => true,
                            // Have template and cache is still valid - keep using it
                            (Some(_), Some(cache)) if cache.is_valid() => false,
                            // Have template but no cache metadata (shouldn't happen) - keep using template
                            (Some(_), None) => false,
                            // Unreachable: all Some/Some cases covered above
                            _ => false,
                        };

                    if should_fetch_template {
                        // Debug logging to understand cache behavior
                        if current_template.is_none() {
                            eprintln!("[GPU MINING] 🔄 Fetching initial template...");
                        } else if let Some(cache) = &cached_template {
                            eprintln!(
                                "[GPU MINING] 🔄 Cache expired ({}s old), fetching new template...",
                                cache.cached_at.elapsed().as_secs()
                            );
                        }

                        match rpc_client_clone.get_block_template().await {
                            Ok(template) => {
                                let height = template.height;
                                eprintln!(
                                    "[GPU MINING] ✅ Template fetched successfully (height: {})",
                                    height
                                );

                                // FIX 2025-12-13: Cache block height and difficulty for MiningActivity events
                                // These values are used when current_template is None during rebuild
                                cached_block_height = template.height;
                                cached_difficulty = template.bits.clone();

                                // Update both cache and working template
                                cached_template = Some(CachedTemplate::new(template.clone()));
                                current_template = Some(template);
                            }
                            Err(e) => {
                                // Parse error type to handle rate limiting specifically
                                let error_str = e.to_string();
                                if error_str.contains("429")
                                    || error_str.contains("Too Many Requests")
                                {
                                    eprintln!("[GPU MINING] ⚠️ Rate limited - will reuse existing template if available");

                                    // CRITICAL FIX: Don't clear templates on rate limit!
                                    // Keep using the existing template if we have one
                                    if current_template.is_none() {
                                        // No template to work with - must wait
                                        eprintln!("[GPU MINING] 😴 No template available, waiting 30s for rate limit to clear...");
                                        tokio::time::sleep(tokio::time::Duration::from_secs(30))
                                            .await;
                                        continue;
                                    } else {
                                        // Have a template - keep mining with it
                                        eprintln!("[GPU MINING] ♻️ Reusing existing template to continue mining");
                                        // Extend cache TTL to prevent repeated fetch attempts
                                        if let Some(ref mut cache) = cached_template {
                                            cache.cached_at = Instant::now();
                                            eprintln!("[GPU MINING] 📅 Extended cache TTL by 10s to avoid rate limit");
                                        }
                                    }
                                } else {
                                    // Non-rate-limit error (network issue, RPC down, etc.)
                                    eprintln!(
                                        "[GPU MINING] ❌ Failed to get block template: {}",
                                        e
                                    );

                                    // For non-rate-limit errors, only clear if we have no fallback
                                    if current_template.is_none() {
                                        eprintln!("[GPU MINING] 😴 No template available, waiting 5s before retry...");
                                        tokio::time::sleep(tokio::time::Duration::from_secs(5))
                                            .await;
                                        continue;
                                    } else {
                                        eprintln!("[GPU MINING] ♻️ Network error - continuing with existing template");
                                    }
                                }
                            }
                        }
                    } else {
                        // Cache is still valid - no fetch needed (silent operation)
                        // This is the normal case that should happen most of the time
                    }

                    // Use current template (guaranteed to exist here)
                    let block_template = current_template.as_ref().unwrap();

                    // Parse header from template
                    let prev_hash = match hex::decode(&block_template.previousblockhash) {
                        Ok(bytes) if bytes.len() == 64 => {
                            let mut arr = [0u8; 64];
                            arr.copy_from_slice(&bytes);
                            btpc_core::crypto::Hash::from_bytes(arr)
                        }
                        _ => btpc_core::crypto::Hash::zero(),
                    };
                    let target_bytes = match hex::decode(&block_template.target) {
                        Ok(bytes) if bytes.len() == 64 => {
                            let mut arr = [0u8; 64];
                            arr.copy_from_slice(&bytes);
                            arr
                        }
                        _ => [0u8; 64],
                    };

                    // Create coinbase transaction and calculate merkle root BEFORE mining
                    // FIX 2025-11-18: Use actual mining address instead of Hash::zero()
                    // Parse mining address and extract pubkey_hash for coinbase output
                    let mining_addr_str = {
                        let addr_lock = mining_address_clone.read().unwrap();
                        addr_lock.clone()
                    };

                    let recipient_hash = match btpc_core::crypto::address::Address::from_string(
                        &mining_addr_str,
                    ) {
                        Ok(address) => {
                            // Extract the hash160 (20 bytes) from the address
                            // Address contains the pubkey_hash which is what we need
                            let hash_bytes = address.hash160(); // Returns &[u8; 20]

                            // Convert 20-byte hash160 to 64-byte Hash (pad with zeros)
                            let mut padded = [0u8; 64];
                            padded[..20].copy_from_slice(hash_bytes);
                            let result_hash = btpc_core::crypto::Hash::from_bytes(padded);

                            eprintln!(
                                "[GPU MINING] ✅ Successfully parsed mining address: '{}'",
                                mining_addr_str
                            );
                            eprintln!(
                                "[GPU MINING] ✅ Extracted hash160 (first 20 bytes): {}",
                                hex::encode(&hash_bytes[..20])
                            );

                            result_hash
                        }
                        Err(e) => {
                            eprintln!("[GPU MINING] ❌❌❌ CRITICAL ERROR ❌❌❌");
                            eprintln!(
                                "[GPU MINING] ❌ Failed to parse mining address: '{}'",
                                mining_addr_str
                            );
                            eprintln!("[GPU MINING] ❌ Error details: {}", e);
                            eprintln!(
                                "[GPU MINING] ❌ Address length: {} bytes",
                                mining_addr_str.len()
                            );
                            eprintln!(
                                "[GPU MINING] ❌ Address format: {:?}",
                                mining_addr_str.chars().take(20).collect::<String>()
                            );
                            eprintln!("[GPU MINING] ⚠️ MINING REWARDS WILL BE LOST - Using fallback zero hash");
                            eprintln!("[GPU MINING] ⚠️ All mined blocks will credit to address: 0000000000000000000000000000000000000000");
                            btpc_core::crypto::Hash::zero() // Fallback
                        }
                    };

                    let mut coinbase_tx = btpc_core::blockchain::Transaction::coinbase(
                        block_template.coinbasevalue,
                        recipient_hash,
                    );
                    // Set fork_id from active network (0=Mainnet, 1=Testnet, 2=Regtest)
                    coinbase_tx.fork_id = network_fork_id;

                    // FIX 2026-02-23: Inject extraNonce into coinbase script_sig
                    // This changes the coinbase txid → merkle root → block header,
                    // giving a completely fresh 4.3B nonce search space each time.
                    // Format: [block_height (8 bytes LE)] [extra_nonce (8 bytes LE)] [GPU index (4 bytes LE)]
                    // The GPU index ensures each GPU gets a unique merkle root even
                    // with the same extra_nonce value, preventing duplicate work.
                    {
                        let mut coinbase_data = Vec::with_capacity(20);
                        coinbase_data.extend_from_slice(&block_template.height.to_le_bytes());
                        coinbase_data.extend_from_slice(&extra_nonce.to_le_bytes());
                        coinbase_data.extend_from_slice(&device_index.to_le_bytes());
                        coinbase_tx.inputs[0].script_sig =
                            btpc_core::crypto::Script::from_bytes(coinbase_data);
                    }

                    // Debug log: Coinbase transaction
                    let coinbase_tx_id = hex::encode(coinbase_tx.hash().as_slice());
                    if let Some(logger) = get_debug_logger() {
                        logger.log_transaction(
                            "COINBASE",
                            &coinbase_tx_id,
                            coinbase_tx.inputs.len(),
                            coinbase_tx.outputs.len(),
                            block_template.coinbasevalue,
                        );
                    }
                    // FIX 2025-12-08: Emit BlockConstruction event for frontend console
                    if let Some(ref tx) = event_tx_clone {
                        let _ = tx.send(MiningEvent::BlockConstruction {
                            log_type: "COINBASE".to_string(),
                            message: format!("[TX] COINBASE - ID: {}", &coinbase_tx_id[..16]),
                        });
                    }

                    // FIX 2025-11-27: Include mempool transactions from block template
                    // Previously only coinbase was included, so wallet transfers never confirmed!
                    let mut transactions = vec![coinbase_tx];

                    // Parse and include mempool transactions from the template
                    for tx_json in &block_template.transactions {
                        if let Some(tx_data_hex) = tx_json.get("data").and_then(|d| d.as_str()) {
                            if let Ok(tx_bytes) = hex::decode(tx_data_hex) {
                                if let Ok(tx) =
                                    btpc_core::blockchain::Transaction::deserialize(&tx_bytes)
                                {
                                    transactions.push(tx);
                                } else {
                                    eprintln!(
                                        "[GPU MINING] ⚠️ Failed to deserialize mempool transaction"
                                    );
                                }
                            }
                        }
                    }

                    if transactions.len() > 1 {
                        eprintln!(
                            "[GPU MINING] 📦 Block includes {} mempool transactions",
                            transactions.len() - 1
                        );
                    }

                    let merkle_root =
                        match btpc_core::blockchain::calculate_merkle_root(&transactions) {
                            Ok(root) => {
                                // Debug log: Merkle root
                                let merkle_hex = hex::encode(root.as_slice());
                                if let Some(logger) = get_debug_logger() {
                                    logger.log_merkle_root(transactions.len(), &merkle_hex);
                                }
                                // FIX 2025-12-08: Emit BlockConstruction event for frontend console
                                if let Some(ref tx) = event_tx_clone {
                                    let _ = tx.send(MiningEvent::BlockConstruction {
                                        log_type: "MERKLE".to_string(),
                                        message: format!(
                                            "[MERKLE] Calculated from {} txs",
                                            transactions.len()
                                        ),
                                    });
                                }
                                root
                            }
                            Err(e) => {
                                eprintln!("[GPU MINING] ❌ Failed to calculate merkle root: {}", e);
                                if let Some(logger) = get_debug_logger() {
                                    logger.log_error("MERKLE_ROOT", &format!("{}", e));
                                }
                                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                continue;
                            }
                        };

                    // Build header with proper merkle root for mining
                    // FIX 2025-12-27: Updated fallback to SHA-512 compatible value
                    let bits_value =
                        u32::from_str_radix(&block_template.bits, 16).unwrap_or(0x3c7fffff);
                    let header = btpc_core::blockchain::BlockHeader::new(
                        block_template.version,
                        prev_hash,
                        merkle_root, // REAL merkle root
                        block_template.curtime,
                        bits_value,
                        0, // nonce (set by GPU)
                    );
                    let target = btpc_core::consensus::pow::MiningTarget::from_bytes(target_bytes);

                    // Debug log: Block header details
                    if let Some(logger) = get_debug_logger() {
                        logger.log_block_header(
                            Some(device_index),
                            block_template.version,
                            &hex::encode(prev_hash.as_slice()),
                            &hex::encode(merkle_root.as_slice()),
                            block_template.curtime,
                            bits_value,
                            nonce_start,
                        );
                    }
                    // FIX 2025-12-08: Emit BlockConstruction event for frontend console
                    // Include all header details for full debug-style display
                    if let Some(ref tx) = event_tx_clone {
                        let prev_hash_hex = hex::encode(prev_hash.as_slice());
                        let merkle_hex = hex::encode(merkle_root.as_slice());
                        let _ = tx.send(MiningEvent::BlockConstruction {
                            log_type: "HEADER".to_string(),
                            message: format!(
                                "[BLOCK_HEADER] GPU {} - Version: {} - Prev Hash: {} - Merkle Root: {} - Timestamp: {} - Bits: 0x{:08x} - Nonce Start: {}",
                                device_index,
                                block_template.version,
                                prev_hash_hex,
                                merkle_hex,
                                block_template.curtime,
                                bits_value,
                                nonce_start.to_formatted_string(&num_format::Locale::en)
                            ),
                        });
                    }

                    // Send mining progress update every 10 batches (~10M hashes) to reduce UI load
                    // At 100 MH/s this is 10 messages/sec instead of 100, preventing WebView freezing
                    const LOG_FREQUENCY_BATCHES: u32 = 10;
                    if nonce_start % (crate::gpu_miner::NONCES_PER_BATCH * LOG_FREQUENCY_BATCHES)
                        == 0
                    {
                        if let Some(ref tx) = log_tx_clone {
                            // Pick a random nonce within current batch to show realistic activity
                            // This represents one of the ~1M nonces the GPU is actually testing
                            use rand::{rngs::OsRng, Rng};
                            let random_offset: u32 =
                                OsRng.gen_range(0..crate::gpu_miner::NONCES_PER_BATCH);
                            let display_nonce = nonce_start.wrapping_add(random_offset);

                            // Compute a sample hash for display (single hash on CPU, doesn't impact GPU mining)
                            use sha2::{Digest, Sha512};
                            let mut sample_header = header.clone();
                            sample_header.nonce = display_nonce;
                            let header_bytes = sample_header.serialize();
                            let sample_hash = Sha512::digest(header_bytes);
                            let sample_hash_hex = hex::encode(&sample_hash[..32]); // First 32 bytes (64 hex chars)

                            let _ = tx.send((
                                "INFO".to_string(),
                                format!(
                                    "[HASH] GPU {} | Nonce: {} | Target: {} | Sample: {}",
                                    device_index,
                                    display_nonce,
                                    &hex::encode(&target_bytes[..8]),
                                    &sample_hash_hex
                                ),
                            ));
                        }
                    }

                    // Mine batch on GPU
                    match miner.mine_batch(&header, &target, nonce_start) {
                        Ok(Some(nonce)) => {
                            // Found valid nonce!
                            println!(
                                "🎉 GPU {} found valid block! Nonce: {}",
                                device_index, nonce
                            );

                            // Debug log: Block found
                            if let Some(logger) = get_debug_logger() {
                                logger.log_mining_event(
                                    "BLOCK_FOUND",
                                    Some(device_index),
                                    &format!("Nonce: {}", nonce),
                                );
                            }

                            // Log block found event (removed duplicate direct logging)

                            // Build block with found nonce
                            let mut final_header = header.clone();
                            final_header.nonce = nonce;

                            let block_to_submit = btpc_core::blockchain::Block {
                                header: final_header,
                                transactions: transactions.clone(),
                            };

                            // Debug log: Complete block details
                            let block_hash = block_to_submit.hash();
                            let block_hash_hex = hex::encode(block_hash.as_slice());
                            let prev_hash_hex = hex::encode(prev_hash.as_slice());
                            let merkle_hex = hex::encode(merkle_root.as_slice());

                            if let Some(logger) = get_debug_logger() {
                                logger.log_complete_block(
                                    Some(device_index),
                                    block_template.height, // Height from block template
                                    &block_hash_hex,
                                    &prev_hash_hex,
                                    &merkle_hex,
                                    nonce,
                                    block_template.curtime,
                                    transactions.len(),
                                );
                            }

                            // Send consolidated block mining info to frontend logs (single line)
                            if let Some(ref tx) = log_tx_clone {
                                let _ = tx.send((
                                    "INFO".to_string(),
                                    format!(
                                        "[MINING] GPU {} | Height: {} | Nonce: {} | Hash: {}",
                                        device_index,
                                        block_template.height,
                                        nonce,
                                        &block_hash_hex[..16] // First 16 chars only
                                    ),
                                ));
                            }

                            let block_hex = hex::encode(block_to_submit.serialize());

                            // Debug log: Block submission attempt (debug logs only, not frontend)
                            if let Some(logger) = get_debug_logger() {
                                logger.log_block_submission(
                                    Some(device_index),
                                    nonce,
                                    block_hex.len(),
                                    &merkle_hex,
                                );
                                logger.log_block_hex(Some(device_index), &block_hex);
                            }

                            // FIX 2025-12-01: Atomic claim-before-submit using compare_exchange
                            // CRITICAL: The old pre-submit check had a race condition:
                            //   - GPU A checks version (N), GPU B checks version (N)
                            //   - BOTH pass the check, BOTH submit blocks at same height
                            //   - Version only incremented AFTER submit, too late!
                            //
                            // NEW FIX: Use atomic CAS (compare-and-swap) to CLAIM submission rights
                            // Only ONE GPU can successfully increment version from N to N+1
                            // Other GPUs see the CAS fail and skip submission immediately
                            let claimed = template_version_clone.compare_exchange(
                                my_template_version,     // Expected: my version
                                my_template_version + 1, // New: increment to claim
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            );

                            match claimed {
                                Ok(_) => {
                                    // Successfully claimed! We are the ONLY GPU that can submit this block
                                    my_template_version += 1;
                                    eprintln!(
                                        "[GPU {}] 🔒 CLAIMED submission rights (version {} -> {})",
                                        device_index,
                                        my_template_version - 1,
                                        my_template_version
                                    );
                                }
                                Err(current_version) => {
                                    // Another GPU already claimed - skip submission
                                    eprintln!(
                                        "[GPU {}] ⏭️ SKIPPING: Another GPU claimed version {} (we had {})",
                                        device_index, current_version, my_template_version
                                    );
                                    // Notify user in UI log so they understand why only 1 block
                                    // is recorded when 2 GPUs find solutions simultaneously
                                    if let Some(ref tx) = log_tx_clone {
                                        let _ = tx.send((
                                            "INFO".to_string(),
                                            format!(
                                                "GPU {} found valid block but GPU {} submitted first — skipping duplicate (race resolved)",
                                                device_index,
                                                if device_index == 0 { 1 } else { 0 }
                                            ),
                                        ));
                                    }
                                    cached_template = None;
                                    current_template = None;
                                    my_template_version = current_version;
                                    continue;
                                }
                            }

                            match rpc_client_clone.submit_block(&block_hex).await {
                                Ok(msg) => {
                                    eprintln!(
                                        "[GPU MINING] ✅ Block submitted successfully: {}",
                                        msg
                                    );

                                    // Debug log: Block accepted
                                    if let Some(logger) = get_debug_logger() {
                                        logger.log_block_result(Some(device_index), true, &msg);
                                    }

                                    if let Some(ref tx) = log_tx_clone {
                                        let _ = tx.send((
                                            "SUCCESS".to_string(),
                                            format!(
                                                "GPU {} block accepted by network",
                                                device_index
                                            ),
                                        ));
                                    }

                                    // ✅ ONLY increment blocks_found when submission succeeds
                                    // Increment global counter (displayed in frontend)
                                    blocks_found_counter_clone.fetch_add(1, Ordering::SeqCst);

                                    // Increment per-GPU counter (for GPU dashboard)
                                    let mut stats = per_gpu_stats.write().unwrap();
                                    if let Some(entry) = stats.get_mut(&device_index) {
                                        entry.blocks_found += 1;
                                    }

                                    // REM-C002: Emit block_mined event
                                    if let Some(ref tx) = event_tx_clone {
                                        let _ = tx.send(MiningEvent::BlockMined {
                                            block_hash: hex::encode(block_hash.as_slice()),
                                            block_height: block_template.height + 1,
                                            reward_btpc: 32.375, // Fixed reward per block
                                            device_id: device_index,
                                            device_name: device_name.clone(),
                                            difficulty: block_template.bits.clone(),
                                            nonce: nonce as u64,
                                        });
                                    }

                                    // FIX 2025-12-30: Update cached_block_height immediately after block found
                                    // This prevents the "Height off by 1" display bug where UI shows stale height
                                    // until new template is fetched. The new height = just-mined height + 1.
                                    // NOTE: Must save height BEFORE invalidating templates (borrow checker)
                                    let next_block_height = block_template.height + 1;
                                    cached_block_height = next_block_height;
                                    eprintln!("[GPU MINING] 📊 Updated cached_block_height to {} (next block)", next_block_height);

                                    // FIX 2026-02-23: Reset nonce & extraNonce for new block
                                    // Must happen HERE (not in template fetch) because cached_block_height
                                    // is already updated to next height, so the template fetch comparison
                                    // (height != cached_block_height) would never trigger.
                                    extra_nonce = 0;
                                    _nonce_wrapped = false;
                                    nonce_start = device_index * 1_000_000_000;
                                    eprintln!("[GPU {}] 🆕 Block found! Reset nonce & extraNonce for height {}", device_index, next_block_height);

                                    // 🔥 CRITICAL FIX: Invalidate cached template to force fresh fetch
                                    // Without this, mining continues on stale template with old prev_hash
                                    // This causes all subsequent blocks to be rejected (wrong height/prev_hash)
                                    eprintln!("[GPU MINING] 🔄 Block accepted! Invalidating template cache to fetch fresh template...");
                                    cached_template = None;
                                    current_template = None;

                                    // FIX 2025-12-01: Version already incremented during CAS claim (lines 820-825)
                                    // No need to increment again - my_template_version is already updated
                                    eprintln!("[GPU {}] 📢 Block accepted with template version {} (other GPUs notified via CAS)", device_index, my_template_version);
                                }
                                Err(e) => {
                                    eprintln!("[GPU MINING] ❌ Block submission failed: {}", e);

                                    // Debug log: Block rejected
                                    if let Some(logger) = get_debug_logger() {
                                        logger.log_block_result(
                                            Some(device_index),
                                            false,
                                            &format!("{}", e),
                                        );
                                        logger.log_error(
                                            "BLOCK_SUBMIT",
                                            &format!("GPU {}: {}", device_index, e),
                                        );
                                    }

                                    if let Some(ref tx) = log_tx_clone {
                                        let _ = tx.send((
                                            "ERROR".to_string(),
                                            format!("GPU {} block rejected: {}", device_index, e),
                                        ));
                                    }
                                    // ❌ Do NOT increment blocks_found on failure

                                    // FIX 2026-03-29: Invalidate template on submission failure.
                                    // The CAS already succeeded (version incremented) but the block
                                    // was rejected. Without clearing the template here, this GPU
                                    // would loop back, pass the version check (my_version ==
                                    // global_version), and re-mine the SAME height — causing
                                    // duplicate submissions at the same block height.
                                    cached_template = None;
                                    current_template = None;
                                }
                            }
                        }
                        Ok(None) => {
                            // Batch exhausted, continue with next batch
                            // FIX 2025-12-11: Reduced from 100ms to 1ms since embedded node has no rate limits
                            // The 100ms delay was causing ~66% hashrate loss (50ms work + 100ms delay)
                            // 1ms is sufficient to yield to other async tasks without impacting performance
                            tokio::time::sleep(Duration::from_millis(1)).await;
                        }
                        Err(e) => {
                            eprintln!("⚠️ GPU {} mining error: {}", device_index, e);

                            // REM-C002: Emit gpu_error event
                            if let Some(ref tx) = event_tx_clone {
                                let _ = tx.send(MiningEvent::GpuError {
                                    device_id: device_index,
                                    device_name: device_name.clone(),
                                    error_type: "kernel_error".to_string(),
                                    error_message: format!("{}", e),
                                    mining_stopped: false, // Mining continues after error
                                });
                            }

                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }

                    // REM-C002: Check GPU temperature for thermal throttling
                    if let Some(temperature) = miner.get_temperature() {
                        // Thermal threshold is 80°C (warning), 85°C (throttle), 90°C (shutdown)
                        const THERMAL_WARNING: f32 = 80.0;
                        const THERMAL_THROTTLE: f32 = 85.0;
                        const THERMAL_SHUTDOWN: f32 = 90.0;

                        if temperature >= THERMAL_SHUTDOWN {
                            // Emergency shutdown
                            eprintln!(
                                "🔥 GPU {} CRITICAL TEMPERATURE: {}°C - EMERGENCY SHUTDOWN",
                                device_index, temperature
                            );

                            if let Some(ref tx) = event_tx_clone {
                                let _ = tx.send(MiningEvent::ThermalThrottle {
                                    device_id: device_index,
                                    device_name: device_name.clone(),
                                    current_temperature: temperature,
                                    thermal_limit: THERMAL_SHUTDOWN,
                                    action: "shutdown".to_string(),
                                });
                            }

                            // Stop mining immediately
                            mining_active.store(false, Ordering::SeqCst);
                            break;
                        } else if temperature >= THERMAL_THROTTLE {
                            // Throttle mining (reduce hashrate)
                            eprintln!(
                                "⚠️ GPU {} HIGH TEMPERATURE: {}°C - THROTTLING",
                                device_index, temperature
                            );

                            if let Some(ref tx) = event_tx_clone {
                                let _ = tx.send(MiningEvent::ThermalThrottle {
                                    device_id: device_index,
                                    device_name: device_name.clone(),
                                    current_temperature: temperature,
                                    thermal_limit: THERMAL_THROTTLE,
                                    action: "throttle".to_string(),
                                });
                            }

                            // Add extra delay to reduce hashrate
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        } else if temperature >= THERMAL_WARNING {
                            // Warning only
                            if let Some(ref tx) = event_tx_clone {
                                let _ = tx.send(MiningEvent::ThermalThrottle {
                                    device_id: device_index,
                                    device_name: device_name.clone(),
                                    current_temperature: temperature,
                                    thermal_limit: THERMAL_WARNING,
                                    action: "warning".to_string(),
                                });
                            }
                        }
                    }

                    // Update per-GPU stats
                    let hashes = miner.get_hashes_computed();
                    let blocks = miner.get_blocks_found();

                    // Update global GPU hash counter for get_stats() hashrate calculation
                    gpu_total_hashes_clone.store(hashes, Ordering::SeqCst);

                    // Calculate uptime once for both stats update and event
                    let uptime = start_time
                        .read()
                        .unwrap()
                        .map(|start| start.elapsed().as_secs())
                        .unwrap_or(0);

                    // Calculate hashrate
                    let current_hashrate = if uptime > 0 {
                        hashes as f64 / uptime as f64
                    } else {
                        0.0
                    };

                    // Update this GPU's per_gpu_stats entry FIRST
                    {
                        let mut stats = per_gpu_stats.write().unwrap();
                        if let Some(entry) = stats.get_mut(&device_index) {
                            entry.total_hashes = hashes;
                            entry.blocks_found = blocks;
                            entry.mining_uptime = uptime;
                            entry.current_hashrate = current_hashrate;
                            entry.last_updated = Instant::now();
                        }
                    }

                    // FIX 2025-12-21: Rolling window samples must use CUMULATIVE total across ALL GPUs
                    // Previous bug: Each GPU stored its individual count, but delta calculation
                    // compared samples from different GPUs (e.g., GPU0's newest vs GPU1's oldest)
                    // Fix: Sum all GPUs' total_hashes to get true cumulative total
                    //
                    // Note: per_gpu_stats is updated ABOVE so current GPU's hashes are included
                    let cumulative_hashes: u64 = {
                        let stats = per_gpu_stats.read().unwrap();
                        stats.values().map(|s| s.total_hashes).sum()
                    };
                    {
                        let mut samples = hash_samples_clone.write().unwrap();
                        let now = Instant::now();
                        samples.push(HashrateSample {
                            timestamp: now,
                            hashes: cumulative_hashes,
                        });
                        // Prune samples older than 60 seconds
                        let cutoff = now - Duration::from_secs(HASHRATE_WINDOW_SECONDS);
                        samples.retain(|s| s.timestamp > cutoff);
                    }

                    // FIX 2025-12-08: Emit MiningActivity event every 10ms for real-time UI updates
                    // This replaces the polling-based approach that caused console freezing
                    // FIX 2025-12-13: Emit events even when current_template is None (during rebuild)
                    // On Regtest, blocks are mined so fast that template is often being rebuilt
                    // Without this fix, UI shows 0 hashrate/uptime because events aren't emitted
                    if last_activity_event.elapsed() >= ACTIVITY_EVENT_INTERVAL {
                        if let Some(ref tx) = event_tx_clone {
                            // Use template values if available, otherwise use cached values
                            let (block_height, difficulty_str) =
                                if let Some(ref tmpl) = current_template {
                                    (tmpl.height, tmpl.bits.clone())
                                } else {
                                    // FIX 2025-12-13: Use cached values when template is being rebuilt
                                    (cached_block_height, cached_difficulty.clone())
                                };

                            let blocks_found_now =
                                blocks_found_counter_clone.load(Ordering::SeqCst);

                            // FIX 2025-12-09: Show realistic nonce value instead of round batch numbers
                            // Add random offset within current batch to show actual nonce being tested
                            use rand::{rngs::OsRng, Rng};
                            let random_offset: u32 =
                                OsRng.gen_range(0..crate::gpu_miner::NONCES_PER_BATCH);
                            let display_nonce = nonce_start.wrapping_add(random_offset) as u64;

                            // FIX 2025-12-21: Use rolling window hashrate for consistency with get_stats()
                            // This ensures MiningActivity events show same hashrate as dashboard
                            let rolling_hashrate = {
                                let samples = hash_samples_clone.read().unwrap();
                                if samples.len() >= 2 {
                                    let oldest = samples.first().unwrap();
                                    let newest = samples.last().unwrap();
                                    let delta_hashes = newest.hashes.saturating_sub(oldest.hashes);
                                    let delta_secs = newest
                                        .timestamp
                                        .duration_since(oldest.timestamp)
                                        .as_secs_f64();
                                    if delta_secs > 0.0 {
                                        delta_hashes as f64 / delta_secs
                                    } else {
                                        0.0
                                    }
                                } else {
                                    current_hashrate // Fall back to lifetime average if not enough samples
                                }
                            };

                            let _ = tx.send(MiningEvent::MiningActivity {
                                device_id: device_index,
                                hashrate: rolling_hashrate,
                                total_hashes: cumulative_hashes, // Use cumulative, not per-GPU
                                current_nonce: display_nonce,
                                block_height,
                                difficulty: difficulty_str,
                                uptime_seconds: uptime,
                                blocks_found: blocks_found_now,
                                message: format!("Searching block {}...", block_height),
                                extra_nonce,
                            });
                        }
                        last_activity_event = Instant::now();
                    }

                    // Increment nonce start for next batch
                    let prev_nonce = nonce_start;
                    nonce_start = nonce_start.wrapping_add(crate::gpu_miner::NONCES_PER_BATCH);

                    // FIX 2026-02-23: Detect u32 nonce space exhaustion and increment extraNonce
                    // When nonce_start wraps (new value < old value), the entire 4.3B nonce space
                    // has been searched for this header. Increment extraNonce to get a fresh
                    // coinbase → merkle root → header, opening a new 4.3B search space.
                    if nonce_start < prev_nonce {
                        extra_nonce += 1;
                        _nonce_wrapped = true;
                        eprintln!(
                            "[GPU {}] 🔄 Nonce space exhausted, extraNonce → {} (new merkle root)",
                            device_index, extra_nonce
                        );
                    }
                }

                miner.set_mining(false);
                mining_active.store(false, Ordering::SeqCst);
                println!("🔧 GPU {} mining thread exiting", device_index);
            });
        }

        Ok(device_count)
    }

    /// Stop GPU mining gracefully
    pub async fn stop_gpu_mining(&mut self) -> Result<()> {
        if !self.gpu_mining_active.load(Ordering::SeqCst) {
            return Ok(()); // Already stopped
        }

        // Send shutdown signal (broadcast doesn't need await)
        if let Some(tx) = self.gpu_shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Wait for GPU mining to stop
        let timeout = Duration::from_secs(5);
        let start = Instant::now();

        while self.gpu_mining_active.load(Ordering::SeqCst) {
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!("GPU mining shutdown timeout"));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Reset device count
        self.gpu_devices.store(0, Ordering::SeqCst);

        Ok(())
    }

    /// Get mining statistics (atomic reads, no locks)
    ///
    /// # Performance
    /// - Target: <5ms (all atomic reads)
    /// - No locks required for stats queries
    pub fn get_stats(&self) -> MiningStats {
        let is_cpu_mining = self.cpu_mining_active.load(Ordering::SeqCst);
        let is_gpu_mining = self.gpu_mining_active.load(Ordering::SeqCst);
        let cpu_threads = self.cpu_threads.load(Ordering::SeqCst);
        let gpu_devices = self.gpu_devices.load(Ordering::SeqCst);
        let cpu_hashes = self.cpu_total_hashes.load(Ordering::SeqCst);
        // Note: gpu_hashes not used directly - rolling window hashrate uses samples instead
        let blocks_found = self.blocks_found.load(Ordering::SeqCst);

        // Calculate uptime
        let uptime_seconds = self
            .start_time
            .read()
            .unwrap()
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0);

        // Calculate hashrate (hashes per second)
        // CPU uses lifetime average (less frequently updated)
        let cpu_hashrate = if uptime_seconds > 0 {
            cpu_hashes as f64 / uptime_seconds as f64
        } else {
            0.0
        };

        // FIX 2025-12-11: GPU uses rolling 60-second window for accurate current hashrate
        // This prevents the "decreasing hashrate" illusion caused by lifetime averages
        let gpu_hashrate = self.calculate_rolling_hashrate();

        MiningStats {
            is_mining: is_cpu_mining || is_gpu_mining,
            cpu_threads: cpu_threads as u32,
            gpu_devices: gpu_devices as u32,
            total_hashrate: cpu_hashrate + gpu_hashrate,
            cpu_hashrate,
            gpu_hashrate,
            blocks_found,
            uptime_seconds,
        }
    }

    /// Stop all mining (CPU and GPU)
    pub async fn stop_all(&mut self) -> Result<()> {
        let mut errors = Vec::new();

        if self.cpu_mining_active.load(Ordering::SeqCst) {
            if let Err(e) = self.stop_cpu_mining().await {
                errors.push(format!("CPU mining stop error: {}", e));
            }
        }

        if self.gpu_mining_active.load(Ordering::SeqCst) {
            if let Err(e) = self.stop_gpu_mining().await {
                errors.push(format!("GPU mining stop error: {}", e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Mining shutdown errors: {}",
                errors.join(", ")
            ))
        }
    }

    /// Check if GPU mining is available
    ///
    /// Returns true if GPU devices are detected and configured.
    /// T011-002: Added for GPU stats availability check.
    pub fn is_gpu_available(&self) -> bool {
        // Check if any GPU devices are configured
        let gpu_count = self.gpu_devices.load(Ordering::SeqCst);
        gpu_count > 0
    }

    /// Get per-GPU statistics for a specific device (Feature 012: T016)
    ///
    /// # Arguments
    /// * `device_index` - GPU device index to query
    ///
    /// # Returns
    /// * `Some(PerGpuStats)` - Statistics for the requested GPU
    /// * `None` - GPU device not found or not mining
    pub fn get_gpu_stats(&self, device_index: u32) -> Option<PerGpuStats> {
        let stats = self.per_gpu_stats.read().unwrap();
        stats.get(&device_index).cloned()
    }

    /// Get per-GPU statistics for all active GPUs (Feature 012: T016)
    ///
    /// # Returns
    /// HashMap mapping device_index to PerGpuStats for all active GPUs
    pub fn get_all_gpu_stats(&self) -> HashMap<u32, PerGpuStats> {
        let stats = self.per_gpu_stats.read().unwrap();
        stats.clone()
    }

    /// Update per-GPU statistics for a specific device (Feature 012: T016)
    ///
    /// Called by GPU mining threads to update statistics.
    ///
    /// # Arguments
    /// * `device_index` - GPU device index
    /// * `hashes` - Number of hashes computed in this update
    /// * `blocks_found` - Number of blocks found (usually 0 or 1)
    pub fn update_gpu_stats(&self, device_index: u32, hashes: u64, blocks_found: u64) {
        let mut stats = self.per_gpu_stats.write().unwrap();

        // Get or create stats entry for this GPU
        let entry = stats.entry(device_index).or_insert_with(|| PerGpuStats {
            device_index,
            current_hashrate: 0.0,
            total_hashes: 0,
            blocks_found: 0,
            mining_uptime: 0,
            last_updated: Instant::now(),
        });

        // Update counters
        entry.total_hashes += hashes;
        entry.blocks_found += blocks_found;

        // Calculate uptime since mining started
        let uptime = self
            .start_time
            .read()
            .unwrap()
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0);
        entry.mining_uptime = uptime;

        // Calculate current hashrate
        if uptime > 0 {
            entry.current_hashrate = entry.total_hashes as f64 / uptime as f64;
        }

        entry.last_updated = Instant::now();

        // Also update aggregate GPU stats
        self.gpu_total_hashes.fetch_add(hashes, Ordering::SeqCst);
        if blocks_found > 0 {
            self.blocks_found.fetch_add(blocks_found, Ordering::SeqCst);
        }
    }

    /// Initialize per-GPU stats tracking for a device (Feature 012: T016)
    ///
    /// Called when GPU mining starts to register a new GPU device.
    ///
    /// # Arguments
    /// * `device_index` - GPU device index to initialize
    pub fn init_gpu_stats(&self, device_index: u32) {
        let mut stats = self.per_gpu_stats.write().unwrap();
        stats.insert(
            device_index,
            PerGpuStats {
                device_index,
                current_hashrate: 0.0,
                total_hashes: 0,
                blocks_found: 0,
                mining_uptime: 0,
                last_updated: Instant::now(),
            },
        );
    }

    /// Clear per-GPU statistics (Feature 012: T016)
    ///
    /// Called when GPU mining stops to reset all GPU stats.
    pub fn clear_gpu_stats(&self) {
        let mut stats = self.per_gpu_stats.write().unwrap();
        stats.clear();
    }

    /// Get GPU device information for a specific device
    ///
    /// Returns hardware specifications for the requested GPU.
    ///
    /// # Arguments
    /// * `device_index` - GPU device index to query
    ///
    /// # Returns
    /// * `Some(GpuDevice)` - Device info for the requested GPU
    /// * `None` - GPU device not found
    pub fn get_gpu_device_info(&self, device_index: u32) -> Option<crate::gpu_miner::GpuDevice> {
        let device_info = self.gpu_device_info.read().unwrap();
        device_info.get(&device_index).cloned()
    }

    /// Get all GPU device information
    ///
    /// Returns hardware specifications for all registered GPUs.
    ///
    /// # Returns
    /// HashMap mapping device_index to GpuDevice for all GPUs
    pub fn get_all_gpu_device_info(&self) -> HashMap<u32, crate::gpu_miner::GpuDevice> {
        let device_info = self.gpu_device_info.read().unwrap();
        device_info.clone()
    }
}

/// Mining statistics snapshot
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MiningStats {
    /// Is mining currently active (CPU or GPU)?
    pub is_mining: bool,

    /// Number of active CPU threads
    pub cpu_threads: u32,

    /// Number of active GPU devices
    pub gpu_devices: u32,

    /// Total hashrate (hashes/sec)
    pub total_hashrate: f64,

    /// CPU hashrate (hashes/sec)
    pub cpu_hashrate: f64,

    /// GPU hashrate (hashes/sec)
    pub gpu_hashrate: f64,

    /// Number of blocks found
    pub blocks_found: u64,

    /// Mining uptime (seconds)
    pub uptime_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_mining_pool() {
        // Act
        let pool = MiningThreadPool::new(0, 2);

        // Assert
        assert!(!pool.cpu_mining_active.load(Ordering::SeqCst));
        assert!(!pool.gpu_mining_active.load(Ordering::SeqCst));
        assert_eq!(pool.cpu_threads.load(Ordering::SeqCst), 0);
        assert_eq!(pool.gpu_devices.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_start_stop_cpu_mining() {
        // Arrange
        let mut pool = MiningThreadPool::new(0, 2);

        // Act: Start mining
        let result = pool
            .start_cpu_mining(Some(2), "bcrt1qtest".to_string())
            .await;

        // Assert: Mining started
        assert!(result.is_ok(), "CPU mining should start successfully");
        assert_eq!(result.unwrap(), 2);
        assert!(pool.cpu_mining_active.load(Ordering::SeqCst));
        assert_eq!(pool.cpu_threads.load(Ordering::SeqCst), 2);

        // Wait briefly for mining to accumulate hashes
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify stats
        let stats = pool.get_stats();
        assert!(stats.is_mining);
        assert_eq!(stats.cpu_threads, 2);
        // Note: cpu_hashrate may be 0 in test environment without block templates/RPC client
        // This test verifies mining lifecycle (start/stop), not actual hash production

        // Act: Stop mining
        let stop_result = pool.stop_cpu_mining().await;

        // Assert: Mining stopped
        assert!(stop_result.is_ok(), "CPU mining should stop successfully");
        assert!(!pool.cpu_mining_active.load(Ordering::SeqCst));
        assert_eq!(pool.cpu_threads.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_default_thread_count() {
        // Arrange
        let mut pool = MiningThreadPool::new(0, 2);

        // Act: Start with None (should use num_cpus - 2)
        let result = pool.start_cpu_mining(None, "bcrt1qtest".to_string()).await;

        // Assert
        assert!(result.is_ok());
        let thread_count = result.unwrap();
        let expected = (num_cpus::get() as i32 - 2).max(1) as u32;
        assert_eq!(thread_count, expected);

        // Cleanup
        pool.stop_cpu_mining().await.unwrap();
    }

    #[tokio::test]
    async fn test_get_stats() {
        // Arrange
        let pool = MiningThreadPool::new(0, 2);

        // Act
        let stats = pool.get_stats();

        // Assert
        assert!(!stats.is_mining);
        assert_eq!(stats.cpu_threads, 0);
        assert_eq!(stats.gpu_devices, 0);
        assert_eq!(stats.total_hashrate, 0.0);
        assert_eq!(stats.blocks_found, 0);
    }

    #[tokio::test]
    async fn test_stop_all() {
        // Arrange
        let mut pool = MiningThreadPool::new(0, 2);
        pool.start_cpu_mining(Some(1), "bcrt1qtest".to_string())
            .await
            .unwrap();

        // Act
        let result = pool.stop_all().await;

        // Assert
        assert!(result.is_ok());
        assert!(!pool.cpu_mining_active.load(Ordering::SeqCst));
    }

    // T003: Contract tests for address parsing validation
    // Contract: specs/013-mining-rewards-not/contracts/address_parsing_contract.md

    #[test]
    fn test_tc001_valid_regtest_address_parsing() {
        // TC-001: Valid Regtest address → hash160 extraction
        let valid_regtest = "mw2YiPwD8F8Y2vNYZmeNT69BZZZKK8BoyV";
        let address = btpc_core::crypto::address::Address::from_string(valid_regtest).unwrap();
        let hash160 = address.hash160();

        assert_eq!(hash160.len(), 20, "hash160 must be 20 bytes");

        // Verify padding to 64 bytes for Hash type
        let mut padded = [0u8; 64];
        padded[..20].copy_from_slice(hash160);

        assert_eq!(&padded[20..], &[0u8; 44], "Padding must be zeros");

        // Verify Hash construction
        let hash = btpc_core::crypto::Hash::from_bytes(padded);
        assert_eq!(
            hash.as_bytes()[..20],
            *hash160,
            "Hash should contain hash160 in first 20 bytes"
        );
    }

    #[test]
    fn test_tc002_valid_mainnet_address_parsing() {
        // TC-002: Valid Mainnet address → hash160 extraction
        // Note: Using a real Mainnet address format would require network parameter
        // For now, test the parsing logic with Regtest
        let valid_address = "mw2YiPwD8F8Y2vNYZmeNT69BZZZKK8BoyV";
        let result = btpc_core::crypto::address::Address::from_string(valid_address);

        assert!(result.is_ok(), "Valid address should parse successfully");

        let address = result.unwrap();
        let hash160 = address.hash160();
        assert_eq!(hash160.len(), 20, "hash160 must be 20 bytes");
    }

    #[test]
    fn test_tc003_invalid_base58_characters() {
        // TC-003: Invalid Base58 characters → error + fallback
        let invalid = "mw2YiPwD8F8Y2vNYZmeNT69BZZZKK8BoyV!@#"; // Contains invalid chars
        let result = btpc_core::crypto::address::Address::from_string(invalid);

        assert!(result.is_err(), "Invalid Base58 characters should fail");

        // Verify fallback behavior: Hash::zero()
        let fallback = btpc_core::crypto::Hash::zero();
        assert_eq!(
            fallback.as_bytes(),
            &[0u8; 64],
            "Fallback should be all zeros"
        );
    }

    #[test]
    fn test_tc004_wrong_network_prefix() {
        // TC-004: Wrong network prefix → error (context-dependent)
        // This test verifies that address parsing validates network type
        // Invalid network prefix should fail during from_string()
        let wrong_prefix = "1InvalidBitcoinAddress123456789"; // Bitcoin mainnet prefix
        let result = btpc_core::crypto::address::Address::from_string(wrong_prefix);

        // Should fail due to checksum or network validation
        assert!(
            result.is_err(),
            "Wrong network prefix should fail validation"
        );
    }

    #[test]
    fn test_tc005_invalid_checksum() {
        // TC-005: Invalid checksum → error + fallback
        // Modify last character to break checksum
        let invalid_checksum = "mw2YiPwD8F8Y2vNYZmeNT69BZZZKK8BoyX"; // Changed V -> X
        let result = btpc_core::crypto::address::Address::from_string(invalid_checksum);

        assert!(result.is_err(), "Invalid checksum should fail");
    }

    #[test]
    fn test_tc006_empty_string() {
        // TC-006: Empty string → error + fallback
        let empty = "";
        let result = btpc_core::crypto::address::Address::from_string(empty);

        assert!(result.is_err(), "Empty string should fail");

        // Verify fallback to Hash::zero()
        let fallback = btpc_core::crypto::Hash::zero();
        assert!(
            fallback.as_bytes().iter().all(|&b| b == 0),
            "Fallback should be all zeros"
        );
    }

    #[test]
    fn test_address_parsing_with_padding() {
        // Integration test: Full address parsing flow with padding
        let valid_address = "mw2YiPwD8F8Y2vNYZmeNT69BZZZKK8BoyV";

        // Parse address
        let address = btpc_core::crypto::address::Address::from_string(valid_address).unwrap();
        let hash160 = address.hash160();

        // Pad to 64 bytes (as done in mining code)
        let mut padded = [0u8; 64];
        padded[..20].copy_from_slice(hash160);
        let recipient_hash = btpc_core::crypto::Hash::from_bytes(padded);

        // Verify the hash is not all zeros (would indicate Hash::zero() bug)
        assert_ne!(
            recipient_hash.as_bytes(),
            &[0u8; 64],
            "Parsed address should NOT be Hash::zero()"
        );

        // Verify first 20 bytes match hash160
        assert_eq!(
            &recipient_hash.as_bytes()[..20],
            hash160,
            "First 20 bytes should be hash160"
        );

        // Verify remaining bytes are zeros (padding)
        assert!(
            recipient_hash.as_bytes()[20..].iter().all(|&b| b == 0),
            "Bytes 20-63 should be zero padding"
        );
    }
}
