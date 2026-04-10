//! RPC method handlers for blockchain operations

#![allow(dead_code)]
#![allow(unused_variables)]

use std::sync::Arc;

use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::{
    crypto::Hash,
    rpc::{server::RpcServer, RpcServerError},
    storage::{blockchain_db::BlockchainDb, utxo_db::UtxoDb},
};

/// Blockchain RPC handlers
pub struct BlockchainRpcHandlers {
    blockchain_db: Arc<RwLock<BlockchainDb>>,
    utxo_db: Arc<RwLock<UtxoDb>>,
    network: crate::Network,
}

impl BlockchainRpcHandlers {
    /// Create new blockchain RPC handlers
    pub fn new(
        blockchain_db: Arc<RwLock<BlockchainDb>>,
        utxo_db: Arc<RwLock<UtxoDb>>,
        network: crate::Network,
    ) -> Self {
        BlockchainRpcHandlers {
            blockchain_db,
            utxo_db,
            network,
        }
    }

    /// Register all blockchain RPC methods
    pub async fn register_methods(&self, server: &RpcServer) {
        let blockchain_db = Arc::clone(&self.blockchain_db);
        let network = self.network;
        server
            .register_method("getblockchaininfo", move |_| {
                Self::get_blockchain_info(&blockchain_db, network)
            })
            .await;

        let blockchain_db = Arc::clone(&self.blockchain_db);
        server
            .register_method("getbestblockhash", move |_| {
                Self::get_best_block_hash(&blockchain_db)
            })
            .await;

        let blockchain_db = Arc::clone(&self.blockchain_db);
        server
            .register_method("getblock", move |params| {
                Self::get_block(&blockchain_db, params)
            })
            .await;

        let blockchain_db = Arc::clone(&self.blockchain_db);
        server
            .register_method("getblockheader", move |params| {
                Self::get_block_header(&blockchain_db, params)
            })
            .await;

        let utxo_db = Arc::clone(&self.utxo_db);
        server
            .register_method("gettxout", move |params| Self::get_tx_out(&utxo_db, params))
            .await;

        // Info methods
        server.register_method("help", Self::help).await;

        server.register_method("uptime", |_| Self::uptime()).await;

        // Network methods
        server
            .register_method("getpeerinfo", |_| Self::get_peer_info())
            .await;

        server
            .register_method("getnetworkinfo", |_| Self::get_network_info())
            .await;

        // Mining methods
        let blockchain_db = Arc::clone(&self.blockchain_db);
        let utxo_db = Arc::clone(&self.utxo_db);
        let network = self.network;
        server
            .register_method("submitblock", move |params| {
                Self::submit_block(&blockchain_db, &utxo_db, network, params)
            })
            .await;

        let blockchain_db = Arc::clone(&self.blockchain_db);
        let network = self.network;
        server
            .register_method("getblocktemplate", move |params| {
                Self::get_block_template(&blockchain_db, network, params)
            })
            .await;

        // Transaction methods
        let blockchain_db = Arc::clone(&self.blockchain_db);
        let utxo_db = Arc::clone(&self.utxo_db);
        server
            .register_method("sendrawtransaction", move |params| {
                Self::send_raw_transaction(&blockchain_db, &utxo_db, params)
            })
            .await;

        // Get raw transaction (for transaction monitoring)
        let blockchain_db = Arc::clone(&self.blockchain_db);
        server
            .register_method("getrawtransaction", move |params| {
                Self::get_raw_transaction(&blockchain_db, params)
            })
            .await;
    }

