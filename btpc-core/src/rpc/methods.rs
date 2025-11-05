//! RPC method handlers implementation
//!
//! This module provides the core RPC method handlers that can be used by different
//! RPC server implementations.

use std::sync::Arc;

use serde_json::{json, Value};
use thiserror::Error;

use crate::{
    blockchain::{Block, BlockHeader, Transaction},
    consensus::{BlockValidator, RewardCalculator, RewardParams, TransactionValidator},
    crypto::Hash,
    rpc::types::*,
    storage::{mempool::Mempool, BlockchainDatabase, BlockchainDbError, UTXODatabase, UTXODbError},
    Network,
};

/// RPC method handler errors
#[derive(Error, Debug)]
pub enum RpcMethodError {
    #[error("Method not found: {0}")]
    MethodNotFound(String),
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
}

/// Callback for broadcasting transactions
pub type TxBroadcastCallback = Arc<dyn Fn(Transaction) + Send + Sync>;

/// Core RPC method handlers
pub struct RpcMethods {
    /// Blockchain database
    blockchain_db: Arc<dyn BlockchainDatabase + Send + Sync>,
    /// UTXO database
    utxo_db: Arc<dyn UTXODatabase + Send + Sync>,
    /// Block validator
    block_validator: Arc<BlockValidator>,
    /// Transaction validator
    tx_validator: Arc<TransactionValidator>,
    /// Network configuration
    network: Network,
    /// Mempool for pending transactions
    mempool: Option<Arc<Mempool>>,
    /// Callback for broadcasting transactions to P2P network
    tx_broadcast_callback: Option<TxBroadcastCallback>,
}

impl RpcMethods {
    /// Create new RPC methods handler
    pub fn new(
        blockchain_db: Arc<dyn BlockchainDatabase + Send + Sync>,
        utxo_db: Arc<dyn UTXODatabase + Send + Sync>,
        block_validator: Arc<BlockValidator>,
        tx_validator: Arc<TransactionValidator>,
        network: Network,
    ) -> Self {
        RpcMethods {
            blockchain_db,
            utxo_db,
            block_validator,
            tx_validator,
            network,
            mempool: None,
            tx_broadcast_callback: None,
        }
    }

    /// Set mempool for transaction handling
    pub fn with_mempool(mut self, mempool: Arc<Mempool>) -> Self {
        self.mempool = Some(mempool);
        self
    }

    /// Set transaction broadcast callback
    pub fn with_tx_broadcast(mut self, callback: TxBroadcastCallback) -> Self {
        self.tx_broadcast_callback = Some(callback);
        self
    }

    /// Get blockchain information
    pub async fn get_blockchain_info(
        &self,
        _params: Option<Value>,
    ) -> Result<Value, RpcMethodError> {
        // Get chain tip
        let tip_block = self
            .blockchain_db
            .get_chain_tip()
            .map_err(|e| RpcMethodError::DatabaseError(e.to_string()))?;

        match tip_block {
            Some(block) => {
                let block_hash = block.hash();
                let height = self.get_block_height(&block_hash)?;

                Ok(json!({
                    "chain": self.network_name(),
                    "blocks": height,
                    "headers": height, // For simplicity, assuming headers == blocks
                    "bestblockhash": hex::encode(block_hash.as_bytes()),
                    "difficulty": self.calculate_difficulty(&block)?,
                    "mediantime": block.header.timestamp,
                    "verificationprogress": 1.0,
                    "chainwork": self.calculate_chain_work(height)?,
                    "size_on_disk": self.estimate_chain_size()?,
                    "pruned": false,
                    "warnings": ""
                }))
            }
            None => {
                // No blocks yet (empty chain)
                Ok(json!({
                    "chain": self.network_name(),
                    "blocks": 0,
                    "headers": 0,
                    "bestblockhash": null,
                    "difficulty": 1.0,
                    "mediantime": 0,
                    "verificationprogress": 0.0,
                    "chainwork": "0",
                    "size_on_disk": 0,
                    "pruned": false,
                    "warnings": ""
                }))
            }
        }
    }

