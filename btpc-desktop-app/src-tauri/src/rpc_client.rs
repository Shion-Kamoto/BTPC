//! JSON-RPC client for communicating with btpc_node
//!
//! This module provides a client to interact with the BTPC blockchain node
//! via JSON-RPC 2.0 protocol to query blockchain state, UTXOs, and transactions.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Block template for mining
#[derive(Debug, Clone, Deserialize)]
pub struct BlockTemplate {
    pub version: u32,
    pub previousblockhash: String,
    pub transactions: Vec<serde_json::Value>,
    pub coinbasevalue: u64,
    pub target: String,
    pub mintime: u64,
    pub curtime: u64,
    pub bits: String,
    pub height: u64,
}

/// Trait for RPC client interface (for generic mining pool usage)
pub trait RpcClientInterface {
    fn get_block_template(&self) -> impl std::future::Future<Output = Result<BlockTemplate>> + Send;
    fn submit_block(&self, block_hex: &str) -> impl std::future::Future<Output = Result<String>> + Send;
}

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: u64,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: Option<u64>,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Blockchain information from RPC
#[derive(Debug, Clone, Deserialize)]
pub struct BlockchainInfo {
    pub chain: Option<String>,
    pub blocks: u64,
    pub headers: Option<u64>,
    #[serde(alias = "bestblockhash")] // ✅ Accept RPC response field name
    pub best_block_hash: Option<String>,
    pub difficulty: f64,
    pub verification_progress: Option<f64>,
}

/// Block information from RPC
#[derive(Debug, Clone, Deserialize)]
pub struct BlockInfo {
    pub hash: String,
    pub height: u64,
    pub version: u32,
    pub merkle_root: String,
    pub time: u64,
    pub nonce: u64,
    pub bits: u32,
    pub difficulty: f64,
    pub tx: Vec<String>, // Transaction IDs
    pub previous_block_hash: Option<String>,
}

/// Transaction information from RPC
#[derive(Debug, Clone, Deserialize)]
pub struct TransactionInfo {
    pub txid: String,
    pub version: u32,
    pub locktime: u32,
    pub vin: Vec<TransactionInput>,
    pub vout: Vec<TransactionOutput>,
    pub size: usize,
    pub weight: usize,
    pub fee: Option<u64>,
    pub confirmations: Option<u64>,  // Number of confirmations
    pub blockhash: Option<String>,   // Block hash if confirmed
    pub blockheight: Option<u64>,    // Block height if confirmed
}

/// Transaction input
#[derive(Debug, Clone, Deserialize)]
pub struct TransactionInput {
    pub txid: String,
    pub vout: u32,
    pub script_sig: String,
    pub sequence: u32,
}

/// Transaction output
#[derive(Debug, Clone, Deserialize)]
pub struct TransactionOutput {
    pub value: u64,
    pub n: u32,
    pub script_pub_key: ScriptPubKey,
}

/// Script public key information
#[derive(Debug, Clone, Deserialize)]
pub struct ScriptPubKey {
    pub asm: String,
    pub hex: String,
    pub req_sigs: Option<u32>,
    pub r#type: String,
    pub addresses: Option<Vec<String>>,
}

/// UTXO information from RPC
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UTXOInfo {
    pub txid: String,
    pub vout: u32,
    pub value: u64,
    pub height: u64,
    pub is_coinbase: bool,
    pub address: Option<String>,
}

/// Network information from RPC
#[derive(Debug, Clone, Deserialize)]
pub struct NetworkInfo {
    pub version: String,
    pub subversion: String,
    pub protocol_version: u32,
    pub connections: u32,
    pub networks: Vec<String>,
}

/// RPC client for btpc_node
pub struct RpcClient {
    /// Node RPC endpoint URL
    endpoint: String,
    /// HTTP client
    client: reqwest::Client,
    /// Request ID counter
    next_id: std::sync::atomic::AtomicU64,
}