    /// Get blockchain information
    fn get_blockchain_info(
        blockchain_db: &Arc<RwLock<BlockchainDb>>,
        network: crate::Network,
    ) -> Result<Value, RpcServerError> {
        use crate::storage::BlockchainDatabase;

        // Get chain tip
        let blockchain_guard = blockchain_db
            .try_read()
            .map_err(|e| RpcServerError::Internal(format!("Lock error: {}", e)))?;
        let tip_block = (*blockchain_guard)
            .get_chain_tip()
            .map_err(|e| RpcServerError::Internal(format!("Failed to get chain tip: {}", e)))?;

        // Convert network to chain name
        let chain_name = match network {
            crate::Network::Mainnet => "main",
            crate::Network::Testnet => "test",
            crate::Network::Regtest => "regtest",
        };

        match tip_block {
            Some(block) => {
                let block_hash = block.hash();
                let height = (*blockchain_guard)
                    .get_block_height(&block_hash)
                    .unwrap_or(0);

                Ok(json!({
                    "chain": chain_name,
                    "blocks": height,
                    "headers": height,
                    "bestblockhash": hex::encode(block_hash.as_bytes()),
                    "difficulty": 1.0,
                    "mediantime": block.header.timestamp,
                    "verificationprogress": 1.0,
                    "initialblockdownload": false,
                    "chainwork": "0000000000000000000000000000000000000000000000000000000000000000",
                    "size_on_disk": 0,
                    "pruned": false
                }))
            }
            None => {
                // No blocks yet
                Ok(json!({
                    "chain": chain_name,
                    "blocks": 0,
                    "headers": 0,
                    "bestblockhash": "0000000000000000000000000000000000000000000000000000000000000000",
                    "difficulty": 1.0,
                    "mediantime": 0,
                    "verificationprogress": 1.0,
                    "initialblockdownload": false,
                    "chainwork": "0000000000000000000000000000000000000000000000000000000000000000",
                    "size_on_disk": 0,
                    "pruned": false
                }))
            }
        }
    }

    /// Get best block hash
    fn get_best_block_hash(
        blockchain_db: &Arc<RwLock<BlockchainDb>>,
    ) -> Result<Value, RpcServerError> {
        use crate::storage::BlockchainDatabase;

        let blockchain_guard = blockchain_db
            .try_read()
            .map_err(|e| RpcServerError::Internal(format!("Lock error: {}", e)))?;
        let tip_block = (*blockchain_guard).get_chain_tip().ok().flatten();
        Ok(json!(tip_block.map(|b| b.hash().to_hex()).unwrap_or_else(|| "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".to_string())))
    }

    /// Get block by hash
    fn get_block(
        blockchain_db: &Arc<RwLock<BlockchainDb>>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        use crate::storage::BlockchainDatabase;

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

        // Parse hash
        let hash = Hash::from_hex(block_hash)
            .map_err(|_| RpcServerError::InvalidParams("Invalid hash format".to_string()))?;

        // Get block from database
        let blockchain_guard = blockchain_db
            .try_read()
            .map_err(|e| RpcServerError::Internal(format!("Lock error: {}", e)))?;
        match (*blockchain_guard).get_block(&hash) {
            Ok(Some(block)) => {
                let height = (*blockchain_guard).get_block_height(&hash).unwrap_or(0);
                Ok(json!({
                    "hash": block_hash,
                    "confirmations": 1,
                    "size": 0, // Would calculate actual size
                    "height": height,
                    "version": block.header.version,
                    "merkleroot": block.header.merkle_root.to_hex(),
                    "tx": block.transactions.iter().map(|tx| tx.hash().to_hex()).collect::<Vec<_>>(),
                    "time": block.header.timestamp,
                    "nonce": block.header.nonce,
                    "bits": format!("{:08x}", block.header.bits),
                    "difficulty": 1.0,
                    "previousblockhash": block.header.prev_hash.to_hex()
                }))
            }
            Ok(None) => Err(RpcServerError::InvalidParams("Block not found".to_string())),
            Err(_) => Err(RpcServerError::Internal("Database error".to_string())),
        }
    }

    /// Get block header by hash
    fn get_block_header(
        blockchain_db: &Arc<RwLock<BlockchainDb>>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        use crate::storage::BlockchainDatabase;

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

        // Parse hash
        let hash = Hash::from_hex(block_hash)
            .map_err(|_| RpcServerError::InvalidParams("Invalid hash format".to_string()))?;

        // Get header from database
        let blockchain_guard = blockchain_db
            .try_read()
            .map_err(|e| RpcServerError::Internal(format!("Lock error: {}", e)))?;
        match (*blockchain_guard).get_header(&hash) {
            Ok(Some(header)) => Ok(json!({
                "hash": block_hash,
                "confirmations": 1,
                "height": 0, // Would get actual height
                "version": header.version,
                "merkleroot": header.merkle_root.to_hex(),
                "time": header.timestamp,
                "nonce": header.nonce,
                "bits": format!("{:08x}", header.bits),
                "difficulty": 1.0,
                "previousblockhash": header.prev_hash.to_hex()
            })),
            Ok(None) => Err(RpcServerError::InvalidParams("Block not found".to_string())),
            Err(_) => Err(RpcServerError::Internal("Database error".to_string())),
        }
    }

