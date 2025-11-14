//! Blockchain synchronization service
//!
//! This module provides a background service that syncs the desktop app's UTXO set
//! with the blockchain state from the btpc_node via RPC.

use anyhow::{anyhow, Result};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

use btpc_desktop_app::rpc_client::{RpcClient, UTXOInfo};
use btpc_desktop_app::utxo_manager::{UTXOManager, UTXO, Transaction, TxInput, TxOutput};
use chrono::Utc;

/// Blockchain synchronization service configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// RPC endpoint host
    pub rpc_host: String,
    /// RPC endpoint port
    pub rpc_port: u16,
    /// Polling interval in seconds
    pub poll_interval_secs: u64,
    /// Maximum blocks to sync per iteration
    pub max_blocks_per_sync: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            rpc_host: "127.0.0.1".to_string(),
            rpc_port: 8332,
            poll_interval_secs: 10,
            max_blocks_per_sync: 100,
        }
    }
}

/// Synchronization statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[derive(Default)]
pub struct SyncStats {
    pub last_sync_time: Option<chrono::DateTime<Utc>>,
    pub current_height: u64,
    pub node_height: u64,
    pub synced_blocks: u64,
    pub pending_blocks: u64,
    pub is_syncing: bool,
    pub last_error: Option<String>,
}


/// Blockchain synchronization service
pub struct BlockchainSyncService {
    /// RPC client for node communication
    rpc_client: Arc<RpcClient>,
    /// UTXO manager to update
    utxo_manager: Arc<Mutex<UTXOManager>>,
    /// Sync configuration
    config: SyncConfig,
    /// Sync statistics
    stats: Arc<Mutex<SyncStats>>,
    /// Whether the service is running
    running: Arc<Mutex<bool>>,
}

