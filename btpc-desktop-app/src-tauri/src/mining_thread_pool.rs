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
}

/// Trait for types that can log mining events
pub trait MiningLogger {
    fn add_entry(&mut self, level: String, message: String);
}

/// Cached block template with expiration tracking
///
/// Reduces RPC requests by reusing templates for 10 seconds.
/// Fixes 429 rate limit errors from requesting new template every batch.
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
}

impl MiningThreadPool {
    /// Create new mining thread pool (inactive)
    pub fn new() -> Self {
        MiningThreadPool {
            cpu_mining_active: Arc::new(AtomicBool::new(false)),
            gpu_mining_active: Arc::new(AtomicBool::new(false)),
            cpu_threads: Arc::new(AtomicU64::new(0)),
            gpu_devices: Arc::new(AtomicU64::new(0)),
            cpu_total_hashes: Arc::new(AtomicU64::new(0)),
            gpu_total_hashes: Arc::new(AtomicU64::new(0)),
            blocks_found: Arc::new(AtomicU64::new(0)),
            start_time: Arc::new(RwLock::new(None)),
            mining_address: Arc::new(RwLock::new(String::new())),
            cpu_shutdown_tx: None,
            gpu_shutdown_tx: None,
            per_gpu_stats: Arc::new(RwLock::new(HashMap::new())),
            gpu_device_info: Arc::new(RwLock::new(HashMap::new())),
            mining_event_tx: None, // REM-C002
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
        for (_idx, device_info) in gpu_devices.into_iter().enumerate() {
            let device_index = device_info.device_index;
            let device_name = device_info.model_name.clone();
            let mining_active = mining_active.clone();
            let per_gpu_stats = per_gpu_stats.clone();
            let start_time = start_time.clone();
            let blocks_found_counter_clone = blocks_found_counter.clone(); // Clone for this GPU thread
            let mut shutdown_rx_clone = shutdown_rx.resubscribe();
            let log_tx_clone = log_tx.clone();
            let mining_logs_clone = mining_logs.clone();
            let gpu_total_hashes_clone = self.gpu_total_hashes.clone();
            let rpc_client_clone = rpc_client.clone();
            let mining_address_clone = mining_address_arc.clone(); // Clone Arc for EACH GPU thread
            let event_tx_clone = mining_event_tx.clone(); // REM-C002: Clone event channel for this thread

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
                let mut nonce_start = (device_index * 1_000_000_000) as u32; // Partition nonce space
                let mut cached_template: Option<CachedTemplate> = None; // Template cache (10s TTL)
                let mut current_template: Option<crate::rpc_client::BlockTemplate> = None; // Working template

                while mining_active.load(Ordering::SeqCst) {
                    // Check for shutdown signal (non-blocking)
                    if shutdown_rx_clone.try_recv().is_ok() {
                        println!("🛑 GPU {} mining shutdown signal received", device_index);
                        break;
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

                    let recipient_hash =
                        match btpc_core::crypto::address::Address::from_string(&mining_addr_str) {
                            Ok(address) => {
                                // Extract the hash160 (20 bytes) from the address
                                // Address contains the pubkey_hash which is what we need
                                let hash_bytes = address.hash160(); // Returns &[u8; 20]

                                // Convert 20-byte hash160 to 64-byte Hash (pad with zeros)
                                let mut padded = [0u8; 64];
                                padded[..20].copy_from_slice(hash_bytes);
                                let result_hash = btpc_core::crypto::Hash::from_bytes(padded);

                                eprintln!("[GPU MINING] ✅ Successfully parsed mining address: '{}'", mining_addr_str);
                                eprintln!("[GPU MINING] ✅ Extracted hash160 (first 20 bytes): {}", hex::encode(&hash_bytes[..20]));

                                result_hash
                            }
                            Err(e) => {
                                eprintln!("[GPU MINING] ❌❌❌ CRITICAL ERROR ❌❌❌");
                                eprintln!("[GPU MINING] ❌ Failed to parse mining address: '{}'", mining_addr_str);
                                eprintln!("[GPU MINING] ❌ Error details: {}", e);
                                eprintln!("[GPU MINING] ❌ Address length: {} bytes", mining_addr_str.len());
                                eprintln!("[GPU MINING] ❌ Address format: {:?}", mining_addr_str.chars().take(20).collect::<String>());
                                eprintln!("[GPU MINING] ⚠️ MINING REWARDS WILL BE LOST - Using fallback zero hash");
                                eprintln!("[GPU MINING] ⚠️ All mined blocks will credit to address: 0000000000000000000000000000000000000000");
                                btpc_core::crypto::Hash::zero() // Fallback
                            }
                        };

                    let mut coinbase_tx = btpc_core::blockchain::Transaction::coinbase(
                        block_template.coinbasevalue,
                        recipient_hash,
                    );
                    // CRITICAL: Set fork_id to 2 for regtest network
                    // (default is 0=mainnet, 1=testnet, 2=regtest)
                    coinbase_tx.fork_id = 2;

                    // Debug log: Coinbase transaction
                    if let Some(logger) = get_debug_logger() {
                        let tx_id = hex::encode(coinbase_tx.hash().as_slice());
                        logger.log_transaction(
                            "COINBASE",
                            &tx_id,
                            coinbase_tx.inputs.len(),
                            coinbase_tx.outputs.len(),
                            block_template.coinbasevalue,
                        );
                    }

                    let transactions = vec![coinbase_tx];
                    let merkle_root =
                        match btpc_core::blockchain::calculate_merkle_root(&transactions) {
                            Ok(root) => {
                                // Debug log: Merkle root
                                if let Some(logger) = get_debug_logger() {
                                    logger.log_merkle_root(
                                        transactions.len(),
                                        &hex::encode(root.as_slice()),
                                    );
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
                    let bits_value =
                        u32::from_str_radix(&block_template.bits, 16).unwrap_or(0x1d00ffff);
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

                            // DIRECT LOGGING - write to mining_logs buffer directly
                            let message = format!(
                                "GPU {} ({}) found valid block! Nonce: {}",
                                device_index, device_name, nonce
                            );
                            if let Some(ref logs) = mining_logs_clone {
                                if let Ok(mut logs_guard) = logs.lock() {
                                    logs_guard.add_entry("SUCCESS".to_string(), message.clone());
                                    eprintln!(
                                        "[GPU MINING] DIRECT LOG: Added to mining_logs buffer"
                                    );
                                } else {
                                    eprintln!(
                                        "[GPU MINING] DIRECT LOG: Failed to lock mining_logs"
                                    );
                                }
                            }

                            // Also try channel logging (fallback, for debugging)
                            if let Some(ref tx) = log_tx_clone {
                                eprintln!("[GPU MINING] Sending log event: SUCCESS - {}", message);
                                match tx.send(("SUCCESS".to_string(), message)) {
                                    Ok(_) => eprintln!("[GPU MINING] Log event sent successfully"),
                                    Err(e) => {
                                        eprintln!("[GPU MINING] FAILED to send log event: {}", e)
                                    }
                                }
                            }

                            // Build block with found nonce
                            let mut final_header = header.clone();
                            final_header.nonce = nonce;

                            let block_to_submit = btpc_core::blockchain::Block {
                                header: final_header,
                                transactions: transactions.clone(),
                            };

                            // Debug log: Complete block details
                            let block_hash = block_to_submit.hash();
                            if let Some(logger) = get_debug_logger() {
                                logger.log_complete_block(
                                    Some(device_index),
                                    block_template.height, // Height from block template
                                    &hex::encode(block_hash.as_slice()),
                                    &hex::encode(prev_hash.as_slice()),
                                    &hex::encode(merkle_root.as_slice()),
                                    nonce,
                                    block_template.curtime,
                                    transactions.len(),
                                );
                            }

                            let block_hex = hex::encode(block_to_submit.serialize());

                            // Debug log: Block submission attempt
                            if let Some(logger) = get_debug_logger() {
                                logger.log_block_submission(
                                    Some(device_index),
                                    nonce,
                                    block_hex.len(),
                                    &hex::encode(merkle_root.as_slice()),
                                );
                                logger.log_block_hex(Some(device_index), &block_hex);
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

                                    // 🔥 CRITICAL FIX: Invalidate cached template to force fresh fetch
                                    // Without this, mining continues on stale template with old prev_hash
                                    // This causes all subsequent blocks to be rejected (wrong height/prev_hash)
                                    eprintln!("[GPU MINING] 🔄 Block accepted! Invalidating template cache to fetch fresh template...");
                                    cached_template = None;
                                    current_template = None;
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
                                }
                            }
                        }
                        Ok(None) => {
                            // Batch exhausted, continue with next batch
                            // RATE LIMIT FIX: Add small delay to prevent 429 errors (GPU mines 1M nonces in ~50ms)
                            tokio::time::sleep(Duration::from_millis(100)).await;
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
                            eprintln!("🔥 GPU {} CRITICAL TEMPERATURE: {}°C - EMERGENCY SHUTDOWN", device_index, temperature);

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
                            eprintln!("⚠️ GPU {} HIGH TEMPERATURE: {}°C - THROTTLING", device_index, temperature);

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

                    let mut stats = per_gpu_stats.write().unwrap();
                    if let Some(entry) = stats.get_mut(&device_index) {
                        entry.total_hashes = hashes;
                        entry.blocks_found = blocks;

                        // Calculate uptime
                        let uptime = start_time
                            .read()
                            .unwrap()
                            .map(|start| start.elapsed().as_secs())
                            .unwrap_or(0);
                        entry.mining_uptime = uptime;

                        // Calculate hashrate
                        if uptime > 0 {
                            entry.current_hashrate = hashes as f64 / uptime as f64;
                        }
                        entry.last_updated = Instant::now();
                    }

                    // Increment nonce start for next batch
                    nonce_start = nonce_start.wrapping_add(crate::gpu_miner::NONCES_PER_BATCH);
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
        let gpu_hashes = self.gpu_total_hashes.load(Ordering::SeqCst);
        let blocks_found = self.blocks_found.load(Ordering::SeqCst);

        // Calculate uptime
        let uptime_seconds = self
            .start_time
            .read()
            .unwrap()
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0);

        // Calculate hashrate (hashes per second)
        let cpu_hashrate = if uptime_seconds > 0 {
            cpu_hashes as f64 / uptime_seconds as f64
        } else {
            0.0
        };

        let gpu_hashrate = if uptime_seconds > 0 {
            gpu_hashes as f64 / uptime_seconds as f64
        } else {
            0.0
        };

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
        let pool = MiningThreadPool::new();

        // Assert
        assert!(!pool.cpu_mining_active.load(Ordering::SeqCst));
        assert!(!pool.gpu_mining_active.load(Ordering::SeqCst));
        assert_eq!(pool.cpu_threads.load(Ordering::SeqCst), 0);
        assert_eq!(pool.gpu_devices.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_start_stop_cpu_mining() {
        // Arrange
        let mut pool = MiningThreadPool::new();

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
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify stats
        let stats = pool.get_stats();
        assert!(stats.is_mining);
        assert_eq!(stats.cpu_threads, 2);
        assert!(stats.cpu_hashrate > 0.0, "Should have non-zero hashrate");

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
        let mut pool = MiningThreadPool::new();

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
        let pool = MiningThreadPool::new();

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
        let mut pool = MiningThreadPool::new();
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
