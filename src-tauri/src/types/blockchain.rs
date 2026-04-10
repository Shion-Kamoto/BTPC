// Blockchain types for block explorer functionality
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub height: u64,
    pub hash: String,
    pub prev_hash: String,
    pub merkle_root: Option<String>,
    pub timestamp: u64,
    pub bits: u32,
    pub nonce: u64,
    pub version: u32,
    pub tx_count: usize,
    pub size: usize,
    pub transactions: Vec<TransactionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    pub hash: String,
    pub block_height: Option<u64>,
    pub inputs: usize,
    pub outputs: usize,
    pub total_value: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainInfo {
    pub height: u64,
    pub total_transactions: u64,
    pub difficulty: f64,
    pub hash_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SearchResult {
    Block(BlockInfo),
    Transaction(TransactionInfo),
}