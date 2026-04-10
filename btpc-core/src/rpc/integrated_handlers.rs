//! Integrated RPC handlers connecting blockchain, consensus, and network components
//!
//! This module provides RPC handlers that have full access to blockchain state,
//! consensus validation, and network synchronization.

#![allow(dead_code)]
#![allow(unused_variables)]

use std::sync::{Arc, RwLock};

use anyhow::Result;
use serde_json::{json, Value};
use tokio::sync::RwLock as TokioRwLock;

use crate::{
    blockchain::{Block, BlockHeader, Transaction, TransactionError},
    consensus::{
        ConsensusEngine, ConsensusParams, DifficultyTarget, RewardCalculator, StorageBlockValidator,
        StorageTransactionValidator, StorageValidationError,
    },
    crypto::{Address, Hash},
    mempool::{Mempool, MempoolError},
    network::{IntegratedSyncManager, SyncStats},
    rpc::{server::RpcServer, types::*, RpcServerError},
    storage::{BlockchainDatabase, StorageError, UTXODatabase},
    Network,
};

/// Integrated RPC handlers with full system access
pub struct IntegratedRpcHandlers {
    /// Blockchain database (sync lock for consensus compatibility)
    blockchain_db: Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
    /// UTXO database (sync lock for consensus compatibility)
    utxo_db: Arc<RwLock<dyn UTXODatabase + Send + Sync>>,
    /// Storage-aware block validator
    block_validator: Arc<StorageBlockValidator>,
    /// Storage-aware transaction validator
    tx_validator: Arc<StorageTransactionValidator>,
    /// Consensus engine (async lock for RPC operations)
    consensus_engine: Arc<TokioRwLock<ConsensusEngine>>,
    /// Integrated sync manager
    sync_manager: Arc<IntegratedSyncManager>,
    /// Memory pool for unconfirmed transactions
    mempool: Arc<RwLock<Mempool>>,
    /// Network type
    network: Network,
}

impl IntegratedRpcHandlers {
    /// Create new integrated RPC handlers
    pub fn new(
        blockchain_db: Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
        utxo_db: Arc<RwLock<dyn UTXODatabase + Send + Sync>>,
        sync_manager: Arc<IntegratedSyncManager>,
        mempool: Arc<RwLock<Mempool>>,
        network: Network,
    ) -> Self {
        // Create validators with RwLock-wrapped databases (Priority 2 fix)
        let block_validator = Arc::new(StorageBlockValidator::new(
            blockchain_db.clone(),
            utxo_db.clone(),
        ));
        let tx_validator = Arc::new(StorageTransactionValidator::new(utxo_db.clone()));
        let consensus_engine = Arc::new(TokioRwLock::new(ConsensusEngine::for_network(network)));

        IntegratedRpcHandlers {
            blockchain_db,
            utxo_db,
            block_validator,
            tx_validator,
            consensus_engine,
            sync_manager,
            mempool,
            network,
        }
    }

    /// Register all integrated RPC methods
    pub async fn register_methods(&self, server: &RpcServer) {
        // Blockchain information methods
        self.register_blockchain_methods(server).await;

        // Transaction methods
        self.register_transaction_methods(server).await;

        // Mining methods
        self.register_mining_methods(server).await;

        // Network methods
        self.register_network_methods(server).await;

        // Consensus methods
        self.register_consensus_methods(server).await;

        // Validation methods
        self.register_validation_methods(server).await;
    }

    /// Register blockchain information methods
    async fn register_blockchain_methods(&self, server: &RpcServer) {
        let blockchain_db = Arc::clone(&self.blockchain_db);
        let network = self.network;
        server.register_method("getblockchaininfo", move |_| {
            // Query the blockchain database for actual values
            let blockchain_guard = blockchain_db
                .try_read()
                .map_err(|e| RpcServerError::Internal(format!("Lock error: {}", e)))?;

            // Get chain tip
            let tip_block = blockchain_guard
                .get_chain_tip()
                .map_err(|e| RpcServerError::Internal(format!("Failed to get chain tip: {}", e)))?;

            // Convert network to chain name
            let chain_name = match network {
                Network::Mainnet => "main",
                Network::Testnet => "test",
                Network::Regtest => "regtest",
            };

            match tip_block {
                Some(block) => {
                    let block_hash = block.hash();
                    let height = blockchain_guard
                        .get_block_height(&block_hash)
                        .unwrap_or(0);

                    // Get difficulty from block header bits
                    let difficulty_bits = block.header.bits;
                    let difficulty = Self::bits_to_difficulty(difficulty_bits);

                    Ok(json!({
                        "chain": chain_name,
                        "blocks": height,
                        "headers": height,
                        "bestblockhash": hex::encode(block_hash.as_bytes()),
                        "difficulty": difficulty,
                        "difficulty_bits": difficulty_bits,
                        "verificationprogress": 1.0,
                        "chainwork": "0000000000000000000000000000000000000000000000000000000000000001"
                    }))
                }
                None => {
                    // No blocks yet - return genesis state
                    // FIX 2026-03-04: Bitcoin-style "difficulty 1"
                    use crate::consensus::constants as cons;
                    let default_bits = match network {
                        Network::Regtest => cons::REGTEST_DIFFICULTY_BITS,
                        _ => cons::INITIAL_DIFFICULTY_BITS, // SHA-512 "difficulty 1"
                    };
                    Ok(json!({
                        "chain": chain_name,
                        "blocks": 0,
                        "headers": 0,
                        "bestblockhash": "0000000000000000000000000000000000000000000000000000000000000000",
                        "difficulty": 1.0,
                        "difficulty_bits": default_bits,
                        "verificationprogress": 1.0,
                        "chainwork": "0000000000000000000000000000000000000000000000000000000000000001"
                    }))
                }
            }
        }).await;

        let blockchain_db = Arc::clone(&self.blockchain_db);
        server
            .register_method("getbestblockhash", move |_| {
                Self::get_best_block_hash(&blockchain_db)
            })
            .await;

        let blockchain_db = Arc::clone(&self.blockchain_db);
        let block_validator = Arc::clone(&self.block_validator);
        server
            .register_method("getblock", move |params| {
                let blockchain_db = Arc::clone(&blockchain_db);
                let block_validator = Arc::clone(&block_validator);
                tokio::runtime::Handle::current().block_on(
                    Self::get_block_with_validation(blockchain_db, block_validator, params)
                )
            })
            .await;

        let blockchain_db = Arc::clone(&self.blockchain_db);
        server
            .register_method("getblockheader", move |params| {
                Self::get_block_header(&blockchain_db, params)
            })
            .await;

        let blockchain_db = Arc::clone(&self.blockchain_db);
        server
            .register_method("getblockcount", move |_| {
                Self::get_block_count(&blockchain_db)
            })
            .await;
    }