    /// Get block by hash or height
    pub async fn get_block(&self, params: Option<Value>) -> Result<Value, RpcMethodError> {
        let params = params
            .ok_or_else(|| RpcMethodError::InvalidParams("Missing parameters".to_string()))?;

        let (block_hash, verbosity) = match params {
            Value::Array(arr) => {
                if arr.is_empty() {
                    return Err(RpcMethodError::InvalidParams(
                        "Missing block hash/height".to_string(),
                    ));
                }

                let hash_or_height = &arr[0];
                let verbosity = arr.get(1).and_then(|v| v.as_u64()).unwrap_or(1) as u32;

                let block_hash = if let Some(height) = hash_or_height.as_u64() {
                    self.get_block_hash_by_height(height as u32)?
                } else if let Some(hash_str) = hash_or_height.as_str() {
                    Hash::from_hex(hash_str).map_err(|_| {
                        RpcMethodError::InvalidParams("Invalid block hash format".to_string())
                    })?
                } else {
                    return Err(RpcMethodError::InvalidParams(
                        "Invalid block hash/height parameter".to_string(),
                    ));
                };

                (block_hash, verbosity)
            }
            _ => {
                return Err(RpcMethodError::InvalidParams(
                    "Invalid parameters format".to_string(),
                ))
            }
        };

        let block = self
            .blockchain_db
            .get_block(&block_hash)
            .map_err(|e| RpcMethodError::DatabaseError(e.to_string()))?
            .ok_or_else(|| RpcMethodError::InternalError("Block not found".to_string()))?;

        match verbosity {
            0 => {
                // Return raw block hex
                let block_bytes = self.serialize_block(&block)?;
                Ok(json!(hex::encode(block_bytes)))
            }
            1 => {
                // Return block info
                Ok(json!(self.block_to_rpc_info(&block)?))
            }
            2 => {
                // Return block info with transactions
                Ok(json!(self.block_to_rpc_info_with_txs(&block)?))
            }
            _ => Err(RpcMethodError::InvalidParams(
                "Invalid verbosity level".to_string(),
            )),
        }
    }

    /// Get block header
    pub async fn get_block_header(&self, params: Option<Value>) -> Result<Value, RpcMethodError> {
        let params = params
            .ok_or_else(|| RpcMethodError::InvalidParams("Missing parameters".to_string()))?;

        let block_hash = self.parse_block_hash_param(&params)?;
        let header = self
            .blockchain_db
            .get_header(&block_hash)
            .map_err(|e| RpcMethodError::DatabaseError(e.to_string()))?
            .ok_or_else(|| RpcMethodError::InternalError("Block header not found".to_string()))?;

        Ok(json!(self.header_to_rpc_info(&header)?))
    }

    /// Get transaction by hash
    pub async fn get_raw_transaction(
        &self,
        params: Option<Value>,
    ) -> Result<Value, RpcMethodError> {
        let params = params
            .ok_or_else(|| RpcMethodError::InvalidParams("Missing parameters".to_string()))?;

        let (tx_hash, verbose) = match params {
            Value::Array(arr) => {
                if arr.is_empty() {
                    return Err(RpcMethodError::InvalidParams(
                        "Missing transaction hash".to_string(),
                    ));
                }

                let tx_hash_str = arr[0].as_str().ok_or_else(|| {
                    RpcMethodError::InvalidParams("Invalid transaction hash".to_string())
                })?;
                let tx_hash = Hash::from_hex(tx_hash_str).map_err(|_| {
                    RpcMethodError::InvalidParams("Invalid transaction hash format".to_string())
                })?;

                let verbose = arr.get(1).and_then(|v| v.as_bool()).unwrap_or(false);

                (tx_hash, verbose)
            }
            _ => {
                return Err(RpcMethodError::InvalidParams(
                    "Invalid parameters format".to_string(),
                ))
            }
        };

        let transaction = self.get_transaction_by_hash(&tx_hash)?;

        if verbose {
            Ok(json!(self.transaction_to_rpc_info(&transaction)?))
        } else {
            let tx_bytes = self.serialize_transaction(&transaction)?;
            Ok(json!(hex::encode(tx_bytes)))
        }
    }

    /// Send raw transaction
    pub async fn send_raw_transaction(
        &self,
        params: Option<Value>,
    ) -> Result<Value, RpcMethodError> {
        let params = params
            .ok_or_else(|| RpcMethodError::InvalidParams("Missing parameters".to_string()))?;

        let tx_hex = params
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcMethodError::InvalidParams("Missing transaction hex".to_string()))?;

        let tx_bytes = hex::decode(tx_hex)
            .map_err(|_| RpcMethodError::InvalidParams("Invalid transaction hex".to_string()))?;

        let transaction = self.deserialize_transaction(&tx_bytes)?;