    /// Get transaction output (UTXO)
    fn get_tx_out(
        utxo_db: &Arc<RwLock<UtxoDb>>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        use crate::storage::UTXODatabase;

        let params = params.ok_or(RpcServerError::InvalidParams(
            "Missing parameters".to_string(),
        ))?;

        if let Value::Array(arr) = params {
            let txid = arr
                .first()
                .and_then(|v| v.as_str())
                .ok_or(RpcServerError::InvalidParams("Invalid txid".to_string()))?;

            let vout = arr
                .get(1)
                .and_then(|v| v.as_u64())
                .ok_or(RpcServerError::InvalidParams("Invalid vout".to_string()))?
                as u32;

            // Parse txid
            let hash = Hash::from_hex(txid)
                .map_err(|_| RpcServerError::InvalidParams("Invalid txid format".to_string()))?;

            let outpoint = crate::blockchain::OutPoint { txid: hash, vout };

            // Get UTXO from database
            let utxo_guard = utxo_db
                .try_read()
                .map_err(|e| RpcServerError::Internal(format!("Lock error: {}", e)))?;
            match (*utxo_guard).get_utxo(&outpoint) {
                Ok(Some(utxo)) => Ok(json!({
                    "bestblock": "0000000000000000000000000000000000000000000000000000000000000000",
                    "confirmations": 1,
                    "value": utxo.output.value as f64 / 100_000_000.0, // Convert to BTC
                    "scriptPubKey": {
                        "asm": "", // Would decode script
                        "hex": hex::encode(utxo.output.script_pubkey.to_bytes()),
                        "type": "unknown" // Would determine script type
                    },
                    "coinbase": utxo.is_coinbase
                })),
                Ok(None) => Ok(Value::Null), // UTXO spent or doesn't exist
                Err(_) => Err(RpcServerError::Internal("Database error".to_string())),
            }
        } else {
            Err(RpcServerError::InvalidParams(
                "Expected array parameters".to_string(),
            ))
        }
    }

    /// Get help information
    fn help(params: Option<Value>) -> Result<Value, RpcServerError> {
        if let Some(Value::Array(arr)) = params {
            if let Some(Value::String(command)) = arr.first() {
                // Return help for specific command
                let help_text = match command.as_str() {
                    "getblockchaininfo" => "Returns blockchain information",
                    "getbestblockhash" => "Returns the hash of the best block",
                    "getblock" => "getblock \"blockhash\" - Returns block information",
                    "getblockheader" => {
                        "getblockheader \"blockhash\" - Returns block header information"
                    }
                    "gettxout" => "gettxout \"txid\" vout - Returns transaction output information",
                    "submitblock" => {
                        "submitblock \"hexdata\" - Submit a mined block to the network"
                    }
                    "getblocktemplate" => "getblocktemplate - Get block template for mining",
                    "help" => "help [\"command\"] - Get help for a command",
                    _ => "Unknown command",
                };
                return Ok(json!(help_text));
            }
        }

        // Return list of all commands
        Ok(json!([
            "getblockchaininfo",
            "getbestblockhash",
            "getblock",
            "getblockheader",
            "gettxout",
            "getpeerinfo",
            "getnetworkinfo",
            "submitblock",
            "getblocktemplate",
            "help",
            "uptime"
        ]))
    }

    /// Get uptime
    fn uptime() -> Result<Value, RpcServerError> {
        // In a real implementation, this would track actual uptime
        Ok(json!(0))
    }

    /// Get peer information
    fn get_peer_info() -> Result<Value, RpcServerError> {
        // In a real implementation, this would return actual peer info
        Ok(json!([]))
    }