    /// Register transaction methods
    async fn register_transaction_methods(&self, server: &RpcServer) {
        let blockchain_db = Arc::clone(&self.blockchain_db);
        let utxo_db = Arc::clone(&self.utxo_db);
        server
            .register_method("gettransaction", move |params| {
                Self::get_transaction(&blockchain_db, params)
            })
            .await;

        let blockchain_db = Arc::clone(&self.blockchain_db);
        server
            .register_method("getrecenttransactions", move |params| {
                Self::get_recent_transactions_handler(&blockchain_db, params)
            })
            .await;

        let utxo_db = Arc::clone(&self.utxo_db);
        server
            .register_method("gettxout", move |params| Self::get_tx_out(&utxo_db, params))
            .await;

        let utxo_db = Arc::clone(&self.utxo_db);
        server
            .register_method("getutxosforaddress", move |params| {
                Self::get_utxos_for_address(&utxo_db, params)
            })
            .await;

        let tx_validator = Arc::clone(&self.tx_validator);
        let blockchain_db = Arc::clone(&self.blockchain_db);
        let utxo_db = Arc::clone(&self.utxo_db);
        let mempool = Arc::clone(&self.mempool);
        let sync_manager = Arc::clone(&self.sync_manager);
        let network_for_tx = self.network;
        server
            .register_method("sendrawtransaction", move |params| {
                // Block on async function since register_method requires sync closure
                tokio::runtime::Handle::current().block_on(
                    Self::send_raw_transaction(
                        Arc::clone(&tx_validator),
                        Arc::clone(&blockchain_db),
                        Arc::clone(&utxo_db),
                        Arc::clone(&mempool),
                        Arc::clone(&sync_manager),
                        network_for_tx,
                        params
                    )
                )
            })
            .await;

        let tx_validator = Arc::clone(&self.tx_validator);
        server
            .register_method("validatetransaction", move |params| {
                let tx_validator = Arc::clone(&tx_validator);
                tokio::runtime::Handle::current().block_on(
                    Self::validate_transaction(tx_validator, params)
                )
            })
            .await;
    }

