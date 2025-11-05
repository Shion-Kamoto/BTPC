//! BTPC Miner
//!
//! A dedicated mining application for BTPC with SHA-512 proof-of-work.
//! Supports both CPU and GPU (OpenCL) mining.

#![allow(unused_variables)]

mod gpu_miner;

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use btpc_core::{
    blockchain::{Block, BlockHeader, Transaction},
    consensus::{
        pow::MiningTarget, RewardCalculator,
    },
    crypto::Hash,
    Network,
};
use clap::{Arg, Command};
use serde::Deserialize;
use tokio::time::interval;

/// Mining configuration
#[derive(Debug, Clone)]
pub struct MinerConfig {
    /// Network to mine on
    pub network: Network,
    /// Number of mining threads
    pub threads: usize,
    /// RPC server address for getting work
    pub rpc_url: String,
    /// Mining address (where rewards go)
    pub mining_address: String,
    /// Coinbase message
    pub coinbase_message: String,
    /// Target hash rate (for testing)
    pub target_hashrate: Option<u64>,
}

impl Default for MinerConfig {
    fn default() -> Self {
        MinerConfig {
            network: Network::Mainnet,
            threads: num_cpus::get(),
            rpc_url: "http://127.0.0.1:8332".to_string(),
            mining_address: "burn_address".to_string(),
            coinbase_message: "BTPC Miner v0.1.0".to_string(),
            target_hashrate: None,
        }
    }
}

/// Mining statistics
#[derive(Debug, Clone)]
pub struct MiningStats {
    /// Total hashes computed
    pub total_hashes: u64,
    /// Current hash rate (hashes per second)
    pub hashrate: f64,
    /// Blocks found
    pub blocks_found: u64,
    /// Mining start time
    pub start_time: Instant,
    /// Last block found time
    pub last_block_time: Option<Instant>,
}

impl Default for MiningStats {
    fn default() -> Self {
        MiningStats {
            total_hashes: 0,
            hashrate: 0.0,
            blocks_found: 0,
            start_time: Instant::now(),
            last_block_time: None,
        }
    }
}

/// Block template from RPC
#[derive(Debug, Clone, Deserialize)]
struct BlockTemplate {
    version: u32,
    height: u32,
    previousblockhash: String,
    #[serde(deserialize_with = "deserialize_bits_from_hex")]
    bits: u32,
    curtime: u64,
}

/// Deserialize bits from hex string (e.g., "1d0fffff") to u32
fn deserialize_bits_from_hex<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let s: String = Deserialize::deserialize(deserializer)?;
    u32::from_str_radix(s.trim_start_matches("0x"), 16)
        .map_err(|e| Error::custom(format!("Failed to parse bits as hex: {}", e)))
}

/// BTPC Miner implementation
pub struct Miner {
    config: MinerConfig,
    stats: Arc<MiningStats>,
    running: Arc<AtomicBool>,
    hash_counter: Arc<AtomicU64>,
}