impl RpcClient {
    /// Create a new RPC client
    pub fn new(host: &str, port: u16) -> Self {
        let endpoint = format!("http://{}:{}", host, port);
        Self {
            endpoint,
            client: reqwest::Client::new(),
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Create RPC client with default settings (localhost:18360 regtest)
    /// Note: Use explicit new() for production with proper network detection
    pub fn default() -> Self {
        Self::new("127.0.0.1", 18360)  // Regtest default for desktop app development
    }

    /// Get next request ID
    fn next_request_id(&self) -> u64 {
        self.next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Make a JSON-RPC call
    async fn call(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: self.next_request_id(),
        };

        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "HTTP error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let rpc_response: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse JSON-RPC response: {}", e))?;

        if let Some(error) = rpc_response.error {
            return Err(anyhow!(
                "RPC error {}: {}",
                error.code,
                error.message
            ));
        }

        rpc_response
            .result
            .ok_or_else(|| anyhow!("No result in RPC response"))
    }

    /// Check if the node is reachable
    pub async fn ping(&self) -> Result<bool> {
        match self.get_blockchain_info().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get blockchain information
    pub async fn get_blockchain_info(&self) -> Result<BlockchainInfo> {
        let result = self.call("getblockchaininfo", None).await?;
        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse blockchain info: {}", e))
    }

    /// Get block by hash
    pub async fn get_block(&self, block_hash: &str) -> Result<BlockInfo> {
        let params = json!([block_hash, true]); // true = verbose
        let result = self.call("getblock", Some(params)).await?;
        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse block info: {}", e))
    }

    /// Get block hash by height
    pub async fn get_block_hash(&self, height: u64) -> Result<String> {
        let params = json!([height]);
        let result = self.call("getblockhash", Some(params)).await?;
        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse block hash: {}", e))
    }

    /// Get block by height
    pub async fn get_block_by_height(&self, height: u64) -> Result<BlockInfo> {
        let block_hash = self.get_block_hash(height).await?;
        self.get_block(&block_hash).await
    }

    /// Get raw transaction
    pub async fn get_raw_transaction(&self, txid: &str, verbose: bool) -> Result<Value> {
        let params = json!([txid, verbose]);
        self.call("getrawtransaction", Some(params)).await
    }

    /// Get transaction information
    pub async fn get_transaction(&self, txid: &str) -> Result<TransactionInfo> {
        let result = self.get_raw_transaction(txid, true).await?;
        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse transaction info: {}", e))
    }

    /// Send raw transaction
    pub async fn send_raw_transaction(&self, hex: &str) -> Result<String> {
        let params = json!([hex]);
        let result = self.call("sendrawtransaction", Some(params)).await?;
        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse transaction ID: {}", e))
    }

    /// Get UTXOs for an address
    pub async fn get_utxos_for_address(&self, address: &str) -> Result<Vec<UTXOInfo>> {
        let params = json!([address]);
        let result = self.call("getutxosforaddress", Some(params)).await?;
        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse UTXO list: {}", e))
    }

    /// Get UTXO by outpoint (txid:vout)
    pub async fn get_utxo(&self, txid: &str, vout: u32) -> Result<Option<UTXOInfo>> {
        let params = json!([txid, vout]);
        let result = self.call("gettxout", Some(params)).await?;

        if result.is_null() {
            return Ok(None);
        }

        let utxo: UTXOInfo = serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse UTXO info: {}", e))?;
        Ok(Some(utxo))
    }

    /// Get balance for an address (sum of UTXOs)
    pub async fn get_address_balance(&self, address: &str) -> Result<u64> {
        let utxos = self.get_utxos_for_address(address).await?;
        let balance = utxos.iter().map(|utxo| utxo.value).sum();
        Ok(balance)
    }

    /// Get network information
    pub async fn get_network_info(&self) -> Result<NetworkInfo> {
        let result = self.call("getnetworkinfo", None).await?;
        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse network info: {}", e))
    }

    /// Get connection count from network info
    pub async fn get_connection_count(&self) -> Result<u32> {
        let network_info = self.get_network_info().await?;
        Ok(network_info.connections)
    }

    /// Get block count (current height)
    pub async fn get_block_count(&self) -> Result<u64> {
        let result = self.call("getblockcount", None).await?;
        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse block count: {}", e))
    }

    /// Get difficulty
    pub async fn get_difficulty(&self) -> Result<f64> {
        let result = self.call("getdifficulty", None).await?;
        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse difficulty: {}", e))
    }

    /// Get mining info
    pub async fn get_mining_info(&self) -> Result<Value> {
        self.call("getmininginfo", None).await
    }

    /// Validate a block
    pub async fn validate_block(&self, block_hash: &str) -> Result<bool> {
        let params = json!([block_hash]);
        let result = self.call("validateblock", Some(params)).await?;
        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse validation result: {}", e))
    }

    /// Get mempool information
    pub async fn get_mempool_info(&self) -> Result<Value> {
        self.call("getmempoolinfo", None).await
    }

    /// Get raw mempool (list of transaction IDs)
    pub async fn get_raw_mempool(&self) -> Result<Vec<String>> {
        let result = self.call("getrawmempool", None).await?;
        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse mempool: {}", e))
    }

    /// Get recent transactions
    pub async fn get_recent_transactions(&self, limit: usize, offset: usize) -> Result<Vec<RecentTransaction>> {
        let params = json!([limit, offset]);
        let result = self.call("getrecenttransactions", Some(params)).await?;

        // Parse the transactions array from the result
        let txs_value = result.get("transactions")
            .ok_or_else(|| anyhow!("No transactions field in response"))?;

        serde_json::from_value(txs_value.clone())
            .map_err(|e| anyhow!("Failed to parse transactions: {}", e))
    }

    /// Get block template for mining
    ///
    /// Returns a block template with header, coinbase transaction, and mining target.
    /// Used by GPU/CPU miners to construct candidate blocks.
    pub async fn get_block_template(&self) -> Result<BlockTemplate> {
        let result = self.call("getblocktemplate", Some(json!([]))).await?;
        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse block template: {}", e))
    }

    /// Submit mined block to network
    ///
    /// # Arguments
    /// * `block_hex` - Hex-encoded serialized block with valid nonce
    ///
    /// Returns block hash if accepted, error if validation fails
    pub async fn submit_block(&self, block_hex: &str) -> Result<String> {
        let result = self.call("submitblock", Some(json!([block_hex]))).await?;

        // submitblock returns null on success, or error message on failure
        if result.is_null() {
            Ok("Block submitted successfully".to_string())
        } else {
            Err(anyhow!("Block submission failed: {}", result))
        }
    }

    /// Estimate smart fee for transaction
    ///
    /// # Arguments
    /// * `conf_target` - Confirmation target in blocks (e.g., 6 for ~1 hour)
    ///
    /// Returns estimated fee rate in satoshis per byte
    pub async fn estimate_smart_fee(&self, conf_target: u64) -> Result<f64> {
        let result = self.call("estimatesmartfee", Some(json!([conf_target]))).await?;

        // Extract feerate from response
        if let Some(feerate) = result.get("feerate") {
            feerate.as_f64()
                .ok_or_else(|| anyhow!("Invalid feerate in response"))
        } else {
            // Fallback to conservative default if RPC doesn't support estimatesmartfee
            Ok(0.00001) // 1000 satoshis per byte default
        }
    }
}

// Implement the trait for RpcClient
impl RpcClientInterface for RpcClient {
    fn get_block_template(&self) -> impl std::future::Future<Output = Result<BlockTemplate>> + Send {
        async move { self.get_block_template().await }
    }