    /// Register mining methods
    async fn register_mining_methods(&self, server: &RpcServer) {
        let blockchain_db = Arc::clone(&self.blockchain_db);
        let mempool = Arc::clone(&self.mempool);
        let network = self.network;
        server.register_method("getblocktemplate", move |_params| {
            // Get chain tip info
            let db = blockchain_db.read().map_err(|e| {
                RpcServerError::Internal(format!("Lock poisoned: {}", e))
            })?;

            let tip = db.get_chain_tip().ok().flatten();
            let (prev_hash, height, tip_bits) = match &tip {
                Some(block) => {
                    let h = db.get_block_height(&block.hash()).unwrap_or(0);
                    (block.hash().to_hex(), h + 1, block.header.bits)
                }
                None => {
                    // Genesis case - FIX 2026-03-04: Bitcoin-style "difficulty 1"
                    use crate::consensus::constants as cons;
                    let initial_bits = match network {
                        Network::Regtest => cons::REGTEST_DIFFICULTY_BITS,
                        _ => cons::INITIAL_DIFFICULTY_BITS,
                    };
                    ("0".repeat(128), 0u32, initial_bits)
                }
            };

            // FIX 2026-03-04: For pre-adjustment heights (< 2016), use "difficulty 1"
            // constant instead of the tip block's bits. The tip may have been mined with
            // EDA-reduced difficulty which would poison all subsequent templates.
            let bits = if (height as u64) < 2016 {
                use crate::consensus::constants as cons;
                match network {
                    Network::Regtest => cons::REGTEST_DIFFICULTY_BITS,
                    _ => cons::INITIAL_DIFFICULTY_BITS,
                }
            } else {
                tip_bits
            };

            // Calculate target from bits - convert to hex string
            let target = DifficultyTarget::from_bits(bits);
            let target_bytes = target.as_bytes();
            let target_hex = hex::encode(target_bytes);

            // Get coinbase value (block reward + fees)
            let reward = RewardCalculator::calculate_reward(height).unwrap_or(0);

            // Get mempool transactions (up to 1000 by fee)
            let (mempool_txs, total_fees): (Vec<Value>, u64) = {
                let mp = mempool.read().map_err(|e| {
                    RpcServerError::Internal(format!("Mempool lock poisoned: {}", e))
                })?;
                let entries = mp.get_transactions_by_fee(1000);
                let fees: u64 = entries.iter().map(|e| e.fee).sum();
                let txs = entries
                    .iter()
                    .map(|entry| {
                        json!({
                            "data": hex::encode(entry.transaction.serialize()),
                            "txid": entry.transaction.hash().to_hex(),
                            "fee": entry.fee,
                            "sigops": entry.transaction.inputs.len() + entry.transaction.outputs.len()
                        })
                    })
                    .collect();
                (txs, fees)
            };

            Ok(json!({
                "version": 1,
                "previousblockhash": prev_hash,
                "height": height,
                "target": target_hex,
                "bits": format!("{:08x}", bits),
                "coinbasevalue": reward + total_fees,
                "transactions": mempool_txs,
                "mutable": ["time", "transactions", "prevblock"],
                "noncerange": "00000000ffffffff",
                "capabilities": ["proposal"],
                "curtime": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            }))
        }).await;

        let block_validator = Arc::clone(&self.block_validator);
        let consensus_engine = Arc::clone(&self.consensus_engine);
        server
            .register_method("submitblock", move |params| {
                let block_validator = Arc::clone(&block_validator);
                let consensus_engine = Arc::clone(&consensus_engine);
                // Use tokio::runtime::Handle to block on async function
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async move {
                        Self::submit_block(block_validator, consensus_engine, params).await
                    })
                })
            })
            .await;

        let consensus_engine = Arc::clone(&self.consensus_engine);
        let blockchain_db = Arc::clone(&self.blockchain_db);
        server
            .register_method("getmininginfo", move |_| {
                // Get mining info from chain tip (single lock acquisition)
                let (difficulty, height, networkhashps) = {
                    let db = blockchain_db.read().map_err(|e| {
                        RpcServerError::Internal(format!("Lock poisoned: {}", e))
                    })?;

                    let tip = db.get_chain_tip().ok().flatten();

                    let h = tip.as_ref()
                        .and_then(|t| db.get_block_height(&t.hash()).ok())
                        .unwrap_or(0);

                    // FIX 2026-02-23: For pre-2016 heights, use constant initial difficulty
                    // instead of tip.header.bits which may contain 20-minute-rule minimum
                    let diff = if (h as u64) < 2016 {
                        DifficultyTarget::initial_for_network(network).as_f64()
                    } else {
                        tip.as_ref()
                            .map(|t| DifficultyTarget::from_bits(t.header.bits).as_f64())
                            .unwrap_or(1.0)
                    };

                    // Estimate network hashrate using last 120 blocks
                    let hashrate = Self::estimate_network_hashrate(&*db, 120);

                    (diff, h, hashrate)
                };
                let reward = RewardCalculator::calculate_reward(height).unwrap_or(0);

                Ok(json!({
                    "blocks": height,
                    "difficulty": difficulty,
                    "networkhashps": networkhashps,
                    "blockprioritypercentage": 5,
                    "blockreward": reward,
                    "algorithm": "SHA-512",
                    "testnet": false
                }))
            })
            .await;
    }

    /// Register network methods
    async fn register_network_methods(&self, server: &RpcServer) {
        let sync_manager = Arc::clone(&self.sync_manager);
        server
            .register_method("getpeerinfo", move |_| {
                let sync_manager = Arc::clone(&sync_manager);
                let stats = tokio::runtime::Handle::current()
                    .block_on(sync_manager.get_sync_stats());
                Ok(json!({
                    "connected_peers": stats.connected_peers,
                    "syncing_peers": stats.syncing_peers,
                    "best_height": stats.best_height
                }))
            })
            .await;

        let sync_manager = Arc::clone(&self.sync_manager);
        server
            .register_method("getsyncinfo", move |_| {
                let sync_manager = Arc::clone(&sync_manager);
                let stats = tokio::runtime::Handle::current()
                    .block_on(sync_manager.get_sync_stats());
                Ok(json!({
                    "state": format!("{:?}", stats.state),
                    "progress": stats.progress,
                    "connected_peers": stats.connected_peers,
                    "syncing_peers": stats.syncing_peers,
                    "best_height": stats.best_height
                }))
            })
            .await;

        let sync_manager = Arc::clone(&self.sync_manager);
        let network = self.network;
        server
            .register_method("getnetworkinfo", move |_| {
                let sync_manager = Arc::clone(&sync_manager);
                let stats = tokio::runtime::Handle::current()
                    .block_on(sync_manager.get_sync_stats());
                Ok(json!({
                    "version": 1000000,
                    "subversion": "/BTPC:0.1.0/",
                    "protocolversion": 70015,
                    "localservices": "0000000000000001",
                    "localservicesnames": ["NETWORK"],
                    "localrelay": true,
                    "timeoffset": 0,
                    "connections": stats.connected_peers,
                    "networkactive": true,
                    "network": format!("{:?}", network)
                }))
            })
            .await;
    }

    /// Register consensus methods
    async fn register_consensus_methods(&self, server: &RpcServer) {
        let consensus_engine = Arc::clone(&self.consensus_engine);
        let blockchain_db = Arc::clone(&self.blockchain_db);
        let network = self.network;
        server
            .register_method("getconsensusinfo", move |_| {
                // Get current height from chain tip
                let height = {
                    let db = blockchain_db.read().map_err(|e| {
                        RpcServerError::Internal(format!("Lock poisoned: {}", e))
                    })?;
                    db.get_chain_tip()
                        .ok()
                        .flatten()
                        .and_then(|tip| db.get_block_height(&tip.hash()).ok())
                        .unwrap_or(0)
                };

                Ok(json!({
                    "network": format!("{:?}", network),
                    "current_height": height,
                    "algorithm": "SHA-512",
                    "signatures": "ML-DSA-87",
                    "reward_algorithm": "linear_decay",
                    "difficulty_adjustment": 2016,
                    "block_time": 600,
                    "max_supply": 21_000_000,
                    "tail_emission_start": crate::economics::BLOCKS_PER_YEAR * 24
                }))
            })
            .await;

        let consensus_engine = Arc::clone(&self.consensus_engine);
        let blockchain_db = Arc::clone(&self.blockchain_db);
        server
            .register_method("getdifficultyinfo", move |_| {
                // Get current difficulty and height from chain tip
                let (difficulty, height) = {
                    let db = blockchain_db.read().map_err(|e| {
                        RpcServerError::Internal(format!("Lock poisoned: {}", e))
                    })?;
                    let tip = db.get_chain_tip().ok().flatten();
                    let diff = tip.as_ref()
                        .map(|t| DifficultyTarget::from_bits(t.header.bits).as_f64())
                        .unwrap_or(1.0);
                    let h = tip.as_ref()
                        .and_then(|t| db.get_block_height(&t.hash()).ok())
                        .unwrap_or(0);
                    (diff, h)
                };

                let next_adjustment = 2016 - (height % 2016);

                Ok(json!({
                    "current_difficulty": difficulty,
                    "next_adjustment": next_adjustment,
                    "target_block_time": 600,
                    "algorithm": "SHA-512"
                }))
            })
            .await;

        let blockchain_db = Arc::clone(&self.blockchain_db);
        server
            .register_method("getrewardinfo", move |params| {
                // Get height from params or use chain tip
                let height = if let Some(Value::Array(arr)) = params {
                    arr.first()
                        .and_then(|v| v.as_u64())
                        .map(|h| h as u32)
                } else {
                    None
                };

                let height = match height {
                    Some(h) => h,
                    None => {
                        // Get current height from chain tip
                        let db = blockchain_db.read().map_err(|e| {
                            RpcServerError::Internal(format!("Lock poisoned: {}", e))
                        })?;
                        db.get_chain_tip()
                            .ok()
                            .flatten()
                            .and_then(|tip| db.get_block_height(&tip.hash()).ok())
                            .unwrap_or(0)
                    }
                };

                let reward = RewardCalculator::calculate_reward(height).unwrap_or(0);
                let reward_btpc = reward as f64 / 100_000_000.0;
                let initial_reward = RewardCalculator::calculate_reward(0).unwrap_or(0);
                let tail_emission = RewardCalculator::calculate_reward(
                    crate::economics::BLOCKS_PER_YEAR * 24
                ).unwrap_or(0);

                Ok(json!({
                    "height": height,
                    "reward": reward,
                    "reward_btpc": reward_btpc,
                    "algorithm": "linear_decay",
                    "initial_reward": initial_reward,
                    "initial_reward_btpc": initial_reward as f64 / 100_000_000.0,
                    "tail_emission": tail_emission,
                    "tail_emission_btpc": tail_emission as f64 / 100_000_000.0,
                    "decay_years": 24,
                    "tail_emission_start_height": crate::economics::BLOCKS_PER_YEAR * 24
                }))
            })
            .await;
    }

    /// Register validation methods
    async fn register_validation_methods(&self, server: &RpcServer) {
        let block_validator = Arc::clone(&self.block_validator);
        let blockchain_db = Arc::clone(&self.blockchain_db);
        server
            .register_method("validateblock", move |params| {
                let block_validator = Arc::clone(&block_validator);
                let blockchain_db = Arc::clone(&blockchain_db);

                // Parse block hash from params
                let params = params.ok_or(RpcServerError::InvalidParams(
                    "Missing block hash".to_string(),
                ))?;

                let block_hash = if let Value::Array(ref arr) = params {
                    arr.first()
                        .and_then(|v| v.as_str())
                        .ok_or(RpcServerError::InvalidParams(
                            "Invalid block hash".to_string(),
                        ))?
                } else {
                    return Err(RpcServerError::InvalidParams(
                        "Expected array parameters".to_string(),
                    ));
                };

                let hash = Hash::from_hex(block_hash)
                    .map_err(|_| RpcServerError::InvalidParams("Invalid hash format".to_string()))?;

                // Get block from database
                let block = {
                    let db = blockchain_db.read().map_err(|e| {
                        RpcServerError::Internal(format!("Lock poisoned: {}", e))
                    })?;
                    db.get_block(&hash)
                        .map_err(|e| RpcServerError::Internal(e.to_string()))?
                        .ok_or(RpcServerError::InvalidParams("Block not found".to_string()))?
                };

                // Validate block
                let validation_result = tokio::runtime::Handle::current()
                    .block_on(block_validator.validate_block_with_context(&block));

                let (valid, error) = match validation_result {
                    Ok(_) => (true, None),
                    Err(e) => (false, Some(format!("{}", e))),
                };

                Ok(json!({
                    "valid": valid,
                    "hash": block_hash,
                    "error": error
                }))
            })
            .await;

        server.register_method("help", Self::help).await;

        server.register_method("uptime", |_| Self::uptime()).await;
    }

    /// Get enhanced blockchain information with consensus data
    async fn get_blockchain_info_integrated(
        blockchain_db: Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
        consensus_engine: Arc<TokioRwLock<ConsensusEngine>>,
    ) -> Result<Value, RpcServerError> {
        let consensus = consensus_engine.read().await;

        // Get chain tip and collect last 11 block timestamps for median time
        let (best_hash, height, difficulty, median_time, size_on_disk) = {
            let db = blockchain_db
                .read()
                .map_err(|e| RpcServerError::Internal(format!("Lock poisoned: {}", e)))?;

            let chain_tip = db.get_chain_tip()
                .map_err(|e| RpcServerError::Internal(e.to_string()))?;

            if let Some(tip) = chain_tip {
                let hash = tip.hash().to_hex();
                let height = consensus.current_height();
                // FIX 2026-02-23: For pre-2016 heights, use constant initial difficulty
                // instead of tip.header.bits which may contain 20-minute-rule minimum
                let difficulty = if (height as u64) < 2016 {
                    DifficultyTarget::initial_for_network(consensus.params().network).as_f64()
                } else {
                    DifficultyTarget::from_bits(tip.header.bits).as_f64()
                };

                // Collect timestamps of last 11 blocks (or fewer if chain is shorter)
                let mut timestamps: Vec<u64> = Vec::with_capacity(11);
                let mut current_hash = tip.hash();

                for _ in 0..11 {
                    if let Ok(Some(block)) = db.get_block(&current_hash) {
                        timestamps.push(block.header.timestamp);
                        if block.header.prev_hash == Hash::zero() {
                            break; // Genesis block reached
                        }
                        current_hash = block.header.prev_hash;
                    } else {
                        break;
                    }
                }

                // Calculate median time
                let median = if timestamps.is_empty() {
                    0
                } else {
                    timestamps.sort_unstable();
                    timestamps[timestamps.len() / 2]
                };

                // Get disk usage from database statistics
                let disk_usage = db.get_disk_usage();

                (hash, height, difficulty, median, disk_usage)
            } else {
                // Get disk usage even when no chain tip
                let disk_usage = db.get_disk_usage();

                (
                    "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    0,
                    1.0,
                    0,
                    disk_usage,
                )
            }
        };

        Ok(json!({
            "chain": match consensus.params().network {
                Network::Mainnet => "main",
                Network::Testnet => "test",
                Network::Regtest => "regtest",
            },
            "blocks": height,
            "headers": height,
            "bestblockhash": best_hash,
            "difficulty": difficulty,
            "mediantime": median_time,
            "verificationprogress": 1.0,
            "initialblockdownload": false,
            "chainwork": "0000000000000000000000000000000000000000000000000000000000000000",
            "size_on_disk": size_on_disk,
            "pruned": false,
            "consensus": {
                "network": format!("{:?}", consensus.params().network),
                "algorithm": "SHA-512",
                "signatures": "ML-DSA-87",
                "reward_algorithm": "linear_decay"
            }
        }))
    }

    /// Get block with validation information
    async fn get_block_with_validation(
        blockchain_db: Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
        block_validator: Arc<StorageBlockValidator>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        let params = params.ok_or(RpcServerError::InvalidParams(
            "Missing block hash".to_string(),
        ))?;

        let block_hash = if let Value::Array(ref arr) = params {
            arr.first()
                .and_then(|v| v.as_str())
                .ok_or(RpcServerError::InvalidParams(
                    "Invalid block hash".to_string(),
                ))?
        } else {
            return Err(RpcServerError::InvalidParams(
                "Expected array parameters".to_string(),
            ));
        };

        let hash = Hash::from_hex(block_hash)
            .map_err(|_| RpcServerError::InvalidParams("Invalid hash format".to_string()))?;

        // Get block data, height, and tip height in a single lock acquisition
        let (block_opt, block_height, tip_height) = {
            let db = blockchain_db
                .read()
                .map_err(|e| RpcServerError::Internal(format!("Lock poisoned: {}", e)))?;

            let block_opt = db.get_block(&hash)
                .map_err(|e| RpcServerError::Internal(e.to_string()))?;

            let block_height = db.get_block_height(&hash).unwrap_or(0);

            let tip_height = db.get_chain_tip()
                .ok()
                .flatten()
                .and_then(|tip| db.get_block_height(&tip.hash()).ok())
                .unwrap_or(0);

            (block_opt, block_height, tip_height)
        };

        match block_opt {
            Some(block) => {
                // Validate block
                let validation_result = block_validator.validate_block_with_context(&block).await;
                let is_valid = validation_result.is_ok();
                let validation_error = if !is_valid {
                    Some(format!("{}", validation_result.unwrap_err()))
                } else {
                    None
                };

                // Calculate confirmations: tip_height - block_height + 1
                let confirmations = if tip_height >= block_height {
                    tip_height - block_height + 1
                } else {
                    1
                };

                // Calculate block size from serialized bytes
                let block_size = block.serialize().len();

                // Calculate difficulty from bits
                let difficulty = DifficultyTarget::from_bits(block.header.bits).as_f64();

                Ok(json!({
                    "hash": block_hash,
                    "confirmations": confirmations,
                    "size": block_size,
                    "height": block_height,
                    "version": block.header.version,
                    "merkleroot": block.header.merkle_root.to_hex(),
                    "tx": block.transactions.iter().map(|tx| tx.hash().to_hex()).collect::<Vec<_>>(),
                    "time": block.header.timestamp,
                    "nonce": block.header.nonce,
                    "bits": format!("{:08x}", block.header.bits),
                    "difficulty": difficulty,
                    "previousblockhash": block.header.prev_hash.to_hex(),
                    "validation": {
                        "valid": is_valid,
                        "error": validation_error
                    }
                }))
            }
            None => Err(RpcServerError::InvalidParams("Block not found".to_string())),
        }
    }

    /// Send raw transaction with validation
    async fn send_raw_transaction(
        tx_validator: Arc<StorageTransactionValidator>,
        blockchain_db: Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
        utxo_db: Arc<RwLock<dyn UTXODatabase + Send + Sync>>,
        mempool: Arc<RwLock<Mempool>>,
        sync_manager: Arc<IntegratedSyncManager>,
        network: Network,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        let params = params.ok_or(RpcServerError::InvalidParams(
            "Missing transaction hex".to_string(),
        ))?;

        let tx_hex = if let Value::Array(ref arr) = params {
            arr.first()
                .and_then(|v| v.as_str())
                .ok_or(RpcServerError::InvalidParams(
                    "Invalid transaction hex".to_string(),
                ))?
        } else {
            return Err(RpcServerError::InvalidParams(
                "Expected array parameters".to_string(),
            ));
        };

        // Deserialize transaction from hex
        let tx_bytes = hex::decode(tx_hex).map_err(|e| {
            RpcServerError::InvalidParams(format!("Invalid hex encoding: {}", e))
        })?;
        let transaction = Transaction::deserialize(&tx_bytes).map_err(|e| {
            RpcServerError::InvalidParams(format!("Invalid transaction format: {}", e))
        })?;

        // FIX 2026-02-21 (H5): Validate fork_id matches this node's network
        // Prevents cross-network transaction replay (e.g., testnet TX on mainnet)
        let expected_fork_id = match network {
            Network::Mainnet => 0u8,
            Network::Testnet => 1u8,
            Network::Regtest => 2u8,
        };
        if transaction.fork_id != expected_fork_id {
            return Err(RpcServerError::InvalidParams(format!(
                "Transaction fork_id {} does not match network {} (expected {})",
                transaction.fork_id,
                network.as_str(),
                expected_fork_id
            )));
        }

        // Validate transaction
        tx_validator
            .validate_transaction_with_context(&transaction)
            .await
            .map_err(|e| RpcServerError::InvalidParams(format!(
                "Transaction validation failed: {}",
                e
            )))?;

        let txid = transaction.hash();

        // Calculate transaction fee (input value - output value)
        let fee = {
            let db = utxo_db.read().map_err(|e| {
                RpcServerError::Internal(format!("Failed to acquire UTXO lock: {}", e))
            })?;

            let mut input_value: u64 = 0;
            for input in &transaction.inputs {
                if let Ok(Some(utxo)) = db.get_utxo(&input.previous_output) {
                    input_value = input_value.saturating_add(utxo.output.value);
                }
            }

            let output_value: u64 = transaction.outputs.iter().map(|o| o.value).sum();
            input_value.saturating_sub(output_value)
        };

        // Add to mempool
        {
            let mut mp = mempool.write().map_err(|e| {
                RpcServerError::Internal(format!("Failed to acquire mempool lock: {}", e))
            })?;

            mp.add_transaction(transaction.clone(), fee).map_err(|e| {
                RpcServerError::InvalidParams(format!("Failed to add to mempool: {}", e))
            })?;
        }

        // Broadcast to network peers via P2P
        // First try INV message (standard Bitcoin protocol)
        match sync_manager.broadcast_transaction(&txid).await {
            Ok(peer_count) => {
                tracing::info!("Broadcast TX {} via INV to {} peers", txid.to_hex(), peer_count);
            }
            Err(e) => {
                // If INV fails (e.g., no peers), try full TX broadcast
                tracing::warn!("INV broadcast failed ({}), trying full TX broadcast", e);
                if let Err(e2) = sync_manager.broadcast_transaction_full(&transaction).await {
                    tracing::warn!("Full TX broadcast also failed: {}", e2);
                    // Don't fail the RPC - transaction is still in mempool for mining
                }
            }
        }

        Ok(json!(txid.to_hex()))
    }

    /// Validate transaction
    async fn validate_transaction(
        tx_validator: Arc<StorageTransactionValidator>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        let params = params.ok_or(RpcServerError::InvalidParams(
            "Missing transaction hex".to_string(),
        ))?;

        let tx_hex = if let Value::Array(ref arr) = params {
            arr.first()
                .and_then(|v| v.as_str())
                .ok_or(RpcServerError::InvalidParams(
                    "Invalid transaction hex".to_string(),
                ))?
        } else {
            return Err(RpcServerError::InvalidParams(
                "Expected array parameters".to_string(),
            ));
        };

        // Deserialize transaction from hex
        let tx_bytes = hex::decode(tx_hex).map_err(|e| {
            RpcServerError::InvalidParams(format!("Invalid hex encoding: {}", e))
        })?;
        let transaction = match Transaction::deserialize(&tx_bytes) {
            Ok(tx) => tx,
            Err(e) => {
                return Ok(json!({
                    "valid": false,
                    "error": format!("Invalid transaction format: {}", e)
                }));
            }
        };

        match tx_validator
            .validate_transaction_with_context(&transaction)
            .await
        {
            Ok(()) => Ok(json!({
                "valid": true,
                "txid": transaction.hash().to_hex()
            })),
            Err(e) => Ok(json!({
                "valid": false,
                "error": format!("{}", e)
            })),
        }
    }

    /// Get block template for mining
    async fn get_block_template(
        blockchain_db: Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
        consensus_engine: Arc<TokioRwLock<ConsensusEngine>>,
        _params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        let consensus = consensus_engine.read().await;
        let chain_tip = blockchain_db
            .read()
            .map_err(|e| RpcServerError::Internal(format!("Lock poisoned: {}", e)))?
            .get_chain_tip()
            .map_err(|e| RpcServerError::Internal(e.to_string()))?;

        let (prev_hash, height) = if let Some(tip) = chain_tip {
            (tip.hash(), consensus.current_height() + 1)
        } else {
            (Hash::zero(), 0)
        };

        let reward = RewardCalculator::calculate_reward(height).unwrap_or(0);

        Ok(json!({
            "version": 1,
            "previousblockhash": prev_hash.to_hex(),
            "height": height,
            "target": "00000000ffff0000000000000000000000000000000000000000000000000000",
            "coinbasevalue": reward,
            "transactions": [],
            "mutable": ["time", "transactions", "prevblock"],
            "noncerange": "00000000ffffffff",
            "capabilities": ["proposal"]
        }))
    }

    /// Submit mined block
    async fn submit_block(
        block_validator: Arc<StorageBlockValidator>,
        consensus_engine: Arc<TokioRwLock<ConsensusEngine>>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        eprintln!(
            "[SUBMITBLOCK] Called with params: {:?}",
            params
                .as_ref()
                .map(|v| format!("{:?}", v).chars().take(100).collect::<String>())
        );

        let params = params.ok_or_else(|| {
            eprintln!("[SUBMITBLOCK ERROR] ❌ No params provided");
            RpcServerError::InvalidParams("Missing block hex".to_string())
        })?;

        let block_hex = if let Value::Array(ref arr) = params {
            eprintln!("[SUBMITBLOCK] Params is array with {} elements", arr.len());
            arr.first().and_then(|v| v.as_str()).ok_or_else(|| {
                eprintln!("[SUBMITBLOCK ERROR] ❌ First array element is not a string");
                RpcServerError::InvalidParams("Invalid block hex".to_string())
            })?
        } else {
            eprintln!(
                "[SUBMITBLOCK ERROR] ❌ Params is not an array: {:?}",
                params
            );
            return Err(RpcServerError::InvalidParams(
                "Expected array parameters".to_string(),
            ));
        };

        // Deserialize block from hex
        eprintln!(
            "[SUBMITBLOCK] Received block_hex length: {}",
            block_hex.len()
        );
        let block_bytes = hex::decode(block_hex).map_err(|e| {
            eprintln!("[SUBMITBLOCK ERROR] ❌ Hex decode failed: {}", e);
            RpcServerError::InvalidParams(format!("Invalid hex encoding: {}", e))
        })?;
        eprintln!("[SUBMITBLOCK] Decoded {} bytes", block_bytes.len());

        let block = Block::deserialize(&block_bytes).map_err(|e| {
            eprintln!("[SUBMITBLOCK ERROR] ❌ Block deserialization failed: {}", e);
            eprintln!(
                "[SUBMITBLOCK ERROR] Block bytes (first 100): {:?}",
                &block_bytes[..block_bytes.len().min(100)]
            );
            RpcServerError::InvalidParams(format!("Failed to deserialize block: {}", e))
        })?;
        eprintln!("[SUBMITBLOCK] ✅ Block deserialized successfully");

        // Validate and apply block
        match block_validator.apply_block(&block).await {
            Ok(()) => {
                // Update consensus engine height
                let mut consensus = consensus_engine.write().await;
                let current_height = consensus.current_height();
                consensus.set_current_height(current_height + 1);

                Ok(json!({
                    "result": "accepted",
                    "hash": block.hash().to_hex()
                }))
            }
            Err(e) => Ok(json!({
                "result": "rejected",
                "error": format!("{}", e)
            })),
        }
    }

    /// Get mining information (legacy helper - use registered getmininginfo method instead)
    #[allow(dead_code)]
    async fn get_mining_info(
        consensus_engine: Arc<TokioRwLock<ConsensusEngine>>,
    ) -> Result<Value, RpcServerError> {
        let consensus = consensus_engine.read().await;
        let height = consensus.current_height();
        let reward = RewardCalculator::calculate_reward(height).unwrap_or(0);
        // Note: For accurate difficulty/hashrate, use the registered getmininginfo which has blockchain_db access

        Ok(json!({
            "blocks": height,
            "difficulty": 1.0,
            "networkhashps": 0,
            "blockprioritypercentage": 5,
            "blockreward": reward,
            "algorithm": "SHA-512",
            "testnet": consensus.params().network != Network::Mainnet
        }))
    }

    /// Get peer information
    async fn get_peer_info(
        sync_manager: Arc<IntegratedSyncManager>,
    ) -> Result<Value, RpcServerError> {
        let stats = sync_manager.get_sync_stats().await;

        Ok(json!({
            "connected_peers": stats.connected_peers,
            "syncing_peers": stats.syncing_peers,
            "best_height": stats.best_height
        }))
    }

    /// Get sync information
    async fn get_sync_info(
        sync_manager: Arc<IntegratedSyncManager>,
    ) -> Result<Value, RpcServerError> {
        let stats = sync_manager.get_sync_stats().await;

        Ok(json!({
            "state": format!("{:?}", stats.state),
            "progress": stats.progress,
            "connected_peers": stats.connected_peers,
            "syncing_peers": stats.syncing_peers,
            "best_height": stats.best_height
        }))
    }

    /// Get network information
    async fn get_network_info(
        sync_manager: Arc<IntegratedSyncManager>,
    ) -> Result<Value, RpcServerError> {
        let stats = sync_manager.get_sync_stats().await;

        Ok(json!({
            "version": 1000000,
            "subversion": "/BTPC:0.1.0/",
            "protocolversion": 70015,
            "localservices": "0000000000000001",
            "localservicesnames": ["NETWORK"],
            "localrelay": true,
            "timeoffset": 0,
            "connections": stats.connected_peers,
            "networkactive": true,
            "networks": [
                {
                    "name": "ipv4",
                    "limited": false,
                    "reachable": true,
                    "proxy": "",
                    "proxy_randomize_credentials": false
                }
            ],
            "relayfee": 0.00001000,
            "incrementalfee": 0.00001000,
            "localaddresses": [],
            "warnings": ""
        }))
    }

    /// Get consensus information
    async fn get_consensus_info(
        consensus_engine: Arc<TokioRwLock<ConsensusEngine>>,
    ) -> Result<Value, RpcServerError> {
        let consensus = consensus_engine.read().await;

        Ok(json!({
            "network": format!("{:?}", consensus.params().network),
            "current_height": consensus.current_height(),
            "algorithm": "SHA-512",
            "signatures": "ML-DSA-87",
            "reward_algorithm": "linear_decay",
            "difficulty_adjustment": 2016,
            "block_time": 600
        }))
    }

    /// Get difficulty information (legacy helper - use registered getdifficultyinfo method instead)
    #[allow(dead_code)]
    async fn get_difficulty_info(
        consensus_engine: Arc<TokioRwLock<ConsensusEngine>>,
    ) -> Result<Value, RpcServerError> {
        let consensus = consensus_engine.read().await;
        // Note: For accurate difficulty, use the registered getdifficultyinfo which has blockchain_db access

        Ok(json!({
            "current_difficulty": 1.0,
            "next_adjustment": 2016 - (consensus.current_height() % 2016),
            "target_block_time": 600,
            "algorithm": "SHA-512"
        }))
    }

    /// Get reward information
    async fn get_reward_info(
        consensus_engine: Arc<TokioRwLock<ConsensusEngine>>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        let consensus = consensus_engine.read().await;

        let height = if let Some(Value::Array(arr)) = params {
            arr.first()
                .and_then(|v| v.as_u64())
                .unwrap_or(consensus.current_height() as u64) as u32
        } else {
            consensus.current_height()
        };

        let reward = RewardCalculator::calculate_reward(height).unwrap_or(0);

        Ok(json!({
            "height": height,
            "reward": reward,
            "reward_btpc": reward as f64 / 100_000_000.0,
            "algorithm": "linear_decay",
            "initial_reward": 3237500000u64,
            "tail_emission": 50000000u64,
            "decay_years": 24
        }))
    }

    /// Validate block
    async fn validate_block(
        block_validator: Arc<StorageBlockValidator>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        let params = params.ok_or(RpcServerError::InvalidParams(
            "Missing block hex".to_string(),
        ))?;

        let block_hex = if let Value::Array(ref arr) = params {
            arr.first()
                .and_then(|v| v.as_str())
                .ok_or(RpcServerError::InvalidParams(
                    "Invalid block hex".to_string(),
                ))?
        } else {
            return Err(RpcServerError::InvalidParams(
                "Expected array parameters".to_string(),
            ));
        };

        // Deserialize block from hex
        let block_bytes = hex::decode(block_hex)
            .map_err(|e| RpcServerError::InvalidParams(format!("Invalid hex encoding: {}", e)))?;

        let block = Block::deserialize(&block_bytes).map_err(|e| {
            RpcServerError::InvalidParams(format!("Failed to deserialize block: {}", e))
        })?;

        match block_validator.validate_block_with_context(&block).await {
            Ok(()) => Ok(json!({
                "valid": true,
                "hash": block.hash().to_hex()
            })),
            Err(e) => Ok(json!({
                "valid": false,
                "error": format!("{}", e)
            })),
        }
    }

    // Helper methods (reused from original handlers)

    /// Convert compact bits format to difficulty value
    /// Formula: difficulty = max_target / current_target
    /// where max_target is 0x00000000FFFF0000... (Bitcoin's easiest target)
    fn bits_to_difficulty(bits: u32) -> f64 {
        // Extract exponent and coefficient from compact format
        let exponent = ((bits >> 24) & 0xFF) as i32;
        let coefficient = (bits & 0x00FFFFFF) as f64;

        if coefficient == 0.0 {
            return 1.0;
        }

        // Calculate target from bits
        // target = coefficient * 256^(exponent - 3)
        let shift = exponent - 3;

        // Reference difficulty 1 target (SHA-512 standard for BTPC)
        // FIX 2025-12-27: Updated to SHA-512 compatible reference
        // SHA-512: 0x3c7fffff means exponent 60, mantissa 0x7fffff
        // This corresponds to ~32 bits of work (4 leading zero bytes)
        let diff1_coefficient = 0x7FFFFF as f64;
        let diff1_exponent = 0x3c_i32;
        let diff1_shift = diff1_exponent - 3;

        // difficulty = diff1_target / current_target
        // = (diff1_coeff * 256^diff1_shift) / (coeff * 256^shift)
        // = (diff1_coeff / coeff) * 256^(diff1_shift - shift)
        let ratio = diff1_coefficient / coefficient;
        let power = (diff1_shift - shift) as f64;

        ratio * 256.0_f64.powf(power)
    }

    fn get_best_block_hash(
        blockchain_db: &Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
    ) -> Result<Value, RpcServerError> {
        // Query the actual best block hash from database
        let blockchain_guard = blockchain_db
            .try_read()
            .map_err(|e| RpcServerError::Internal(format!("Lock error: {}", e)))?;

        match blockchain_guard.get_chain_tip() {
            Ok(Some(block)) => {
                let hash = block.hash();
                Ok(json!(hex::encode(hash.as_bytes())))
            }
            Ok(None) => Ok(json!("0000000000000000000000000000000000000000000000000000000000000000")),
            Err(e) => Err(RpcServerError::Internal(format!("Failed to get chain tip: {}", e))),
        }
    }

    fn get_block_header(
        blockchain_db: &Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        // Implementation similar to original
        Ok(json!({}))
    }

    fn get_block_count(
        blockchain_db: &Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
    ) -> Result<Value, RpcServerError> {
        // Query the actual block count from database
        let blockchain_guard = blockchain_db
            .try_read()
            .map_err(|e| RpcServerError::Internal(format!("Lock error: {}", e)))?;

        match blockchain_guard.get_chain_tip() {
            Ok(Some(block)) => {
                let hash = block.hash();
                let height = blockchain_guard.get_block_height(&hash).unwrap_or(0);
                Ok(json!(height))
            }
            Ok(None) => Ok(json!(0)),
            Err(e) => Err(RpcServerError::Internal(format!("Failed to get chain tip: {}", e))),
        }
    }

    fn get_transaction(
        blockchain_db: &Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        // Implementation similar to original
        Ok(json!({}))
    }

    fn get_recent_transactions_handler(
        blockchain_db: &Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        let (limit, offset) = match params {
            Some(Value::Array(arr)) => {
                let limit = arr.first().and_then(|v| v.as_u64()).unwrap_or(10).min(100) as usize;
                let offset = arr.get(1).and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                (limit, offset)
            }
            None => (10, 0),
            _ => {
                return Err(RpcServerError::InvalidParams(
                    "Invalid parameters format".to_string(),
                ))
            }
        };

        // Get chain tip and its height to start iteration
        let (tip_block, tip_height) = {
            let db = blockchain_db
                .read()
                .map_err(|e| RpcServerError::Internal(format!("Lock poisoned: {}", e)))?;
            let tip = db.get_chain_tip()
                .map_err(|e| RpcServerError::Internal(e.to_string()))?;
            let height = tip.as_ref()
                .and_then(|t| db.get_block_height(&t.hash()).ok())
                .unwrap_or(0) as u64;
            (tip, height)
        };

        let mut transactions = Vec::new();
        let mut skipped = 0;
        let mut current_hash = tip_block.as_ref().map(|b| b.hash());
        let mut current_height = tip_height;

        // Iterate backwards through the blockchain
        while transactions.len() < limit && current_hash.is_some() {
            let Some(hash) = current_hash else {
                break; // Redundant but explicit
            };
            let block = blockchain_db
                .read()
                .map_err(|e| RpcServerError::Internal(format!("Lock poisoned: {}", e)))?
                .get_block(&hash)
                .map_err(|e| RpcServerError::Internal(e.to_string()))?;

            if let Some(block) = block {
                // Iterate through transactions in reverse (newest first)
                for tx in block.transactions.iter().rev() {
                    if skipped < offset {
                        skipped += 1;
                        continue;
                    }

                    if transactions.len() >= limit {
                        break;
                    }

                    transactions.push(json!({
                        "hash": hex::encode(tx.hash().as_bytes()),
                        "block_height": current_height,
                        "block_hash": hex::encode(hash.as_bytes()),
                        "timestamp": block.header.timestamp,
                        "inputs": tx.inputs.len(),
                        "outputs": tx.outputs.len(),
                        "total_value": tx.outputs.iter().map(|o| o.value).sum::<u64>(),
                    }));
                }

                // Move to previous block
                current_hash = Some(block.header.prev_hash);
                current_height = current_height.saturating_sub(1);
            } else {
                break;
            }
        }

        Ok(json!({
            "transactions": transactions,
            "count": transactions.len(),
            "limit": limit,
            "offset": offset,
        }))
    }

    fn get_tx_out(
        utxo_db: &Arc<RwLock<dyn UTXODatabase + Send + Sync>>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        // Implementation similar to original
        Ok(json!({}))
    }

    /// Get UTXOs for a specific address
    /// RPC method: getutxosforaddress
    /// Params: ["address_string"]
    fn get_utxos_for_address(
        utxo_db: &Arc<RwLock<dyn UTXODatabase + Send + Sync>>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        // Parse address from params
        let address_str = match params {
            Some(Value::Array(arr)) if !arr.is_empty() => {
                arr[0]
                    .as_str()
                    .ok_or_else(|| RpcServerError::InvalidParams("Address must be a string".into()))?
                    .to_string()
            }
            Some(Value::Object(obj)) => obj
                .get("address")
                .and_then(|v| v.as_str())
                .ok_or_else(|| RpcServerError::InvalidParams("Missing 'address' parameter".into()))?
                .to_string(),
            _ => {
                return Err(RpcServerError::InvalidParams(
                    "Missing address parameter".into(),
                ))
            }
        };

        // Parse the address to get the pubkey hash
        let address = Address::from_string(&address_str).map_err(|e| {
            RpcServerError::InvalidParams(format!("Invalid address '{}': {:?}", address_str, e))
        })?;

        let pubkey_hash = address.hash160();

        // Query UTXOs from database
        let db = utxo_db.read().map_err(|e| {
            RpcServerError::Internal(format!("Failed to acquire database lock: {}", e))
        })?;

        let utxos = db.get_utxos_for_pubkey_hash(pubkey_hash).map_err(|e| {
            RpcServerError::Internal(format!("Failed to query UTXOs: {:?}", e))
        })?;

        // Convert UTXOs to JSON response
        let utxo_list: Vec<Value> = utxos
            .iter()
            .map(|utxo| {
                json!({
                    "txid": utxo.outpoint.txid.to_string(),
                    "vout": utxo.outpoint.vout,
                    "value": utxo.output.value,
                    "height": utxo.height,
                    "is_coinbase": utxo.is_coinbase,
                })
            })
            .collect();

        Ok(json!(utxo_list))
    }

    fn help(params: Option<Value>) -> Result<Value, RpcServerError> {
        if let Some(Value::Array(arr)) = params {
            if let Some(Value::String(command)) = arr.first() {
                let help_text = match command.as_str() {
                    "getblockchaininfo" => "Returns comprehensive blockchain information including consensus data",
                    "getblock" => "getblock \"blockhash\" - Returns block information with validation status",
                    "getutxosforaddress" => "getutxosforaddress \"address\" - Returns all UTXOs for the specified address",
                    "sendrawtransaction" => "sendrawtransaction \"hexstring\" - Validates and broadcasts a transaction",
                    "validatetransaction" => "validatetransaction \"hexstring\" - Validates a transaction without broadcasting",
                    "getblocktemplate" => "Returns a block template for mining",
                    "submitblock" => "submitblock \"hexdata\" - Submits a mined block",
                    "getmininginfo" => "Returns mining-related information",
                    "getpeerinfo" => "Returns information about network peers",
                    "getsyncinfo" => "Returns blockchain synchronization status",
                    "getconsensusinfo" => "Returns consensus algorithm information",
                    "getdifficultyinfo" => "Returns difficulty adjustment information",
                    "getrewardinfo" => "getrewardinfo [height] - Returns block reward information",
                    "validateblock" => "validateblock \"hexstring\" - Validates a block",
                    _ => "Unknown command",
                };
                return Ok(json!(help_text));
            }
        }

        Ok(json!([
            "getblockchaininfo",
            "getbestblockhash",
            "getblock",
            "getblockheader",
            "getblockcount",
            "gettransaction",
            "gettxout",
            "getutxosforaddress",
            "sendrawtransaction",
            "validatetransaction",
            "getblocktemplate",
            "submitblock",
            "getmininginfo",
            "getpeerinfo",
            "getsyncinfo",
            "getnetworkinfo",
            "getconsensusinfo",
            "getdifficultyinfo",
            "getrewardinfo",
            "validateblock",
            "help",
            "uptime"
        ]))
    }

    fn uptime() -> Result<Value, RpcServerError> {
        Ok(json!(0))
    }

    /// Estimate network hashrate based on block timestamps and difficulty
    /// Uses the last `num_blocks` blocks to calculate average block time
    fn estimate_network_hashrate(
        db: &dyn BlockchainDatabase,
        num_blocks: u32,
    ) -> f64 {
        // Get chain tip
        let tip = match db.get_chain_tip() {
            Ok(Some(tip)) => tip,
            _ => return 0.0,
        };

        if num_blocks < 2 {
            return 0.0;
        }

        // Get current difficulty
        let difficulty = DifficultyTarget::from_bits(tip.header.bits).as_f64();

        // Walk back num_blocks blocks to get time span
        let tip_time = tip.header.timestamp;
        let mut current_hash = tip.header.prev_hash;
        let mut blocks_counted = 1u32;

        let mut oldest_time = tip_time;

        while blocks_counted < num_blocks {
            match db.get_block(&current_hash) {
                Ok(Some(block)) => {
                    oldest_time = block.header.timestamp;
                    blocks_counted += 1;

                    if block.header.prev_hash == Hash::zero() {
                        // Reached genesis
                        break;
                    }
                    current_hash = block.header.prev_hash;
                }
                _ => break,
            }
        }

        if blocks_counted < 2 || tip_time <= oldest_time {
            return 0.0;
        }

        // Calculate seconds per block
        let time_span = (tip_time - oldest_time) as f64;
        let seconds_per_block = time_span / (blocks_counted - 1) as f64;

        if seconds_per_block <= 0.0 {
            return 0.0;
        }

        // Network hashrate = difficulty * 2^32 / seconds_per_block
        // For SHA-512 (64-byte hash), we use 2^32 as a reasonable approximation
        // This gives hashes/second
        difficulty * 4294967296.0 / seconds_per_block
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::{
        network::SyncConfig,
        storage::{
            database::{Database, DatabaseConfig},
            BlockchainDb, UtxoDb,
        },
    };

    async fn create_test_handlers() -> (IntegratedRpcHandlers, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_config = DatabaseConfig::test();
        let database = Arc::new(Database::open(temp_dir.path(), db_config).unwrap());

        // Wrap databases in RwLock for StorageBlockValidator (Priority 2 fix)
        let blockchain_db = Arc::new(RwLock::new(BlockchainDb::new(database.clone())))
            as Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>;
        let utxo_db = Arc::new(RwLock::new(UtxoDb::new(database)))
            as Arc<RwLock<dyn UTXODatabase + Send + Sync>>;

        let sync_manager = Arc::new(IntegratedSyncManager::new(
            blockchain_db.clone(),
            utxo_db.clone(),
            SyncConfig::default(),
        ));

        let mempool = Arc::new(std::sync::RwLock::new(crate::mempool::Mempool::new()));

        let handlers =
            IntegratedRpcHandlers::new(blockchain_db, utxo_db, sync_manager, mempool, Network::Regtest);

        (handlers, temp_dir)
    }

    #[tokio::test]
    async fn test_handlers_creation() {
        let (handlers, _temp_dir) = create_test_handlers().await;

        // Test basic functionality
        let consensus = handlers.consensus_engine.read().await;
        assert_eq!(consensus.params().network, Network::Regtest);
    }

    #[tokio::test]
    async fn test_blockchain_info() {
        let (handlers, _temp_dir) = create_test_handlers().await;

        let result = IntegratedRpcHandlers::get_blockchain_info_integrated(
            handlers.blockchain_db.clone(),
            handlers.consensus_engine.clone(),
        )
        .await;

        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(info.get("consensus").is_some());
    }

    #[tokio::test]
    async fn test_help_system() {
        let help_result = IntegratedRpcHandlers::help(None).unwrap();

        if let Value::Array(commands) = help_result {
            assert!(commands.len() > 10);
            assert!(commands.contains(&json!("getblockchaininfo")));
            assert!(commands.contains(&json!("validateblock")));
        } else {
            panic!("Expected array of commands");
        }
    }
}