        // Validate transaction
        self.tx_validator
            .validate_transaction(&transaction)
            .map_err(|e| RpcMethodError::ValidationError(e.to_string()))?;

        let tx_hash = transaction.hash();

        // Add to mempool if available
        if let Some(mempool) = &self.mempool {
            mempool.add_transaction(transaction.clone())
                .map_err(|e| RpcMethodError::InternalError(format!("Mempool error: {}", e)))?;

            println!("✅ Transaction {} added to mempool", tx_hash.to_hex());
        }

        // Broadcast to P2P network if callback is set
        if let Some(broadcast_fn) = &self.tx_broadcast_callback {
            broadcast_fn(transaction);
            println!("📡 Broadcasting transaction {} to peers", tx_hash.to_hex());
        }

        Ok(json!(hex::encode(tx_hash.as_bytes())))
    }

    /// Get recent transactions from the blockchain
    ///
    /// Parameters:
    /// - limit: Maximum number of transactions to return (default: 10, max: 100)
    /// - offset: Number of transactions to skip (default: 0)
    pub async fn get_recent_transactions(
        &self,
        params: Option<Value>,
    ) -> Result<Value, RpcMethodError> {
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
                return Err(RpcMethodError::InvalidParams(
                    "Invalid parameters format".to_string(),
                ))
            }
        };

        // Get chain tip to start iteration
        let tip_block = self
            .blockchain_db
            .get_chain_tip()
            .map_err(|e| RpcMethodError::DatabaseError(e.to_string()))?;

        let mut transactions = Vec::new();
        let mut skipped = 0;
        let mut current_hash = tip_block.as_ref().map(|b| b.hash());

        // Iterate backwards through the blockchain
        while transactions.len() < limit && current_hash.is_some() {
            let Some(hash) = current_hash else {
                break; // Redundant but explicit
            };
            let block = self
                .blockchain_db
                .get_block(&hash)
                .map_err(|e| RpcMethodError::DatabaseError(e.to_string()))?;

            if let Some(block) = block {
                let block_height = self.get_block_height(&hash)?;

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
                        "block_height": block_height,
                        "block_hash": hex::encode(hash.as_bytes()),
                        "timestamp": block.header.timestamp,
                        "inputs": tx.inputs.len(),
                        "outputs": tx.outputs.len(),
                        "total_value": tx.outputs.iter().map(|o| o.value).sum::<u64>(),
                    }));
                }

                // Move to previous block
                current_hash = Some(block.header.prev_hash);
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

    /// Get network information
    pub async fn get_network_info(&self, _params: Option<Value>) -> Result<Value, RpcMethodError> {
        Ok(json!({
            "version": env!("CARGO_PKG_VERSION"),
            "subversion": format!("/BTPC:{}/", env!("CARGO_PKG_VERSION")),
            "protocolversion": 70015,
            "localservices": "0000000000000409",
            "localrelay": true,
            "timeoffset": 0,
            "networkactive": true,
            "connections": 0, // Would be actual peer count in real implementation
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

    /// Get mining information
    pub async fn get_mining_info(&self, _params: Option<Value>) -> Result<Value, RpcMethodError> {
        let tip_block = self
            .blockchain_db
            .get_chain_tip()
            .map_err(|e| RpcMethodError::DatabaseError(e.to_string()))?;

        match tip_block {
            Some(block) => {
                let height = self.get_block_height(&block.hash())?;
                let difficulty = self.calculate_difficulty(&block)?;

                Ok(json!({
                    "blocks": height,
                    "currentblockweight": block.size(),
                    "currentblocktx": block.transactions.len(),
                    "difficulty": difficulty,
                    "networkhashps": self.estimate_network_hashrate()?,
                    "pooledtx": 0, // Would be mempool size in real implementation
                    "chain": self.network_name(),
                    "warnings": ""
                }))
            }
            None => Ok(json!({
                "blocks": 0,
                "currentblockweight": 0,
                "currentblocktx": 0,
                "difficulty": 1.0,
                "networkhashps": 0.0,
                "pooledtx": 0,
                "chain": self.network_name(),
                "warnings": ""
            })),
        }
    }

    /// Get reward information
    pub async fn get_reward_info(&self, params: Option<Value>) -> Result<Value, RpcMethodError> {
        let height = if let Some(params) = params {
            params
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_u64())
                .unwrap_or_else(|| {
                    // Get current height
                    self.blockchain_db
                        .get_chain_tip()
                        .ok()
                        .flatten()
                        .and_then(|block| self.get_block_height(&block.hash()).ok())
                        .unwrap_or(0) as u64
                })
        } else {
            // Get current height
            self.blockchain_db
                .get_chain_tip()
                .ok()
                .flatten()
                .and_then(|block| self.get_block_height(&block.hash()).ok())
                .unwrap_or(0) as u64
        };

        let params = match self.network {
            Network::Mainnet => RewardParams::mainnet(),
            Network::Testnet => RewardParams::testnet(),
            Network::Regtest => RewardParams::regtest(),
        };

        let reward = RewardCalculator::calculate_block_reward(height as u32)
            .map_err(|e| RpcMethodError::InternalError(e.to_string()))?;
        let total_supply = RewardCalculator::calculate_total_supply(height as u32, &params)
            .map_err(|e| RpcMethodError::InternalError(e.to_string()))?;
        let inflation_rate = RewardCalculator::calculate_inflation_rate(height as u32, &params)
            .map_err(|e| RpcMethodError::InternalError(e.to_string()))?;

        Ok(json!({
            "height": height,
            "reward": reward as f64 / 100_000_000.0, // Convert to BTPC
            "total_supply": total_supply as f64 / 100_000_000.0,
            "inflation_rate": inflation_rate,
            "is_tail_emission": height >= RewardCalculator::tail_emission_start_height(&params) as u64,
            "decay_complete": height >= 24 * 52560 // 24 years
        }))
    }

    /// Validate a block
    pub async fn validate_block(&self, params: Option<Value>) -> Result<Value, RpcMethodError> {
        let params = params
            .ok_or_else(|| RpcMethodError::InvalidParams("Missing parameters".to_string()))?;

        let block_hex = params
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcMethodError::InvalidParams("Missing block hex".to_string()))?;

        let block_bytes = hex::decode(block_hex)
            .map_err(|_| RpcMethodError::InvalidParams("Invalid block hex".to_string()))?;

        let block = self.deserialize_block(&block_bytes)?;

        // Validate block
        match self.block_validator.validate_block(&block) {
            Ok(_) => Ok(json!({
                "valid": true,
                "errors": []
            })),
            Err(e) => Ok(json!({
                "valid": false,
                "errors": [e.to_string()]
            })),
        }
    }

    // Helper methods

    fn network_name(&self) -> &'static str {
        match self.network {
            Network::Mainnet => "main",
            Network::Testnet => "test",
            Network::Regtest => "regtest",
        }
    }

    fn parse_block_hash_param(&self, params: &Value) -> Result<Hash, RpcMethodError> {
        let hash_str = params
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcMethodError::InvalidParams("Missing block hash".to_string()))?;

        Hash::from_hex(hash_str)
            .map_err(|_| RpcMethodError::InvalidParams("Invalid block hash format".to_string()))
    }

    fn get_block_height(&self, block_hash: &Hash) -> Result<u32, RpcMethodError> {
        // Query the database for block height using the trait method
        self.blockchain_db.get_block_height(block_hash)
            .map_err(|e| RpcMethodError::DatabaseError(e.to_string()))
    }

    fn get_block_hash_by_height(&self, _height: u32) -> Result<Hash, RpcMethodError> {
        // In a real implementation, this would query the database for block hash by height
        // For now, return a placeholder
        Err(RpcMethodError::InternalError(
            "Height-based lookup not implemented".to_string(),
        ))
    }

    fn calculate_difficulty(&self, _block: &Block) -> Result<f64, RpcMethodError> {
        // In a real implementation, this would calculate actual difficulty
        Ok(1.0)
    }

    fn calculate_chain_work(&self, _height: u32) -> Result<String, RpcMethodError> {
        // In a real implementation, this would calculate cumulative work
        Ok("0".to_string())
    }

    fn estimate_chain_size(&self) -> Result<u64, RpcMethodError> {
        // In a real implementation, this would estimate actual chain size
        Ok(0)
    }

    fn estimate_network_hashrate(&self) -> Result<f64, RpcMethodError> {
        // In a real implementation, this would estimate network hashrate
        Ok(0.0)
    }

    fn get_transaction_by_hash(&self, _tx_hash: &Hash) -> Result<Transaction, RpcMethodError> {
        // In a real implementation, this would query the database for transaction
        Err(RpcMethodError::InternalError(
            "Transaction lookup not implemented".to_string(),
        ))
    }

    fn serialize_block(&self, _block: &Block) -> Result<Vec<u8>, RpcMethodError> {
        // In a real implementation, this would serialize the block
        Ok(vec![])
    }

    fn serialize_transaction(&self, _transaction: &Transaction) -> Result<Vec<u8>, RpcMethodError> {
        // In a real implementation, this would serialize the transaction
        Ok(vec![])
    }

    fn deserialize_block(&self, _bytes: &[u8]) -> Result<Block, RpcMethodError> {
        // In a real implementation, this would deserialize the block
        Err(RpcMethodError::InternalError(
            "Block deserialization not implemented".to_string(),
        ))
    }

    fn deserialize_transaction(&self, _bytes: &[u8]) -> Result<Transaction, RpcMethodError> {
        // In a real implementation, this would deserialize the transaction
        Err(RpcMethodError::InternalError(
            "Transaction deserialization not implemented".to_string(),
        ))
    }

    fn block_to_rpc_info(&self, block: &Block) -> Result<RpcBlockInfo, RpcMethodError> {
        let block_hash = block.hash();
        let height = self.blockchain_db.get_block_height(&block_hash)
            .unwrap_or(0);

        Ok(RpcBlockInfo {
            hash: hex::encode(block_hash.as_bytes()),
            confirmations: 1, // Placeholder
            size: block.size() as u32,
            height,
            version: block.header.version,
            merkle_root: hex::encode(block.header.merkle_root.as_bytes()),
            tx: block
                .transactions
                .iter()
                .map(|tx| hex::encode(tx.hash().as_bytes()))
                .collect(),
            time: block.header.timestamp as u32,
            nonce: block.header.nonce,
            bits: format!("{:08x}", block.header.bits),
            difficulty: 1.0, // Placeholder
            previous_block_hash: hex::encode(block.header.prev_hash.as_bytes()),
        })
    }

    fn block_to_rpc_info_with_txs(&self, block: &Block) -> Result<Value, RpcMethodError> {
        let mut block_info = json!(self.block_to_rpc_info(block)?);

        // Replace tx hashes with full transaction objects
        let tx_objects: Result<Vec<_>, _> = block
            .transactions
            .iter()
            .map(|tx| self.transaction_to_rpc_info(tx))
            .collect();

        block_info["tx"] = json!(tx_objects?);
        Ok(block_info)
    }

    fn header_to_rpc_info(&self, header: &BlockHeader) -> Result<RpcBlockHeader, RpcMethodError> {
        Ok(RpcBlockHeader {
            hash: hex::encode(header.hash().as_bytes()),
            confirmations: 1, // Placeholder
            height: 0,        // Placeholder
            version: header.version,
            merkle_root: hex::encode(header.merkle_root.as_bytes()),
            time: header.timestamp as u32,
            nonce: header.nonce,
            bits: format!("{:08x}", header.bits),
            difficulty: 1.0, // Placeholder
            previous_block_hash: hex::encode(header.prev_hash.as_bytes()),
        })
    }

    fn transaction_to_rpc_info(
        &self,
        transaction: &Transaction,
    ) -> Result<RpcTransactionInfo, RpcMethodError> {
        Ok(RpcTransactionInfo {
            txid: hex::encode(transaction.hash().as_bytes()),
            hash: hex::encode(transaction.hash().as_bytes()),
            version: transaction.version,
            size: 0,   // Placeholder
            vsize: 0,  // Placeholder
            weight: 0, // Placeholder
            lock_time: transaction.lock_time,
            vin: transaction
                .inputs
                .iter()
                .map(|input| RpcTxInput {
                    txid: hex::encode(input.previous_output.txid.as_bytes()),
                    vout: input.previous_output.vout,
                    script_sig: RpcScriptSig {
                        asm: "".to_string(), // Placeholder
                        hex: hex::encode(input.script_sig.to_bytes()),
                    },
                    sequence: input.sequence,
                })
                .collect(),
            vout: transaction
                .outputs
                .iter()
                .enumerate()
                .map(|(n, output)| RpcTxOutput {
                    value: output.value as f64 / 100_000_000.0,
                    n: n as u32,
                    script_pub_key: RpcScriptPubKey {
                        asm: "".to_string(), // Placeholder
                        hex: hex::encode(output.script_pubkey.to_bytes()),
                        script_type: "unknown".to_string(),
                        addresses: None,
                    },
                })
                .collect(),
            hex: "".to_string(), // Placeholder
            block_hash: None,
            confirmations: None,
            time: None,
            block_time: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::storage::{BlockchainDatabase, UTXODatabase};

    // Mock implementations for testing
    struct MockBlockchainDb;
    impl BlockchainDatabase for MockBlockchainDb {
        fn store_block(&mut self, _block: &Block) -> Result<(), BlockchainDbError> {
            Ok(())
        }

        fn get_block(&self, _hash: &Hash) -> Result<Option<Block>, BlockchainDbError> {
            Ok(None)
        }

        fn get_header(&self, _hash: &Hash) -> Result<Option<BlockHeader>, BlockchainDbError> {
            Ok(None)
        }

        fn get_chain_tip(&self) -> Result<Option<Block>, BlockchainDbError> {
            Ok(None)
        }

        fn get_block_height(&self, _hash: &Hash) -> Result<u32, BlockchainDbError> {
            Ok(0)
        }

        fn has_transaction(&self, _txid: &Hash) -> Result<bool, BlockchainDbError> {
            Ok(false)
        }

        fn store_transaction(&mut self, _txid: &Hash, _block_hash: &Hash) -> Result<(), BlockchainDbError> {
            Ok(())
        }
    }

    struct MockUtxoDb;
    impl UTXODatabase for MockUtxoDb {
        fn store_utxo(&mut self, _utxo: &crate::blockchain::UTXO) -> Result<(), UTXODbError> {
            Ok(())
        }

        fn get_utxo(
            &self,
            _outpoint: &crate::blockchain::OutPoint,
        ) -> Result<Option<crate::blockchain::UTXO>, UTXODbError> {
            Ok(None)
        }

        fn remove_utxo(
            &mut self,
            _outpoint: &crate::blockchain::OutPoint,
        ) -> Result<(), UTXODbError> {
            Ok(())
        }

        fn apply_utxo_batch(
            &mut self,
            _to_remove: &[&crate::blockchain::OutPoint],
            _to_add: &[&crate::blockchain::UTXO],
        ) -> Result<(), UTXODbError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_get_blockchain_info_empty_chain() {
        let blockchain_db = Arc::new(MockBlockchainDb) as Arc<dyn BlockchainDatabase + Send + Sync>;
        let utxo_db = Arc::new(MockUtxoDb) as Arc<dyn UTXODatabase + Send + Sync>;
        let block_validator = Arc::new(BlockValidator::new());
        let tx_validator = Arc::new(TransactionValidator::new());

        let methods = RpcMethods::new(
            blockchain_db,
            utxo_db,
            block_validator,
            tx_validator,
            Network::Regtest,
        );

        let result = methods.get_blockchain_info(None).await.unwrap();

        assert_eq!(result["chain"], "regtest");
        assert_eq!(result["blocks"], 0);
    }

    #[tokio::test]
    async fn test_get_network_info() {
        let blockchain_db = Arc::new(MockBlockchainDb) as Arc<dyn BlockchainDatabase + Send + Sync>;
        let utxo_db = Arc::new(MockUtxoDb) as Arc<dyn UTXODatabase + Send + Sync>;
        let block_validator = Arc::new(BlockValidator::new());
        let tx_validator = Arc::new(TransactionValidator::new());

        let methods = RpcMethods::new(
            blockchain_db,
            utxo_db,
            block_validator,
            tx_validator,
            Network::Mainnet,
        );

        let result = methods.get_network_info(None).await.unwrap();

        assert!(result["version"].is_string());
        assert_eq!(result["networkactive"], true);
    }

    #[tokio::test]
    async fn test_get_reward_info() {
        let blockchain_db = Arc::new(MockBlockchainDb) as Arc<dyn BlockchainDatabase + Send + Sync>;
        let utxo_db = Arc::new(MockUtxoDb) as Arc<dyn UTXODatabase + Send + Sync>;
        let block_validator = Arc::new(BlockValidator::new());
        let tx_validator = Arc::new(TransactionValidator::new());

        let methods = RpcMethods::new(
            blockchain_db,
            utxo_db,
            block_validator,
            tx_validator,
            Network::Mainnet,
        );

        let result = methods.get_reward_info(Some(json!([100]))).await.unwrap();

        assert_eq!(result["height"], 100);
        assert!(result["reward"].is_number());
        assert!(result["total_supply"].is_number());
    }
}
