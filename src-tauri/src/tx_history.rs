//! SQLite-based transaction history storage
//!
//! This module provides reliable transaction history storage using SQLite,
//! replacing the previous RocksDB-based tx_storage.rs implementation.
//!
//! Key advantages over RocksDB for transaction history:
//! - UPSERT eliminates race conditions between broadcast and mining paths
//! - Automatic index management via SQL
//! - COALESCE preserves sender_address from broadcast when mining confirms
//! - Simpler queries with SQL WHERE/ORDER BY
//!
//! Constitution Article V: Structured logging for all database operations

use anyhow::{anyhow, Result};
use btpc_core::crypto::address::{Address, AddressType};
use btpc_core::crypto::Script;
use btpc_core::Network;
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use tracing::{debug, info, warn};

// Re-export structures from utxo_manager for compatibility
use crate::utxo_manager::{Transaction, TxInput, TxOutput, UTXO};

/// Decode address from script_pubkey bytes
fn decode_address_from_script(script_bytes: &[u8], network: Network) -> Option<String> {
    let script = Script::deserialize(script_bytes).ok()?;
    let pubkey_hash = script.extract_pubkey_hash()?;
    let address = Address::from_hash(pubkey_hash, network, AddressType::P2PKH);
    Some(address.to_string())
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputWithAddress {
    pub value: u64,
    pub address: Option<String>,
    pub script_pubkey: Vec<u8>,
}

/// SQLite-based transaction history storage
///
/// Thread-safe via internal Mutex on the Connection.
/// Uses WAL mode for better concurrent read performance.
pub struct TransactionHistory {
    conn: Mutex<Connection>,
    network: Network,
}

impl TransactionHistory {
    /// Open or create a new transaction history database
    pub fn open<P: AsRef<Path>>(path: P, network: Network) -> Result<Self> {
        let db_path = path.as_ref().join("tx_history.db");

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;

        // Configure SQLite for optimal performance
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA busy_timeout=5000;
             PRAGMA cache_size=-64000;
             PRAGMA foreign_keys=ON;",
        )?;

        // Create schema
        Self::create_schema(&conn)?;

        info!(
            "Opened tx_history SQLite database for network: {:?}",
            network
        );
        debug!("Database path: {:?}", db_path);

        Ok(Self {
            conn: Mutex::new(conn),
            network,
        })
    }

    /// Create database schema
    fn create_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "-- Main transactions table
             CREATE TABLE IF NOT EXISTS transactions (
                 txid TEXT PRIMARY KEY,
                 version INTEGER NOT NULL,
                 lock_time INTEGER NOT NULL,
                 fork_id INTEGER NOT NULL DEFAULT 0,
                 block_height INTEGER,
                 confirmed_at INTEGER,
                 is_coinbase INTEGER NOT NULL DEFAULT 0,
                 sender_address TEXT,
                 raw_inputs TEXT NOT NULL,
                 raw_outputs TEXT NOT NULL,
                 created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
             );

             -- Address involvement (many-to-many)
             CREATE TABLE IF NOT EXISTS tx_addresses (
                 txid TEXT NOT NULL,
                 address TEXT NOT NULL,
                 is_sender INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY (txid, address),
                 FOREIGN KEY (txid) REFERENCES transactions(txid) ON DELETE CASCADE
             );

             -- UTXOs table
             CREATE TABLE IF NOT EXISTS utxos (
                 txid TEXT NOT NULL,
                 vout INTEGER NOT NULL,
                 value_credits INTEGER NOT NULL,
                 address TEXT NOT NULL,
                 block_height INTEGER NOT NULL DEFAULT 0,
                 is_coinbase INTEGER NOT NULL DEFAULT 0,
                 created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                 spent INTEGER NOT NULL DEFAULT 0,
                 spent_in_tx TEXT,
                 spent_at_height INTEGER,
                 script_pubkey BLOB NOT NULL,
                 PRIMARY KEY (txid, vout)
             );

             -- Indexes for fast queries
             CREATE INDEX IF NOT EXISTS idx_tx_confirmed ON transactions(confirmed_at DESC);
             CREATE INDEX IF NOT EXISTS idx_tx_block ON transactions(block_height DESC);
             CREATE INDEX IF NOT EXISTS idx_tx_coinbase ON transactions(is_coinbase) WHERE is_coinbase = 1;
             CREATE INDEX IF NOT EXISTS idx_addr_tx ON tx_addresses(address);
             CREATE INDEX IF NOT EXISTS idx_utxo_address ON utxos(address) WHERE spent = 0;
             CREATE INDEX IF NOT EXISTS idx_utxo_spent ON utxos(spent);"
        )?;

        debug!("Transaction history schema created/verified");
        Ok(())
    }

    /// Get the network this storage operates on
    pub fn network(&self) -> &Network {
        &self.network
    }

    /// Add or update a transaction (UPSERT)
    ///
    /// This is the key improvement over RocksDB:
    /// - Single atomic operation handles both broadcast and confirmation
    /// - COALESCE preserves sender_address from broadcast when mining confirms with NULL
    /// - No manual deduplication needed
    pub fn add_transaction(&self, tx: &Transaction, address: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        // Serialize inputs/outputs as JSON
        let inputs_json = serde_json::to_string(&tx.inputs)?;
        let outputs_json = serde_json::to_string(&tx.outputs)?;
        let confirmed_at = tx.confirmed_at.map(|dt| dt.timestamp());

        debug!(
            "UPSERT transaction: txid={}, address={}, block_height={:?}, sender={:?}",
            &tx.txid[..16.min(tx.txid.len())],
            &address[..20.min(address.len())],
            tx.block_height,
            tx.sender_address.as_ref().map(|s| &s[..20.min(s.len())])
        );

        // UPSERT transaction - COALESCE preserves existing non-null values
        conn.execute(
            "INSERT INTO transactions (txid, version, lock_time, fork_id, block_height, confirmed_at, is_coinbase, sender_address, raw_inputs, raw_outputs)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(txid) DO UPDATE SET
                 block_height = COALESCE(excluded.block_height, block_height),
                 confirmed_at = COALESCE(excluded.confirmed_at, confirmed_at),
                 sender_address = COALESCE(sender_address, excluded.sender_address),
                 raw_inputs = excluded.raw_inputs,
                 raw_outputs = excluded.raw_outputs",
            params![
                tx.txid,
                tx.version,
                tx.lock_time,
                tx.fork_id,
                tx.block_height,
                confirmed_at,
                tx.is_coinbase as i32,
                tx.sender_address,
                inputs_json,
                outputs_json,
            ],
        )?;

        // Determine if this address is the sender
        let is_sender = tx
            .sender_address
            .as_ref()
            .map(|s| s == address)
            .unwrap_or(false);

        // UPSERT address association
        conn.execute(
            "INSERT INTO tx_addresses (txid, address, is_sender)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(txid, address) DO UPDATE SET
                 is_sender = MAX(is_sender, excluded.is_sender)",
            params![tx.txid, address, is_sender as i32],
        )?;

        // Add UTXOs for outputs
        let network = self.network;
        for (vout, output) in tx.outputs.iter().enumerate() {
            let output_address = if tx.is_coinbase && vout == 0 {
                address.to_string()
            } else {
                match decode_address_from_script(&output.script_pubkey, network) {
                    Some(addr) => addr,
                    None => continue,
                }
            };

            let block_height = tx.block_height.unwrap_or(0);
            let created_at = tx
                .confirmed_at
                .map(|dt| dt.timestamp())
                .unwrap_or_else(|| Utc::now().timestamp());

            conn.execute(
                "INSERT INTO utxos (txid, vout, value_credits, address, block_height, is_coinbase, created_at, script_pubkey)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(txid, vout) DO UPDATE SET
                     block_height = COALESCE(excluded.block_height, block_height),
                     address = excluded.address",
                params![
                    tx.txid,
                    vout as u32,
                    output.value,
                    output_address,
                    block_height,
                    (tx.is_coinbase && vout == 0) as i32,
                    created_at,
                    output.script_pubkey,
                ],
            )?;
        }

        info!(
            "Transaction {} added/updated for address {}",
            &tx.txid[..16.min(tx.txid.len())],
            &address[..20.min(address.len())]
        );
        Ok(())
    }

    /// Add a UTXO directly (for mining rewards)
    pub fn add_utxo(&self, utxo: &UTXO) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        let created_at = utxo.created_at.timestamp();

        conn.execute(
            "INSERT INTO utxos (txid, vout, value_credits, address, block_height, is_coinbase, created_at, spent, script_pubkey)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(txid, vout) DO UPDATE SET
                 block_height = excluded.block_height,
                 address = excluded.address",
            params![
                utxo.txid,
                utxo.vout,
                utxo.value_credits,
                utxo.address,
                utxo.block_height,
                utxo.is_coinbase as i32,
                created_at,
                utxo.spent as i32,
                utxo.script_pubkey,
            ],
        )?;

        Ok(())
    }

    /// Mark a UTXO as spent
    pub fn mark_utxo_spent(
        &self,
        txid: &str,
        vout: u32,
        spent_in_tx: &str,
        spent_at_height: u64,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        let rows = conn.execute(
            "UPDATE utxos SET spent = 1, spent_in_tx = ?1, spent_at_height = ?2
             WHERE txid = ?3 AND vout = ?4",
            params![spent_in_tx, spent_at_height, txid, vout],
        )?;

        if rows == 0 {
            warn!("UTXO not found to mark as spent: {}:{}", txid, vout);
        }

        Ok(())
    }

    /// Get a single transaction by ID
    pub fn get_transaction(&self, txid: &str) -> Result<Option<Transaction>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        let result = conn.query_row(
            "SELECT version, lock_time, fork_id, block_height, confirmed_at, is_coinbase, sender_address, raw_inputs, raw_outputs
             FROM transactions WHERE txid = ?1",
            params![txid],
            |row| {
                let version: u32 = row.get(0)?;
                let lock_time: u32 = row.get(1)?;
                let fork_id: u8 = row.get(2)?;
                let block_height: Option<u64> = row.get(3)?;
                let confirmed_at_ts: Option<i64> = row.get(4)?;
                let is_coinbase: bool = row.get::<_, i32>(5)? != 0;
                let sender_address: Option<String> = row.get(6)?;
                let inputs_json: String = row.get(7)?;
                let outputs_json: String = row.get(8)?;

                Ok((version, lock_time, fork_id, block_height, confirmed_at_ts, is_coinbase, sender_address, inputs_json, outputs_json))
            },
        ).optional()?;

        match result {
            Some((
                version,
                lock_time,
                fork_id,
                block_height,
                confirmed_at_ts,
                is_coinbase,
                sender_address,
                inputs_json,
                outputs_json,
            )) => {
                let inputs: Vec<TxInput> = serde_json::from_str(&inputs_json)?;
                let outputs: Vec<TxOutput> = serde_json::from_str(&outputs_json)?;
                let confirmed_at = confirmed_at_ts.and_then(|ts| Utc.timestamp_opt(ts, 0).single());

                Ok(Some(Transaction {
                    txid: txid.to_string(),
                    version,
                    inputs,
                    outputs,
                    lock_time,
                    fork_id,
                    block_height,
                    confirmed_at,
                    is_coinbase,
                    sender_address,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get a UTXO by txid and vout
    pub fn get_utxo(&self, txid: &str, vout: u32) -> Result<Option<UTXO>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        let result = conn.query_row(
            "SELECT value_credits, address, block_height, is_coinbase, created_at, spent, spent_in_tx, spent_at_height, script_pubkey
             FROM utxos WHERE txid = ?1 AND vout = ?2",
            params![txid, vout],
            |row| {
                let value_credits: u64 = row.get(0)?;
                let address: String = row.get(1)?;
                let block_height: u64 = row.get(2)?;
                let is_coinbase: bool = row.get::<_, i32>(3)? != 0;
                let created_at_ts: i64 = row.get(4)?;
                let spent: bool = row.get::<_, i32>(5)? != 0;
                let spent_in_tx: Option<String> = row.get(6)?;
                let spent_at_height: Option<u64> = row.get(7)?;
                let script_pubkey: Vec<u8> = row.get(8)?;

                Ok((value_credits, address, block_height, is_coinbase, created_at_ts, spent, spent_in_tx, spent_at_height, script_pubkey))
            },
        ).optional()?;

        match result {
            Some((
                value_credits,
                address,
                block_height,
                is_coinbase,
                created_at_ts,
                spent,
                spent_in_tx,
                spent_at_height,
                script_pubkey,
            )) => {
                let created_at = Utc
                    .timestamp_opt(created_at_ts, 0)
                    .single()
                    .unwrap_or_else(Utc::now);

                Ok(Some(UTXO {
                    txid: txid.to_string(),
                    vout,
                    value_credits,
                    value_btp: value_credits as f64 / 100_000_000.0,
                    address,
                    block_height,
                    is_coinbase,
                    created_at,
                    spent,
                    spent_in_tx,
                    spent_at_height,
                    script_pubkey,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get all unspent UTXOs from database
    pub fn get_all_unspent_utxos(&self) -> Result<Vec<UTXO>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT txid, vout, value_credits, address, block_height, is_coinbase, created_at, script_pubkey
             FROM utxos WHERE spent = 0"
        )?;

        let utxos = stmt
            .query_map([], |row| {
                let txid: String = row.get(0)?;
                let vout: u32 = row.get(1)?;
                let value_credits: u64 = row.get(2)?;
                let address: String = row.get(3)?;
                let block_height: u64 = row.get(4)?;
                let is_coinbase: bool = row.get::<_, i32>(5)? != 0;
                let created_at_ts: i64 = row.get(6)?;
                let script_pubkey: Vec<u8> = row.get(7)?;

                let created_at = Utc
                    .timestamp_opt(created_at_ts, 0)
                    .single()
                    .unwrap_or_else(Utc::now);

                Ok(UTXO {
                    txid,
                    vout,
                    value_credits,
                    value_btp: value_credits as f64 / 100_000_000.0,
                    address,
                    block_height,
                    is_coinbase,
                    created_at,
                    spent: false,
                    spent_in_tx: None,
                    spent_at_height: None,
                    script_pubkey,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        debug!("Retrieved {} unspent UTXOs from tx_history", utxos.len());
        Ok(utxos)
    }

    /// Get a transaction with addresses decoded from UTXOs
    /// FIX 2025-12-12: Use single lock scope to avoid deadlock from nested get_transaction/get_utxo calls
    pub fn get_transaction_with_addresses(
        &self,
        txid: &str,
    ) -> Result<Option<TransactionWithOutputs>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        // Get transaction data
        let result = conn.query_row(
            "SELECT version, lock_time, fork_id, block_height, confirmed_at, is_coinbase, sender_address, raw_inputs, raw_outputs
             FROM transactions WHERE txid = ?1",
            params![txid],
            |row| {
                let version: u32 = row.get(0)?;
                let lock_time: u32 = row.get(1)?;
                let fork_id: u8 = row.get(2)?;
                let block_height: Option<u64> = row.get(3)?;
                let confirmed_at_ts: Option<i64> = row.get(4)?;
                let is_coinbase: bool = row.get::<_, i32>(5)? != 0;
                let sender_address: Option<String> = row.get(6)?;
                let inputs_json: String = row.get(7)?;
                let outputs_json: String = row.get(8)?;

                Ok((version, lock_time, fork_id, block_height, confirmed_at_ts, is_coinbase, sender_address, inputs_json, outputs_json))
            },
        ).optional()?;

        let (
            _version,
            lock_time,
            _fork_id,
            block_height,
            confirmed_at_ts,
            is_coinbase,
            sender_address,
            inputs_json,
            outputs_json,
        ) = match result {
            Some(r) => r,
            None => return Ok(None),
        };

        let inputs: Vec<TxInput> = serde_json::from_str(&inputs_json)?;
        let outputs: Vec<TxOutput> = serde_json::from_str(&outputs_json)?;
        let confirmed_at = confirmed_at_ts.and_then(|ts| Utc.timestamp_opt(ts, 0).single());
        let network = self.network;

        // Build outputs with addresses - query UTXO table within same lock
        let mut outputs_with_addresses = Vec::new();
        for (vout, output) in outputs.iter().enumerate() {
            let address: Option<String> = conn
                .query_row(
                    "SELECT address FROM utxos WHERE txid = ?1 AND vout = ?2",
                    params![txid, vout as u32],
                    |row| row.get(0),
                )
                .optional()?
                .or_else(|| decode_address_from_script(&output.script_pubkey, network));

            outputs_with_addresses.push(OutputWithAddress {
                value: output.value,
                address,
                script_pubkey: output.script_pubkey.clone(),
            });
        }

        Ok(Some(TransactionWithOutputs {
            txid: txid.to_string(),
            inputs,
            outputs: outputs_with_addresses,
            lock_time,
            block_height,
            confirmed_at,
            is_coinbase,
            sender_address,
        }))
    }

    /// Get paginated transactions for an address (pending first, then newest confirmed)
    pub fn get_transactions_for_address(
        &self,
        address: &str,
        pagination: PaginationParams,
    ) -> Result<PaginatedTransactions> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        debug!(
            "Querying transactions for address: {}, offset={}, limit={}",
            &address[..20.min(address.len())],
            pagination.offset,
            pagination.limit
        );

        // Get total count
        let total_count: usize = conn.query_row(
            "SELECT COUNT(DISTINCT t.txid)
             FROM transactions t
             JOIN tx_addresses ta ON t.txid = ta.txid
             WHERE ta.address = ?1",
            params![address],
            |row| row.get(0),
        )?;

        // Get paginated transactions with UTXO addresses joined in single query
        // FIX 2025-12-12: Avoid nested mutex locks by joining UTXO data in main query
        // Order: pending first (confirmed_at IS NULL), then by block_height DESC
        let mut stmt = conn.prepare(
            "SELECT t.txid, t.version, t.lock_time, t.fork_id, t.block_height, t.confirmed_at,
                    t.is_coinbase, t.sender_address, t.raw_inputs, t.raw_outputs
             FROM transactions t
             JOIN tx_addresses ta ON t.txid = ta.txid
             WHERE ta.address = ?1
             ORDER BY
                 CASE WHEN t.confirmed_at IS NULL THEN 0 ELSE 1 END,
                 COALESCE(t.block_height, 999999999) DESC
             LIMIT ?2 OFFSET ?3",
        )?;

        let network = self.network;

        // Collect raw transaction data first (while holding the lock)
        let raw_transactions: Vec<_> = stmt
            .query_map(
                params![address, pagination.limit as i64, pagination.offset as i64],
                |row| {
                    let txid: String = row.get(0)?;
                    let _version: u32 = row.get(1)?;
                    let lock_time: u32 = row.get(2)?;
                    let _fork_id: u8 = row.get(3)?;
                    let block_height: Option<u64> = row.get(4)?;
                    let confirmed_at_ts: Option<i64> = row.get(5)?;
                    let is_coinbase: bool = row.get::<_, i32>(6)? != 0;
                    let sender_address: Option<String> = row.get(7)?;
                    let inputs_json: String = row.get(8)?;
                    let outputs_json: String = row.get(9)?;

                    Ok((
                        txid,
                        lock_time,
                        block_height,
                        confirmed_at_ts,
                        is_coinbase,
                        sender_address,
                        inputs_json,
                        outputs_json,
                    ))
                },
            )?
            .filter_map(|r| r.ok())
            .collect();

        // Now build the full transaction objects (still holding the same lock, no nested locking)
        let mut transactions: Vec<TransactionWithOutputs> = Vec::new();

        for (
            txid,
            lock_time,
            block_height,
            confirmed_at_ts,
            is_coinbase,
            mut sender_address,
            inputs_json,
            outputs_json,
        ) in raw_transactions
        {
            let inputs: Vec<TxInput> = match serde_json::from_str(&inputs_json) {
                Ok(i) => i,
                Err(_) => continue,
            };
            let outputs: Vec<TxOutput> = match serde_json::from_str(&outputs_json) {
                Ok(o) => o,
                Err(_) => continue,
            };
            let confirmed_at = confirmed_at_ts.and_then(|ts| Utc.timestamp_opt(ts, 0).single());

            // Build outputs with addresses - query UTXO table for each output
            let mut outputs_with_addresses: Vec<OutputWithAddress> = Vec::new();
            for (vout, output) in outputs.iter().enumerate() {
                // Try to get address from UTXO table
                let output_address: Option<String> = conn
                    .query_row(
                        "SELECT address FROM utxos WHERE txid = ?1 AND vout = ?2",
                        params![&txid, vout as u32],
                        |row| row.get(0),
                    )
                    .optional()?
                    .or_else(|| decode_address_from_script(&output.script_pubkey, network));

                outputs_with_addresses.push(OutputWithAddress {
                    value: output.value,
                    address: output_address,
                    script_pubkey: output.script_pubkey.clone(),
                });
            }

            // Derive sender_address from inputs if not set
            if sender_address.is_none() && !is_coinbase && !inputs.is_empty() {
                let first_input = &inputs[0];
                sender_address = conn
                    .query_row(
                        "SELECT address FROM utxos WHERE txid = ?1 AND vout = ?2",
                        params![&first_input.prev_txid, first_input.prev_vout],
                        |row| row.get(0),
                    )
                    .optional()?;
            }

            transactions.push(TransactionWithOutputs {
                txid,
                inputs,
                outputs: outputs_with_addresses,
                lock_time,
                block_height,
                confirmed_at,
                is_coinbase,
                sender_address,
            });
        }

        let has_more = pagination.offset + transactions.len() < total_count;

        info!(
            "Returned {} transactions for address {} (total: {})",
            transactions.len(),
            &address[..20.min(address.len())],
            total_count
        );

        Ok(PaginatedTransactions {
            transactions,
            total_count,
            offset: pagination.offset,
            limit: pagination.limit,
            has_more,
        })
    }

    /// Get paginated transactions for an address with type filtering
    ///
    /// tx_type:
    /// - None or "all" = all transactions
    /// - "sent" = non-coinbase where sender_address = wallet address
    /// - "received" = non-coinbase where sender_address != wallet address (or NULL)
    /// - "mining" = coinbase transactions only
    #[allow(clippy::type_complexity)]
    pub fn get_transactions_for_address_filtered(
        &self,
        address: &str,
        pagination: PaginationParams,
        tx_type: Option<&str>,
    ) -> Result<PaginatedTransactions> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        // Build WHERE clause based on tx_type filter
        let (type_filter, type_filter_count) = match tx_type {
            Some("sent") => (
                "AND t.is_coinbase = 0 AND t.sender_address = ?4",
                "AND t.is_coinbase = 0 AND t.sender_address = ?2",
            ),
            Some("received") => (
                "AND t.is_coinbase = 0 AND (t.sender_address IS NULL OR t.sender_address != ?4)",
                "AND t.is_coinbase = 0 AND (t.sender_address IS NULL OR t.sender_address != ?2)",
            ),
            Some("mining") => ("AND t.is_coinbase = 1", "AND t.is_coinbase = 1"),
            _ => ("", ""), // "all" or None = no filter
        };

        debug!(
            "Querying transactions for address: {}, type={:?}, offset={}, limit={}",
            &address[..20.min(address.len())],
            tx_type,
            pagination.offset,
            pagination.limit
        );

        // Get total count with filter
        let count_query = format!(
            "SELECT COUNT(DISTINCT t.txid)
             FROM transactions t
             JOIN tx_addresses ta ON t.txid = ta.txid
             WHERE ta.address = ?1 {}",
            type_filter_count
        );

        let total_count: usize = if tx_type == Some("sent") || tx_type == Some("received") {
            conn.query_row(&count_query, params![address, address], |row| row.get(0))?
        } else {
            conn.query_row(&count_query, params![address], |row| row.get(0))?
        };

        // Get paginated transactions with filter
        let select_query = format!(
            "SELECT t.txid, t.version, t.lock_time, t.fork_id, t.block_height, t.confirmed_at,
                    t.is_coinbase, t.sender_address, t.raw_inputs, t.raw_outputs
             FROM transactions t
             JOIN tx_addresses ta ON t.txid = ta.txid
             WHERE ta.address = ?1 {}
             ORDER BY
                 CASE WHEN t.confirmed_at IS NULL THEN 0 ELSE 1 END,
                 COALESCE(t.block_height, 999999999) DESC
             LIMIT ?2 OFFSET ?3",
            type_filter
        );

        let network = self.network;

        // Execute query with appropriate params
        // Note: Collect results immediately to avoid borrow checker issues with stmt lifetime
        let raw_transactions: Vec<(
            String,
            u32,
            Option<u64>,
            Option<i64>,
            bool,
            Option<String>,
            String,
            String,
        )> = if tx_type == Some("sent") || tx_type == Some("received") {
            let mut stmt = conn.prepare(&select_query)?;
            let rows = stmt.query_map(
                params![
                    address,
                    pagination.limit as i64,
                    pagination.offset as i64,
                    address
                ],
                |row| {
                    let txid: String = row.get(0)?;
                    let _version: u32 = row.get(1)?;
                    let lock_time: u32 = row.get(2)?;
                    let _fork_id: u8 = row.get(3)?;
                    let block_height: Option<u64> = row.get(4)?;
                    let confirmed_at_ts: Option<i64> = row.get(5)?;
                    let is_coinbase: bool = row.get::<_, i32>(6)? != 0;
                    let sender_address: Option<String> = row.get(7)?;
                    let inputs_json: String = row.get(8)?;
                    let outputs_json: String = row.get(9)?;
                    Ok((
                        txid,
                        lock_time,
                        block_height,
                        confirmed_at_ts,
                        is_coinbase,
                        sender_address,
                        inputs_json,
                        outputs_json,
                    ))
                },
            )?;
            rows.filter_map(|r| r.ok()).collect()
        } else {
            let mut stmt = conn.prepare(&select_query)?;
            let rows = stmt.query_map(
                params![address, pagination.limit as i64, pagination.offset as i64],
                |row| {
                    let txid: String = row.get(0)?;
                    let _version: u32 = row.get(1)?;
                    let lock_time: u32 = row.get(2)?;
                    let _fork_id: u8 = row.get(3)?;
                    let block_height: Option<u64> = row.get(4)?;
                    let confirmed_at_ts: Option<i64> = row.get(5)?;
                    let is_coinbase: bool = row.get::<_, i32>(6)? != 0;
                    let sender_address: Option<String> = row.get(7)?;
                    let inputs_json: String = row.get(8)?;
                    let outputs_json: String = row.get(9)?;
                    Ok((
                        txid,
                        lock_time,
                        block_height,
                        confirmed_at_ts,
                        is_coinbase,
                        sender_address,
                        inputs_json,
                        outputs_json,
                    ))
                },
            )?;
            rows.filter_map(|r| r.ok()).collect()
        };

        // Build full transaction objects
        let mut transactions: Vec<TransactionWithOutputs> = Vec::new();

        for (
            txid,
            lock_time,
            block_height,
            confirmed_at_ts,
            is_coinbase,
            mut sender_address,
            inputs_json,
            outputs_json,
        ) in raw_transactions
        {
            let inputs: Vec<TxInput> = match serde_json::from_str(&inputs_json) {
                Ok(i) => i,
                Err(_) => continue,
            };
            let outputs: Vec<TxOutput> = match serde_json::from_str(&outputs_json) {
                Ok(o) => o,
                Err(_) => continue,
            };
            let confirmed_at = confirmed_at_ts.and_then(|ts| Utc.timestamp_opt(ts, 0).single());

            // Build outputs with addresses
            let mut outputs_with_addresses: Vec<OutputWithAddress> = Vec::new();
            for (vout, output) in outputs.iter().enumerate() {
                let output_address: Option<String> = conn
                    .query_row(
                        "SELECT address FROM utxos WHERE txid = ?1 AND vout = ?2",
                        params![&txid, vout as u32],
                        |row| row.get(0),
                    )
                    .optional()?
                    .or_else(|| decode_address_from_script(&output.script_pubkey, network));

                outputs_with_addresses.push(OutputWithAddress {
                    value: output.value,
                    address: output_address,
                    script_pubkey: output.script_pubkey.clone(),
                });
            }

            // Derive sender_address from inputs if not set
            if sender_address.is_none() && !is_coinbase && !inputs.is_empty() {
                let first_input = &inputs[0];
                sender_address = conn
                    .query_row(
                        "SELECT address FROM utxos WHERE txid = ?1 AND vout = ?2",
                        params![&first_input.prev_txid, first_input.prev_vout],
                        |row| row.get(0),
                    )
                    .optional()?;
            }

            transactions.push(TransactionWithOutputs {
                txid,
                inputs,
                outputs: outputs_with_addresses,
                lock_time,
                block_height,
                confirmed_at,
                is_coinbase,
                sender_address,
            });
        }

        let has_more = pagination.offset + transactions.len() < total_count;

        info!(
            "Returned {} filtered ({:?}) transactions for address {} (total: {})",
            transactions.len(),
            tx_type,
            &address[..20.min(address.len())],
            total_count
        );

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
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT txid, vout, value_credits, block_height, is_coinbase, created_at, script_pubkey
             FROM utxos WHERE address = ?1 AND spent = 0",
        )?;

        let utxos = stmt
            .query_map(params![address], |row| {
                let txid: String = row.get(0)?;
                let vout: u32 = row.get(1)?;
                let value_credits: u64 = row.get(2)?;
                let block_height: u64 = row.get(3)?;
                let is_coinbase: bool = row.get::<_, i32>(4)? != 0;
                let created_at_ts: i64 = row.get(5)?;
                let script_pubkey: Vec<u8> = row.get(6)?;

                let created_at = Utc
                    .timestamp_opt(created_at_ts, 0)
                    .single()
                    .unwrap_or_else(Utc::now);

                Ok(UTXO {
                    txid,
                    vout,
                    value_credits,
                    value_btp: value_credits as f64 / 100_000_000.0,
                    address: address.to_string(),
                    block_height,
                    is_coinbase,
                    created_at,
                    spent: false,
                    spent_in_tx: None,
                    spent_at_height: None,
                    script_pubkey,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(utxos)
    }

    /// Get balance for an address
    pub fn get_balance(&self, address: &str) -> Result<(u64, f64)> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        let total_credits: u64 = conn.query_row(
            "SELECT COALESCE(SUM(value_credits), 0) FROM utxos WHERE address = ?1 AND spent = 0",
            params![address],
            |row| row.get(0),
        )?;

        let total_btp = total_credits as f64 / 100_000_000.0;
        Ok((total_credits, total_btp))
    }

    /// Get transaction count for an address
    pub fn get_transaction_count(&self, address: &str) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        let count: usize = conn.query_row(
            "SELECT COUNT(DISTINCT txid) FROM tx_addresses WHERE address = ?1",
            params![address],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    /// Get coinbase transactions (mining history) for an address
    pub fn get_coinbase_transactions(&self, address: &str) -> Result<Vec<TransactionWithOutputs>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT t.txid, t.version, t.lock_time, t.fork_id, t.block_height, t.confirmed_at,
                    t.raw_inputs, t.raw_outputs
             FROM transactions t
             JOIN tx_addresses ta ON t.txid = ta.txid
             WHERE ta.address = ?1 AND t.is_coinbase = 1
             ORDER BY t.block_height DESC",
        )?;

        let transactions: Vec<TransactionWithOutputs> = stmt
            .query_map(params![address], |row| {
                let txid: String = row.get(0)?;
                let _version: u32 = row.get(1)?;
                let lock_time: u32 = row.get(2)?;
                let _fork_id: u8 = row.get(3)?;
                let block_height: Option<u64> = row.get(4)?;
                let confirmed_at_ts: Option<i64> = row.get(5)?;
                let inputs_json: String = row.get(6)?;
                let outputs_json: String = row.get(7)?;

                Ok((
                    txid,
                    lock_time,
                    block_height,
                    confirmed_at_ts,
                    inputs_json,
                    outputs_json,
                ))
            })?
            .filter_map(|result| match result {
                Ok((txid, lock_time, block_height, confirmed_at_ts, inputs_json, outputs_json)) => {
                    let inputs: Vec<TxInput> = serde_json::from_str(&inputs_json).ok()?;
                    let outputs: Vec<TxOutput> = serde_json::from_str(&outputs_json).ok()?;
                    let confirmed_at =
                        confirmed_at_ts.and_then(|ts| Utc.timestamp_opt(ts, 0).single());

                    let outputs_with_addresses: Vec<OutputWithAddress> = outputs
                        .into_iter()
                        .map(|output| OutputWithAddress {
                            value: output.value,
                            address: Some(address.to_string()),
                            script_pubkey: output.script_pubkey,
                        })
                        .collect();

                    Some(TransactionWithOutputs {
                        txid,
                        inputs,
                        outputs: outputs_with_addresses,
                        lock_time,
                        block_height,
                        confirmed_at,
                        is_coinbase: true,
                        sender_address: None,
                    })
                }
                Err(_) => None,
            })
            .collect();

        Ok(transactions)
    }

    /// Flush changes to disk (SQLite WAL checkpoint)
    pub fn flush(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        debug!("tx_history WAL checkpoint completed");
        Ok(())
    }

    /// Sync WAL to disk
    pub fn sync_wal(&self) -> Result<()> {
        self.flush()
    }

    /// Get diagnostic information for debugging
    pub fn get_diagnostic_info(&self, address: &str) -> Result<String> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        let mut output = format!(
            "=== TX DIAGNOSTIC FOR {} ===\n",
            &address[..30.min(address.len())]
        );

        // Count transactions
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tx_addresses WHERE address = ?1",
            params![address],
            |row| row.get(0),
        )?;

        let pending: i64 = conn.query_row(
            "SELECT COUNT(*) FROM transactions t
             JOIN tx_addresses ta ON t.txid = ta.txid
             WHERE ta.address = ?1 AND t.confirmed_at IS NULL",
            params![address],
            |row| row.get(0),
        )?;

        let coinbase: i64 = conn.query_row(
            "SELECT COUNT(*) FROM transactions t
             JOIN tx_addresses ta ON t.txid = ta.txid
             WHERE ta.address = ?1 AND t.is_coinbase = 1",
            params![address],
            |row| row.get(0),
        )?;

        output.push_str("\n--- SUMMARY ---\n");
        output.push_str(&format!("Total transactions: {}\n", total));
        output.push_str(&format!("Pending: {}\n", pending));
        output.push_str(&format!("Confirmed: {}\n", total - pending));
        output.push_str(&format!("Coinbase: {}\n", coinbase));

        // List recent transactions
        output.push_str("\n--- RECENT TRANSACTIONS ---\n");
        let mut stmt = conn.prepare(
            "SELECT t.txid, t.block_height, t.confirmed_at, t.is_coinbase, t.sender_address
             FROM transactions t
             JOIN tx_addresses ta ON t.txid = ta.txid
             WHERE ta.address = ?1
             ORDER BY COALESCE(t.confirmed_at, 9999999999) DESC
             LIMIT 10",
        )?;

        let rows = stmt.query_map(params![address], |row| {
            let txid: String = row.get(0)?;
            let block_height: Option<u64> = row.get(1)?;
            let confirmed_at: Option<i64> = row.get(2)?;
            let is_coinbase: bool = row.get::<_, i32>(3)? != 0;
            let sender_address: Option<String> = row.get(4)?;
            Ok((
                txid,
                block_height,
                confirmed_at,
                is_coinbase,
                sender_address,
            ))
        })?;

        for (txid, block_height, confirmed_at, is_coinbase, sender_address) in rows.flatten() {
            let status = if confirmed_at.is_some() {
                "CONFIRMED"
            } else {
                "PENDING"
            };
            let sender = sender_address
                .map(|s| format!("sender={}...", &s[..20.min(s.len())]))
                .unwrap_or_else(|| "sender=null".to_string());
            output.push_str(&format!(
                "  {} txid={}... block={:?} coinbase={} {}\n",
                status,
                &txid[..16.min(txid.len())],
                block_height,
                is_coinbase,
                sender
            ));
        }

        Ok(output)
    }

    /// Clean up orphaned pending transactions (placeholder - SQLite makes this simpler)
    pub fn cleanup_orphaned_pending_transactions(&self, _max_age_seconds: i64) -> Result<usize> {
        // With SQLite, orphaned transactions are less of a problem
        // The UPSERT ensures clean state transitions
        // This is a no-op for now, can be implemented if needed
        Ok(0)
    }

    /// Clear all data (for testing)
    #[cfg(test)]
    pub fn clear_all(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        conn.execute_batch(
            "DELETE FROM tx_addresses;
             DELETE FROM transactions;
             DELETE FROM utxos;",
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_tx(
        txid: &str,
        is_coinbase: bool,
        block_height: Option<u64>,
        sender: Option<&str>,
    ) -> Transaction {
        Transaction {
            txid: txid.to_string(),
            version: 1,
            inputs: vec![],
            outputs: vec![TxOutput {
                value: 100_000_000,
                script_pubkey: vec![],
            }],
            lock_time: 0,
            fork_id: 2,
            block_height,
            confirmed_at: block_height.map(|_| Utc::now()),
            is_coinbase,
            sender_address: sender.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_no_duplicates_pending_to_confirmed() {
        let temp_dir = TempDir::new().unwrap();
        let history = TransactionHistory::open(temp_dir.path(), Network::Regtest).unwrap();

        let address = "mTestAddress123";
        let txid = "abc123def456";

        // Add as pending
        let pending_tx = create_test_tx(txid, false, None, Some("mSender"));
        history.add_transaction(&pending_tx, address).unwrap();

        let result = history
            .get_transactions_for_address(address, PaginationParams::default())
            .unwrap();
        assert_eq!(result.total_count, 1);

        // Add as confirmed
        let confirmed_tx = create_test_tx(txid, false, Some(100), Some("mSender"));
        history.add_transaction(&confirmed_tx, address).unwrap();

        // Should still be 1 transaction
        let result = history
            .get_transactions_for_address(address, PaginationParams::default())
            .unwrap();
        assert_eq!(result.total_count, 1);
        assert_eq!(result.transactions[0].block_height, Some(100));
    }

    #[test]
    fn test_sender_address_preserved() {
        let temp_dir = TempDir::new().unwrap();
        let history = TransactionHistory::open(temp_dir.path(), Network::Regtest).unwrap();

        let address = "mTestAddress";
        let txid = "preserve_sender_test";

        // Broadcast sets sender_address
        let broadcast_tx = create_test_tx(txid, false, None, Some("mSenderAddress"));
        history.add_transaction(&broadcast_tx, address).unwrap();

        // Mining confirms but has no sender (simulates current bug scenario)
        let mut mined_tx = create_test_tx(txid, false, Some(50), None);
        mined_tx.sender_address = None; // Mining path doesn't know sender
        history.add_transaction(&mined_tx, address).unwrap();

        // Sender should be preserved from broadcast
        let result = history.get_transaction(txid).unwrap().unwrap();
        assert_eq!(result.sender_address, Some("mSenderAddress".to_string()));
        assert_eq!(result.block_height, Some(50)); // But confirmation is updated
    }

    #[test]
    fn test_multi_address_transaction() {
        let temp_dir = TempDir::new().unwrap();
        let history = TransactionHistory::open(temp_dir.path(), Network::Regtest).unwrap();

        let sender = "mSender123";
        let recipient = "mRecipient456";
        let txid = "multi_addr_tx";

        let tx = create_test_tx(txid, false, Some(100), Some(sender));

        // Add for both addresses
        history.add_transaction(&tx, sender).unwrap();
        history.add_transaction(&tx, recipient).unwrap();

        // Each address sees exactly 1 transaction
        let sender_result = history
            .get_transactions_for_address(sender, PaginationParams::default())
            .unwrap();
        assert_eq!(sender_result.total_count, 1);

        let recipient_result = history
            .get_transactions_for_address(recipient, PaginationParams::default())
            .unwrap();
        assert_eq!(recipient_result.total_count, 1);
    }

    #[test]
    fn test_coinbase_transactions() {
        let temp_dir = TempDir::new().unwrap();
        let history = TransactionHistory::open(temp_dir.path(), Network::Regtest).unwrap();

        let miner = "mMinerAddress";

        // Add 5 coinbase transactions
        for i in 1..=5 {
            let tx = create_test_tx(&format!("coinbase_{}", i), true, Some(i as u64), None);
            history.add_transaction(&tx, miner).unwrap();
        }

        let coinbase_txs = history.get_coinbase_transactions(miner).unwrap();
        assert_eq!(coinbase_txs.len(), 5);

        // All should be coinbase
        for tx in &coinbase_txs {
            assert!(tx.is_coinbase);
        }
    }

    #[test]
    fn test_balance_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let history = TransactionHistory::open(temp_dir.path(), Network::Regtest).unwrap();

        let address = "mBalanceTest";

        // Add UTXO directly
        let utxo = UTXO {
            txid: "utxo_test".to_string(),
            vout: 0,
            value_credits: 500_000_000, // 5 BTPC
            value_btp: 5.0,
            address: address.to_string(),
            block_height: 100,
            is_coinbase: true,
            created_at: Utc::now(),
            spent: false,
            spent_in_tx: None,
            spent_at_height: None,
            script_pubkey: vec![],
        };
        history.add_utxo(&utxo).unwrap();

        let (credits, btp) = history.get_balance(address).unwrap();
        assert_eq!(credits, 500_000_000);
        assert!((btp - 5.0).abs() < 0.0001);
    }
}
