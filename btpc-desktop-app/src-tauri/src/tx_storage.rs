// RocksDB-based transaction and UTXO storage for high-performance queries
// Supports pagination, indexing, and efficient lookups
//
// Constitution Article V: Structured logging for all database operations

use anyhow::{anyhow, Result};
use rocksdb::{DB, BoundColumnFamily, IteratorMode, Options, WriteBatch};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use tracing::{info, warn, debug, instrument};

// Re-export structures from utxo_manager
use btpc_desktop_app::utxo_manager::{UTXO, Transaction, TxInput};

/// Column family names
const CF_TRANSACTIONS: &str = "transactions";       // txid -> Transaction
const CF_TX_BY_BLOCK: &str = "tx_by_block";        // block_height:timestamp:txid -> txid (for ordering)
const CF_TX_BY_ADDRESS: &str = "tx_by_address";    // address:timestamp:txid -> txid (for address queries)
const CF_UTXOS: &str = "utxos";                     // txid:vout -> UTXO
const CF_UTXOS_BY_ADDRESS: &str = "utxos_by_address"; // address:txid:vout -> txid:vout (index)

/// Pagination parameters for transaction queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    pub offset: usize,
    pub limit: usize,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
        }
    }
}

/// Paginated transaction response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedTransactions {
    pub transactions: Vec<TransactionWithOutputs>,
    pub total_count: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

/// Transaction with decoded output information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionWithOutputs {
    pub txid: String,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<OutputWithAddress>,
    pub lock_time: u32,
    pub block_height: Option<u64>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub is_coinbase: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputWithAddress {
    pub value: u64,
    pub address: Option<String>,
    pub script_pubkey: Vec<u8>,
}

/// RocksDB-based transaction storage
pub struct TransactionStorage {
    db: Arc<DB>,
}

impl TransactionStorage {
    /// Open or create a new transaction storage database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Define column families
        let cfs = vec![
            CF_TRANSACTIONS,
            CF_TX_BY_BLOCK,
            CF_TX_BY_ADDRESS,
            CF_UTXOS,
            CF_UTXOS_BY_ADDRESS,
        ];

        let db = DB::open_cf(&opts, path, &cfs)
            .map_err(|e| anyhow!("Failed to open RocksDB: {}", e))?;

