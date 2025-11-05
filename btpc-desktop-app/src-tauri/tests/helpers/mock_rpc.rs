//! Mock RPC client for testing without real btpc_node

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use btpc_core::{Transaction, Block, Hash};
use serde_json::Value;

/// Mock RPC client that simulates btpc_node responses
#[derive(Clone)]
pub struct MockRpcClient {
    /// In-memory mempool (txid -> transaction)
    mempool: Arc<Mutex<HashMap<String, Transaction>>>,

    /// Block height tracker
    block_height: Arc<Mutex<u64>>,

    /// Fee rate (satoshis per byte)
    fee_rate: Arc<Mutex<u64>>,

    /// UTXO set (for get_utxo queries)
    utxos: Arc<Mutex<HashMap<String, Vec<(u32, u64)>>>>, // address -> [(index, amount)]
}

impl MockRpcClient {
    /// Create new mock RPC client with default values
    pub fn new() -> Self {
        Self {
            mempool: Arc::new(Mutex::new(HashMap::new())),
            block_height: Arc::new(Mutex::new(100)), // Start at block 100
            fee_rate: Arc::new(Mutex::new(1000)), // 1000 sat/byte default
            utxos: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Set fee rate for fee estimation tests
    pub fn set_fee_rate(&self, rate: u64) {
        *self.fee_rate.lock().unwrap() = rate;
    }

    /// Get current fee rate (simulates RPC estimatefee)
    pub fn get_fee_rate(&self) -> u64 {
        *self.fee_rate.lock().unwrap()
    }

    /// Add UTXO for an address (for wallet testing)
    pub fn add_utxo(&self, address: &str, index: u32, amount: u64) {
        let mut utxos = self.utxos.lock().unwrap();
        utxos.entry(address.to_string())
            .or_insert_with(Vec::new)
            .push((index, amount));
    }

    /// Get UTXOs for address (simulates RPC getutxo)
    pub fn get_utxos(&self, address: &str) -> Vec<(u32, u64)> {
        self.utxos.lock().unwrap()
            .get(address)
            .cloned()
            .unwrap_or_default()
    }

    /// Broadcast transaction (simulates RPC sendrawtransaction)
    pub fn broadcast_transaction(&self, tx_hex: &str) -> Result<String, String> {
        // Parse transaction ID from hex (simplified)
        let tx_id = format!("mock_tx_{}", tx_hex.len());

        // Add to mempool
        // Note: We're not actually parsing the Transaction here for simplicity
        // Real implementation would deserialize from hex
        self.mempool.lock().unwrap().insert(tx_id.clone(), Transaction::default());

        Ok(tx_id)
    }

    /// Get transaction status (simulates RPC gettransaction)
    pub fn get_transaction_status(&self, tx_id: &str) -> Option<TransactionStatus> {
        let mempool = self.mempool.lock().unwrap();
        if mempool.contains_key(tx_id) {
            Some(TransactionStatus {
                confirmations: 0,
                block_hash: None,
                in_mempool: true,
            })
        } else {
            None
        }
    }

    /// Mine blocks (move mempool txs to confirmed)
    pub fn mine_blocks(&self, count: u32) {
        let mut height = self.block_height.lock().unwrap();
        *height += count as u64;

        // Clear mempool (txs now "confirmed")
        self.mempool.lock().unwrap().clear();
    }

    /// Get current block height
    pub fn get_block_height(&self) -> u64 {
        *self.block_height.lock().unwrap()
    }

    /// Check if transaction is in mempool
    pub fn is_in_mempool(&self, tx_id: &str) -> bool {
        self.mempool.lock().unwrap().contains_key(tx_id)
    }
}

impl Default for MockRpcClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction status from RPC
#[derive(Debug, Clone)]
pub struct TransactionStatus {
    pub confirmations: u32,
    pub block_hash: Option<Hash>,
    pub in_mempool: bool,
}