impl BlockchainSyncService {
    /// Create a new blockchain sync service
    pub fn new(
        utxo_manager: Arc<Mutex<UTXOManager>>,
        config: SyncConfig,
    ) -> Self {
        let rpc_client = Arc::new(RpcClient::new(&config.rpc_host, config.rpc_port));

        Self {
            rpc_client,
            utxo_manager,
            config,
            stats: Arc::new(Mutex::new(SyncStats::default())),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the synchronization service (background task)
    pub fn start(&self) -> Result<()> {
        let mut running = self.running.lock().unwrap();
        if *running {
            return Err(anyhow!("Sync service is already running"));
        }
        *running = true;

        let rpc_client = self.rpc_client.clone();
        let utxo_manager = self.utxo_manager.clone();
        let stats = self.stats.clone();
        let running_flag = self.running.clone();
        let poll_interval = self.config.poll_interval_secs;
        let max_blocks = self.config.max_blocks_per_sync;

        // Spawn background sync task
        tokio::spawn(async move {
            println!("🔄 Blockchain sync service started");

            while *running_flag.lock().unwrap() {
                // Perform sync iteration
                match Self::sync_iteration(
                    &rpc_client,
                    &utxo_manager,
                    &stats,
                    max_blocks,
                )
                .await
                {
                    Ok(synced_count) => {
                        if synced_count > 0 {
                            println!("✅ Synced {} blocks", synced_count);
                        }
                    }
                    Err(e) => {
                        // Silently log sync errors to stats (node may not be ready yet)
                        // eprintln!("❌ Sync error: {}", e);
                        let mut stats_guard = stats.lock().unwrap();
                        stats_guard.last_error = Some(e.to_string());
                    }
                }

                // Sleep until next iteration
                sleep(Duration::from_secs(poll_interval)).await;
            }

            println!("🛑 Blockchain sync service stopped");
        });

        Ok(())
    }

    /// Stop the synchronization service
    pub fn stop(&self) {
        let mut running = self.running.lock().unwrap();
        *running = false;
    }

    /// Check if the sync service is running
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }

    /// Get current synchronization statistics
    pub fn get_stats(&self) -> SyncStats {
        self.stats.lock().unwrap().clone()
    }

    /// Perform a single sync iteration
    async fn sync_iteration(
        rpc_client: &RpcClient,
        utxo_manager: &Arc<Mutex<UTXOManager>>,
        stats: &Arc<Mutex<SyncStats>>,
        max_blocks: u64,
    ) -> Result<u64> {
        // Mark as syncing
        {
            let mut stats_guard = stats.lock().unwrap();
            stats_guard.is_syncing = true;
        }

        // Check if node is reachable
        if !rpc_client.ping().await? {
            return Err(anyhow!("Node is not reachable"));
        }

        // Get blockchain info from node
        let blockchain_info = rpc_client.get_blockchain_info().await?;
        let node_height = blockchain_info.blocks;

        // Get current height from local state
        let current_height = {
            let stats_guard = stats.lock().unwrap();
            stats_guard.current_height
        };

        // Calculate how many blocks to sync
        let pending_blocks = node_height.saturating_sub(current_height);

        if pending_blocks == 0 {
            // Already synced
            let mut stats_guard = stats.lock().unwrap();
            stats_guard.node_height = node_height;
            stats_guard.pending_blocks = 0;
            stats_guard.is_syncing = false;
            stats_guard.last_sync_time = Some(Utc::now());
            return Ok(0);
        }

        // Sync up to max_blocks per iteration
        let blocks_to_sync = pending_blocks.min(max_blocks);

        let mut synced_count = 0;
        for height in (current_height + 1)..=(current_height + blocks_to_sync) {
            match Self::sync_block(rpc_client, utxo_manager, height).await {
                Ok(_) => {
                    synced_count += 1;

                    // Update stats
                    let mut stats_guard = stats.lock().unwrap();
                    stats_guard.current_height = height;
                    stats_guard.synced_blocks += 1;
                }
                Err(e) => {
                    eprintln!("Failed to sync block {}: {}", height, e);
                    return Err(e);
                }
            }
        }

        // Update final stats
        {
            let mut stats_guard = stats.lock().unwrap();
            stats_guard.node_height = node_height;
            stats_guard.pending_blocks = node_height - stats_guard.current_height;
            stats_guard.is_syncing = false;
            stats_guard.last_sync_time = Some(Utc::now());
            stats_guard.last_error = None;
        }

        Ok(synced_count)
    }

    /// Sync a single block from the node
    async fn sync_block(
        rpc_client: &RpcClient,
        utxo_manager: &Arc<Mutex<UTXOManager>>,
        height: u64,
    ) -> Result<()> {
        // Get block from node
        let block_info = rpc_client.get_block_by_height(height).await?;

        println!("📦 Processing block {} ({} txs)", height, block_info.tx.len());

        // Process each transaction in the block
        for (tx_index, txid) in block_info.tx.iter().enumerate() {
            // Get transaction details
            let tx_info = rpc_client.get_transaction(txid).await?;

            let is_coinbase = tx_index == 0; // First tx in block is coinbase

            // Process transaction outputs (create new UTXOs)
            for output in &tx_info.vout {
                // Extract address from script_pub_key
                let address = if let Some(addresses) = &output.script_pub_key.addresses {
                    if !addresses.is_empty() {
                        addresses[0].clone()
                    } else {
                        continue; // Skip outputs without addresses
                    }
                } else {
                    continue;
                };

                // Create UTXO
                let utxo = UTXO {
                    txid: txid.clone(),
                    vout: output.n,
                    value_credits: output.value,
                    value_btp: output.value as f64 / 100_000_000.0,
                    address: address.clone(),
                    block_height: height,
                    is_coinbase,
                    created_at: Utc::now(),
                    spent: false,
                    spent_in_tx: None,
                    spent_at_height: None,
                    script_pubkey: hex::decode(&output.script_pub_key.hex)
                        .unwrap_or_default(),
                };

                // Add UTXO to manager
                let mut utxo_manager_guard = utxo_manager.lock().unwrap();
                if let Err(e) = utxo_manager_guard.add_utxo(utxo) {
                    eprintln!("Failed to add UTXO {}:{}: {}", txid, output.n, e);
                }
            }

            // Process transaction inputs (mark UTXOs as spent)
            if !is_coinbase {
                for input in &tx_info.vin {
                    let mut utxo_manager_guard = utxo_manager.lock().unwrap();
                    if let Err(e) = utxo_manager_guard.spend_utxo(
                        &input.txid,
                        input.vout,
                        txid.clone(),
                        height,
                    ) {
                        // It's okay if UTXO is not found (might not belong to our wallet)
                        if !e.to_string().contains("not found") {
                            eprintln!("Failed to mark UTXO as spent: {}", e);
                        }
                    }
                }
            }

            // Add transaction to history
            let transaction = Transaction {
                txid: txid.clone(),
                version: tx_info.version,
                inputs: tx_info
                    .vin
                    .iter()
                    .map(|input| TxInput {
                        prev_txid: input.txid.clone(),
                        prev_vout: input.vout,
                        signature_script: hex::decode(&input.script_sig).unwrap_or_default(),
                        sequence: input.sequence,
                    })
                    .collect(),
                outputs: tx_info
                    .vout
                    .iter()
                    .map(|output| TxOutput {
                        value: output.value,
                        script_pubkey: hex::decode(&output.script_pub_key.hex)
                            .unwrap_or_default(),
                    })
                    .collect(),
                lock_time: tx_info.locktime,
                fork_id: 2, // Regtest network (sync service)
                block_height: Some(height),
                confirmed_at: Some(Utc::now()),
                is_coinbase,
            };

            let mut utxo_manager_guard = utxo_manager.lock().unwrap();
            if let Err(e) = utxo_manager_guard.add_transaction(transaction) {
                eprintln!("Failed to add transaction to history: {}", e);
            }
        }

        println!("✅ Block {} processed successfully", height);
        Ok(())
    }

    /// Manually trigger a sync (useful for on-demand syncing)
    pub async fn sync_now(&self) -> Result<u64> {
        Self::sync_iteration(
            &self.rpc_client,
            &self.utxo_manager,
            &self.stats,
            self.config.max_blocks_per_sync,
        )
        .await
    }

    /// Sync UTXOs for a specific address
    pub async fn sync_address_utxos(&self, address: &str) -> Result<Vec<UTXOInfo>> {
        self.rpc_client.get_utxos_for_address(address).await
    }

    /// Get balance for an address directly from the node
    pub async fn get_address_balance_from_node(&self, address: &str) -> Result<u64> {
        self.rpc_client.get_address_balance(address).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::path::PathBuf;

    #[test]
    fn test_sync_config_default() {
        let config = SyncConfig::default();
        assert_eq!(config.rpc_host, "127.0.0.1");
        assert_eq!(config.rpc_port, 8332);
        assert_eq!(config.poll_interval_secs, 10);
    }

    #[test]
    fn test_sync_stats_default() {
        let stats = SyncStats::default();
        assert_eq!(stats.current_height, 0);
        assert_eq!(stats.node_height, 0);
        assert!(!stats.is_syncing);
    }

    #[tokio::test]
    async fn test_sync_service_creation() {
        let temp_dir = TempDir::new().unwrap();
        let utxo_manager = Arc::new(Mutex::new(
            UTXOManager::new(temp_dir.path().to_path_buf()).unwrap(),
        ));

        let sync_service = BlockchainSyncService::new(utxo_manager, SyncConfig::default());

        assert!(!sync_service.is_running());
        let stats = sync_service.get_stats();
        assert_eq!(stats.current_height, 0);
    }

    #[tokio::test]
    #[ignore] // Only run with a running node
    async fn test_sync_now_integration() {
        let temp_dir = TempDir::new().unwrap();
        let utxo_manager = Arc::new(Mutex::new(
            UTXOManager::new(temp_dir.path().to_path_buf()).unwrap(),
        ));

        let sync_service = BlockchainSyncService::new(utxo_manager, SyncConfig::default());

        let result = sync_service.sync_now().await;
        assert!(result.is_ok());
    }
}