impl Miner {
    /// Create a new miner instance
    pub fn new(config: MinerConfig) -> Self {
        Miner {
            config,
            stats: Arc::new(MiningStats::default()),
            running: Arc::new(AtomicBool::new(false)),
            hash_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Start mining
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting BTPC Miner...");
        println!("Network: {:?}", self.config.network);
        println!("Threads: {}", self.config.threads);
        println!("RPC URL: {}", self.config.rpc_url);
        println!("Mining address: {}", self.config.mining_address);

        self.running.store(true, Ordering::SeqCst);

        // Start mining threads
        let mut handles = Vec::new();
        for thread_id in 0..self.config.threads {
            let handle = self.start_mining_thread(thread_id).await?;
            handles.push(handle);
        }

        // Start statistics reporter
        let stats_handle = self.start_stats_reporter().await;

        // Wait for user to stop mining
        println!("Press Ctrl+C to stop mining");
        tokio::signal::ctrl_c().await?;

        println!("\nStopping miner...");
        self.running.store(false, Ordering::SeqCst);

        // Wait for all threads to finish
        for handle in handles {
            handle.join().map_err(|_| "Thread join failed")?;
        }

        println!("Mining stopped");
        self.print_final_stats();

        Ok(())
    }

    /// Start a mining thread
    async fn start_mining_thread(
        &self,
        thread_id: usize,
    ) -> Result<thread::JoinHandle<()>, Box<dyn std::error::Error>> {
        let running = Arc::clone(&self.running);
        let hash_counter = Arc::clone(&self.hash_counter);
        let config = self.config.clone();

        let handle = thread::spawn(move || {
            println!("Mining thread {} started", thread_id);

            while running.load(Ordering::SeqCst) {
                match Self::mine_block(&config, &hash_counter) {
                    Ok(Some(block)) => {
                        println!("🎉 Block found by thread {}!", thread_id);
                        println!("Block hash: {}", block.hash().to_hex());

                        // Submit block to node via RPC
                        if let Err(e) = Self::submit_block_to_node(&config, &block) {
                            eprintln!("Failed to submit block: {}", e);
                        } else {
                            println!("✅ Block submitted successfully!");
                        }
                    }
                    Ok(None) => {
                        // No block found in this round, continue
                    }
                    Err(e) => {
                        eprintln!("Mining error in thread {}: {}", thread_id, e);
                        thread::sleep(Duration::from_secs(1));
                    }
                }
            }

            println!("Mining thread {} stopped", thread_id);
        });

        Ok(handle)
    }

    /// Mine a single block attempt
    fn mine_block(
        config: &MinerConfig,
        hash_counter: &Arc<AtomicU64>,
    ) -> Result<Option<Block>, Box<dyn std::error::Error>> {
        // Create a mining block template
        let block_template = Self::create_block_template(config)?;

        // Use actual network difficulty from template
        use btpc_core::consensus::DifficultyTarget;
        let difficulty_target = DifficultyTarget::from_bits(block_template.header.bits);
        let target = MiningTarget::from_bytes(*difficulty_target.as_bytes());

        // Mine the block
        let start_nonce = rand::random::<u32>();
        const NONCE_RANGE: u32 = 100_000; // Mine 100k nonces before checking for new work

        for nonce_offset in 0..NONCE_RANGE {
            let nonce = start_nonce.wrapping_add(nonce_offset);

            // Update block header nonce
            let mut mining_header = block_template.header.clone();
            mining_header.nonce = nonce;

            // Calculate hash
            let block_hash = mining_header.hash();
            hash_counter.fetch_add(1, Ordering::Relaxed);

            // Check if hash meets target
            if block_hash.meets_target(&target.as_hash()) {
                // Found a valid block!
                let mut found_block = block_template.clone();
                found_block.header.nonce = nonce;
                return Ok(Some(found_block));
            }
        }

        Ok(None) // No block found in this round
    }

    /// Create a block template for mining
    fn create_block_template(config: &MinerConfig) -> Result<Block, Box<dyn std::error::Error>> {
        use serde_json::json;

        // Fetch block template from node via RPC
        let client = reqwest::blocking::Client::new();
        let request = json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "getblocktemplate",
            "params": []
        });

        let response = client
            .post(&config.rpc_url)
            .header("Content-Type", "application/json")
            .body(request.to_string())
            .send()?;

        if !response.status().is_success() {
            return Err(format!("RPC HTTP error: {}", response.status()).into());
        }

        let response_text = response.text()?;
        let response_json: serde_json::Value = serde_json::from_str(&response_text)?;

        // Check for RPC error
        if let Some(error) = response_json.get("error") {
            if !error.is_null() {
                return Err(format!("RPC error: {}", error).into());
            }
        }

        // Parse template from result
        let result = response_json.get("result")
            .ok_or("Missing result in RPC response")?;

        let template: BlockTemplate = serde_json::from_value(result.clone())?;

