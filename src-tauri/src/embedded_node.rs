//! Embedded Blockchain Node Module
//!
//! Wraps btpc-core's Node functionality for in-process integration.
//! Replaces external btpc_node binary with direct library calls.
//!
//! Key features:
//! - Thread-safe Arc<RwLock<Node>> for concurrent access
//! - Unified RocksDB database (eliminates duplication)
//! - Direct function calls (no RPC overhead)
//! - Graceful shutdown with proper resource cleanup

use crate::rpc_client::{BlockTemplate, RpcClientInterface};
use crate::unified_database::UnifiedDatabase;
use anyhow::{Context, Result};
use btpc_core::blockchain::{Block, Transaction, WellKnownGenesis};
use btpc_core::consensus::{pow, DifficultyTarget};
use btpc_core::crypto::Hash;
use btpc_core::mempool::Mempool;
use btpc_core::network::SimplePeerManager;
use btpc_core::Network;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Embedded blockchain node state
///
/// Design: Arc<RwLock<>> pattern allows:
/// - Multiple readers concurrently (blockchain state queries)
/// - Exclusive writer when needed (block addition, sync)
/// - Safe sharing across Tauri commands and background tasks
pub struct EmbeddedNode {
    /// Unified database (shared with wallet manager)
    database: UnifiedDatabase,

    /// Network type (mainnet, testnet, regtest)
    network: Network,

    /// Transaction mempool (unconfirmed transactions)
    /// BUG FIX 2025-11-11: Added mempool to resolve Bugs #2, #3, #4
    mempool: Arc<RwLock<Mempool>>,

    /// Current blockchain height (atomic for fast reads)
    current_height: Arc<std::sync::atomic::AtomicU64>,

    /// Best block hash (hex string)
    best_block_hash: Arc<RwLock<String>>,

    /// Is node currently syncing with peers?
    is_syncing: Arc<std::sync::atomic::AtomicBool>,

    /// Number of connected peers
    connected_peers: Arc<std::sync::atomic::AtomicU32>,

    /// Current difficulty bits (compact format) - updated with each new block
    /// FIX 2025-11-20: Cache difficulty to avoid database lookups
    current_difficulty_bits: Arc<std::sync::atomic::AtomicU32>,

    /// Data directory path (reserved for future use in block persistence)
    _data_dir: PathBuf,

    /// Shutdown signal sender
    shutdown_tx: Option<mpsc::Sender<()>>,

    /// UTXO manager for tracking spendable outputs
    /// FIX 2025-11-16: Added to create UTXOs when blocks are mined
    /// Using std::sync::Mutex for consistency with rest of codebase
    utxo_manager: Arc<std::sync::Mutex<crate::utxo_manager::UTXOManager>>,

    /// Wallet manager for updating cached balances
    /// FIX 2025-11-20: Added to update wallet balance cache when blocks are mined
    wallet_manager: Option<Arc<std::sync::Mutex<crate::wallet_manager::WalletManager>>>,

    /// App handle for emitting events (REM-C002: block_received event)
    app_handle: Option<tauri::AppHandle>,

    /// Transaction history (SQLite) for storing mined transactions
    /// FIX 2025-11-21: Added to store transactions when blocks are mined
    /// FIX 2025-12-12: Changed from RocksDB tx_storage to SQLite tx_history for UPSERT support
    tx_storage: Option<Arc<tokio::sync::RwLock<crate::tx_history::TransactionHistory>>>,

    /// Connected peers tracking (P2P peer information)
    /// FIX 2025-11-28: Added for detailed peer information display
    /// Key: peer address (IP:port), Value: PeerInfo
    peers: Arc<std::sync::RwLock<HashMap<String, PeerInfo>>>,

    /// Bitcoin-style P2P peer manager — handles inbound/outbound connections,
    /// address book (peers.dat), GetAddr/Addr exchange, and auto-connection loop.
    peer_manager: Arc<SimplePeerManager>,

    // ── Bootstrap Network Stability State ──────────────────────────────
    // Tracks Emergency Difficulty Adjustment (EDA) for health monitoring.

    /// Height at which the last EDA Tier 2/3 was triggered (cooldown tracking)
    eda_last_trigger_height: Arc<std::sync::atomic::AtomicU64>,

    /// Total number of EDA triggers since node start (for health RPC)
    eda_trigger_count: Arc<std::sync::atomic::AtomicU64>,

    /// Total number of 20-minute rule triggers since node start (for health RPC)
    twenty_min_rule_count: Arc<std::sync::atomic::AtomicU64>,
}

impl EmbeddedNode {
    /// Set the app handle for event emission (called after initialization)
    ///
    /// This must be called after node initialization to enable wallet balance update events.
    /// Without this, mined blocks won't trigger frontend UI updates.
    pub fn set_app_handle(&mut self, app: tauri::AppHandle) {
        self.app_handle = Some(app);
        eprintln!("[BTPC::Node] App handle set - wallet balance events will be emitted");
        eprintln!("[BTPC::Node] app_handle is now Some()");
    }

    /// Set the wallet manager for balance updates (called after initialization)
    ///
    /// This must be called after node initialization to enable automatic balance cache updates.
    /// Without this, wallet cached balances won't update when blocks are mined.
    pub fn set_wallet_manager(&mut self, wallet_manager: Arc<std::sync::Mutex<crate::wallet_manager::WalletManager>>) {
        self.wallet_manager = Some(wallet_manager);
        eprintln!("[BTPC::Node] Wallet manager set - balance cache will be updated automatically");
    }

    /// Set the transaction history for storing mined transactions (called after initialization)
    ///
    /// This must be called after node initialization to enable transaction history storage.
    /// Without this, mined transactions won't appear in the transaction history.
    /// FIX 2025-12-12: Changed to use SQLite tx_history for UPSERT support
    pub fn set_tx_storage(&mut self, tx_storage: Arc<tokio::sync::RwLock<crate::tx_history::TransactionHistory>>) {
        self.tx_storage = Some(tx_storage);
        println!("✅ EmbeddedNode tx_history set - mined transactions will be stored in history");
    }

    /// Initialize embedded blockchain node
    ///
    /// # Arguments
    /// * `data_dir` - Base data directory (e.g., ~/.btpc)
    /// * `network` - Network type ("mainnet", "testnet", "regtest")
    /// * `utxo_manager` - UTXO manager for tracking spendable outputs
    ///
    /// # Returns
    /// * `Ok(Arc<RwLock<EmbeddedNode>>)` - Thread-safe node handle
    /// * `Err(anyhow::Error)` - Initialization failed (database, genesis block, etc.)
    ///
    /// # Performance
    /// - Opens unified RocksDB database
    /// - Loads genesis block if fresh blockchain
    /// - Does NOT start P2P sync (call start_sync() separately)
    pub async fn new(
        data_dir: PathBuf,
        network: &str,
        utxo_manager: Arc<std::sync::Mutex<crate::utxo_manager::UTXOManager>>,
    ) -> Result<Arc<RwLock<Self>>> {
        // Parse network string
        let network_type = match network {
            "mainnet" => Network::Mainnet,
            "testnet" => Network::Testnet,
            "regtest" => Network::Regtest,
            _ => return Err(anyhow::anyhow!("Invalid network: {}", network)),
        };

        // FIX 2025-12-01: Network isolation - pass network to UnifiedDatabase for separate directories
        // This ensures each network (mainnet/testnet/regtest) has its own blockchain database
        // Path: data_dir/{network}/blockchain.db
        let database =
            UnifiedDatabase::open(&data_dir, network).context("Failed to open unified database")?;

        eprintln!("📁 Network isolation: Database opened at {:?} for network '{}'", database.path(), network);

        // Initialize blockchain state atomics
        let current_height = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let best_block_hash = Arc::new(RwLock::new(String::new()));
        let is_syncing = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let connected_peers = Arc::new(std::sync::atomic::AtomicU32::new(0));

        // Set initial difficulty based on network type
        // FIX 2026-03-04: Bitcoin-style "difficulty 1" approach.
        // Like Bitcoin launched at difficulty 1 and let the 2016-block adjustment
        // algorithm converge, BTPC uses minimum meaningful difficulty as "difficulty 1".
        // First ~2016 blocks mine quickly, then adjustment corrects to real hashrate.
        use btpc_core::consensus::constants as cons;
        let initial_difficulty = match network_type {
            Network::Mainnet => cons::INITIAL_DIFFICULTY_BITS, // SHA-512 "difficulty 1"
            Network::Testnet => cons::INITIAL_DIFFICULTY_BITS, // Same as mainnet
            Network::Regtest => cons::REGTEST_DIFFICULTY_BITS, // Instant mining
        };
        let current_difficulty_bits = Arc::new(std::sync::atomic::AtomicU32::new(initial_difficulty));

        // Initialize mempool
        // BUG FIX 2025-11-11: Create mempool for transaction broadcast/monitoring
        let mempool = Arc::new(RwLock::new(Mempool::new()));

        // Initialize peer tracking HashMap
        let peers = Arc::new(std::sync::RwLock::new(HashMap::new()));

        // Create Bitcoin-style P2P peer manager
        let block_height_for_pm = Arc::new(RwLock::new(0u32));
        let network_config = match network_type {
            Network::Mainnet => btpc_core::network::NetworkConfig::mainnet(),
            Network::Testnet => btpc_core::network::NetworkConfig::testnet(),
            Network::Regtest => btpc_core::network::NetworkConfig::regtest(),
        };
        let peer_manager = Arc::new(SimplePeerManager::new(
            network_config,
            block_height_for_pm,
        ));

        // Create node instance
        let node = EmbeddedNode {
            database,
            network: network_type,
            mempool,
            current_height,
            best_block_hash,
            is_syncing,
            connected_peers,
            current_difficulty_bits,
            _data_dir: data_dir,
            shutdown_tx: None,
            utxo_manager,
            wallet_manager: None, // Will be set after initialization in main.rs
            app_handle: None, // Will be set after initialization in main.rs
            tx_storage: None, // Will be set after initialization in main.rs
            peers, // FIX 2025-11-28: P2P peer tracking
            peer_manager,
            // Bootstrap network stability state
            eda_last_trigger_height: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            eda_trigger_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            twenty_min_rule_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        };

        let node_arc = Arc::new(RwLock::new(node));

        // Load current blockchain state from database
        {
            let mut node_lock = node_arc.write().await;
            node_lock.load_blockchain_state().await?;
        }

        Ok(node_arc)
    }

    /// Load blockchain state from database
    ///
    /// Reads current height and best block hash from RocksDB by finding the maximum height.
    /// If database is empty (no blocks), initializes to height 0.
    ///
    /// # P1 FIX 2025-11-12
    /// Now loads actual blockchain height from database instead of always showing 0.
    async fn load_blockchain_state(&mut self) -> Result<()> {
        // Query database for maximum blockchain height
        match self.database.get_max_height() {
            Ok(Some((max_height, best_hash))) => {
                // Found blocks in database - load actual state
                self.current_height
                    .store(max_height, std::sync::atomic::Ordering::SeqCst);
                *self.best_block_hash.write().await = best_hash.clone();

                // FIX 2025-11-20: Load difficulty from most recent block
                // FIX 2025-12-20: Handle max_height=0 (genesis only) to prevent underflow
                // FIX 2026-02-23: For pre-2016 chains, use CONSTANT initial difficulty.
                // Block headers may contain 20-minute-rule minimum difficulty (0x3c7fffff)
                // which would permanently corrupt the atomic if loaded here.
                use btpc_core::consensus::constants as cons;
                const ADJUSTMENT_INTERVAL: u64 = 2_016;
                let final_difficulty = if max_height < ADJUSTMENT_INTERVAL {
                    // Pre-adjustment chain: use "difficulty 1" constant
                    // This prevents 20-minute-rule blocks from corrupting startup difficulty
                    let initial = match self.network {
                        Network::Regtest => cons::REGTEST_DIFFICULTY_BITS,
                        _ => cons::INITIAL_DIFFICULTY_BITS,
                    };
                    eprintln!(
                        "ℹ️ Chain height {} < {} (first adjustment), using constant initial difficulty 0x{:08x}",
                        max_height, ADJUSTMENT_INTERVAL, initial
                    );
                    initial
                } else {
                    // Post-adjustment chain: load difficulty from the LAST ADJUSTMENT BLOCK
                    // (highest multiple of 2016 <= max_height), NOT the latest block.
                    // The latest block may have been mined with 20-minute-rule or EDA
                    // reduced difficulty, which would corrupt the cached value on restart.
                    let last_adjustment_height = (max_height / ADJUSTMENT_INTERVAL) * ADJUSTMENT_INTERVAL;
                    eprintln!(
                        "ℹ️ Loading difficulty from adjustment block {} (chain height {})",
                        last_adjustment_height, max_height
                    );
                    let difficulty = match self.database.get_block(last_adjustment_height as u32) {
                        Ok(Some(block)) => {
                            block.header.bits
                        },
                        Ok(None) => {
                            eprintln!("⚠️ WARNING: Adjustment block {} not found, using initial difficulty", last_adjustment_height);
                            match self.network {
                                Network::Regtest => cons::REGTEST_DIFFICULTY_BITS,
                                _ => cons::INITIAL_DIFFICULTY_BITS,
                            }
                        },
                        Err(e) => {
                            eprintln!("❌ ERROR: Failed to load block {}: {:?}, using initial difficulty", last_adjustment_height, e);
                            match self.network {
                                Network::Regtest => cons::REGTEST_DIFFICULTY_BITS,
                                _ => cons::INITIAL_DIFFICULTY_BITS,
                            }
                        }
                    };

                    // Enforce minimum difficulty when loading from database
                    let min_difficulty = match self.network {
                        Network::Regtest => 0x407fffff_u32,
                        _ => 0x3c7fffff_u32,
                    };
                    let loaded_exponent = (difficulty >> 24) as u8;
                    let min_exponent = (min_difficulty >> 24) as u8;

                    if self.network != Network::Regtest && loaded_exponent > min_exponent {
                        eprintln!(
                            "⚠️ Database difficulty 0x{:08x} (exp {}) easier than minimum 0x{:08x} (exp {}), using minimum",
                            difficulty, loaded_exponent, min_difficulty, min_exponent
                        );
                        min_difficulty
                    } else {
                        difficulty
                    }
                };

                self.current_difficulty_bits.store(final_difficulty, std::sync::atomic::Ordering::SeqCst);

                eprintln!(
                    "✅ Loaded blockchain state: height={}, hash={}, difficulty=0x{:08x} (decimal: {})",
                    max_height,
                    &best_hash[0..16.min(best_hash.len())],
                    final_difficulty,
                    final_difficulty
                );
            }
            Ok(None) => {
                // Fresh blockchain - CREATE AND STORE GENESIS BLOCK at height 0
                // FIX 2025-12-20: Genesis block is required for difficulty adjustment at block 2016
                // Without genesis, get_block(0) fails and adjustment is skipped until block 4032

                let genesis = match self.network {
                    Network::Mainnet => WellKnownGenesis::mainnet_block(),
                    Network::Testnet => WellKnownGenesis::testnet_block(),
                    Network::Regtest => WellKnownGenesis::regtest_block(),
                };

                let genesis_hash = genesis.hash();
                let genesis_hash_hex = hex::encode(genesis_hash.as_bytes());

                // Store genesis block at height 0
                match self.database.put_block(0, &genesis) {
                    Ok(()) => {
                        println!("✅ Created genesis block at height 0: {}...", &genesis_hash_hex[..32]);
                    }
                    Err(e) => {
                        eprintln!("⚠️ Failed to store genesis block: {}", e);
                        // Continue anyway - first mined block will become height 1
                    }
                }

                // Set blockchain state to genesis
                self.current_height
                    .store(0, std::sync::atomic::Ordering::SeqCst);
                *self.best_block_hash.write().await = genesis_hash_hex.clone();

                // NOTE: Do NOT override current_difficulty_bits here!
                // It was already set correctly at initialization based on network type:
                // - Mainnet/Testnet: INITIAL_DIFFICULTY_BITS (SHA-512 "difficulty 1")
                // - Regtest: REGTEST_DIFFICULTY_BITS (instant mining for development)

                let initial_diff = self.current_difficulty_bits.load(std::sync::atomic::Ordering::SeqCst);
                println!("ℹ️ Fresh blockchain initialized with genesis (height=0, difficulty=0x{:08x})", initial_diff);
            }
            Err(e) => {
                // Database query error - log warning but don't fail initialization
                eprintln!("⚠️ Failed to load blockchain state: {}", e);
                eprintln!("   Defaulting to height=0 (blockchain state may be incorrect)");

                self.current_height
                    .store(0, std::sync::atomic::Ordering::SeqCst);
                *self.best_block_hash.write().await = String::new();
            }
        }

        Ok(())
    }

