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
    blockchain::{Block, BlockHeader, Transaction},
    consensus::{
        ConsensusEngine, ConsensusParams, RewardCalculator, StorageBlockValidator,
        StorageTransactionValidator, StorageValidationError,
    },
    crypto::Hash,
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
    /// Network type
    network: Network,
}

impl IntegratedRpcHandlers {
    /// Create new integrated RPC handlers
    pub fn new(
        blockchain_db: Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
        utxo_db: Arc<RwLock<dyn UTXODatabase + Send + Sync>>,
        sync_manager: Arc<IntegratedSyncManager>,
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
        let consensus_engine = Arc::clone(&self.consensus_engine);
        server.register_method("getblockchaininfo", move |_| {
            let blockchain_db = Arc::clone(&blockchain_db);
            let consensus_engine = Arc::clone(&consensus_engine);
            // For now, return a placeholder until we implement proper async RPC support
            Ok(json!({
                "chain": "main",
                "blocks": 0,
                "headers": 0,
                "bestblockhash": "0000000000000000000000000000000000000000000000000000000000000000",
                "difficulty": 1.0,
                "verificationprogress": 1.0,
                "chainwork": "0000000000000000000000000000000000000000000000000000000000000001"
            }))
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
                // Placeholder for getblock - returns simple block info
                Ok(json!({
                    "hash": "0000000000000000000000000000000000000000000000000000000000000000",
                    "confirmations": 0,
                    "size": 0,
                    "height": 0,
                    "version": 1,
                    "tx": [],
                    "time": 0,
                    "nonce": 0,
                    "bits": "1d00ffff",
                    "difficulty": 1.0,
                    "validation": {
                        "valid": true,
                        "error": null
                    }
                }))
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

        let tx_validator = Arc::clone(&self.tx_validator);
        let blockchain_db = Arc::clone(&self.blockchain_db);
        server
            .register_method("sendrawtransaction", move |params| {
                // Placeholder for sendrawtransaction
                Ok(json!(
                    "0000000000000000000000000000000000000000000000000000000000000000"
                ))
            })
            .await;

        let tx_validator = Arc::clone(&self.tx_validator);
        server
            .register_method("validatetransaction", move |params| {
                // Placeholder for validatetransaction
                Ok(json!({
                    "valid": true,
                    "txid": "0000000000000000000000000000000000000000000000000000000000000000"
                }))
            })
            .await;
    }

    /// Register mining methods
    async fn register_mining_methods(&self, server: &RpcServer) {
        let blockchain_db = Arc::clone(&self.blockchain_db);
        let consensus_engine = Arc::clone(&self.consensus_engine);
        server.register_method("getblocktemplate", move |params| {
            // Placeholder for getblocktemplate
            Ok(json!({
                "version": 1,
                "previousblockhash": "0000000000000000000000000000000000000000000000000000000000000000",
                "height": 0,
                "target": "00000000ffff0000000000000000000000000000000000000000000000000000",
                "coinbasevalue": 5000000000u64,
                "transactions": [],
                "mutable": ["time", "transactions", "prevblock"],
                "noncerange": "00000000ffffffff",
                "capabilities": ["proposal"]
            }))
        }).await;

        let block_validator = Arc::clone(&self.block_validator);
        let consensus_engine = Arc::clone(&self.consensus_engine);
        server
            .register_method("submitblock", move |params| {
                // Placeholder for submitblock
                Ok(json!({
                    "result": "accepted",
                    "hash": "0000000000000000000000000000000000000000000000000000000000000000"
                }))
            })
            .await;

        let consensus_engine = Arc::clone(&self.consensus_engine);
        server
            .register_method("getmininginfo", move |_| {
                // Placeholder for getmininginfo
                Ok(json!({
                    "blocks": 0,
                    "difficulty": 1.0,
                    "networkhashps": 0,
                    "blockprioritypercentage": 5,
                    "blockreward": 5000000000u64,
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
                // Placeholder for getpeerinfo
                Ok(json!({
                    "connected_peers": 0,
                    "syncing_peers": 0,
                    "best_height": 0
                }))
            })
            .await;

        let sync_manager = Arc::clone(&self.sync_manager);
        server
            .register_method("getsyncinfo", move |_| {
                // Placeholder for getsyncinfo
                Ok(json!({
                    "state": "Idle",
                    "progress": 1.0,
                    "connected_peers": 0,
                    "syncing_peers": 0,
                    "best_height": 0
                }))
            })
            .await;

        let sync_manager = Arc::clone(&self.sync_manager);
        server
            .register_method("getnetworkinfo", move |_| {
                // Placeholder for getnetworkinfo
                Ok(json!({
                    "version": 1000000,
                    "subversion": "/BTPC:0.1.0/",
                    "protocolversion": 70015,
                    "localservices": "0000000000000001",
                    "localservicesnames": ["NETWORK"],
                    "localrelay": true,
                    "timeoffset": 0,
                    "connections": 0,
                    "networkactive": true
                }))
            })
            .await;
    }

    /// Register consensus methods
    async fn register_consensus_methods(&self, server: &RpcServer) {
        let consensus_engine = Arc::clone(&self.consensus_engine);
        server
            .register_method("getconsensusinfo", move |_| {
                // Placeholder for getconsensusinfo
                Ok(json!({
                    "network": "Regtest",
                    "current_height": 0,
                    "algorithm": "SHA-512",
                    "signatures": "ML-DSA-87",
                    "reward_algorithm": "linear_decay",
                    "difficulty_adjustment": 2016,
                    "block_time": 600
                }))
            })
            .await;

        let consensus_engine = Arc::clone(&self.consensus_engine);
        server
            .register_method("getdifficultyinfo", move |_| {
                // Placeholder for getdifficultyinfo
                Ok(json!({
                    "current_difficulty": 1.0,
                    "next_adjustment": 2016,
                    "target_block_time": 600,
                    "algorithm": "SHA-512"
                }))
            })
            .await;

        let consensus_engine = Arc::clone(&self.consensus_engine);
        server
            .register_method("getrewardinfo", move |params| {
                // Placeholder for getrewardinfo
                Ok(json!({
                    "height": 0,
                    "reward": 5000000000u64,
                    "reward_btpc": 50.0,
                    "algorithm": "linear_decay",
                    "initial_reward": 3237500000u64,
                    "tail_emission": 50000000u64,
                    "decay_years": 24
                }))
            })
            .await;
    }

    /// Register validation methods
    async fn register_validation_methods(&self, server: &RpcServer) {
        let block_validator = Arc::clone(&self.block_validator);
        server
            .register_method("validateblock", move |params| {
                // Placeholder for validateblock
                Ok(json!({
                    "valid": true,
                    "hash": "0000000000000000000000000000000000000000000000000000000000000000"
                }))
            })
            .await;

        server
            .register_method("help", Self::help)
            .await;

        server.register_method("uptime", |_| Self::uptime()).await;
    }

    /// Get enhanced blockchain information with consensus data
    async fn get_blockchain_info_integrated(
        blockchain_db: Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
        consensus_engine: Arc<TokioRwLock<ConsensusEngine>>,
    ) -> Result<Value, RpcServerError> {
        let consensus = consensus_engine.read().await;
        let chain_tip = blockchain_db
            .read()
            .map_err(|e| RpcServerError::Internal(format!("Lock poisoned: {}", e)))?
            .get_chain_tip()
            .map_err(|e| RpcServerError::Internal(e.to_string()))?;

        let (best_hash, height, difficulty) = if let Some(tip) = chain_tip {
            let hash = tip.hash().to_hex();
            let height = consensus.current_height();
            let difficulty = 1.0; // TODO: Calculate actual difficulty
            (hash, height, difficulty)
        } else {
            (
                "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                0,
                1.0,
            )
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
            "mediantime": 0, // TODO: Calculate median time
            "verificationprogress": 1.0,
            "initialblockdownload": false,
            "chainwork": "0000000000000000000000000000000000000000000000000000000000000000",
            "size_on_disk": 0, // TODO: Get actual disk usage
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

        let db = blockchain_db
            .read()
            .map_err(|e| RpcServerError::Internal(format!("Lock poisoned: {}", e)))?;
        match db.get_block(&hash) {
            Ok(Some(block)) => {
                // Validate block
                let validation_result = block_validator.validate_block_with_context(&block).await;
                let is_valid = validation_result.is_ok();
                let validation_error = if !is_valid {
                    Some(format!("{}", validation_result.unwrap_err()))
                } else {
                    None
                };

                Ok(json!({
                    "hash": block_hash,
                    "confirmations": 1, // TODO: Calculate actual confirmations
                    "size": 0, // TODO: Calculate actual size
                    "height": 0, // TODO: Get actual height
                    "version": block.header.version,
                    "merkleroot": block.header.merkle_root.to_hex(),
                    "tx": block.transactions.iter().map(|tx| tx.hash().to_hex()).collect::<Vec<_>>(),
                    "time": block.header.timestamp,
                    "nonce": block.header.nonce,
                    "bits": format!("{:08x}", block.header.bits),
                    "difficulty": 1.0, // TODO: Calculate difficulty
                    "previousblockhash": block.header.prev_hash.to_hex(),
                    "validation": {
                        "valid": is_valid,
                        "error": validation_error
                    }
                }))
            }
            Ok(None) => Err(RpcServerError::InvalidParams("Block not found".to_string())),
            Err(_) => Err(RpcServerError::Internal("Database error".to_string())),
        }
    }

    /// Send raw transaction with validation
    async fn send_raw_transaction(
        tx_validator: Arc<StorageTransactionValidator>,
        blockchain_db: Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
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

        // TODO: Deserialize transaction from hex
        // For now, create a dummy transaction
        let transaction = Transaction::create_test_transfer(1000000, Hash::random());

        // Validate transaction
        match tx_validator
            .validate_transaction_with_context(&transaction)
            .await
        {
            Ok(()) => {
                let txid = transaction.hash();
                // TODO: Add to mempool and broadcast
                Ok(json!(txid.to_hex()))
            }
            Err(e) => Err(RpcServerError::InvalidParams(format!(
                "Transaction validation failed: {}",
                e
            ))),
        }
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

        // TODO: Deserialize transaction from hex
        let transaction = Transaction::create_test_transfer(1000000, Hash::random());

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

        // TODO: Deserialize block from hex
        let block = Block::create_test_block();

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

    /// Get mining information
    async fn get_mining_info(
        consensus_engine: Arc<TokioRwLock<ConsensusEngine>>,
    ) -> Result<Value, RpcServerError> {
        let consensus = consensus_engine.read().await;
        let height = consensus.current_height();
        let reward = RewardCalculator::calculate_reward(height).unwrap_or(0);

        Ok(json!({
            "blocks": height,
            "difficulty": 1.0, // TODO: Calculate actual difficulty
            "networkhashps": 0, // TODO: Estimate network hashrate
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

    /// Get difficulty information
    async fn get_difficulty_info(
        consensus_engine: Arc<TokioRwLock<ConsensusEngine>>,
    ) -> Result<Value, RpcServerError> {
        let consensus = consensus_engine.read().await;

        Ok(json!({
            "current_difficulty": 1.0, // TODO: Calculate actual difficulty
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

        // TODO: Deserialize block from hex
        let block = Block::create_test_block();

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
    fn get_best_block_hash(
        blockchain_db: &Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
    ) -> Result<Value, RpcServerError> {
        Ok(json!(
            "0000000000000000000000000000000000000000000000000000000000000000"
        ))
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
        // TODO: Get actual block count
        Ok(json!(0))
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
                let limit = arr.first()
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10)
                    .min(100) as usize;
                let offset = arr
                    .get(1)
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
                (limit, offset)
            }
            None => (10, 0),
            _ => {
                return Err(RpcServerError::InvalidParams(
                    "Invalid parameters format".to_string(),
                ))
            }
        };

        // Get chain tip to start iteration
        let tip_block = blockchain_db
            .read()
            .map_err(|e| RpcServerError::Internal(format!("Lock poisoned: {}", e)))?
            .get_chain_tip()
            .map_err(|e| RpcServerError::Internal(e.to_string()))?;

        let mut transactions = Vec::new();
        let mut skipped = 0;
        let mut current_hash = tip_block.as_ref().map(|b| b.hash());
        let mut current_height = 0u64; // TODO: Get actual height

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

    fn help(params: Option<Value>) -> Result<Value, RpcServerError> {
        if let Some(Value::Array(arr)) = params {
            if let Some(Value::String(command)) = arr.first() {
                let help_text = match command.as_str() {
                    "getblockchaininfo" => "Returns comprehensive blockchain information including consensus data",
                    "getblock" => "getblock \"blockhash\" - Returns block information with validation status",
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

        let handlers =
            IntegratedRpcHandlers::new(blockchain_db, utxo_db, sync_manager, Network::Regtest);

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