        // Parse prev_hash
        let prev_hash = if template.previousblockhash.is_empty() || template.previousblockhash == "0000000000000000000000000000000000000000000000000000000000000000" {
            Hash::zero()
        } else {
            Hash::from_hex(&template.previousblockhash)?
        };

        // Create coinbase transaction with correct height
        let coinbase_tx = Self::create_coinbase_transaction(config, template.height)?;

        // Calculate merkle root (simplified - just coinbase for now)
        let merkle_root = coinbase_tx.hash();

        // Create block header with actual network difficulty
        let header = BlockHeader {
            version: template.version,
            prev_hash,
            merkle_root,
            timestamp: template.curtime,
            bits: template.bits,  // Real network difficulty!
            nonce: 0,
        };

        Ok(Block {
            header,
            transactions: vec![coinbase_tx],
        })
    }

    /// Create coinbase transaction
    fn create_coinbase_transaction(
        config: &MinerConfig,
        height: u32,
    ) -> Result<Transaction, Box<dyn std::error::Error>> {
        // Calculate block reward
        let reward = RewardCalculator::calculate_reward(height).unwrap_or(0);

        // Create coinbase input
        let coinbase_input = btpc_core::blockchain::TransactionInput {
            previous_output: btpc_core::blockchain::OutPoint {
                txid: Hash::zero(),
                vout: 0xffffffff,
            },
            script_sig: btpc_core::crypto::Script::from_bytes(config.coinbase_message.as_bytes().to_vec()),
            sequence: 0xffffffff,
        };

        // Create reward output
        let reward_output = btpc_core::blockchain::TransactionOutput {
            value: reward,
            script_pubkey: btpc_core::crypto::Script::from_bytes(
                config.mining_address.as_bytes().to_vec(),
            ),
        };

        Ok(Transaction {
            version: 1,
            inputs: vec![coinbase_input],
            outputs: vec![reward_output],
            lock_time: 0,
            fork_id: 0, // Testnet fork_id is 0
        })
    }

    /// Start statistics reporter
    async fn start_stats_reporter(&self) -> tokio::task::JoinHandle<()> {
        let hash_counter = Arc::clone(&self.hash_counter);
        let running = Arc::clone(&self.running);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));
            let mut last_hashes = 0u64;
            let mut last_time = Instant::now();

            while running.load(Ordering::SeqCst) {
                interval.tick().await;

                let current_hashes = hash_counter.load(Ordering::SeqCst);
                let current_time = Instant::now();

                let hash_diff = current_hashes - last_hashes;
                let time_diff = current_time.duration_since(last_time).as_secs_f64();

                let hashrate = if time_diff > 0.0 {
                    hash_diff as f64 / time_diff
                } else {
                    0.0
                };

                println!(
                    "Mining: {:.0} H/s | Total: {} hashes | Uptime: {:.1}m",
                    hashrate,
                    current_hashes,
                    current_time.duration_since(last_time).as_secs_f64() / 60.0
                );

                last_hashes = current_hashes;
                last_time = current_time;
            }
        })
    }

    /// Submit a mined block to the node via RPC
    fn submit_block_to_node(
        config: &MinerConfig,
        block: &Block,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use serde_json::json;

        // Serialize block to hex
        let block_hex = hex::encode(block.serialize());

        // Create RPC request
        let request = json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "submitblock",
            "params": [block_hex]
        });

        // Send HTTP request
        let client = reqwest::blocking::Client::new();
        let response = client
            .post(&config.rpc_url)
            .header("Content-Type", "application/json")
            .body(request.to_string())
            .send()?;

        // Check if submission was successful
        if response.status().is_success() {
            let response_text = response.text()?;
            let response_json: serde_json::Value = serde_json::from_str(&response_text)?;

            if let Some(error) = response_json.get("error") {
                if !error.is_null() {
                    return Err(format!("RPC error: {}", error).into());
                }
            }

            Ok(())
        } else {
            Err(format!("HTTP error: {}", response.status()).into())
        }
    }

    /// Print final mining statistics
    fn print_final_stats(&self) {
        let total_hashes = self.hash_counter.load(Ordering::SeqCst);
        let elapsed = self.stats.start_time.elapsed();
        let avg_hashrate = total_hashes as f64 / elapsed.as_secs_f64();

        println!("\n=== Final Mining Statistics ===");
        println!("Total hashes: {}", total_hashes);
        println!("Mining time: {:.1} minutes", elapsed.as_secs_f64() / 60.0);
        println!("Average hashrate: {:.0} H/s", avg_hashrate);
        println!("Blocks found: {}", self.stats.blocks_found);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let matches = Command::new("btpc-miner")
        .version("0.1.0")
        .about("BTPC Miner - SHA-512 proof-of-work mining")
        .arg(
            Arg::new("network")
                .long("network")
                .value_name("NETWORK")
                .help("Network to mine on (mainnet, testnet, regtest)")
                .default_value("mainnet"),
        )
        .arg(
            Arg::new("threads")
                .long("threads")
                .short('t')
                .value_name("COUNT")
                .help("Number of mining threads (default: auto-detect)"),
        )
        .arg(
            Arg::new("rpc-url")
                .long("rpc-url")
                .value_name("URL")
                .help("RPC server URL")
                .default_value("http://127.0.0.1:8332"),
        )
        .arg(
            Arg::new("address")
                .long("address")
                .value_name("ADDRESS")
                .help("Mining address for rewards")
                .default_value("burn_address"),
        )
        .arg(
            Arg::new("message")
                .long("message")
                .value_name("MESSAGE")
                .help("Coinbase message")
                .default_value("BTPC Miner v0.1.0"),
        )
        .arg(
            Arg::new("gpu")
                .long("gpu")
                .short('g')
                .help("Enable GPU mining (requires --features gpu at compile time)")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Parse configuration
    let network = match matches.get_one::<String>("network").unwrap().as_str() {
        "mainnet" => Network::Mainnet,
        "testnet" => Network::Testnet,
        "regtest" => Network::Regtest,
        _ => {
            eprintln!("Invalid network. Use mainnet, testnet, or regtest");
            std::process::exit(1);
        }
    };

    let threads: usize = matches
        .get_one::<String>("threads")
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(num_cpus::get);

    let rpc_url = matches.get_one::<String>("rpc-url").unwrap().clone();
    let mining_address = matches.get_one::<String>("address").unwrap().clone();
    let coinbase_message = matches.get_one::<String>("message").unwrap().clone();
    let use_gpu = matches.get_flag("gpu");

    // Check GPU availability if requested
    if use_gpu {
        #[cfg(feature = "gpu")]
        {
            println!("GPU mining requested");
            let gpu_config = gpu_miner::GpuMinerConfig::default();
            match gpu_miner::GpuMiner::new(gpu_config) {
                Ok(gpu) => {
                    println!("✅ GPU mining enabled: {}", gpu.device_info());
                    println!("⚠️  Note: Full GPU acceleration requires optimized OpenCL kernels");
                    println!("   Currently using CPU-fallback implementation");
                }
                Err(e) => {
                    eprintln!("❌ GPU mining failed: {}", e);
                    eprintln!("   Falling back to CPU mining");
                }
            }
        }

        #[cfg(not(feature = "gpu"))]
        {
            eprintln!("❌ GPU mining not available - btpc_miner was not compiled with GPU support");
            eprintln!("   Compile with: cargo build --features gpu");
            eprintln!("   Falling back to CPU mining");
        }
    }

    // Create miner configuration
    let config = MinerConfig {
        network,
        threads,
        rpc_url,
        mining_address,
        coinbase_message,
        target_hashrate: None,
    };

    // Create and start miner
    let miner = Miner::new(config);
    miner.start().await?;

    Ok(())
}