    fn submit_block(&self, block_hex: &str) -> impl std::future::Future<Output = Result<String>> + Send {
        let block_hex = block_hex.to_string();
        async move { self.submit_block(&block_hex).await }
    }
}

/// Recent transaction info
#[derive(Debug, Clone, Deserialize)]
pub struct RecentTransaction {
    pub hash: String,
    pub block_height: Option<u64>,
    pub block_hash: Option<String>,
    pub timestamp: u64,
    pub inputs: usize,
    pub outputs: usize,
    pub total_value: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rpc_client_creation() {
        let client = RpcClient::new("127.0.0.1", 8334);
        assert_eq!(client.endpoint, "http://127.0.0.1:8334");
    }

    #[tokio::test]
    async fn test_default_client() {
        let client = RpcClient::default();
        assert_eq!(client.endpoint, "http://127.0.0.1:18360");  // Updated for regtest default
    }

    #[tokio::test]
    async fn test_request_id_increment() {
        let client = RpcClient::default();
        let id1 = client.next_request_id();
        let id2 = client.next_request_id();
        let id3 = client.next_request_id();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    // Integration tests require a running btpc_node
    #[tokio::test]
    #[ignore] // Only run when node is available
    async fn test_get_blockchain_info_integration() {
        let client = RpcClient::default();
        let result = client.get_blockchain_info().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Only run when node is available
    async fn test_ping_integration() {
        let client = RpcClient::default();
        let reachable = client.ping().await.unwrap();
        assert!(reachable);
    }
}