        Ok(Self {
            db: Arc::new(db),
        })
    }

    /// Get a column family handle
    fn cf_handle(&self, name: &str) -> Result<Arc<BoundColumnFamily<'_>>> {
        self.db
            .cf_handle(name)
            .ok_or_else(|| anyhow!("Column family '{}' not found", name))
    }

    /// Add a new transaction and update all indexes
    #[instrument(skip(self, tx), fields(txid = %tx.txid, is_coinbase = %tx.is_coinbase))]
    pub fn add_transaction(&self, tx: &Transaction, address: &str) -> Result<()> {
        debug!("Adding transaction to storage");
        let mut batch = WriteBatch::default();

        // Serialize transaction
        let tx_bytes = bincode::serialize(tx)?;
        let cf_tx = self.cf_handle(CF_TRANSACTIONS)?;
        batch.put_cf(&cf_tx, tx.txid.as_bytes(), tx_bytes);

        // Add to block index (for time-ordered queries)
        if let (Some(block_height), Some(confirmed_at)) = (tx.block_height, tx.confirmed_at) {
            let cf_by_block = self.cf_handle(CF_TX_BY_BLOCK)?;
            let timestamp = confirmed_at.timestamp();
            let block_key = format!("{:020}:{:020}:{}", block_height, timestamp, tx.txid);
            batch.put_cf(&cf_by_block, block_key.as_bytes(), tx.txid.as_bytes());
            debug!(block_height, "Indexed transaction by block");
        }

        // Add to address index
        let cf_by_address = self.cf_handle(CF_TX_BY_ADDRESS)?;
        let timestamp = tx.confirmed_at.map(|t| t.timestamp()).unwrap_or(0);
        let address_key = format!("{}:{:020}:{}", address, timestamp, tx.txid);
        batch.put_cf(&cf_by_address, address_key.as_bytes(), tx.txid.as_bytes());

        // Add outputs as UTXOs
        for (vout, output) in tx.outputs.iter().enumerate() {
            // Decode address from output if possible
            // For coinbase transactions, we can extract the address
            if tx.is_coinbase && vout == 0 {
                let utxo = UTXO {
                    txid: tx.txid.clone(),
                    vout: vout as u32,
                    value_credits: output.value,
                    value_btp: output.value as f64 / 100_000_000.0,
                    address: address.to_string(),
                    block_height: tx.block_height.unwrap_or(0),
                    is_coinbase: true,
                    created_at: tx.confirmed_at.unwrap_or_else(Utc::now),
                    spent: false,
                    spent_in_tx: None,
                    spent_at_height: None,
                    script_pubkey: output.script_pubkey.clone(),
                };

                self.add_utxo_to_batch(&mut batch, &utxo)?;
                debug!(vout, value_credits = output.value, "Added UTXO to batch");
            }
        }

        // Write batch atomically
        self.db.write(batch)?;
        info!("Transaction added to storage successfully");
        Ok(())
    }

    /// Add a UTXO to a write batch
    fn add_utxo_to_batch(&self, batch: &mut WriteBatch, utxo: &UTXO) -> Result<()> {
        let utxo_key = format!("{}:{}", utxo.txid, utxo.vout);
        let utxo_bytes = bincode::serialize(utxo)?;

        let cf_utxos = self.cf_handle(CF_UTXOS)?;
        batch.put_cf(&cf_utxos, utxo_key.as_bytes(), utxo_bytes);

        // Add to address index
        let cf_by_address = self.cf_handle(CF_UTXOS_BY_ADDRESS)?;
        let address_utxo_key = format!("{}:{}:{}", utxo.address, utxo.txid, utxo.vout);
        batch.put_cf(&cf_by_address, address_utxo_key.as_bytes(), utxo_key.as_bytes());

        Ok(())
    }

    /// Add a UTXO directly (for mining rewards)
    pub fn add_utxo(&self, utxo: &UTXO) -> Result<()> {
        let mut batch = WriteBatch::default();
        self.add_utxo_to_batch(&mut batch, utxo)?;
        self.db.write(batch)?;
        Ok(())
    }

    /// Mark a UTXO as spent
    pub fn mark_utxo_spent(&self, txid: &str, vout: u32, spent_in_tx: &str, spent_at_height: u64) -> Result<()> {
        let utxo_key = format!("{}:{}", txid, vout);
        let cf_utxos = self.cf_handle(CF_UTXOS)?;

        // Get existing UTXO
        if let Some(utxo_bytes) = self.db.get_cf(&cf_utxos, utxo_key.as_bytes())? {
            let mut utxo: UTXO = bincode::deserialize(&utxo_bytes)?;

            // Mark as spent
            utxo.spent = true;
            utxo.spent_in_tx = Some(spent_in_tx.to_string());
            utxo.spent_at_height = Some(spent_at_height);

            // Write back
            let updated_bytes = bincode::serialize(&utxo)?;
            self.db.put_cf(&cf_utxos, utxo_key.as_bytes(), updated_bytes)?;
            Ok(())
        } else {
            Err(anyhow!("UTXO not found: {}:{}", txid, vout))
        }
    }

    /// Get a single transaction by ID
    pub fn get_transaction(&self, txid: &str) -> Result<Option<Transaction>> {
        let cf_tx = self.cf_handle(CF_TRANSACTIONS)?;

        if let Some(tx_bytes) = self.db.get_cf(&cf_tx, txid.as_bytes())? {
            let tx: Transaction = bincode::deserialize(&tx_bytes)?;
            Ok(Some(tx))
        } else {
            Ok(None)
        }
    }

    /// Get paginated transactions for an address (most recent first)
    #[instrument(skip(self), fields(offset = pagination.offset, limit = pagination.limit))]
    pub fn get_transactions_for_address(
        &self,
        address: &str,
        pagination: PaginationParams,
    ) -> Result<PaginatedTransactions> {
        debug!("Fetching paginated transactions for address");
        let cf_by_address = self.cf_handle(CF_TX_BY_ADDRESS)?;
        let cf_tx = self.cf_handle(CF_TRANSACTIONS)?;

        // Create prefix for address
        let prefix = format!("{}:", address);

        // Collect all transaction IDs for this address (reverse chronological)
        let mut tx_ids = Vec::new();
        let iter = self.db.iterator_cf(&cf_by_address, IteratorMode::From(prefix.as_bytes(), rocksdb::Direction::Forward));

        for item in iter {
            let (key, txid_bytes) = item?;
            let key_str = String::from_utf8_lossy(&key);

            // Stop if we've moved past this address's transactions
            if !key_str.starts_with(&prefix) {
                break;
            }

            let txid = String::from_utf8_lossy(&txid_bytes).to_string();
            tx_ids.push(txid);
        }

        // Reverse for most-recent-first ordering
        tx_ids.reverse();

        // Calculate pagination
        let total_count = tx_ids.len();
        let has_more = pagination.offset + pagination.limit < total_count;
        let end = (pagination.offset + pagination.limit).min(total_count);

        debug!(total_count, has_more, "Calculated pagination");

        // Get paginated slice
        let paginated_ids = &tx_ids[pagination.offset..end];

        // Fetch full transactions
        let mut transactions = Vec::new();
        for txid in paginated_ids {
            if let Some(tx_bytes) = self.db.get_cf(&cf_tx, txid.as_bytes())? {
                let tx: Transaction = bincode::deserialize(&tx_bytes)?;

                // Convert to TransactionWithOutputs
                let outputs_with_address: Vec<OutputWithAddress> = tx.outputs.iter().map(|out| {
                    OutputWithAddress {
                        value: out.value,
                        address: Some(address.to_string()), // Simplified for now
                        script_pubkey: out.script_pubkey.clone(),
                    }
                }).collect();

                transactions.push(TransactionWithOutputs {
                    txid: tx.txid,
                    inputs: tx.inputs,
                    outputs: outputs_with_address,
                    lock_time: tx.lock_time,
                    block_height: tx.block_height,
                    confirmed_at: tx.confirmed_at,
                    is_coinbase: tx.is_coinbase,
                });
            }
        }

        info!(transactions_returned = transactions.len(), "Paginated query complete");

        Ok(PaginatedTransactions {
            transactions,
            total_count,
            offset: pagination.offset,
            limit: pagination.limit,
            has_more,
        })
    }

    /// Get all unspent UTXOs for an address
    pub fn get_unspent_utxos(&self, address: &str) -> Result<Vec<UTXO>> {
        let cf_by_address = self.cf_handle(CF_UTXOS_BY_ADDRESS)?;
        let cf_utxos = self.cf_handle(CF_UTXOS)?;

        let prefix = format!("{}:", address);
        let mut utxos = Vec::new();

        let iter = self.db.iterator_cf(&cf_by_address, IteratorMode::From(prefix.as_bytes(), rocksdb::Direction::Forward));

        for item in iter {
            let (key, utxo_key_bytes) = item?;
            let key_str = String::from_utf8_lossy(&key);

            // Stop if we've moved past this address
            if !key_str.starts_with(&prefix) {
                break;
            }

            // Get the actual UTXO
            let utxo_key = String::from_utf8_lossy(&utxo_key_bytes);
            if let Some(utxo_bytes) = self.db.get_cf(&cf_utxos, utxo_key.as_bytes())? {
                let utxo: UTXO = bincode::deserialize(&utxo_bytes)?;

                // Only include unspent UTXOs
                if !utxo.spent {
                    utxos.push(utxo);
                }
            }
        }

        Ok(utxos)
    }

    /// Get balance for an address
    pub fn get_balance(&self, address: &str) -> Result<(u64, f64)> {
        let utxos = self.get_unspent_utxos(address)?;
        let total_credits: u64 = utxos.iter().map(|u| u.value_credits).sum();
        let total_btp = total_credits as f64 / 100_000_000.0;
        Ok((total_credits, total_btp))
    }

    /// Get transaction count for an address
    pub fn get_transaction_count(&self, address: &str) -> Result<usize> {
        let cf_by_address = self.cf_handle(CF_TX_BY_ADDRESS)?;
        let prefix = format!("{}:", address);

        let mut count = 0;
        let iter = self.db.iterator_cf(&cf_by_address, IteratorMode::From(prefix.as_bytes(), rocksdb::Direction::Forward));

        for item in iter {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);

            if !key_str.starts_with(&prefix) {
                break;
            }

            count += 1;
        }

        Ok(count)
    }

    /// Get coinbase transactions (mining history) for an address
    /// Returns most recent first
    pub fn get_coinbase_transactions(&self, address: &str) -> Result<Vec<TransactionWithOutputs>> {
        let cf_by_address = self.cf_handle(CF_TX_BY_ADDRESS)?;
        let cf_tx = self.cf_handle(CF_TRANSACTIONS)?;

        // Create prefix for address
        let prefix = format!("{}:", address);

        // Collect all transaction IDs for this address
        let mut tx_ids = Vec::new();
        let iter = self.db.iterator_cf(&cf_by_address, IteratorMode::From(prefix.as_bytes(), rocksdb::Direction::Forward));

        for item in iter {
            let (key, txid_bytes) = item?;
            let key_str = String::from_utf8_lossy(&key);

            // Stop if we've moved past this address's transactions
            if !key_str.starts_with(&prefix) {
                break;
            }

            let txid = String::from_utf8_lossy(&txid_bytes).to_string();
            tx_ids.push(txid);
        }

        // Reverse for most-recent-first ordering
        tx_ids.reverse();

        // Fetch full transactions and filter for coinbase only
        let mut coinbase_txs = Vec::new();
        for txid in tx_ids {
            if let Some(tx_bytes) = self.db.get_cf(&cf_tx, txid.as_bytes())? {
                let tx: Transaction = bincode::deserialize(&tx_bytes)?;

                // Only include coinbase transactions
                if tx.is_coinbase {
                    // Convert to TransactionWithOutputs
                    let outputs_with_address: Vec<OutputWithAddress> = tx.outputs.iter().map(|out| {
                        OutputWithAddress {
                            value: out.value,
                            address: Some(address.to_string()),
                            script_pubkey: out.script_pubkey.clone(),
                        }
                    }).collect();

                    coinbase_txs.push(TransactionWithOutputs {
                        txid: tx.txid,
                        inputs: tx.inputs,
                        outputs: outputs_with_address,
                        lock_time: tx.lock_time,
                        block_height: tx.block_height,
                        confirmed_at: tx.confirmed_at,
                        is_coinbase: tx.is_coinbase,
                    });
                }
            }
        }

        Ok(coinbase_txs)
    }

    /// Clear all data (for testing)
    pub fn clear_all(&self) -> Result<()> {
        let cfs = vec![
            CF_TRANSACTIONS,
            CF_TX_BY_BLOCK,
            CF_TX_BY_ADDRESS,
            CF_UTXOS,
            CF_UTXOS_BY_ADDRESS,
        ];

        for cf_name in cfs {
            let cf = self.cf_handle(cf_name)?;
            let iter = self.db.iterator_cf(&cf, IteratorMode::Start);

            for item in iter {
                let (key, _) = item?;
                self.db.delete_cf(&cf, &key)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::utxo_manager::{Transaction, TxOutput};

    #[test]
    fn test_transaction_storage_basic() {
        let temp_dir = TempDir::new().unwrap();
        let storage = TransactionStorage::open(temp_dir.path().join("test_db")).unwrap();

        // Create test transaction
        let tx = Transaction {
            txid: "test_tx_1".to_string(),
            version: 1,
            inputs: vec![],
            outputs: vec![TxOutput {
                value: 5000000000,
                script_pubkey: vec![],
            }],
            lock_time: 0,
            block_height: Some(1),
            confirmed_at: Some(Utc::now()),
            is_coinbase: true,
        };

        let address = "test_address";

        // Add transaction
        storage.add_transaction(&tx, address).unwrap();

        // Query transaction
        let result = storage.get_transaction("test_tx_1").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().txid, "test_tx_1");

        // Query by address
        let paginated = storage.get_transactions_for_address(
            address,
            PaginationParams::default(),
        ).unwrap();

        assert_eq!(paginated.total_count, 1);
        assert_eq!(paginated.transactions.len(), 1);
        assert!(!paginated.has_more);
    }

    #[test]
    fn test_pagination() {
        let temp_dir = TempDir::new().unwrap();
        let storage = TransactionStorage::open(temp_dir.path().join("test_db")).unwrap();

        let address = "test_address";

        // Add 100 transactions
        for i in 0..100 {
            let tx = Transaction {
                txid: format!("tx_{}", i),
                version: 1,
                inputs: vec![],
                outputs: vec![TxOutput {
                    value: 1000000,
                    script_pubkey: vec![],
                }],
                lock_time: 0,
                block_height: Some(i),
                confirmed_at: Some(Utc::now()),
                is_coinbase: false,
            };

            storage.add_transaction(&tx, address).unwrap();
        }

        // Test first page
        let page1 = storage.get_transactions_for_address(
            address,
            PaginationParams { offset: 0, limit: 50 },
        ).unwrap();

        assert_eq!(page1.total_count, 100);
        assert_eq!(page1.transactions.len(), 50);
        assert!(page1.has_more);

        // Test second page
        let page2 = storage.get_transactions_for_address(
            address,
            PaginationParams { offset: 50, limit: 50 },
        ).unwrap();

        assert_eq!(page2.total_count, 100);
        assert_eq!(page2.transactions.len(), 50);
        assert!(!page2.has_more);
    }
}