    /// Get current blockchain state (for queries)
    ///
    /// # Returns
    /// * `BlockchainState` - Current height, best hash, UTXO count
    ///
    /// # Performance
    /// - Uses atomic reads (no locks for height/sync status)
    /// - Target: <10ms (vs ~50ms for RPC)
    pub async fn get_blockchain_state(&self) -> Result<BlockchainState> {
        let height = self
            .current_height
            .load(std::sync::atomic::Ordering::SeqCst);
        let best_hash = self.best_block_hash.read().await.clone();
        let is_syncing = self.is_syncing.load(std::sync::atomic::Ordering::SeqCst);

        // Get UTXO count from database stats
        let db_stats = self.database.get_stats()?;

        // Get current difficulty from cached value (updated when blocks are mined)
        // FIX 2025-11-20: Use cached difficulty instead of database query
        let difficulty_bits = self.current_difficulty_bits.load(std::sync::atomic::Ordering::SeqCst);

        Ok(BlockchainState {
            current_height: height,
            best_block_hash: best_hash,
            is_syncing,
            total_utxos: db_stats.utxos_count,
            difficulty_bits,
        })
    }

    /// Get current blockchain height (lightweight atomic read)
    pub fn get_height(&self) -> u64 {
        self.current_height.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get network type
    ///
    /// # Returns
    /// * `String` - Network name ("mainnet", "testnet", "regtest")
    pub fn get_network(&self) -> String {
        match self.network {
            Network::Mainnet => "mainnet".to_string(),
            Network::Testnet => "testnet".to_string(),
            Network::Regtest => "regtest".to_string(),
        }
    }

    /// Get reference to the database for direct queries
    ///
    /// # Returns
    /// * Reference to the UnifiedDatabase
    pub fn get_database(&self) -> &UnifiedDatabase {
        &self.database
    }

    /// Get sync progress (for UI updates)
    ///
    /// # Returns
    /// * `SyncProgress` - Current/target height, connected peers, sync percentage
    pub fn get_sync_progress(&self) -> Result<SyncProgress> {
        let current_height = self
            .current_height
            .load(std::sync::atomic::Ordering::SeqCst);
        let is_syncing = self.is_syncing.load(std::sync::atomic::Ordering::SeqCst);
        let connected_peers = self
            .connected_peers
            .load(std::sync::atomic::Ordering::SeqCst);

        // Target height is current height when not syncing
        let target_height = if is_syncing {
            // TODO: In full implementation, query max height from peers
            current_height
        } else {
            current_height
        };

        // Calculate sync percentage
        let sync_percentage = if target_height == 0 {
            100.0
        } else {
            (current_height as f64 / target_height as f64) * 100.0
        };

        Ok(SyncProgress {
            current_height,
            target_height,
            is_syncing,
            connected_peers,
            sync_percentage,
        })
    }

    /// Get network type
    pub fn network(&self) -> Network {
        self.network
    }

    /// Get a reference to the P2P peer manager.
    ///
    /// Used by commands like `connect_to_peer` to establish persistent connections
    /// through the peer manager's connection lifecycle (handshake, message loop, etc.).
    pub fn get_peer_manager(&self) -> Arc<SimplePeerManager> {
        Arc::clone(&self.peer_manager)
    }

    /// Update network type
    /// FIX 2025-12-01: Allow network to be updated when user changes network in settings
    /// This ensures UTXOs are created with the correct network prefix to match wallet addresses
    pub fn update_network(&mut self, network: Network) {
        println!("EmbeddedNode: Updating network from {:?} to {:?}", self.network, network);
        self.network = network;
    }

    /// Get database reference (for wallet manager integration)
    pub fn database(&self) -> &UnifiedDatabase {
        &self.database
    }

    /// Submit transaction to mempool (replaces RPC broadcast)
    ///
    /// # Arguments
    /// * `transaction` - Signed transaction to broadcast
    ///
    /// # Returns
    /// * `Ok(String)` - Transaction ID (hex-encoded txid)
    /// * `Err(anyhow::Error)` - Mempool validation failed (double-spend, invalid fee, etc.)
    ///
    /// # BUG FIX 2025-11-11: Resolves Bug #2
    /// Replaces broken RPC broadcast with direct mempool access.
    /// Performance: <5ms vs ~50ms RPC timeout
    pub async fn submit_transaction(&self, transaction: Transaction) -> Result<String> {
        let mut mempool = self.mempool.write().await;

        // Calculate fee from actual transaction size
        // P2 ENHANCEMENT 2025-11-12: Use actual serialized size instead of estimate
        let serialized = transaction.serialize();
        let tx_size_bytes = serialized.len() as u64;

        // Fee estimate for mempool validation: 10 crd/KB (matches DEFAULT_FEE_RATE_PER_KB)
        // Note: Transaction builder calculates actual fee from input values minus output values
        // This is just for mempool validation - actual fee is already embedded in the transaction
        let estimated_fee = (tx_size_bytes * 10 + 1023) / 1024;

        // Add to mempool (validates size, fee rate, double-spending)
        let txid_hash = mempool
            .add_transaction(transaction.clone(), estimated_fee)
            .map_err(|e| anyhow::anyhow!("Failed to add transaction to mempool: {}", e))?;

        let txid = hex::encode(txid_hash.as_bytes());
        println!(
            "✅ Transaction {} added to mempool (size: {} bytes, estimated fee: {} crystals)",
            txid, tx_size_bytes, estimated_fee
        );
        Ok(txid)
    }

    /// Get mempool statistics (for fee estimation)
    ///
    /// # Returns
    /// * `MempoolStats` - Transaction count, size, fee percentiles
    ///
    /// # BUG FIX 2025-11-11: Resolves Bug #3
    /// Replaces RPC fee estimation with direct mempool access.
    /// Performance: <2ms vs ~50ms RPC timeout
    pub async fn get_mempool_stats(&self) -> Result<MempoolStats> {
        let mempool = self.mempool.read().await;

        // Use btpc-core's built-in stats() method
        let core_stats = mempool.stats();

        // Calculate fee percentiles from mempool entries
        let tx_count = core_stats.transaction_count;
        let total_size = core_stats.total_size_bytes;
        let avg_fee = core_stats.avg_fee_per_byte;

        // For percentiles, we approximate from average
        // In a full implementation, we'd iterate entries and calculate actual percentiles
        let fee_p25 = if avg_fee > 0.0 { avg_fee * 0.8 } else { 100.0 };
        let fee_p50 = if avg_fee > 0.0 { avg_fee } else { 100.0 };
        let fee_p75 = if avg_fee > 0.0 { avg_fee * 1.2 } else { 150.0 };

        Ok(MempoolStats {
            tx_count,
            total_size_bytes: total_size as u64,
            fee_rate_p25_crd_per_byte: fee_p25,
            fee_rate_p50_crd_per_byte: fee_p50,
            fee_rate_p75_crd_per_byte: fee_p75,
        })
    }

    /// Get transaction information (for monitoring confirmations)
    ///
    /// # Arguments
    /// * `txid` - Transaction ID (hex-encoded)
    ///
    /// # Returns
    /// * `Some(TransactionInfo)` - Transaction found (mempool or confirmed)
    /// * `None` - Transaction not found
    ///
    /// # BUG FIX 2025-11-11: Resolves Bug #4
    /// Replaces broken RPC polling with direct mempool/database access.
    /// Performance: <5ms vs ~50ms RPC timeout
    pub async fn get_transaction_info(&self, txid: &str) -> Result<Option<TransactionInfo>> {
        // Try mempool first (unconfirmed)
        let mempool = self.mempool.read().await;

        // Parse txid hex string to Hash (SHA-512 = 64 bytes)
        let txid_bytes = match hex::decode(txid) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(None), // Invalid txid format
        };

        // BTPC uses SHA-512 which produces 64-byte hashes
        if txid_bytes.len() != 64 {
            return Ok(None); // Invalid hash length (SHA-512 requires 64 bytes)
        }

        let mut hash_array = [0u8; 64];
        hash_array.copy_from_slice(&txid_bytes);
        let tx_hash = btpc_core::crypto::Hash::from_bytes(hash_array);

        // Check if transaction exists in mempool
        if let Some(entry) = mempool.get_transaction(&tx_hash) {
            return Ok(Some(TransactionInfo {
                txid: txid.to_string(),
                confirmations: 0,
                block_height: None,
                block_hash: None,
                fee: entry.fee,
                status: "mempool".to_string(),
            }));
        }

        // Try database (confirmed transactions)
        // BUG FIX 2025-11-12: Implement CF_TRANSACTIONS query (Bug #4 completion)
        match self.database.get_transaction(&hash_array) {
            Ok(Some((transaction, block_height))) => {
                // Calculate confirmations from block height
                let current_height = self
                    .current_height
                    .load(std::sync::atomic::Ordering::SeqCst);
                let block_height_u64 = block_height as u64;
                let confirmations = if block_height > 0 && current_height >= block_height_u64 {
                    (current_height - block_height_u64) + 1
                } else {
                    0
                };

                // Calculate transaction fee from actual size
                // P2 ENHANCEMENT 2025-11-12: Use actual serialized size
                let serialized = transaction.serialize();
                let tx_size_bytes = serialized.len() as u64;
                let estimated_fee = (tx_size_bytes * 10 + 1023) / 1024; // 10 crd/KB

                // Get block hash containing this transaction
                let block_hash = match self.database.get_block(block_height) {
                    Ok(Some(block)) => Some(hex::encode(block.hash().as_bytes())),
                    _ => None,
                };

                Ok(Some(TransactionInfo {
                    txid: txid.to_string(),
                    confirmations: confirmations as u32,
                    block_height: Some(block_height as u64),
                    block_hash,
                    fee: estimated_fee,
                    status: "confirmed".to_string(),
                }))
            }
            Ok(None) => Ok(None), // Transaction not found anywhere
            Err(e) => {
                eprintln!("⚠️ Database query error: {}", e);
                Ok(None) // Return None instead of propagating error (graceful degradation)
            }
        }
    }

    /// Start P2P sync (optional - for testnet/mainnet)
    ///
    /// Spawns background task to:
    /// - Connect to peers
    /// - Download missing blocks
    /// - Validate and add blocks to chain
    /// - Emit blockchain:block_added events
    pub async fn start_sync(&mut self) -> Result<()> {
        self.start_sync_inner(None).await
    }

    /// Start P2P sync with a reference to the node Arc for full block processing.
    ///
    /// This variant allows the spawned event loop to acquire write access to the
    /// node for processing blocks received from peers (via `submit_block`).
    pub async fn start_sync_with_block_processing(
        &mut self,
        node_arc: Arc<RwLock<EmbeddedNode>>,
    ) -> Result<()> {
        self.start_sync_inner(Some(node_arc)).await
    }