    /// Get network information
    fn get_network_info() -> Result<Value, RpcServerError> {
        Ok(json!({
            "version": 1000000, // Version number
            "subversion": "/BTPC:0.1.0/",
            "protocolversion": 70015,
            "localservices": "0000000000000001",
            "localservicesnames": ["NETWORK"],
            "localrelay": true,
            "timeoffset": 0,
            "connections": 0,
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

    /// Submit a mined block to the blockchain
    fn submit_block(
        blockchain_db: &Arc<RwLock<BlockchainDb>>,
        utxo_db: &Arc<RwLock<UtxoDb>>,
        network: crate::Network,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        use crate::blockchain::Block;
        use crate::consensus::ConsensusEngine;
        use crate::storage::{BlockchainDatabase, UTXODatabase};
        use crate::Network;

        let params = params.ok_or(RpcServerError::InvalidParams(
            "Missing block data".to_string(),
        ))?;

        // Get block hex string from params
        let block_hex = if let Value::Array(ref arr) = params {
            arr.first()
                .and_then(|v| v.as_str())
                .ok_or(RpcServerError::InvalidParams(
                    "Invalid block data".to_string(),
                ))?
        } else if let Value::String(ref hex) = params {
            hex.as_str()
        } else {
            return Err(RpcServerError::InvalidParams(
                "Expected hex string or array with hex string".to_string(),
            ));
        };

        // Decode hex to bytes
        eprintln!("[SUBMITBLOCK] Received hex length: {}", block_hex.len());
        let block_bytes = hex::decode(block_hex).map_err(|e| {
            eprintln!("[SUBMITBLOCK ERROR] ❌ Hex decode failed: {}", e);
            RpcServerError::InvalidParams(format!("Invalid hex: {}", e))
        })?;
        eprintln!("[SUBMITBLOCK] Decoded {} bytes", block_bytes.len());
        eprintln!(
            "[SUBMITBLOCK] First 200 bytes: {:?}",
            &block_bytes[..block_bytes.len().min(200)]
        );

        // Deserialize block
        let block = Block::deserialize(&block_bytes).map_err(|e| {
            eprintln!("[SUBMITBLOCK ERROR] ❌ Deserialization failed: {}", e);
            RpcServerError::InvalidParams(format!("Invalid block format: {}", e))
        })?;
        eprintln!(
            "[SUBMITBLOCK] ✅ Block deserialized successfully, {} transactions",
            block.transactions.len()
        );

        // Get block hash before validation
        let block_hash = block.hash();

        // Get previous block for validation context
        let (prev_block, current_height) = {
            let blockchain_guard = blockchain_db
                .try_read()
                .map_err(|e| RpcServerError::Internal(format!("Lock error: {}", e)))?;

            // Get the previous block
            let prev = if block.header.prev_hash == crate::crypto::Hash::zero() {
                // Genesis block - no previous block
                None
            } else {
                (*blockchain_guard)
                    .get_block(&block.header.prev_hash)
                    .map_err(|e| {
                        RpcServerError::Internal(format!("Failed to get previous block: {}", e))
                    })?
            };

            // Get current chain height from the previous block
            let height = if let Some(ref prev_block) = prev {
                (*blockchain_guard)
                    .get_block_height(&prev_block.hash())
                    .unwrap_or(0)
            } else {
                0 // Genesis block will be at height 1
            };

            (prev, height)
        };

        // Validate block using ConsensusEngine with correct network type
        let mut consensus = ConsensusEngine::for_network(network);

        // Set current height for validation context
        consensus.set_current_height(current_height);

        // VALIDATE BLOCK - This enforces minimum block time, difficulty, and other consensus rules
        consensus
            .validate_block(&block, prev_block.as_ref())
            .map_err(|e| {
                RpcServerError::InvalidParams(format!("Block validation failed: {}", e))
            })?;

        // Only store if validation passed
        // Store block in database
        {
            let mut blockchain_guard = blockchain_db
                .try_write()
                .map_err(|e| RpcServerError::Internal(format!("Lock error: {}", e)))?;
            (*blockchain_guard)
                .store_block(&block)
                .map_err(|e| RpcServerError::Internal(format!("Failed to store block: {}", e)))?;
        }

        // Get block height after storing
        let height = {
            let blockchain_guard = blockchain_db
                .try_read()
                .map_err(|e| RpcServerError::Internal(format!("Lock error: {}", e)))?;
            (*blockchain_guard)
                .get_block_height(&block_hash)
                .unwrap_or(0)
        };

        // Store UTXOs from coinbase and transactions
        use crate::blockchain::{utxo::UTXO, OutPoint};

        // Store coinbase UTXOs
        if !block.transactions.is_empty() {
            let coinbase_tx = &block.transactions[0];
            let mut utxo_guard = utxo_db
                .try_write()
                .map_err(|e| RpcServerError::Internal(format!("Lock error: {}", e)))?;
            for (vout, output) in coinbase_tx.outputs.iter().enumerate() {
                let outpoint = OutPoint {
                    txid: coinbase_tx.hash(),
                    vout: vout as u32,
                };
                let utxo = UTXO::new(outpoint, output.clone(), height, true);
                (*utxo_guard).store_utxo(&utxo).map_err(|e| {
                    RpcServerError::Internal(format!("Failed to store UTXO: {}", e))
                })?;
            }
        }

        // Return null on success (Bitcoin-compatible)
        Ok(Value::Null)
    }

    /// Send raw transaction
    fn send_raw_transaction(
        blockchain_db: &Arc<RwLock<BlockchainDb>>,
        utxo_db: &Arc<RwLock<UtxoDb>>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        use crate::blockchain::Transaction;
        use crate::consensus::TransactionValidator;

        eprintln!("[SENDRAWTRANSACTION] Called with params: {:?}", params);

        let params = params.ok_or(RpcServerError::InvalidParams(
            "Missing transaction hex".to_string(),
        ))?;

        // Get transaction hex from params
        let tx_hex = if let Value::Array(ref arr) = params {
            eprintln!("[SENDRAWTRANSACTION] Params is array with {} elements", arr.len());
            if arr.is_empty() {
                return Err(RpcServerError::InvalidParams(
                    "Empty array - transaction hex required".to_string(),
                ));
            }
            let first = arr.first().unwrap();
            eprintln!("[SENDRAWTRANSACTION] First element type: {:?}", first);
            first
                .as_str()
                .ok_or(RpcServerError::InvalidParams(
                    "Transaction hex must be a string".to_string(),
                ))?
        } else if let Value::String(ref hex) = params {
            eprintln!("[SENDRAWTRANSACTION] Params is string");
            hex.as_str()
        } else {
            eprintln!("[SENDRAWTRANSACTION] Params is neither array nor string");
            return Err(RpcServerError::InvalidParams(
                "Expected hex string or array with hex string".to_string(),
            ));
        };

        eprintln!("[SENDRAWTRANSACTION] Transaction hex length: {}", tx_hex.len());

        // Decode hex to bytes
        let tx_bytes = hex::decode(tx_hex).map_err(|e| {
            RpcServerError::InvalidParams(format!("Invalid transaction hex: {}", e))
        })?;

        // Deserialize transaction
        let transaction = Transaction::deserialize(&tx_bytes).map_err(|e| {
            RpcServerError::InvalidParams(format!("Invalid transaction format: {}", e))
        })?;

        // Validate transaction
        let tx_validator = TransactionValidator::new();
        tx_validator
            .validate_transaction(&transaction)
            .map_err(|e| RpcServerError::InvalidParams(format!("Transaction validation failed: {}", e)))?;

        // Get transaction hash
        let txid = transaction.hash();

        // In a real implementation, this would add the transaction to the mempool
        // and broadcast to peers. For now, just return the transaction ID
        Ok(json!(txid.to_hex()))
    }

    /// Get raw transaction by txid
    ///
    /// Returns transaction info if found in the blockchain.
    /// Used by transaction monitor to check confirmation status.
    fn get_raw_transaction(
        blockchain_db: &Arc<RwLock<BlockchainDb>>,
        params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        use crate::storage::BlockchainDatabase;

        let params = params.ok_or_else(|| {
            RpcServerError::InvalidParams("Missing transaction ID parameter".to_string())
        })?;

        // Extract txid from params (can be [txid, verbose] or just txid)
        let txid_str = match &params {
            Value::Array(arr) => arr
                .first()
                .and_then(|v| v.as_str())
                .ok_or_else(|| RpcServerError::InvalidParams("Invalid txid in array".to_string()))?,
            Value::String(s) => s.as_str(),
            _ => {
                return Err(RpcServerError::InvalidParams(
                    "Expected txid string or array".to_string(),
                ))
            }
        };

        // Get verbose flag (default true for Bitcoin compatibility)
        let verbose = match &params {
            Value::Array(arr) => arr.get(1).and_then(|v| v.as_bool()).unwrap_or(true),
            _ => true,
        };

        // Search for transaction in blockchain
        let blockchain_guard = blockchain_db
            .try_read()
            .map_err(|e| RpcServerError::Internal(format!("Lock error: {}", e)))?;

        // Get chain tip to start traversal
        let tip_block = match (*blockchain_guard).get_chain_tip() {
            Ok(Some(tip)) => tip,
            _ => {
                return Err(RpcServerError::InvalidParams(format!(
                    "Transaction {} not found - blockchain empty",
                    txid_str
                )))
            }
        };

        let tip_height = (*blockchain_guard)
            .get_block_height(&tip_block.hash())
            .unwrap_or(0);

        // Traverse backwards from chain tip (up to 1000 blocks)
        let mut current_block = Some(tip_block);
        let mut blocks_checked = 0u32;

        while let Some(block) = current_block {
            if blocks_checked >= 1000 {
                break;
            }

            let block_height = (*blockchain_guard)
                .get_block_height(&block.hash())
                .unwrap_or(0);

            // Search transactions in this block
            for tx in &block.transactions {
                let tx_hash = tx.hash();
                let tx_hex = hex::encode(tx_hash.as_bytes());

                // Check if this is the transaction we're looking for
                if tx_hex == txid_str || txid_str.starts_with(&tx_hex[..8.min(tx_hex.len())]) {
                    let confirmations = tip_height.saturating_sub(block_height) + 1;
                    let block_hash = hex::encode(block.hash().as_bytes());

                    if verbose {
                        return Ok(json!({
                            "txid": tx_hex,
                            "hash": tx_hex,
                            "version": tx.version,
                            "size": tx.serialize().len(),
                            "confirmations": confirmations,
                            "blockheight": block_height,
                            "blockhash": block_hash,
                            "blocktime": block.header.timestamp,
                            "time": block.header.timestamp,
                            "hex": hex::encode(tx.serialize())
                        }));
                    } else {
                        return Ok(json!(hex::encode(tx.serialize())));
                    }
                }
            }

            // Move to previous block
            let prev_hash = block.header.prev_hash;
            if prev_hash == crate::crypto::Hash::zero() {
                break; // Reached genesis
            }

            current_block = (*blockchain_guard).get_block(&prev_hash).ok().flatten();
            blocks_checked += 1;
        }

        // Transaction not found
        Err(RpcServerError::InvalidParams(format!(
            "Transaction {} not found in blockchain",
            txid_str
        )))
    }

    /// Get block template for mining
    fn get_block_template(
        blockchain_db: &Arc<RwLock<BlockchainDb>>,
        network: crate::Network,
        _params: Option<Value>,
    ) -> Result<Value, RpcServerError> {
        use crate::{
            blockchain::{BlockHeader, OutPoint, Transaction, TransactionInput, TransactionOutput},
            consensus::DifficultyTarget,
            storage::BlockchainDatabase,
        };

        // Get current chain tip
        let blockchain_guard = blockchain_db
            .try_read()
            .map_err(|e| RpcServerError::Internal(format!("Lock error: {}", e)))?;
        let tip_block = (*blockchain_guard)
            .get_chain_tip()
            .map_err(|e| RpcServerError::Internal(format!("Failed to get chain tip: {}", e)))?;

        let (prev_hash, height, timestamp) = if let Some(ref tip) = tip_block {
            let hash = tip.hash();
            let height = (*blockchain_guard).get_block_height(&hash).unwrap_or(0);
            (hash, height + 1, tip.header.timestamp + 120) // 2 min after previous
        } else {
            (Hash::zero(), 1, 0)
        };

        // Create coinbase transaction template
        let coinbase_tx = Transaction {
            version: 1,
            inputs: vec![TransactionInput {
                previous_output: OutPoint {
                    txid: Hash::zero(),
                    vout: 0xffffffff,
                },
                script_sig: crate::crypto::Script::new(),
                sequence: 0xffffffff,
            }],
            outputs: vec![TransactionOutput {
                value: 5_000_000_000, // 50 BTPC
                script_pubkey: crate::crypto::Script::new(),
            }],
            lock_time: 0,
            fork_id: 0, // Default to mainnet (Issue #6)
        };

        // Calculate merkle root
        let merkle_root = coinbase_tx.hash();

        // Calculate proper difficulty based on blockchain height
        // This should adjust every 2016 blocks
        let difficulty_bits = if height == 0 {
            // Genesis block uses network minimum
            DifficultyTarget::minimum_for_network(network).bits
        } else {
            // For regtest, check if we need difficulty adjustment
            const ADJUSTMENT_INTERVAL: u32 = 2016;

            if height % ADJUSTMENT_INTERVAL == 0 && height > 0 {
                // This is an adjustment block - calculate new difficulty
                use crate::consensus::DifficultyAdjustment;

                // For simplicity on regtest, since we don't have easy access to historical blocks,
                // let's do a simple difficulty increase to demonstrate the adjustment mechanism
                if let Some(tip) = &tip_block {
                    let previous_target = DifficultyTarget::from_bits(tip.header.bits);

                    // For regtest demonstration: simulate that blocks are coming too fast
                    // This will make difficulty harder (smaller target = harder to mine)
                    // In a real implementation, we'd calculate based on actual block times
                    let simulated_actual_timespan = 600; // Pretend blocks came in 10 minutes (too fast!)
                    let target_timespan = DifficultyAdjustment::get_target_timespan(); // 2 weeks worth

                    let new_target = DifficultyAdjustment::adjust_difficulty(
                        &previous_target,
                        simulated_actual_timespan,
                        target_timespan,
                    );

                    eprintln!("⚡ DIFFICULTY ADJUSTMENT at height {}:", height);
                    eprintln!("   Previous difficulty bits: 0x{:08x}", tip.header.bits);
                    eprintln!("   Simulated timespan: {} seconds (blocks came too fast!)", simulated_actual_timespan);
                    eprintln!("   Target timespan: {} seconds", target_timespan);
                    eprintln!("   New difficulty bits: 0x{:08x}", new_target.bits);
                    eprintln!("   Difficulty should be HARDER now (smaller target)");

                    new_target.bits
                } else {
                    // No previous block, use network minimum
                    DifficultyTarget::minimum_for_network(network).bits
                }
            } else {
                // Use the difficulty from the previous block
                if let Some(tip) = &tip_block {
                    tip.header.bits
                } else {
                    DifficultyTarget::minimum_for_network(network).bits
                }
            }
        };

        let target = DifficultyTarget::from_bits(difficulty_bits);

        // Create block header template
        let header = BlockHeader {
            version: 1,
            prev_hash,
            merkle_root,
            timestamp,
            bits: target.bits,
            nonce: 0,
        };

        // Create block template
        let block = crate::blockchain::Block {
            header,
            transactions: vec![coinbase_tx],
        };

        // Serialize block to hex
        let block_hex = hex::encode(block.serialize());

        Ok(json!({
            "version": 1,
            "previousblockhash": hex::encode(prev_hash.as_bytes()),
            "transactions": [],
            "coinbasevalue": 5_000_000_000u64,
            "target": hex::encode(target.target),
            "mintime": timestamp,
            "curtime": timestamp,
            "bits": format!("{:08x}", target.bits),
            "height": height,
            "data": block_hex
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_all_commands() {
        let result = BlockchainRpcHandlers::help(None).unwrap();

        if let Value::Array(commands) = result {
            assert!(!commands.is_empty());
            assert!(commands.contains(&json!("help")));
        } else {
            panic!("Expected array of commands");
        }
    }

    #[test]
    fn test_help_specific_command() {
        let params = json!(["getblock"]);
        let result = BlockchainRpcHandlers::help(Some(params)).unwrap();

        if let Value::String(help_text) = result {
            assert!(help_text.contains("getblock"));
        } else {
            panic!("Expected help string");
        }
    }

    #[test]
    fn test_uptime() {
        let result = BlockchainRpcHandlers::uptime().unwrap();
        assert!(result.is_number());
    }
}