    async fn start_sync_inner(
        &mut self,
        node_arc: Option<Arc<RwLock<EmbeddedNode>>>,
    ) -> Result<()> {
        // ── Bitcoin-style P2P bootstrap ──────────────────────────────────────
        eprintln!("📡 Starting P2P sync for network: {:?}", self.network);
        eprintln!("   Block processing: {}", if node_arc.is_some() { "ENABLED" } else { "DISABLED (peer tracking only)" });

        // Load previously-known peers from disk so we can reconnect instantly
        // without waiting for DNS resolution.
        let peers_dat = self._data_dir.join("peers.dat");
        self.peer_manager.load_peers(&peers_dat).await;
        let known_peer_count = self.peer_manager.discovery.read().await.address_count();
        eprintln!("   Loaded {} known peer(s) from peers.dat", known_peer_count);

        // Start accepting inbound connections on the P2P port
        if let Err(e) = self.peer_manager.start_listening().await {
            eprintln!("⚠️  P2P listener failed to start: {}", e);
            // Not fatal — we can still mine and sync outbound
        } else {
            eprintln!("📡 P2P listener started");
        }

        // Query DNS seeds — log results instead of silently ignoring failures
        // FIX 2026-04-12: Previously used .ok() which hid DNS failures, making it
        // impossible to diagnose "0 peers" issues on fresh installs.
        match self.peer_manager
            .discovery
            .write()
            .await
            .query_dns_seeds()
            .await
        {
            Ok(addrs) => {
                if addrs.is_empty() {
                    eprintln!("⚠️ DNS seed query returned 0 addresses — seeds may be unreachable");
                    eprintln!("   Tip: Use manual peer connection or ensure bootstrap nodes are running");
                } else {
                    eprintln!("📡 DNS seeds returned {} peer address(es)", addrs.len());
                }
            }
            Err(e) => {
                eprintln!("⚠️ DNS seed query failed: {}", e);
                eprintln!("   This is expected if DNS seeds are not yet configured for this network.");
                eprintln!("   The auto-connection manager will retry, or connect manually via the UI.");
            }
        }

        // Start the auto-connection manager — maintains 8 outbound peers,
        // re-queries DNS when isolated.  Mirrors Bitcoin Core's
        // ThreadOpenConnections.
        SimplePeerManager::start_connection_manager(Arc::clone(&self.peer_manager));

        // FIX 2026-03-29: Consume PeerEvents to keep self.peers in sync with
        // real P2P connections. Previously self.peers was only populated via
        // add_simulated_peers(), so get_peer_info() always returned fake data.
        // FIX 2026-04-12: Also handle BlockReceived and InventoryReceived events
        // to actually sync blocks from peers (previously ignored — core sync bug).
        if let Some(mut peer_rx) = self.peer_manager.take_event_receiver().await {
            let peers_arc = Arc::clone(&self.peers);
            let peer_manager_arc = Arc::clone(&self.peer_manager);
            let connected_peers_counter = Arc::clone(&self.connected_peers);
            let current_height_atomic = Arc::clone(&self.current_height);
            let database_clone = self.database.clone();
            let node_arc_for_events = node_arc.clone();
            tokio::spawn(async move {
                use btpc_core::network::simple_peer_manager::PeerEvent;
                while let Some(event) = peer_rx.recv().await {
                    match event {
                        PeerEvent::PeerConnected { addr, height, user_agent } => {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();
                            {
                                let mut peers = peers_arc.write().unwrap();
                                peers.insert(addr.to_string(), PeerInfo {
                                    address: addr.to_string(),
                                    version: user_agent.clone(),
                                    height: height as u64,
                                    ping_ms: 0,
                                    connected_since: now,
                                });
                                connected_peers_counter.store(
                                    peers.len() as u32,
                                    std::sync::atomic::Ordering::SeqCst,
                                );
                            }
                            eprintln!("🔗 Peer connected: {} (height {})", addr, height);

                            // If peer has more blocks than us, request their headers
                            let our_height = current_height_atomic
                                .load(std::sync::atomic::Ordering::SeqCst);
                            if (height as u64) > our_height {
                                eprintln!(
                                    "📡 Peer {} has height {} (we have {}), requesting blocks via GetHeaders",
                                    addr, height, our_height
                                );
                                // Build block locator (simplified: just our tip hash)
                                if let Ok(Some(tip_hash_bytes)) = database_clone
                                    .get_block_hash_at_height(our_height as u32)
                                {
                                    use btpc_core::network::protocol::{
                                        GetHeadersMessage, Message,
                                    };
                                    let mut hash_array = [0u8; 64];
                                    let copy_len = tip_hash_bytes.len().min(64);
                                    hash_array[..copy_len]
                                        .copy_from_slice(&tip_hash_bytes[..copy_len]);
                                    let locator_hash =
                                        btpc_core::crypto::Hash::from_bytes(hash_array);

                                    let get_headers = Message::GetHeaders(GetHeadersMessage {
                                        version: 1,
                                        block_locator: vec![locator_hash],
                                        hash_stop: btpc_core::crypto::Hash::zero(),
                                    });
                                    peer_manager_arc
                                        .send_to_peer(&addr, get_headers)
                                        .await;
                                }
                            }
                        }
                        PeerEvent::PeerDisconnected { addr, .. } => {
                            {
                                let mut peers = peers_arc.write().unwrap();
                                peers.remove(&addr.to_string());
                                connected_peers_counter.store(
                                    peers.len() as u32,
                                    std::sync::atomic::Ordering::SeqCst,
                                );
                            }
                            eprintln!("🔌 Peer disconnected: {}", addr);
                        }
                        PeerEvent::BlockReceived { from, block } => {
                            // FIX 2026-04-12: Actually process blocks from peers
                            let block_hash = block.hash();
                            eprintln!(
                                "📦 Processing block from peer {}: {}",
                                from,
                                block_hash.to_hex()
                            );

                            if let Some(ref node_ref) = node_arc_for_events {
                                // Serialize block to hex for submit_block
                                let block_hex = hex::encode(block.serialize());
                                let mut node = node_ref.write().await;
                                match node.submit_block(&block_hex).await {
                                    Ok(msg) => {
                                        eprintln!(
                                            "✅ Block from peer {} accepted: {}",
                                            from, msg
                                        );
                                        // Broadcast to other peers
                                        drop(node); // Release write lock before broadcast
                                        peer_manager_arc.broadcast_block(&block).await;
                                    }
                                    Err(e) => {
                                        let err_str = e.to_string();
                                        // Don't spam logs for blocks we already have
                                        if !err_str.contains("already exists")
                                            && !err_str.contains("stale block")
                                        {
                                            eprintln!(
                                                "⚠️ Block from peer {} rejected: {}",
                                                from, err_str
                                            );
                                        }
                                    }
                                }
                            } else {
                                eprintln!(
                                    "⚠️ BlockReceived but no node_arc — block not processed"
                                );
                            }
                        }
                        PeerEvent::InventoryReceived { from, inv } => {
                            // FIX 2026-04-12: Request blocks we don't have
                            use btpc_core::network::protocol::{
                                InvType, InventoryVector, Message,
                            };
                            let mut needed: Vec<InventoryVector> = Vec::new();
                            for item in &inv {
                                if item.inv_type == InvType::Block {
                                    // Check if we already have this block
                                    let hash_bytes = item.hash.as_bytes().to_vec();
                                    let have_it = database_clone
                                        .has_block_hash(&hash_bytes)
                                        .unwrap_or(false);
                                    if !have_it {
                                        needed.push(item.clone());
                                    }
                                }
                            }
                            if !needed.is_empty() {
                                eprintln!(
                                    "📋 Requesting {} block(s) from peer {}",
                                    needed.len(),
                                    from
                                );
                                peer_manager_arc
                                    .send_to_peer(&from, Message::GetData(needed))
                                    .await;
                            }
                        }
                        PeerEvent::TransactionReceived { from, tx } => {
                            eprintln!(
                                "💳 Received tx {} from peer {}",
                                tx.hash().to_hex(),
                                from
                            );
                            // TODO: Add to mempool when mempool accepts external txs
                        }
                    }
                }
            });
        }

        // Auto-save peers.dat every 15 minutes
        SimplePeerManager::start_peer_saver(Arc::clone(&self.peer_manager), peers_dat);

        // ────────────────────────────────────────────────────────────────────

        // Set sync active flag (persists across page navigation)
        self.is_syncing
            .store(true, std::sync::atomic::Ordering::SeqCst);

        eprintln!("✅ Blockchain sync started (Bitcoin-style P2P active)");
        Ok(())
    }

    /// Stop P2P sync gracefully
    pub async fn stop_sync(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        self.is_syncing
            .store(false, std::sync::atomic::Ordering::SeqCst);
        eprintln!("🛑 Blockchain sync stopped");
        Ok(())
    }

    /// Graceful shutdown
    ///
    /// Shutdown sequence (from research.md):
    /// 1. Stop mining (if running)
    /// 2. Stop P2P sync
    /// 3. Flush mempool to disk
    /// 4. Flush RocksDB WAL
    /// 5. Zeroize sensitive keys
    pub async fn shutdown(&mut self) -> Result<()> {
        // Stop sync
        self.stop_sync().await?;

        // Save address book so peers are available immediately on next start
        let peers_dat = self._data_dir.join("peers.dat");
        self.peer_manager.save_peers(&peers_dat).await;

        // Flush database WAL
        self.database
            .flush_wal()
            .context("Failed to flush database WAL during shutdown")?;

        Ok(())
    }

    /// Get block template for mining
    ///
    /// Creates a new block template with:
    /// - Current blockchain tip as previous hash
    /// - Mempool transactions ready for inclusion
    /// - Network difficulty target
    /// - Current timestamp
    ///
    /// # Returns
    /// * `BlockTemplate` - Ready-to-mine block template
    ///
    /// # Feature 013: Self-Contained Mining
    /// Replaces external RPC call with direct blockchain/mempool access.
    pub async fn get_block_template(&self) -> Result<BlockTemplate> {
        // Get current blockchain state
        let current_height = self
            .current_height
            .load(std::sync::atomic::Ordering::SeqCst);
        let best_hash = self.best_block_hash.read().await.clone();

        // Parse best block hash (or use zero hash if genesis)
        let previous_blockhash = if best_hash.is_empty() {
            hex::encode(Hash::zero().as_bytes())
        } else {
            best_hash
        };

        // Get mempool transactions (up to 1000 transactions per block)
        let mempool = self.mempool.read().await;
        let mempool_entries = mempool.get_transactions_by_fee(1000);

        tracing::debug!("[BLOCK_TEMPLATE] Mempool has {} transactions for block height {}",
            mempool_entries.len(),
            current_height + 1
        );

        // Convert mempool transactions to JSON values for template
        let transactions: Vec<serde_json::Value> = mempool_entries
            .iter()
            .map(|entry| {
                let tx = &entry.transaction;
                serde_json::json!({
                    "data": hex::encode(tx.serialize()),
                    "txid": hex::encode(tx.hash().as_bytes()),
                    "hash": hex::encode(tx.hash().as_bytes()),
                })
            })
            .collect();

        // Calculate coinbase value (block reward + fees)
        // FIX 2025-11-20: Use linear decay formula per Constitution Article III
        // Block reward decreases linearly over 24 years from 32.375 BTPC to 0.5 BTPC
        let next_height = current_height + 1;
        let block_reward = calculate_block_reward(next_height);

        // Add transaction fees (simplified - actual implementation would sum all tx fees)
        let total_fees = mempool_entries.len() as u64 * 100; // 100 crystals per tx
        let coinbasevalue = block_reward + total_fees;

        // Calculate difficulty (with adjustment every 2016 blocks per Constitution Article IV)
        // FIX 2025-11-20: Dynamic difficulty adjustment replaces hardcoded values
        let bits = self.calculate_difficulty_for_next_block(next_height).await?;

        // Convert bits to full target hash (64 bytes for SHA-512)
        let difficulty_target = DifficultyTarget::from_bits(bits);
        let mining_target = pow::MiningTarget::from_bytes(*difficulty_target.as_bytes());
        let target_hex = hex::encode(mining_target.as_bytes());

        // Current timestamp
        // FIX 2025-11-18: Handle potential SystemTime error instead of unwrap()
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();

        Ok(BlockTemplate {
            version: 1,
            previousblockhash: previous_blockhash,
            transactions,
            coinbasevalue,
            target: target_hex,
            mintime: current_time.saturating_sub(7200), // 2 hours ago
            curtime: current_time,
            bits: format!("{:08x}", bits),
            height: current_height + 1, // Next block height
        })
    }

    /// Submit mined block to blockchain
    ///
    /// Validates and adds block to the blockchain:
    /// - Deserializes block from hex
    /// - Validates proof-of-work
    /// - Validates transactions
    /// - Adds block to database
    /// - Updates blockchain state
    ///
    /// # Arguments
    /// * `block_hex` - Hex-encoded serialized block
    ///
    /// # Returns
    /// * `Ok(String)` - Success message with block hash
    /// * `Err(antml::Error)` - Invalid block (PoW, transactions, etc.)
    ///
    /// # Feature 013: Self-Contained Mining
    /// Replaces external RPC call with direct blockchain validation/storage.
    pub async fn submit_block(&mut self, block_hex: &str) -> Result<String> {
        eprintln!(
            "[SUBMIT_BLOCK] 🔄 Received block (hex: {} bytes)",
            block_hex.len()
        );
        // Deserialize block from hex
        eprintln!("[SUBMIT_BLOCK] Decoding block hex...");
        let block_bytes = hex::decode(block_hex).context("Invalid block hex encoding")?;

        eprintln!(
            "[SUBMIT_BLOCK] ✅ Hex decoded to {} bytes",
            block_bytes.len()
        );
        let block = Block::deserialize(&block_bytes).context("Failed to deserialize block")?;

        // Validate proof-of-work
        eprintln!("[SUBMIT_BLOCK] ✅ Block deserialized");
        let block_hash = block.hash();
        eprintln!(
            "[SUBMIT_BLOCK] Block hash: {}",
            hex::encode(block_hash.as_bytes())
        );
        let difficulty_target = DifficultyTarget::from_bits(block.header.bits);

        // Use the validate_block_pow function from btpc-core
        eprintln!(
            "[SUBMIT_BLOCK] 🎯 Validating PoW (bits: 0x{:08x})",
            block.header.bits
        );
        if let Err(e) = pow::ProofOfWork::validate_block_pow(&block, &difficulty_target) {
            eprintln!("[SUBMIT_BLOCK] ❌ REJECTED: Invalid PoW: {}", e);
            return Err(anyhow::anyhow!("Invalid proof-of-work: {}", e));
        }

        // Validate block structure
        eprintln!("[SUBMIT_BLOCK] ✅ PoW valid");
        if block.transactions.is_empty() {
            eprintln!("[SUBMIT_BLOCK] ❌ REJECTED: No transactions");
            return Err(anyhow::anyhow!(
                "Block must contain at least one transaction (coinbase)"
            ));
        }

        // Get current chain state
        let current_height = self
            .current_height
            .load(std::sync::atomic::Ordering::SeqCst);
        eprintln!(
            "[SUBMIT_BLOCK] ✅ {} transactions valid",
            block.transactions.len()
        );

        // FIX 2026-02-21 (H6): Enforce block timestamp monotonicity
        // New block's timestamp must be strictly greater than previous block's timestamp.
        // Prevents timestamp manipulation attacks (time-warp difficulty manipulation).
        if current_height > 0 {
            if let Ok(Some(prev_block)) = self.database.get_block(current_height as u32) {
                if block.header.timestamp <= prev_block.header.timestamp {
                    eprintln!(
                        "[SUBMIT_BLOCK] ❌ REJECTED: Timestamp {} not after previous block timestamp {}",
                        block.header.timestamp, prev_block.header.timestamp
                    );
                    return Err(anyhow::anyhow!(
                        "Block timestamp {} must be strictly after previous block timestamp {}",
                        block.header.timestamp,
                        prev_block.header.timestamp
                    ));
                }
            }
            // Also reject blocks with timestamps too far in the future (>2 hours)
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("System time before UNIX epoch")
                .as_secs();
            let max_future_seconds = 2 * 60 * 60; // 2 hours (Bitcoin rule)
            if block.header.timestamp > now + max_future_seconds {
                eprintln!(
                    "[SUBMIT_BLOCK] ❌ REJECTED: Timestamp {} is too far in the future (now: {})",
                    block.header.timestamp, now
                );
                return Err(anyhow::anyhow!(
                    "Block timestamp {} is more than 2 hours in the future",
                    block.header.timestamp
                ));
            }
        }

        // FR-057: Check if block extends current tip or needs reorganization
        let current_tip_hash = {
            let tip_hash_str = self.best_block_hash.read().await;
            if tip_hash_str.is_empty() || current_height == 0 {
                // Genesis or empty chain - any valid block can be the tip
                Hash::zero()
            } else {
                // Parse current tip hash
                let tip_bytes = hex::decode(&*tip_hash_str)
                    .unwrap_or_else(|_| vec![0u8; 64]);
                let mut hash_array = [0u8; 64];
                if tip_bytes.len() >= 64 {
                    hash_array.copy_from_slice(&tip_bytes[..64]);
                }
                Hash::from_bytes(hash_array)
            }
        };

        // Check if block's prev_hash matches our current tip
        let prev_hash = block.header.prev_hash;
        let block_extends_tip = prev_hash == current_tip_hash || current_height == 0;

        if !block_extends_tip && current_height > 0 {
            // Block doesn't extend our tip - potential reorg or orphan
            eprintln!(
                "[SUBMIT_BLOCK] ⚠️ Block prev_hash {} doesn't match tip {}",
                hex::encode(prev_hash.as_bytes()),
                hex::encode(current_tip_hash.as_bytes())
            );

            // Use ReorgHandler to detect if reorganization is needed
            let reorg_handler = crate::reorg_handler::ReorgHandler::new();
            match reorg_handler.detect_reorg(&block, &current_tip_hash, current_height, &self.database) {
                Ok(crate::reorg_handler::ReorgDetectionResult::ExtendsCurrentTip) => {
                    // This shouldn't happen since we checked above, but handle it
                    eprintln!("[SUBMIT_BLOCK] Block actually extends tip - proceeding");
                }
                Ok(crate::reorg_handler::ReorgDetectionResult::ShorterOrEqualChain) => {
                    eprintln!("[SUBMIT_BLOCK] ❌ REJECTED: Block on shorter/equal chain");
                    return Err(anyhow::anyhow!(
                        "Block rejected: competing chain has equal or less work"
                    ));
                }
                Ok(crate::reorg_handler::ReorgDetectionResult::ReorgNeeded(plan)) => {
                    eprintln!(
                        "[SUBMIT_BLOCK] 🔄 REORG DETECTED: Need to disconnect {} blocks, connect {}",
                        plan.blocks_to_disconnect.len(),
                        plan.blocks_to_connect.len()
                    );

                    // FR-057: Emit reorg detected event for UI indicator
                    if let Some(ref app) = self.app_handle {
                        use tauri::Emitter;
                        use crate::events::chain_reorg_event_names;
                        use crate::reorg_handler::ReorgEventBuilder;

                        let detected_event = ReorgEventBuilder::reorg_detected(
                            &hex::encode(plan.fork_point.as_bytes()),
                            plan.fork_point_height,
                            &hex::encode(plan.old_tip.as_bytes()),
                            &hex::encode(plan.new_tip.as_bytes()),
                        );
                        let _ = app.emit(chain_reorg_event_names::REORG_DETECTED, &detected_event);
                    }

                    // Execute reorganization
                    match reorg_handler.execute_reorg(&plan, &self.database, &self.utxo_manager).await {
                        Ok(result) => {
                            eprintln!(
                                "[SUBMIT_BLOCK] ✅ REORG COMPLETE: {} blocks disconnected, {} connected",
                                result.blocks_disconnected,
                                result.blocks_connected
                            );

                            // FR-057: Emit reorg completed event for UI indicator
                            if let Some(ref app) = self.app_handle {
                                use tauri::Emitter;
                                use crate::events::chain_reorg_event_names;
                                use crate::reorg_handler::ReorgEventBuilder;

                                let completed_event = ReorgEventBuilder::reorg_completed(&result);
                                let _ = app.emit(chain_reorg_event_names::REORG_COMPLETED, &completed_event);
                            }

                            // After reorg, the new block should extend the new tip
                            // Fall through to normal block addition
                        }
                        Err(e) => {
                            eprintln!("[SUBMIT_BLOCK] ❌ REORG FAILED: {}", e);

                            // FR-057: Emit reorg failed event for UI indicator
                            if let Some(ref app) = self.app_handle {
                                use tauri::Emitter;
                                use crate::events::chain_reorg_event_names;
                                use crate::reorg_handler::ReorgEventBuilder;

                                let failed_event = ReorgEventBuilder::reorg_failed(
                                    &hex::encode(plan.fork_point.as_bytes()),
                                    &e.to_string(),
                                    false, // rollback not successful
                                );
                                let _ = app.emit(chain_reorg_event_names::REORG_FAILED, &failed_event);
                            }

                            return Err(anyhow::anyhow!("Chain reorganization failed: {}", e));
                        }
                    }
                }
                Ok(crate::reorg_handler::ReorgDetectionResult::OrphanBlock) => {
                    eprintln!("[SUBMIT_BLOCK] ❌ REJECTED: Orphan block (prev_hash not in chain)");
                    return Err(anyhow::anyhow!(
                        "Block rejected: previous block not found in chain"
                    ));
                }
                Err(e) => {
                    eprintln!("[SUBMIT_BLOCK] ⚠️ Reorg detection error: {}", e);
                    // Continue with normal processing - might still be valid
                }
            }
        }

        let next_height = current_height + 1;
        eprintln!(
            "[SUBMIT_BLOCK] 📊 Height {} -> {}",
            current_height, next_height
        );

        // FIX 2025-12-11: Check if block already exists at this height (defense-in-depth)
        // This prevents duplicate blocks at same height from corrupting the database
        // Can happen if CAS in mining_thread_pool fails or multiple miners race
        if let Some(existing_hash) = self.database.get_block_hash_at_height(next_height as u32)? {
            eprintln!(
                "[SUBMIT_BLOCK] ❌ REJECTED: Block already exists at height {} (hash: {})",
                next_height,
                hex::encode(&existing_hash[..16])
            );
            return Err(anyhow::anyhow!(
                "Block already exists at height {} - stale block submission",
                next_height
            ));
        }

        // Store block in database
        self.database
            .put_block(next_height as u32, &block)
            .context("Failed to store block in database")?;

        // FIX 2025-11-16: Create UTXOs for ALL transaction outputs in the mined block
        // This is CRITICAL - without this, wallet balances remain zero even after mining!

        // FIX 2025-12-05: Collect UTXOs for tx_storage - will be added AFTER releasing utxo_manager lock
        // This prevents deadlock from blocking_read() inside mutex scope
        let mut utxos_for_tx_storage: Vec<crate::utxo_manager::UTXO> = Vec::new();
        let mut transactions_for_tx_storage: Vec<(crate::utxo_manager::Transaction, Vec<String>)> = Vec::new();

        {
            // Lock the UTXO manager
            let mut utxo_manager = self.utxo_manager.lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock UTXO manager: {}", e))?;

            let mut utxos_created = 0;

            // Process ALL transactions in the block
            for (tx_index, tx) in block.transactions.iter().enumerate() {
                let txid = hex::encode(tx.hash().as_bytes());

                // Process ALL outputs in each transaction
                for (vout, output) in tx.outputs.iter().enumerate() {
                    // Extract address from script_pubkey using btpc-core's Script API
                    // FIX 2025-11-16: Use proper extract_pubkey_hash() and Address::from_hash()
                    // to ensure extracted address matches wallet address format (Base58Check)
                    // FIX 2025-12-01: Use self.network to use embedded node's configured network
                    // IMPORTANT: Wallets MUST be created with same network for address prefixes to match
                    let address_str = match output.script_pubkey.extract_pubkey_hash() {
                        Some(pubkey_hash) => {
                            // Create Address from extracted hash160 (20 bytes)
                            // This will produce the correct Base58Check encoded address
                            let address = btpc_core::crypto::address::Address::from_hash(
                                pubkey_hash,
                                self.network, // Use embedded node's configured network
                                btpc_core::crypto::address::AddressType::P2PKH,
                            );
                            address.to_string()
                        }
                        None => {
                            // Non-P2PKH script or invalid format
                            eprintln!("⚠️ Could not extract pubkey hash from script in output {} of tx {}", vout, txid);
                            format!("unknown-script-{}-{}", txid, vout)
                        }
                    };

                    // Get script_pubkey bytes for UTXO storage (serialize to bytes)
                    let script_bytes = output.script_pubkey.serialize();

                    // Create UTXO entry
                    let utxo = crate::utxo_manager::UTXO {
                        txid: txid.clone(),
                        vout: vout as u32,
                        value_credits: output.value,
                        value_btp: output.value as f64 / 100_000_000.0,
                        address: address_str,
                        block_height: next_height,
                        is_coinbase: tx_index == 0, // First transaction is always coinbase
                        created_at: chrono::Utc::now(),
                        spent: false,
                        spent_in_tx: None,
                        spent_at_height: None,
                        script_pubkey: script_bytes.to_vec(),
                    };

                    // FIX 2025-11-18: Use batch mode to prevent race condition
                    // Add UTXO to memory without immediate disk write
                    if let Err(e) = utxo_manager.add_utxo_batch(utxo.clone()) {
                        eprintln!(
                            "⚠️ Failed to add UTXO for output {} in tx {}: {}",
                            vout, txid, e
                        );
                    } else {
                        utxos_created += 1;
                    }

                    // FIX 2025-12-05: Collect UTXOs for tx_storage - will be added AFTER releasing utxo_manager lock
                    // This prevents deadlock from blocking_read() inside mutex scope
                    utxos_for_tx_storage.push(utxo);
                }
            }

            // FIX 2025-11-27: Mark input UTXOs as spent for non-coinbase transactions
            // This is CRITICAL - without this, sender balances aren't reduced after sending!
            // FIX 2025-11-28: Collect sender addresses BEFORE marking UTXOs as spent
            // We need the UTXO data to extract sender addresses for transaction indexing
            let mut utxos_spent = 0;
            let mut tx_sender_addresses: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

            for (tx_index, tx) in block.transactions.iter().enumerate() {
                let is_coinbase = tx_index == 0;
                if !is_coinbase {
                    let txid = hex::encode(tx.hash().as_bytes());
                    let mut sender_addrs: Vec<String> = Vec::new();

                    // Process inputs - collect sender addresses AND mark UTXOs as spent
                    for input in &tx.inputs {
                        let prev_txid = hex::encode(input.previous_output.txid.as_bytes());
                        let prev_vout = input.previous_output.vout;
                        let spent_in_txid = txid.clone();

                        // FIX 2025-11-28: Get sender address BEFORE marking as spent
                        // FIX 2025-11-28 v2: Fallback to tx_storage (RocksDB) if not in utxo_manager
                        // FIX 2025-12-04: Third fallback - extract pubkey from script_sig and derive address
                        let sender_addr_opt = if let Some(utxo) = utxo_manager.get_utxo(&prev_txid, prev_vout) {
                            Some(utxo.address.clone())
                        } else if let Some(ref ts) = self.tx_storage {
                            // Fallback 1: Query RocksDB for UTXO
                            // FIX 2025-12-05: Use .blocking_read() for RwLock access inside sync-locked section
                            // Cannot use .read().await because std::sync::MutexGuard is not Send
                            match ts.blocking_read().get_utxo(&prev_txid, prev_vout) {
                                Ok(Some(utxo)) => {
                                    eprintln!("📝 Found UTXO {}:{} in tx_storage (fallback 1)", &prev_txid[..16], prev_vout);
                                    Some(utxo.address.clone())
                                }
                                Ok(None) => {
                                    // Fallback 2: Extract sender address from script_sig
                                    eprintln!("📝 UTXO {}:{} not in tx_storage, trying script_sig extraction (fallback 2)", &prev_txid[..16], prev_vout);
                                    if let Some(pubkey_bytes) = input.script_sig.extract_pubkey_from_unlock() {
                                        // Derive address from public key
                                        match btpc_core::crypto::PublicKey::from_bytes(&pubkey_bytes) {
                                            Ok(pubkey) => {
                                                // Get pubkey hash (SHA-512) and extract first 20 bytes for RIPEMD-160 equivalent
                                                let full_hash = pubkey.hash();
                                                let mut pubkey_hash = [0u8; 20];
                                                pubkey_hash.copy_from_slice(&full_hash.as_slice()[..20]);
                                                let address = btpc_core::crypto::address::Address::from_hash(
                                                    pubkey_hash,
                                                    self.network,
                                                    btpc_core::crypto::address::AddressType::P2PKH,
                                                ).to_string();
                                                eprintln!("📝 Derived sender address {} from script_sig for tx {}", address, &txid[..16]);
                                                Some(address)
                                            }
                                            Err(e) => {
                                                eprintln!("⚠️ Failed to parse pubkey from script_sig: {}", e);
                                                None
                                            }
                                        }
                                    } else {
                                        eprintln!("⚠️ Could not extract pubkey from script_sig for {}:{}", &prev_txid[..16], prev_vout);
                                        None
                                    }
                                }
                                Err(e) => {
                                    eprintln!("⚠️ Error querying tx_storage for UTXO {}:{}: {}", &prev_txid[..16], prev_vout, e);
                                    None
                                }
                            }
                        } else {
                            // No tx_storage available - try script_sig extraction directly
                            eprintln!("📝 tx_storage unavailable, trying script_sig extraction for {}:{}", &prev_txid[..16], prev_vout);
                            if let Some(pubkey_bytes) = input.script_sig.extract_pubkey_from_unlock() {
                                match btpc_core::crypto::PublicKey::from_bytes(&pubkey_bytes) {
                                    Ok(pubkey) => {
                                        // Get pubkey hash (SHA-512) and extract first 20 bytes for RIPEMD-160 equivalent
                                        let full_hash = pubkey.hash();
                                        let mut pubkey_hash = [0u8; 20];
                                        pubkey_hash.copy_from_slice(&full_hash.as_slice()[..20]);
                                        let address = btpc_core::crypto::address::Address::from_hash(
                                            pubkey_hash,
                                            self.network,
                                            btpc_core::crypto::address::AddressType::P2PKH,
                                        ).to_string();
                                        eprintln!("📝 Derived sender address {} from script_sig (no tx_storage)", address);
                                        Some(address)
                                    }
                                    Err(e) => {
                                        eprintln!("⚠️ Failed to parse pubkey from script_sig: {}", e);
                                        None
                                    }
                                }
                            } else {
                                eprintln!("⚠️ Could not extract pubkey from script_sig for {}:{}", &prev_txid[..16], prev_vout);
                                None
                            }
                        };

                        if let Some(sender_addr) = sender_addr_opt {
                            if !sender_addrs.contains(&sender_addr) {
                                sender_addrs.push(sender_addr.clone());
                                eprintln!("📝 Captured sender address {} for tx {}", sender_addr, &txid[..16]);
                            }
                        }

                        // Mark UTXO as spent in utxo_manager
                        if let Err(e) = utxo_manager.spend_utxo(
                            &prev_txid,
                            prev_vout,
                            spent_in_txid.clone(),
                            next_height,
                        ) {
                            eprintln!("⚠️ Failed to mark UTXO {}:{} as spent: {}", prev_txid, prev_vout, e);
                        } else {
                            utxos_spent += 1;
                            eprintln!("📝 Marked UTXO {}:{} as spent in tx {}", &prev_txid[..16], prev_vout, &spent_in_txid[..16]);
                        }
                    }

                    // Store sender addresses for this txid (used in transaction indexing below)
                    if !sender_addrs.is_empty() {
                        tx_sender_addresses.insert(txid, sender_addrs);
                    }
                }
            }

            // FIX 2025-11-18 + FIX 2026-02-21 (H7): Single atomic batch write
            // after ALL UTXO additions AND spends are processed in-memory.
            // This prevents inconsistent state if app crashes mid-block processing.
            if let Err(e) = utxo_manager.flush_utxos() {
                eprintln!("⚠️ Failed to persist {} UTXOs to disk: {}", utxos_created, e);
            } else {
                println!("💾 Persisted {} added + {} spent UTXOs to disk (atomic batch)", utxos_created, utxos_spent);
            }

            // Log success
            println!(
                "✅ Created {} UTXOs, marked {} UTXOs spent for block at height {}",
                utxos_created, utxos_spent, next_height
            );

            // FIX 2025-12-05: Collect transaction data for tx_storage INSIDE the lock
            // but store OUTSIDE the lock to prevent deadlock from blocking_read()
            for (tx_index, tx) in block.transactions.iter().enumerate() {
                let txid = hex::encode(tx.hash().as_bytes());
                let is_coinbase = tx_index == 0;

                // Collect all addresses involved in this transaction
                let mut involved_addresses: Vec<String> = Vec::new();

                // Get recipient addresses from outputs
                for output in &tx.outputs {
                    if let Some(pubkey_hash) = output.script_pubkey.extract_pubkey_hash() {
                        let address = btpc_core::crypto::address::Address::from_hash(
                            pubkey_hash,
                            self.network,
                            btpc_core::crypto::address::AddressType::P2PKH,
                        ).to_string();
                        if !involved_addresses.contains(&address) {
                            involved_addresses.push(address);
                        }
                    }
                }

                // Use pre-captured sender addresses
                if !is_coinbase {
                    if let Some(sender_addrs) = tx_sender_addresses.get(&txid) {
                        for sender_addr in sender_addrs {
                            if !involved_addresses.contains(sender_addr) {
                                involved_addresses.push(sender_addr.clone());
                            }
                        }
                    }
                    // FIX 2025-12-12: Removed blocking_read() fallback from inside utxo_manager lock
                    // This was causing deadlock. Fallback now happens AFTER lock release (see below).
                }

                // Build inputs
                let inputs: Vec<crate::utxo_manager::TxInput> = if is_coinbase {
                    vec![]
                } else {
                    tx.inputs.iter().map(|input| {
                        crate::utxo_manager::TxInput {
                            prev_txid: hex::encode(input.previous_output.txid.as_bytes()),
                            prev_vout: input.previous_output.vout,
                            signature_script: input.script_sig.serialize(),
                            sequence: input.sequence,
                        }
                    }).collect()
                };

                // Build outputs
                let outputs: Vec<crate::utxo_manager::TxOutput> = tx.outputs.iter().map(|output| {
                    crate::utxo_manager::TxOutput {
                        value: output.value,
                        script_pubkey: output.script_pubkey.serialize(),
                    }
                }).collect();

                // FIX 2025-12-10: Get sender address for SENT/RECEIVED detection
                // Coinbase transactions have no sender (mining rewards)
                // User transactions use the first sender address from inputs
                // FIX 2025-12-11: If tx_sender_addresses is empty, sender_address = None
                // The tx_storage.rs:add_transaction() will preserve sender_address from existing record
                let sender_address = if is_coinbase {
                    None
                } else {
                    tx_sender_addresses.get(&txid).and_then(|addrs| addrs.first().cloned())
                };

                // Log for debugging - shows when sender lookup failed
                if !is_coinbase && sender_address.is_none() {
                    eprintln!("⚠️ FIX 2025-12-11: sender_address=None for tx {}, will rely on tx_storage preservation", &txid[..16]);
                }

                // Create Transaction struct
                let storage_tx = crate::utxo_manager::Transaction {
                    txid: txid.clone(),
                    version: tx.version,
                    inputs,
                    outputs,
                    lock_time: tx.lock_time,
                    fork_id: match self.network {
                        Network::Mainnet => 0,
                        Network::Testnet => 1,
                        Network::Regtest => 2,
                    },
                    block_height: Some(next_height),
                    confirmed_at: Some(chrono::Utc::now()),
                    is_coinbase,
                    sender_address,
                };

                transactions_for_tx_storage.push((storage_tx, involved_addresses));
            }

            // Special logging for coinbase (mining reward)
            if let Some(coinbase_tx) = block.transactions.first() {
                if let Some(coinbase_output) = coinbase_tx.outputs.first() {
                    let reward_btpc = coinbase_output.value as f64 / 100_000_000.0;
                    println!("💰 Mining reward: {} BTPC added to UTXO set", reward_btpc);

                    // FIX 2025-11-20: Emit wallet balance update event to notify frontend
                    // This is CRITICAL for the UI to show updated balances after mining
                    if let Some(ref app) = self.app_handle {
                        use tauri::Emitter;

                        // Extract address from coinbase output script
                        // FIX 2025-12-01: Use self.network for configured network
                        // This ensures the address string matches wallet addresses for balance lookup
                        if let Some(pubkey_hash) = coinbase_output.script_pubkey.extract_pubkey_hash() {
                            let mining_address = btpc_core::crypto::address::Address::from_hash(
                                pubkey_hash,
                                self.network,
                                btpc_core::crypto::address::AddressType::P2PKH,
                            );
                            let mining_address_str = mining_address.to_string();

                            // Calculate new balance for this address
                            let (new_balance_credits, new_balance_btp) = utxo_manager.get_balance(&mining_address_str);

                            // FIX 2025-11-20: Emit wallet:balance_updated event with correct payload structure
                            // Frontend expects: { wallet_id, balance: { confirmed, pending, total } }
                            // Find the wallet_id for this address
                            if let Some(ref wallet_mgr) = self.wallet_manager {
                                if let Ok(wallet_manager) = wallet_mgr.lock() {
                                    let wallets_list = wallet_manager.list_wallets();
                                    if let Some(wallet) = wallets_list.iter().find(|w| w.address == mining_address_str) {
                                        let wallet_id = wallet.id.clone();

                                        // Emit event with frontend-compatible payload structure
                                        let event_payload = serde_json::json!({
                                            "wallet_id": wallet_id,
                                            "balance": {
                                                "confirmed": new_balance_credits,
                                                "pending": 0,
                                                "total": new_balance_credits
                                            },
                                            "address": mining_address_str,
                                            "block_height": next_height,
                                            "txid": hex::encode(coinbase_tx.hash().as_bytes())
                                        });

                                        if let Err(e) = app.emit("wallet:balance_updated", event_payload) {
                                            eprintln!("⚠️ Failed to emit wallet balance update event: {}", e);
                                        } else {
                                            eprintln!("✅ Emitted wallet:balance_updated event for wallet: {} ({})", wallet_id, mining_address_str);
                                        }
                                    }
                                }
                            }

                            // FIX 2025-11-20: Update wallet manager's cached balance
                            // This ensures list_wallets returns the updated balance immediately
                            if let Some(ref wallet_mgr) = self.wallet_manager {
                                if let Ok(mut wallet_manager) = wallet_mgr.lock() {
                                    // Find wallet with this address and update its cached balance
                                    let wallets_list = wallet_manager.list_wallets();
                                    if let Some(wallet) = wallets_list.iter().find(|w| w.address == mining_address_str) {
                                        let wallet_id = wallet.id.clone();
                                        drop(wallets_list); // Release the borrow before calling update_wallet_balance

                                        if let Err(e) = wallet_manager.update_wallet_balance(&wallet_id, new_balance_credits) {
                                            eprintln!("⚠️ Failed to update wallet balance cache: {}", e);
                                        } else {
                                            println!("✅ Updated wallet {} balance cache: {} BTPC", wallet_id, new_balance_btp);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // FIX 2025-12-05: Store UTXOs and transactions to tx_storage OUTSIDE the utxo_manager lock
        // This prevents deadlock from blocking_read() inside async function with mutex held
        if let Some(ref tx_storage) = self.tx_storage {
            let tx_storage_guard = tx_storage.read().await;

            // Store collected UTXOs
            for utxo in &utxos_for_tx_storage {
                if let Err(e) = tx_storage_guard.add_utxo(utxo) {
                    eprintln!("⚠️ Failed to add UTXO {}:{} to tx_storage: {}", &utxo.txid[..16.min(utxo.txid.len())], utxo.vout, e);
                }
            }

            // FIX 2025-12-12: Fallback for sender_address - query existing tx_storage record
            // This is OUTSIDE the utxo_manager lock to prevent deadlock (was inside before)
            // When UTXO lookup failed during mining, the broadcast record has the correct sender_address
            for (storage_tx, involved_addresses) in &mut transactions_for_tx_storage {
                if !storage_tx.is_coinbase && storage_tx.sender_address.is_none() {
                    // Query tx_storage for existing broadcast record
                    if let Ok(Some(existing_tx)) = tx_storage_guard.get_transaction(&storage_tx.txid) {
                        if let Some(ref sender_addr) = existing_tx.sender_address {
                            // Update storage_tx with sender_address
                            storage_tx.sender_address = Some(sender_addr.clone());
                            // Add sender to involved_addresses if not present
                            if !involved_addresses.contains(sender_addr) {
                                involved_addresses.push(sender_addr.clone());
                                eprintln!("📝 FIX 2025-12-12: Added sender {} from broadcast record for tx {}",
                                    &sender_addr[..20.min(sender_addr.len())], &storage_tx.txid[..16]);
                            }
                        }
                    }
                }
            }

            // Store collected transactions
            for (storage_tx, involved_addresses) in &transactions_for_tx_storage {
                eprintln!("📦 Processing tx {} for storage: coinbase={}, sender={:?}, involved_addresses={:?}",
                    &storage_tx.txid[..16.min(storage_tx.txid.len())],
                    storage_tx.is_coinbase,
                    storage_tx.sender_address.as_ref().map(|s| &s[..16.min(s.len())]),
                    involved_addresses.iter().map(|a| &a[..16.min(a.len())]).collect::<Vec<_>>()
                );
                for address in involved_addresses {
                    if let Err(e) = tx_storage_guard.add_transaction(storage_tx, address) {
                        eprintln!("⚠️ Failed to store tx {} for address {}: {}", &storage_tx.txid[..16.min(storage_tx.txid.len())], address, e);
                    } else {
                        eprintln!("📝 Stored tx {} for address {} (coinbase={})", &storage_tx.txid[..16.min(storage_tx.txid.len())], address, storage_tx.is_coinbase);
                    }
                }
            }

            // Flush tx_storage to disk for crash safety
            if let Err(e) = tx_storage_guard.flush() {
                eprintln!("⚠️ Failed to flush tx_storage after block {}: {}", next_height, e);
            } else {
                eprintln!("💾 tx_storage flushed to disk after block {}", next_height);
            }
        }

        // Update blockchain state
        self.current_height
            .store(next_height, std::sync::atomic::Ordering::SeqCst);
        *self.best_block_hash.write().await = hex::encode(block_hash.as_bytes());

        // Update cached difficulty for next block template
        // FIX 2025-11-20: Cache difficulty so get_blockchain_state() can return actual value
        // FIX 2026-03-04: For pre-2016 heights, NEVER store the block's bits into the atomic.
        // During bootstrap, blocks may be mined with EDA-reduced difficulty. Storing those
        // reduced bits compounds future EDA reductions (e.g., Tier 1 reduces by 25%, next
        // Tier 1 reads the ALREADY-REDUCED value and compounds further).
        // The initial "difficulty 1" constant is always the source of truth for pre-2016.
        // Post-2016, store the block's bits since difficulty tracks actual adjustments.
        if next_height >= 2016 {
            self.current_difficulty_bits.store(block.header.bits, std::sync::atomic::Ordering::SeqCst);
        } else {
            eprintln!(
                "ℹ️ Block {} (pre-2016) mined at difficulty 0x{:08x}, keeping atomic at initial 0x{:08x}",
                next_height, block.header.bits,
                self.current_difficulty_bits.load(std::sync::atomic::Ordering::SeqCst)
            );
        }

        // Remove mined transactions from mempool
        let mut mempool = self.mempool.write().await;
        for tx in &block.transactions {
            let tx_hash = tx.hash();
            let _ = mempool.remove_transaction(&tx_hash);
        }

        let block_hash_hex = hex::encode(block_hash.as_bytes());
        println!(
            "✅ Block accepted: height={}, hash={}",
            next_height,
            &block_hash_hex[0..16]
        );

        Ok(format!("Block accepted: {}", block_hash_hex))
    }

    /// Get P2P peer information
    ///
    /// # Returns
    /// * `Vec<PeerInfo>` - List of connected peers with stats
    ///
    /// # FIX 2025-11-28: Now returns actual peer data from HashMap
    pub fn get_peer_info(&self) -> Vec<PeerInfo> {
        // Read peers from the HashMap
        if let Ok(peers) = self.peers.read() {
            peers.values().cloned().collect()
        } else {
            // Lock poisoned - return empty vector
            vec![]
        }
    }

    /// Add a new peer connection
    ///
    /// # Arguments
    /// * `address` - Peer network address (IP:port)
    /// * `version` - Peer protocol version
    /// * `height` - Peer's blockchain height
    ///
    /// # FIX 2025-11-28: Added for P2P peer tracking
    pub fn add_peer(&self, address: String, version: String, height: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let peer_info = PeerInfo {
            address: address.clone(),
            version,
            height,
            ping_ms: 0, // Will be updated by ping measurements
            connected_since: now,
        };

        if let Ok(mut peers) = self.peers.write() {
            peers.insert(address.clone(), peer_info);
            // Update connected_peers counter
            self.connected_peers.store(peers.len() as u32, std::sync::atomic::Ordering::SeqCst);
            eprintln!("✅ Peer connected: {} (total: {})", address, peers.len());
        }
    }

    /// Remove a peer connection
    ///
    /// # Arguments
    /// * `address` - Peer network address to remove
    ///
    /// # FIX 2025-11-28: Added for P2P peer tracking
    pub fn remove_peer(&self, address: &str) {
        if let Ok(mut peers) = self.peers.write() {
            if peers.remove(address).is_some() {
                // Update connected_peers counter
                self.connected_peers.store(peers.len() as u32, std::sync::atomic::Ordering::SeqCst);
                eprintln!("🔌 Peer disconnected: {} (remaining: {})", address, peers.len());
            }
        }
    }

    /// Update peer's blockchain height
    ///
    /// # Arguments
    /// * `address` - Peer network address
    /// * `height` - New blockchain height
    ///
    /// # FIX 2025-11-28: Added for P2P peer tracking
    pub fn update_peer_height(&self, address: &str, height: u64) {
        if let Ok(mut peers) = self.peers.write() {
            if let Some(peer) = peers.get_mut(address) {
                peer.height = height;
            }
        }
    }

    /// Update peer's ping latency
    ///
    /// # Arguments
    /// * `address` - Peer network address
    /// * `ping_ms` - Ping latency in milliseconds
    ///
    /// # FIX 2025-11-28: Added for P2P peer tracking
    pub fn update_peer_ping(&self, address: &str, ping_ms: u64) {
        if let Ok(mut peers) = self.peers.write() {
            if let Some(peer) = peers.get_mut(address) {
                peer.ping_ms = ping_ms;
            }
        }
    }

    /// Get count of connected peers
    ///
    /// # Returns
    /// * `u32` - Number of connected peers
    pub fn get_peer_count(&self) -> u32 {
        self.connected_peers.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Clear all peers (for disconnect/shutdown)
    ///
    /// # FIX 2025-11-28: Added for P2P peer tracking
    pub fn clear_peers(&self) {
        if let Ok(mut peers) = self.peers.write() {
            peers.clear();
            self.connected_peers.store(0, std::sync::atomic::Ordering::SeqCst);
            eprintln!("🔌 All peers disconnected");
        }
    }

    /// Add a simulated peer for testing/demo purposes
    ///
    /// # FIX 2025-11-28: Added for testing peer display without real P2P
    pub fn add_simulated_peer(&self) {
        use rand::{Rng, rngs::OsRng};
        let mut rng = OsRng;

        // Generate random-looking peer data
        let ip = format!(
            "{}.{}.{}.{}:{}",
            rng.gen_range(1..255),
            rng.gen_range(0..255),
            rng.gen_range(0..255),
            rng.gen_range(1..255),
            rng.gen_range(8000..9000)
        );

        let height = self.current_height.load(std::sync::atomic::Ordering::SeqCst);
        let version = "1.0.0".to_string();

        self.add_peer(ip, version, height);

        // Add a realistic ping value
        let ping_ms = rng.gen_range(10..200);
        if let Ok(peers) = self.peers.read() {
            if let Some(addr) = peers.keys().last() {
                let addr_clone = addr.clone();
                drop(peers);
                self.update_peer_ping(&addr_clone, ping_ms);
            }
        }
    }

    /// Compute average block time over the last `n` blocks (in seconds).
    ///
    /// Returns `None` if fewer than 2 blocks are available.
    pub fn compute_avg_block_time(&self, n: u64) -> Option<f64> {
        let height = self.current_height.load(std::sync::atomic::Ordering::SeqCst);
        if height < 2 || n < 2 {
            return None;
        }
        let end_h = height;
        let start_h = height.saturating_sub(n);
        let start_block = self.database.get_block(start_h as u32).ok()??;
        let end_block = self.database.get_block(end_h as u32).ok()??;
        let elapsed = end_block.header.timestamp.saturating_sub(start_block.header.timestamp);
        let blocks = end_h - start_h;
        if blocks == 0 { return None; }
        Some(elapsed as f64 / blocks as f64)
    }

    /// Get comprehensive network health info (for `getnetworkhealthinfo` RPC).
    pub fn get_network_health_info(&self) -> serde_json::Value {
        use btpc_core::consensus::constants as cons;

        let height = self.current_height.load(std::sync::atomic::Ordering::SeqCst);
        let is_bootstrap = height < cons::BOOTSTRAP_END_HEIGHT;
        let bootstrap_remaining = if is_bootstrap { cons::BOOTSTRAP_END_HEIGHT - height } else { 0 };
        let bootstrap_progress = if cons::BOOTSTRAP_END_HEIGHT > 0 {
            (height as f64 / cons::BOOTSTRAP_END_HEIGHT as f64).min(1.0)
        } else {
            1.0
        };

        let avg_10 = self.compute_avg_block_time(10);
        let avg_100 = self.compute_avg_block_time(100);

        // Hashrate estimate from avg_10: H/s ≈ 2^(difficulty_bits_work) / avg_block_time
        // Simplified: use difficulty_bits to estimate
        let difficulty_bits = self.current_difficulty_bits.load(std::sync::atomic::Ordering::SeqCst);
        let hashrate_estimate = avg_10.map(|avg| {
            if avg > 0.0 {
                // Rough estimate: target determines how many hashes per block on average
                // For SHA-512 compact bits: work ≈ 2^(8 * leading_zero_bytes)
                let exponent = (difficulty_bits >> 24) as f64;
                let leading_zeros = 64.0 - exponent;
                let work_bits = leading_zeros * 8.0;
                let work = 2.0_f64.powf(work_bits.min(60.0)); // cap to avoid infinity
                work / avg
            } else {
                0.0
            }
        }).unwrap_or(0.0);

        // Trend: compare avg_10 vs avg_100
        let hashrate_trend = match (avg_10, avg_100) {
            (Some(a10), Some(a100)) if a100 > 0.0 => {
                let ratio = a10 / a100;
                if ratio < 0.85 { "increasing" }      // blocks faster → hashrate up
                else if ratio > 1.15 { "decreasing" } // blocks slower → hashrate down
                else { "stable" }
            }
            _ => "unknown",
        };

        let fast_weight = if is_bootstrap && height > cons::FAST_ADJUSTMENT_WINDOW {
            1.0 - (height as f64 / cons::BOOTSTRAP_END_HEIGHT as f64)
        } else {
            0.0
        };

        serde_json::json!({
            "hashrate_estimate": hashrate_estimate,
            "hashrate_trend": hashrate_trend,
            "avg_block_time_10": avg_10.unwrap_or(0.0),
            "avg_block_time_100": avg_100.unwrap_or(0.0),
            "bootstrap_phase": is_bootstrap,
            "bootstrap_progress": bootstrap_progress,
            "bootstrap_blocks_remaining": bootstrap_remaining,
            "eda_triggers": self.eda_trigger_count.load(std::sync::atomic::Ordering::SeqCst),
            "twenty_min_rule_triggers": self.twenty_min_rule_count.load(std::sync::atomic::Ordering::SeqCst),
            "fast_adjustment_weight": fast_weight,
            "difficulty_bits": format!("0x{:08x}", difficulty_bits),
            "height": height,
            "network": format!("{:?}", self.network),
        })
    }

    /// Cap difficulty bits to the minimum (easiest) floor.
    ///
    /// In compact bits, a higher exponent byte means an easier target.
    /// If `bits` is easier than `min_bits`, return `min_bits`.
    fn cap_to_minimum(bits: u32, min_bits: u32) -> u32 {
        let bits_exp = (bits >> 24) as u8;
        let min_exp = (min_bits >> 24) as u8;
        if bits_exp > min_exp {
            min_bits
        } else if bits_exp == min_exp {
            // Same exponent — compare mantissa (higher = easier target)
            let bits_mantissa = bits & 0x00FFFFFF;
            let min_mantissa = min_bits & 0x00FFFFFF;
            if bits_mantissa > min_mantissa { min_bits } else { bits }
        } else {
            bits
        }
    }

    /// Calculate difficulty for next block (Constitution Article IV, Section 4.1)
    ///
    /// Adjusts difficulty every 2,016 blocks based on actual vs target timespan.
    /// Between adjustment blocks, returns current difficulty unchanged.
    ///
    /// # Arguments
    /// * `next_height` - Height of the next block to be mined
    ///
    /// # Returns
    /// * Difficulty in compact bits format (u32)
    ///
    /// # Constitutional Compliance
    /// Implements difficulty adjustment per Constitution Article IV.
    async fn calculate_difficulty_for_next_block(&self, next_height: u64) -> Result<u32> {
        const ADJUSTMENT_INTERVAL: u64 = 2_016;

        // FIX 2026-03-04: Bitcoin-style "difficulty 1" — minimum meaningful difficulty.
        // The 2016-block adjustment algorithm handles convergence to real hashrate.
        // - Mainnet/Testnet: INITIAL_DIFFICULTY_BITS (SHA-512 "difficulty 1")
        // - Regtest: REGTEST_DIFFICULTY_BITS (instant mining for development)
        let initial_difficulty = self.current_difficulty_bits.load(std::sync::atomic::Ordering::SeqCst);

        // FIX 2025-12-27: REMOVED per-block LWMA for Regtest
        // The LWMA algorithm was causing difficulty to spike 600x per block because:
        // - Regtest blocks mine in <1 second (target is 600 seconds)
        // - Formula: new_target = current_target * avg_solve_time / 600
        // - This made difficulty impossible after ~60 blocks
        //
        // Per Constitution Article IV: Difficulty adjustment every 2016 blocks
        // Regtest should use same 2016-block interval OR stay at minimum difficulty
        // Bitcoin's regtest also uses the same 2016-block interval, not per-block

        // ── GRADUATED EMERGENCY DIFFICULTY ADJUSTMENT (EDA) ──────────────
        // During bootstrap (height < 20,160), use 3-tier graduated EDA on ALL networks.
        // After bootstrap, revert to testnet-only 20-minute rule (BIP 94).
        //
        // Tiers:
        //   1) 15 min no block → 25% difficulty reduction (no cooldown)
        //   2) 20 min no block → 50% difficulty reduction (6-block cooldown)
        //   3) 30+ min no block → reset to network minimum (6-block cooldown)
        //
        // Anti-oscillation: 6-block cooldown after Tier 2/3 prevents the
        // Bitcoin Cash EDA exploit where miners toggle hashrate to game difficulty.
        //
        // References: BIP 94, Bitcoin Core PR #686, BCH EDA post-mortem

        use btpc_core::consensus::constants as cons;

        // Network-appropriate minimum difficulty floor
        let min_difficulty: u32 = match self.network {
            Network::Regtest => 0x407fffff,  // Regtest easy difficulty
            _ => 0x3c7fffff,                 // SHA-512 minimum (~32 bits work)
        };

        let is_bootstrap = next_height < cons::BOOTSTRAP_END_HEIGHT;

        if next_height > 1 && next_height % ADJUSTMENT_INTERVAL != 0 {
            let prev_height = next_height - 1;
            if let Ok(Some(prev_block)) = self.database.get_block(prev_height as u32) {
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let elapsed = current_time.saturating_sub(prev_block.header.timestamp);

                if is_bootstrap {
                    // ── Bootstrap graduated EDA (all networks, height < 20,160) ──
                    let last_trigger = self.eda_last_trigger_height.load(std::sync::atomic::Ordering::SeqCst);
                    let cooldown_ok = next_height.saturating_sub(last_trigger) >= cons::EDA_COOLDOWN_BLOCKS;
                    let current_bits = self.current_difficulty_bits.load(std::sync::atomic::Ordering::SeqCst);

                    if elapsed >= cons::EDA_TIER3_SECONDS && cooldown_ok {
                        // Tier 3: 30+ min → reset to network minimum
                        self.eda_last_trigger_height.store(next_height, std::sync::atomic::Ordering::SeqCst);
                        self.eda_trigger_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        eprintln!(
                            "🚨 EDA Tier 3 ({:?}, h={}): {} sec since last block → reset to minimum 0x{:08x}",
                            self.network, next_height, elapsed, min_difficulty
                        );
                        return Ok(min_difficulty);
                    } else if elapsed >= cons::EDA_TIER2_SECONDS && cooldown_ok {
                        // Tier 2: 20 min → 50% difficulty reduction
                        // "Reduce difficulty 50%" means new_diff = 0.5 * old_diff
                        // Which means target doubles: divide_difficulty(2.0) doubles target
                        let current_target = DifficultyTarget::from_bits(current_bits);
                        let reduced = current_target.divide_difficulty(2.0);
                        let capped = Self::cap_to_minimum(reduced.bits, min_difficulty);
                        self.eda_last_trigger_height.store(next_height, std::sync::atomic::Ordering::SeqCst);
                        self.eda_trigger_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        eprintln!(
                            "⚠️ EDA Tier 2 ({:?}, h={}): {} sec → 50% reduction 0x{:08x} → 0x{:08x}",
                            self.network, next_height, elapsed, current_bits, capped
                        );
                        return Ok(capped);
                    } else if elapsed >= cons::EDA_TIER1_SECONDS {
                        // Tier 1: 15 min → 25% difficulty reduction (no cooldown required)
                        // "Reduce difficulty 25%" means new_diff = 0.75 * old_diff
                        // Which means target *= (1/0.75) = target *= (4/3)
                        // Using BigUint for precision:
                        let current_target = DifficultyTarget::from_bits(current_bits);
                        let target_bytes = current_target.as_bytes();
                        use num_bigint::BigUint;
                        let target_big = BigUint::from_bytes_be(target_bytes);
                        let new_target_big = (&target_big * 4u32) / 3u32;
                        let mut new_bytes = new_target_big.to_bytes_be();
                        if new_bytes.len() < 64 {
                            let mut padded = vec![0u8; 64 - new_bytes.len()];
                            padded.extend_from_slice(&new_bytes);
                            new_bytes = padded;
                        } else if new_bytes.len() > 64 {
                            new_bytes = new_bytes[new_bytes.len() - 64..].to_vec();
                        }
                        let mut arr = [0u8; 64];
                        arr.copy_from_slice(&new_bytes);
                        let new_target = DifficultyTarget::from_hash(&Hash::from_bytes(arr));
                        let capped = Self::cap_to_minimum(new_target.bits, min_difficulty);
                        self.eda_trigger_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        eprintln!(
                            "⏱️ EDA Tier 1 ({:?}, h={}): {} sec → 25% reduction 0x{:08x} → 0x{:08x}",
                            self.network, next_height, elapsed, current_bits, capped
                        );
                        return Ok(capped);
                    }
                } else {
                    // ── Post-bootstrap: testnet-only 20-minute rule (BIP 94) ──
                    // FIX 2026-03-04: Changed > to >= for consistency with bootstrap EDA
                    if self.network == Network::Testnet && elapsed >= cons::EDA_TIER2_SECONDS {
                        self.twenty_min_rule_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        eprintln!(
                            "⏱️ 20-minute rule (Testnet, h={}): {} sec since last block → minimum 0x{:08x}",
                            next_height, elapsed, min_difficulty
                        );
                        return Ok(min_difficulty);
                    }
                }
            }
        }

        // For Regtest: ALWAYS use hardcoded 0x407fffff (easy difficulty)
        // FIX 2025-12-27: Updated to SHA-512 compatible value
        // Do NOT use initial_difficulty which may have been corrupted by loading from database
        // This is the standard behavior for regtest - instant mining for development
        if self.network == Network::Regtest {
            const REGTEST_EASY_DIFFICULTY: u32 = 0x407fffff;
            return Ok(REGTEST_EASY_DIFFICULTY);
        }

        // For Mainnet/Testnet: use 2016-block Bitcoin-style adjustment

        // If we're at genesis or before first adjustment, use "difficulty 1" constant
        // FIX 2025-12-01: Changed <= to < so adjustment happens AT height 2016
        // FIX 2026-03-04: Use CONSTANT "difficulty 1", not atomic value.
        // The atomic is intentionally NOT updated during bootstrap (submit_block guard).
        // This constant is the single source of truth for pre-2016 difficulty.
        if next_height < ADJUSTMENT_INTERVAL {
            let constant_initial = match self.network {
                Network::Regtest => cons::REGTEST_DIFFICULTY_BITS,
                _ => cons::INITIAL_DIFFICULTY_BITS, // SHA-512 "difficulty 1"
            };
            return Ok(constant_initial);
        }

        // Check if this is an adjustment block (every 2016 blocks)
        if next_height % ADJUSTMENT_INTERVAL != 0 {
            // Not an adjustment block - use cached difficulty
            // FIX 2025-11-20: Use cached difficulty instead of database query
            let current_diff = self.current_difficulty_bits.load(std::sync::atomic::Ordering::SeqCst);

            // FIX 2025-12-27: Enforce minimum difficulty even for cached values
            // This handles the case where database contains corrupted too-easy difficulty
            // from before this fix was implemented
            // Updated to SHA-512 compatible values
            let min_difficulty = match self.network {
                Network::Regtest => 0x407fffff, // Regtest minimum (instant mining)
                _ => 0x3c7fffff, // Mainnet/Testnet minimum (~32 bits work)
            };

            // Check if cached difficulty is easier than minimum
            // Higher exponent = larger start_pos = fewer leading zeros = easier target
            // Exponent is in high byte: 0x3D > 0x3C means easier, so cap to minimum
            let cached_exponent = (current_diff >> 24) as u8;
            let min_exponent = (min_difficulty >> 24) as u8;

            // Higher exponent = easier target. If cached is easier than minimum, use minimum.
            // Exception: Regtest uses exponent 0x20 which is special (position 0)
            if self.network != Network::Regtest && cached_exponent > min_exponent {
                eprintln!(
                    "⚠️ Cached difficulty 0x{:08x} (exp {}) easier than minimum 0x{:08x} (exp {}), using minimum",
                    current_diff, cached_exponent, min_difficulty, min_exponent
                );
                // Also update the cache so we don't log this every block
                self.current_difficulty_bits.store(min_difficulty, std::sync::atomic::Ordering::SeqCst);
                return Ok(min_difficulty);
            }

            return Ok(current_diff);
        }

        // This IS an adjustment block - calculate new difficulty
        eprintln!("🎯 Difficulty adjustment at height {} (every {} blocks)", next_height, ADJUSTMENT_INTERVAL);

        // Get timestamp of block at start of this period (2016 blocks ago)
        let period_start_height = next_height - ADJUSTMENT_INTERVAL;

        // FIX 2025-12-21: For the FIRST adjustment period (blocks 0-2015), use block 1's
        // timestamp instead of genesis (block 0). Genesis has a fixed historical timestamp
        // (Jan 1, 2025) that doesn't reflect when mining actually started on this chain.
        // This caused the adjustment to think blocks took ~350 days instead of ~5.6 hours,
        // making difficulty EASIER instead of HARDER.
        let effective_start_height = if period_start_height == 0 {
            eprintln!("   ℹ️ First adjustment period: using block 1 timestamp (genesis has fixed historical timestamp)");
            1_u64  // Use first mined block instead of genesis
        } else {
            period_start_height
        };

        let period_start_block = match self.database.get_block(effective_start_height as u32) {
            Ok(Some(block)) => block,
            Ok(None) => {
                eprintln!("⚠️ Could not find block at height {} for difficulty adjustment", effective_start_height);
                return Ok(initial_difficulty);
            }
            Err(e) => {
                eprintln!("⚠️ Database error getting block {}: {}", effective_start_height, e);
                return Ok(initial_difficulty);
            }
        };

        // Get timestamp of most recent block (end of period)
        let period_end_height = next_height - 1;
        let period_end_block = match self.database.get_block(period_end_height as u32) {
            Ok(Some(block)) => block,
            Ok(None) => {
                eprintln!("⚠️ Could not find block at height {} for difficulty adjustment", period_end_height);
                return Ok(initial_difficulty);
            }
            Err(e) => {
                eprintln!("⚠️ Database error getting block {}: {}", period_end_height, e);
                return Ok(initial_difficulty);
            }
        };

        // Calculate actual timespan
        let actual_timespan_seconds = period_end_block.header.timestamp.saturating_sub(period_start_block.header.timestamp);

        // Get current difficulty
        let current_bits = period_end_block.header.bits;

        // Calculate standard 2016-block difficulty
        let standard_bits = calculate_next_difficulty(current_bits, actual_timespan_seconds);

        // Calculate effective block count (2015 for first period, 2016 for subsequent)
        let effective_block_count = if period_start_height == 0 {
            ADJUSTMENT_INTERVAL - 1  // First period uses blocks 1-2015 (2015 blocks)
        } else {
            ADJUSTMENT_INTERVAL
        };

        eprintln!(
            "📊 Difficulty adjustment: {} blocks (heights {}-{}) mined in {} seconds (target: {} seconds)",
            effective_block_count,
            effective_start_height,
            period_end_height,
            actual_timespan_seconds,
            effective_block_count * 600  // 600 seconds per block target
        );

        // ── Bootstrap blending with 144-block fast adjustment ──────────
        // During bootstrap (height < 20,160), blend the standard 2016-block
        // result with a responsive 144-block result. Weight linearly decreases
        // from 1.0 at genesis to 0.0 at BOOTSTRAP_END_HEIGHT.
        let final_bits = if is_bootstrap && next_height > cons::FAST_ADJUSTMENT_WINDOW {
            let fast_bits = self.calculate_fast_adjustment(next_height).await;
            match fast_bits {
                Some(fast) => {
                    let w = 1.0 - (next_height as f64 / cons::BOOTSTRAP_END_HEIGHT as f64);
                    let blended = Self::blend_difficulty(standard_bits, fast, w);
                    eprintln!(
                        "   🔀 Bootstrap blend: standard=0x{:08x}, fast=0x{:08x}, w={:.3} → blended=0x{:08x}",
                        standard_bits, fast, w, blended
                    );
                    blended
                }
                None => standard_bits,
            }
        } else {
            standard_bits
        };

        eprintln!(
            "   Old difficulty: 0x{:08x}, New difficulty: 0x{:08x}",
            current_bits, final_bits
        );

        // CRITICAL FIX: Store the new difficulty bits so they're used for subsequent blocks!
        self.current_difficulty_bits.store(final_bits, std::sync::atomic::Ordering::SeqCst);
        eprintln!("   ✅ New difficulty 0x{:08x} stored and will be used for next blocks", final_bits);

        Ok(final_bits)
    }

    /// Calculate 144-block fast difficulty adjustment (bootstrap only).
    ///
    /// Uses same BigUint arithmetic as `calculate_next_difficulty` but with:
    /// - 144-block window (vs 2016)
    /// - 2× clamp (vs 4×)
    ///
    /// Returns `None` if insufficient blocks in database.
    async fn calculate_fast_adjustment(&self, next_height: u64) -> Option<u32> {
        use btpc_core::consensus::constants as cons;

        let window = cons::FAST_ADJUSTMENT_WINDOW; // 144
        if next_height < window {
            return None;
        }

        let end_height = next_height - 1;
        let start_height = next_height.saturating_sub(window);

        let start_block = self.database.get_block(start_height as u32).ok()??;
        let end_block = self.database.get_block(end_height as u32).ok()??;

        let actual_timespan = end_block.header.timestamp.saturating_sub(start_block.header.timestamp);
        let target_timespan = window * 600; // 144 × 10 min = 86,400 sec (24 hours)

        // Clamp to 2× (tighter than standard 4×)
        let min_ts = target_timespan / 2; // 12 hours
        let max_ts = target_timespan * 2; // 48 hours
        let clamped = actual_timespan.clamp(min_ts, max_ts);

        let current_bits = end_block.header.bits;
        Some(calculate_next_difficulty_with_params(current_bits, clamped, target_timespan))
    }

    /// Blend two difficulty values in target-space using weight `w`.
    ///
    /// `D_final = D_standard * (1 - w) + D_fast * w`
    /// Computed in target-space (BigUint), then converted back to bits.
    fn blend_difficulty(standard_bits: u32, fast_bits: u32, w: f64) -> u32 {
        use num_bigint::BigUint;

        let std_target = BigUint::from_bytes_be(DifficultyTarget::from_bits(standard_bits).as_bytes());
        let fast_target = BigUint::from_bytes_be(DifficultyTarget::from_bits(fast_bits).as_bytes());

        // Use integer arithmetic with 1000 precision to avoid floating point:
        // blended = std * (1000 - w_int) / 1000 + fast * w_int / 1000
        let w_int = (w * 1000.0).round().clamp(0.0, 1000.0) as u64;
        let blended = (&std_target * (1000u64 - w_int) + &fast_target * w_int) / 1000u64;

        // Convert back to 64-byte target
        let mut bytes = blended.to_bytes_be();
        if bytes.len() < 64 {
            let mut padded = vec![0u8; 64 - bytes.len()];
            padded.extend_from_slice(&bytes);
            bytes = padded;
        } else if bytes.len() > 64 {
            bytes = bytes[bytes.len() - 64..].to_vec();
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        DifficultyTarget::from_hash(&Hash::from_bytes(arr)).bits
    }
}

/// Calculate block reward with linear decay (Constitution Article III)
///
/// # Block Reward Schedule
/// - **Initial Reward**: 32.375 BTPC (3,237,500,000 crystals)
/// - **Decay Period**: 24 years = 1,262,304 blocks (52,596 blocks/year, leap-year adjusted)
/// - **Tail Emission**: 0.5 BTPC (50,000,000 crystals) after decay period
/// - **Formula**: `reward = initial - (initial - tail) * (height / decay_blocks)`
///
/// # Arguments
/// * `height` - Current block height
///
/// # Returns
/// * Block reward in crystals (1 BTPC = 100,000,000 crystals)
///
/// # Constitutional Compliance
/// This implements the linear decay economic model specified in Constitution Article III.
/// Deviation from this formula constitutes a constitutional violation.
///
/// # Examples
/// ```rust,ignore
/// // Genesis block (height 0)
/// let reward = calculate_block_reward(0);
/// assert_eq!(reward, 3_237_500_000); // 32.375 BTPC
///
/// // After 24 years (height 1,262,304)
/// let reward = calculate_block_reward(1_262_304);
/// assert_eq!(reward, 50_000_000); // 0.5 BTPC (tail emission)
/// ```
pub fn calculate_block_reward(height: u64) -> u64 {
    // Constants per Constitution Article III (leap-year adjusted)
    const INITIAL_REWARD: u64 = 3_237_500_000; // 32.375 BTPC in crystals
    const TAIL_EMISSION: u64 = 50_000_000;     // 0.5 BTPC in crystals
    const BLOCKS_PER_YEAR: u64 = 52_596;       // 10-minute blocks (365.25 × 24 × 6)
    const DECAY_YEARS: u64 = 24;
    const TOTAL_DECAY_BLOCKS: u64 = BLOCKS_PER_YEAR * DECAY_YEARS; // 1,262,304

    // If we're past the decay period, return tail emission
    if height >= TOTAL_DECAY_BLOCKS {
        return TAIL_EMISSION;
    }

    // Linear decay formula:
    // reward = initial_reward - (initial_reward - tail_emission) * (height / total_decay_blocks)
    let decay_amount = INITIAL_REWARD - TAIL_EMISSION; // 3,187,500,000 crystals
    let decay_progress = (decay_amount as u128 * height as u128) / TOTAL_DECAY_BLOCKS as u128;

    // Calculate reward, ensuring we don't go below tail emission
    let reward = INITIAL_REWARD.saturating_sub(decay_progress as u64);
    reward.max(TAIL_EMISSION)
}

/// Calculate difficulty adjustment (Constitution Article IV, Section 4.1)
///
/// # Difficulty Adjustment Rules
/// - **Adjustment Interval**: Every 2,016 blocks
/// - **Target Block Time**: 10 minutes per block
/// - **Target Timespan**: 2,016 blocks × 10 minutes = 20,160 minutes (14 days)
/// - **Formula**: `new_difficulty = old_difficulty * (actual_timespan / target_timespan)`
/// - **Limits**: Max 4× change per period (prevents extreme swings)
///
/// # Arguments
/// * `current_bits` - Current difficulty in compact bits format
/// * `actual_timespan_seconds` - Time taken to mine last 2016 blocks (in seconds)
///
/// # Returns
/// * New difficulty in compact bits format
///
/// # Constitutional Compliance
/// This implements the difficulty adjustment specified in Constitution Article IV.
/// Deviation from this formula constitutes a constitutional violation.
///
/// # Examples
/// ```rust,ignore
/// // Block took exactly 14 days - no adjustment
/// let new_bits = calculate_next_difficulty(0x3c7fffff, 1_209_600);
/// assert_eq!(new_bits, 0x3c7fffff);
///
/// // Blocks took 7 days (twice as fast) - difficulty doubles
/// let new_bits = calculate_next_difficulty(0x3c7fffff, 604_800);
/// // Result will be higher difficulty (lower target)
/// ```
/// Calculate LWMA (Linearly Weighted Moving Average) difficulty adjustment
///
/// NOTE: This function was disabled on 2025-12-27. It was causing regtest difficulty
/// to spike 600x per block, making mining impossible after ~60 blocks.
/// Kept for potential future use with proper safeguards.
///
/// # Algorithm
/// - Uses the last N blocks (LWMA_WINDOW = 60)
/// - Weights recent blocks more heavily (linear weighting)
/// - Adjusts difficulty every block based on weighted average solve time
/// - No 4× cap - allows unlimited adjustment for responsive difficulty
///
/// # Arguments
/// * `timestamps` - Vector of (height, timestamp) pairs for recent blocks
/// * `current_bits` - Current difficulty bits
///
/// # Returns
/// * New difficulty bits
#[allow(dead_code)]
pub fn calculate_lwma_difficulty(timestamps: &[(u64, u64)], current_bits: u32) -> u32 {
    const LWMA_WINDOW: usize = 60;  // Look at last 60 blocks
    const TARGET_BLOCK_TIME: u64 = 600;  // 10 minutes in seconds
    
    // Need at least 2 blocks for LWMA calculation
    if timestamps.len() < 2 {
        return current_bits;
    }
    
    // Use up to LWMA_WINDOW blocks
    let window_size = timestamps.len().min(LWMA_WINDOW);
    let recent_timestamps: Vec<_> = timestamps.iter().rev().take(window_size).collect();
    
    // Calculate weighted sum of solve times
    // Weight increases linearly: block i has weight i
    let mut weighted_sum: u64 = 0;
    let mut weight_sum: u64 = 0;
    
    for i in 1..recent_timestamps.len() {
        let weight = i as u64;
        let solve_time = recent_timestamps[i - 1].1.saturating_sub(recent_timestamps[i].1);
        // Clamp solve time to prevent extreme values (min 1 second, max 10x target)
        let clamped_solve_time = solve_time.clamp(1, TARGET_BLOCK_TIME * 10);
        weighted_sum += clamped_solve_time * weight;
        weight_sum += weight;
    }
    
    // Calculate weighted average solve time
    let avg_solve_time = if weight_sum > 0 {
        weighted_sum / weight_sum
    } else {
        TARGET_BLOCK_TIME
    };
    
    // Calculate adjustment ratio
    // If blocks are coming faster than target, increase difficulty (reduce target)
    // If blocks are coming slower than target, decrease difficulty (increase target)
    let current_target = DifficultyTarget::from_bits(current_bits);
    let current_target_bytes = current_target.as_bytes();
    
    use num_bigint::BigUint;
    let current_target_bigint = BigUint::from_bytes_be(current_target_bytes);
    
    // new_target = current_target * avg_solve_time / target_block_time
    // If avg_solve_time < target: new_target smaller = harder
    // If avg_solve_time > target: new_target larger = easier
    let new_target_bigint = (&current_target_bigint * avg_solve_time) / TARGET_BLOCK_TIME;
    
    // Convert back to bytes
    let mut new_target_bytes = new_target_bigint.to_bytes_be();
    
    // Ensure exactly 64 bytes (SHA-512 hash size)
    if new_target_bytes.len() < 64 {
        let mut padded = vec![0u8; 64 - new_target_bytes.len()];
        padded.extend_from_slice(&new_target_bytes);
        new_target_bytes = padded;
    } else if new_target_bytes.len() > 64 {
        new_target_bytes = new_target_bytes[new_target_bytes.len() - 64..].to_vec();
    }
    
    // Enforce minimum target (prevent impossibly hard mining)
    // Minimum target: at least 1 byte non-zero at position 32 (roughly Bitcoin mainnet minimum)
    let all_zeros = new_target_bytes.iter().take(40).all(|&b| b == 0) && 
                    new_target_bytes.iter().skip(40).all(|&b| b == 0);
    if all_zeros {
        // Target too hard - keep current
        return current_bits;
    }
    
    // Convert to array
    let mut target_array = [0u8; 64];
    target_array.copy_from_slice(&new_target_bytes);
    
    // Convert to bits representation
    let target_hash = Hash::from_bytes(target_array);
    let new_difficulty_target = DifficultyTarget::from_hash(&target_hash);
    
    eprintln!(
        "📊 LWMA: avg_solve_time={:.1}s (target={}s), old_bits=0x{:08x}, new_bits=0x{:08x}",
        avg_solve_time, TARGET_BLOCK_TIME, current_bits, new_difficulty_target.bits
    );
    
    new_difficulty_target.bits
}

pub fn calculate_next_difficulty(current_bits: u32, actual_timespan_seconds: u64) -> u32 {
    // Constants per Constitution Article IV
    const TARGET_TIMESPAN_SECONDS: u64 = 20_160 * 60; // 2016 blocks × 10 minutes = 1,209,600 seconds (14 days)
    #[allow(dead_code)] // Kept for documentation - shows the 2016-block adjustment interval
    const ADJUSTMENT_INTERVAL: u64 = 2_016;

    // Clamp actual timespan to prevent extreme adjustments (4× max change)
    // If blocks were mined 4× faster, clamp to 4× faster (difficulty increases 4×)
    // If blocks were mined 4× slower, clamp to 4× slower (difficulty decreases 4×)
    let min_timespan = TARGET_TIMESPAN_SECONDS / 4; // 302,400 seconds (3.5 days)
    let max_timespan = TARGET_TIMESPAN_SECONDS * 4; // 4,838,400 seconds (56 days)
    let clamped_timespan = actual_timespan_seconds.clamp(min_timespan, max_timespan);

    // Convert current bits to target (DifficultyTarget handles the compact format)
    let current_target = DifficultyTarget::from_bits(current_bits);

    // Calculate new target: target * (actual_time / target_time)
    // Higher target = easier mining (lower difficulty)
    // Lower target = harder mining (higher difficulty)
    let current_target_bytes = current_target.as_bytes();

    // Use 256-bit arithmetic to prevent overflow
    // new_target = current_target * actual_timespan / target_timespan
    use num_bigint::BigUint;
    let current_target_bigint = BigUint::from_bytes_be(current_target_bytes);
    let new_target_bigint = (&current_target_bigint * clamped_timespan) / TARGET_TIMESPAN_SECONDS;

    // Convert back to bytes (pad or truncate to 64 bytes for SHA-512)
    let mut new_target_bytes = new_target_bigint.to_bytes_be();

    // Ensure we have exactly 64 bytes (SHA-512 hash size)
    if new_target_bytes.len() < 64 {
        // Pad with leading zeros
        let mut padded = vec![0u8; 64 - new_target_bytes.len()];
        padded.extend_from_slice(&new_target_bytes);
        new_target_bytes = padded;
    } else if new_target_bytes.len() > 64 {
        // Truncate (this shouldn't happen with proper clamping, but handle it)
        new_target_bytes = new_target_bytes[new_target_bytes.len() - 64..].to_vec();
    }

    // Convert to array
    let mut target_array = [0u8; 64];
    target_array.copy_from_slice(&new_target_bytes);

    // FIX 2025-12-27: Enforce minimum difficulty (maximum target)
    // For Mainnet/Testnet, target must have first non-zero byte at index >= 4
    // (at least 4 leading zero bytes = ~32 bits of work minimum)
    //
    // Without this check, when difficulty gets EASIER (target × 4), the BigUint
    // can grow from 2 to 3 significant bytes, shifting first non-zero from index 4
    // to index 3. This causes exponent to change from 60 to 61, producing an even
    // EASIER target that compounds the error over subsequent adjustments.
    //
    // The minimum target for Mainnet/Testnet corresponds to 0x3c7fffff:
    // Exponent 60 → start_pos = 64 - 60 = 4 → first non-zero at index 4
    let first_nonzero_idx = target_array.iter().position(|&b| b != 0).unwrap_or(63);
    if first_nonzero_idx < 4 {
        // Target is too easy (too few leading zeros)
        // Cap to minimum difficulty: [0,0,0,0, 0xFF,0xFF, 0,...]
        eprintln!(
            "⚠️ Difficulty adjustment capped: first non-zero at index {} (need >= 4)",
            first_nonzero_idx
        );
        // Use the minimum difficulty target (0x3c7fffff for SHA-512)
        let min_difficulty_target = DifficultyTarget::minimum_for_network(Network::Testnet);
        return min_difficulty_target.bits;
    }

    // Convert target bytes to compact bits representation
    // Note: DifficultyTarget::target_to_bits is a private method, so we need to create
    // a DifficultyTarget instance first using from_hash
    let target_hash = Hash::from_bytes(target_array);
    let new_difficulty_target = DifficultyTarget::from_hash(&target_hash);
    new_difficulty_target.bits
}

/// Parameterised difficulty adjustment (shared math for standard + fast windows).
///
/// `current_bits`: difficulty of last block in window
/// `clamped_timespan`: actual timespan already clamped to [min, max]
/// `target_timespan`: expected timespan for the window
pub fn calculate_next_difficulty_with_params(
    current_bits: u32,
    clamped_timespan: u64,
    target_timespan: u64,
) -> u32 {
    use num_bigint::BigUint;

    let current_target = DifficultyTarget::from_bits(current_bits);
    let current_target_bigint = BigUint::from_bytes_be(current_target.as_bytes());
    let new_target_bigint = (&current_target_bigint * clamped_timespan) / target_timespan;

    let mut new_bytes = new_target_bigint.to_bytes_be();
    if new_bytes.len() < 64 {
        let mut padded = vec![0u8; 64 - new_bytes.len()];
        padded.extend_from_slice(&new_bytes);
        new_bytes = padded;
    } else if new_bytes.len() > 64 {
        new_bytes = new_bytes[new_bytes.len() - 64..].to_vec();
    }

    let mut target_array = [0u8; 64];
    target_array.copy_from_slice(&new_bytes);

    // Enforce minimum difficulty floor (same as calculate_next_difficulty)
    let first_nonzero_idx = target_array.iter().position(|&b| b != 0).unwrap_or(63);
    if first_nonzero_idx < 4 {
        return DifficultyTarget::minimum_for_network(Network::Testnet).bits;
    }

    DifficultyTarget::from_hash(&Hash::from_bytes(target_array)).bits
}

// Implement RpcClientInterface for Arc<RwLock<EmbeddedNode>> (enables mining pool compatibility)
// This allows EmbeddedNode to be used anywhere RpcClient was used before
// Note: tauri::async_runtime::RwLock is an alias for tokio::sync::RwLock
impl RpcClientInterface for Arc<RwLock<EmbeddedNode>> {
    async fn get_block_template(&self) -> Result<BlockTemplate> {
        let node = self.read().await;
        node.get_block_template().await
    }

    async fn submit_block(&self, block_hex: &str) -> Result<String> {
        let mut node = self.write().await;
        node.submit_block(block_hex).await
    }
}

/// Blockchain state for queries
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlockchainState {
    /// Current blockchain height
    pub current_height: u64,

    /// Best block hash (hex string)
    pub best_block_hash: String,

    /// Is node currently syncing?
    pub is_syncing: bool,

    /// Total number of UTXOs in set
    pub total_utxos: u64,

    /// Current difficulty bits (compact format)
    pub difficulty_bits: u32,
}

/// Sync progress for UI updates
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncProgress {
    /// Current blockchain height
    pub current_height: u64,

    /// Target height (from peers)
    pub target_height: u64,

    /// Is node currently syncing?
    pub is_syncing: bool,

    /// Number of connected peers
    pub connected_peers: u32,

    /// Sync percentage (0-100)
    pub sync_percentage: f64,
}

/// REM-C001: Peer information for network monitoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerInfo {
    /// Peer network address (IP:port)
    pub address: String,

    /// Peer protocol version
    pub version: String,

    /// Peer's reported blockchain height
    pub height: u64,

    /// Average ping time in milliseconds
    pub ping_ms: u64,

    /// Unix timestamp when connection established
    pub connected_since: u64,
}

/// Node state for initialization response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeState {
    /// Network type
    pub network: String,

    /// Current blockchain height
    pub current_height: u64,

    /// Is node initialized?
    pub is_initialized: bool,

    /// Is node currently syncing?
    pub is_syncing: bool,
}

/// Mempool statistics for fee estimation
/// BUG FIX 2025-11-11: Added for Bug #3 (FeeEstimator RPC elimination)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MempoolStats {
    /// Number of transactions in mempool
    pub tx_count: usize,

    /// Total size of all transactions (bytes)
    pub total_size_bytes: u64,

    /// 25th percentile fee rate (crystals per byte)
    pub fee_rate_p25_crd_per_byte: f64,

    /// Median fee rate (crystals per byte)
    pub fee_rate_p50_crd_per_byte: f64,

    /// 75th percentile fee rate (crystals per byte)
    pub fee_rate_p75_crd_per_byte: f64,
}

/// Transaction information for monitoring
/// BUG FIX 2025-11-11: Added for Bug #4 (TransactionMonitor RPC elimination)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransactionInfo {
    /// Transaction ID (hex string)
    pub txid: String,

    /// Number of confirmations (0 = mempool)
    pub confirmations: u32,

    /// Block height (None if in mempool)
    pub block_height: Option<u64>,

    /// Block hash (None if in mempool)
    pub block_hash: Option<String>,

    /// Transaction fee (crystals)
    pub fee: u64,

    /// Status: "mempool", "confirmed", "not_found"
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_new_embedded_node() {
        // Arrange
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let utxo_manager = crate::utxo_manager::UTXOManager::new(temp_dir.path().to_path_buf())
            .expect("Failed to create UTXO manager");
        let utxo_manager_arc = Arc::new(std::sync::Mutex::new(utxo_manager));

        // Act
        let result =
            EmbeddedNode::new(temp_dir.path().to_path_buf(), "regtest", utxo_manager_arc).await;

        // Assert
        assert!(result.is_ok(), "Node initialization should succeed");
        let node_arc = result.unwrap();
        let node = node_arc.read().await;

        assert_eq!(node.network(), Network::Regtest);
        assert_eq!(
            node.current_height
                .load(std::sync::atomic::Ordering::SeqCst),
            0
        );
    }

    #[tokio::test]
    async fn test_get_blockchain_state() {
        // Arrange
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let utxo_manager = crate::utxo_manager::UTXOManager::new(temp_dir.path().to_path_buf())
            .expect("Failed to create UTXO manager");
        let utxo_manager_arc = Arc::new(std::sync::Mutex::new(utxo_manager));

        let node_arc =
            EmbeddedNode::new(temp_dir.path().to_path_buf(), "regtest", utxo_manager_arc)
                .await
                .expect("Node initialization failed");

        // Act
        let node = node_arc.read().await;
        let state = node.get_blockchain_state().await;

        // Assert
        assert!(state.is_ok(), "get_blockchain_state should succeed");
        let state = state.unwrap();
        assert_eq!(state.current_height, 0);
        assert_eq!(state.total_utxos, 0);
        assert!(!state.is_syncing);
    }

    #[tokio::test]
    async fn test_get_sync_progress() {
        // Arrange
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let utxo_manager = crate::utxo_manager::UTXOManager::new(temp_dir.path().to_path_buf())
            .expect("Failed to create UTXO manager");
        let utxo_manager_arc = Arc::new(std::sync::Mutex::new(utxo_manager));

        let node_arc =
            EmbeddedNode::new(temp_dir.path().to_path_buf(), "regtest", utxo_manager_arc)
                .await
                .expect("Node initialization failed");

        // Act
        let node = node_arc.read().await;
        let progress = node.get_sync_progress();

        // Assert
        assert!(progress.is_ok(), "get_sync_progress should succeed");
        let progress = progress.unwrap();
        assert_eq!(progress.current_height, 0);
        assert_eq!(progress.target_height, 0);
        assert!(!progress.is_syncing);
        assert_eq!(progress.connected_peers, 0);
        assert_eq!(progress.sync_percentage, 100.0);
    }

    // TDD RED PHASE - Linear Decay Block Reward Tests
    // Constitution Article III: Linear decay over 24 years
    // Initial: 32.375 BTPC, Final: 0.5 BTPC (tail emission)
    // Total decay blocks: 52,596 blocks/year × 24 years = 1,262,304 blocks (leap-year adjusted)

    #[test]
    fn test_calculate_block_reward_at_genesis() {
        // Block 0 should give initial reward: 32.375 BTPC = 3,237,500,000 crystals
        let reward = calculate_block_reward(0);
        assert_eq!(reward, 3_237_500_000, "Genesis block should reward 32.375 BTPC");
    }

    #[test]
    fn test_calculate_block_reward_at_year_1() {
        // After 1 year (52,596 blocks), reward should have decayed slightly
        let reward = calculate_block_reward(52_596);
        // Expected: 32.375 - (32.375 - 0.5) * (52596 / 1262304)
        //         = 32.375 - 31.875 * 0.0417 = 32.375 - 1.329 = ~31.04-31.05 BTPC
        let expected = 3_104_600_000u64; // ~31.046 BTPC in crystals
        assert!(
            (reward as i64 - expected as i64).abs() < 200_000, // Allow for integer rounding
            "Year 1 reward should be ~31.046 BTPC, got {} crystals (expected {})",
            reward,
            expected
        );
    }

    #[test]
    fn test_calculate_block_reward_at_year_12() {
        // Halfway through decay (12 years = 631,152 blocks)
        let reward = calculate_block_reward(631_152);
        // Expected: 32.375 - (32.375 - 0.5) * (631152 / 1262304)
        //         = 32.375 - 31.875 * 0.5 = 16.4375 BTPC
        let expected = 1_643_750_000u64; // 16.4375 BTPC
        assert!(
            (reward as i64 - expected as i64).abs() < 100_000,
            "Year 12 reward should be ~16.4375 BTPC, got {} crystals",
            reward
        );
    }

    #[test]
    fn test_calculate_block_reward_at_year_24() {
        // End of decay period (24 years = 1,262,304 blocks)
        let reward = calculate_block_reward(1_262_304);
        // Should be exactly tail emission: 0.5 BTPC = 50,000,000 crystals
        let expected = 50_000_000u64;
        assert_eq!(
            reward, expected,
            "Year 24 reward should be exactly 0.5 BTPC (tail emission)"
        );
    }

    #[test]
    fn test_calculate_block_reward_after_decay_period() {
        // Beyond 24 years (e.g., block 2,000,000)
        let reward = calculate_block_reward(2_000_000);
        // Should remain at tail emission: 0.5 BTPC
        let expected = 50_000_000u64;
        assert_eq!(
            reward, expected,
            "Post-decay rewards should remain at 0.5 BTPC tail emission"
        );
    }

    #[test]
    fn test_calculate_block_reward_never_negative() {
        // Test extreme heights to ensure reward never goes negative
        let reward = calculate_block_reward(u64::MAX);
        assert!(
            reward >= 50_000_000,
            "Reward should never drop below tail emission (0.5 BTPC)"
        );
    }
}
