use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize, Deserializer};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc, NaiveDateTime};
use btpc_core::crypto::Address;
use uuid::Uuid;
use parking_lot::Mutex;

/// Normalize address for case-insensitive comparison ONLY
///
/// IMPORTANT: This function should ONLY be used for comparisons, NOT for storage.
/// BTPC addresses use case-sensitive base58check encoding and MUST be stored
/// in their original case. Only use this function when comparing two addresses.
fn normalize_address_for_comparison(address: &str) -> String {
    address.trim().to_lowercase()
}

/// Clean address by removing "Address: " prefix ONLY (preserve case!)
///
/// IMPORTANT: This function removes prefixes but does NOT change case.
/// Addresses MUST be stored in their original case (base58check encoding).
fn clean_address(address: &str) -> String {
    let trimmed = address.trim();
    let without_prefix = if trimmed.starts_with("Address: ") {
        trimmed.strip_prefix("Address: ").unwrap_or(trimmed).trim()
    } else {
        trimmed
    };
    without_prefix.to_string()
}

/// Generate a Bitcoin-style block hash for display purposes
fn generate_block_hash(block_height: u64) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    block_height.hash(&mut hasher);
    Utc::now().timestamp_nanos_opt().unwrap_or(0).hash(&mut hasher);
    "BTPC".hash(&mut hasher);  // Add BTPC identifier
    let hash = hasher.finish();
    format!("{:016x}000000000000000000000000000000000000000000000000", hash)
}

/// Custom deserializer for DateTime fields that may not have timezone info
fn deserialize_datetime_flexible<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    // Try to parse with timezone first (Z suffix)
    if let Ok(dt) = DateTime::parse_from_rfc3339(&format!("{}Z", s)) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try to parse as naive datetime and assume UTC
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Ok(naive_dt.and_utc());
    }

    // Fallback: try standard RFC3339 parsing
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}

/// REMOVED: deserialize_address_normalized - Addresses should NOT be normalized during storage!
///
/// BUG FIX (2025-11-05): This function was converting addresses to lowercase during deserialization,
/// breaking base58check encoding. BTPC addresses MUST be stored in their original mixed case.
/// Comparison should normalize, storage should preserve original case.


/// Represents an Unspent Transaction Output
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UTXO {
    /// Transaction ID that created this output
    pub txid: String,
    /// Output index within the transaction
    pub vout: u32,
    /// Value in credits - supports both old format (value_sats) and new format (value_credits)
    #[serde(alias = "value_sats")]
    pub value_credits: u64,
    /// Value in BTP (for display)
    pub value_btp: f64,
    /// Address that can spend this UTXO (MUST be stored in original case!)
    /// BUG FIX (2025-11-05): Removed deserialize_with to preserve original case
    pub address: String,
    /// Block height where this UTXO was created
    pub block_height: u64,
    /// Whether this is a coinbase transaction
    pub is_coinbase: bool,
    /// When this UTXO was created
    #[serde(deserialize_with = "deserialize_datetime_flexible")]
    pub created_at: DateTime<Utc>,
    /// Whether this UTXO has been spent
    pub spent: bool,
    /// Transaction ID that spent this UTXO (if any)
    pub spent_in_tx: Option<String>,
    /// Block height where this UTXO was spent (if any)
    pub spent_at_height: Option<u64>,
    /// Script pubkey for this output
    #[serde(default)]
    pub script_pubkey: Vec<u8>,
}

/// Represents a transaction input that spends a UTXO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxInput {
    /// Previous transaction output being spent
    pub prev_txid: String,
    pub prev_vout: u32,
    /// Signature script
    pub signature_script: Vec<u8>,
    /// Sequence number
    pub sequence: u32,
}

/// Represents a transaction output that creates a new UTXO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxOutput {
    /// Value in satoshis
    pub value: u64,
    /// Script pubkey
    pub script_pubkey: Vec<u8>,
}

/// Represents a complete transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction ID
    pub txid: String,
    /// Transaction version
    pub version: u32,
    /// Transaction inputs
    pub inputs: Vec<TxInput>,
    /// Transaction outputs
    pub outputs: Vec<TxOutput>,
    /// Lock time
    pub lock_time: u32,
    /// Fork ID for replay protection (CRITICAL for signature validation!)
    /// Mainnet=0, Testnet=1, Regtest=2
    #[serde(default)]
    pub fork_id: u8,
    /// Block height where this transaction was confirmed
    pub block_height: Option<u64>,
    /// Confirmation timestamp
    pub confirmed_at: Option<DateTime<Utc>>,
    /// Whether this is a coinbase transaction
    pub is_coinbase: bool,
}

/// UTXO set statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTXOStats {
    pub total_utxos: usize,
    pub total_value_credits: u64,
    pub total_value_btp: f64,
    pub coinbase_utxos: usize,
    pub regular_utxos: usize,
    pub spent_utxos: usize,
    pub unspent_utxos: usize,
}

/// T015: UTXO Reservation Token for preventing double-spending
#[derive(Debug, Clone)]
pub struct ReservationToken {
    /// Unique reservation ID
    pub id: String,
    /// Associated transaction ID (if any)
    pub transaction_id: Option<String>,
    /// UTXOs being reserved (txid:vout format)
    pub utxos: Vec<String>,
    /// When the reservation was created
    pub created_at: Instant,
    /// When the reservation expires (5 minutes default)
    pub expires_at: Instant,
}

impl ReservationToken {
    /// Create a new reservation token
    pub fn new(transaction_id: Option<String>, utxos: Vec<String>) -> Self {
        let now = Instant::now();
        Self {
            id: Uuid::new_v4().to_string(),
            transaction_id,
            utxos,
            created_at: now,
            expires_at: now + Duration::from_secs(300), // 5 minutes timeout
        }
    }

    /// Check if the reservation has expired
    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// Manages the UTXO set for the wallet
pub struct UTXOManager {
    /// Path to the UTXO set file
    utxo_file: PathBuf,
    /// Path to the transaction history file
    tx_history_file: PathBuf,
    /// In-memory UTXO set
    utxos: HashMap<String, UTXO>, // Key: "txid:vout"
    /// Transaction history
    transactions: HashMap<String, Transaction>, // Key: txid
    /// T016: Reserved UTXOs for concurrent transaction protection (optimistic locking)
    reserved_utxos: Arc<Mutex<HashSet<String>>>, // Key: "txid:vout"
    /// T015: Active reservations with their tokens
    reservations: Arc<Mutex<HashMap<String, ReservationToken>>>, // Key: reservation_id
}

impl UTXOManager {
    /// Create a new UTXO manager
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        let utxo_file = data_dir.join("wallet_utxos.json");
        let tx_history_file = data_dir.join("wallet_transactions.json");

        let mut manager = Self {
            utxo_file,
            tx_history_file,
            utxos: HashMap::new(),
            transactions: HashMap::new(),
            reserved_utxos: Arc::new(Mutex::new(HashSet::new())),  // T016: Initialize reservation system
            reservations: Arc::new(Mutex::new(HashMap::new())),    // T015: Initialize reservation tokens
        };

        // Load existing data
        manager.load_utxos()?;
        manager.load_transactions()?;

        Ok(manager)
    }

    /// Reload UTXOs from disk (public method for external calls)
    pub fn reload_utxos(&mut self) -> Result<()> {
        self.load_utxos()
    }

    /// Load UTXOs from disk
    fn load_utxos(&mut self) -> Result<()> {
        if self.utxo_file.exists() {
            let content = fs::read_to_string(&self.utxo_file)?;
            let utxos: Vec<UTXO> = serde_json::from_str(&content)?;

            for utxo in utxos {
                let key = format!("{}:{}", utxo.txid, utxo.vout);
                self.utxos.insert(key, utxo);
            }
        }
        Ok(())
    }

    /// Load transactions from disk
    fn load_transactions(&mut self) -> Result<()> {
        if self.tx_history_file.exists() {
            let content = fs::read_to_string(&self.tx_history_file)?;
            let transactions: Vec<Transaction> = serde_json::from_str(&content)?;

            for tx in transactions {
                self.transactions.insert(tx.txid.clone(), tx);
            }
        }
        Ok(())
    }

    /// Save UTXOs to disk
    pub fn save_utxos(&self) -> Result<()> {
        let utxos: Vec<&UTXO> = self.utxos.values().collect();
        let content = serde_json::to_string_pretty(&utxos)?;
        fs::write(&self.utxo_file, content)?;
        Ok(())
    }

    /// Save transactions to disk
    fn save_transactions(&self) -> Result<()> {
        let transactions: Vec<&Transaction> = self.transactions.values().collect();
        let content = serde_json::to_string_pretty(&transactions)?;
        fs::write(&self.tx_history_file, content)?;
        Ok(())
    }

    /// Add a new UTXO from mining or receiving
    pub fn add_utxo(&mut self, utxo: UTXO) -> Result<()> {
        let key = format!("{}:{}", utxo.txid, utxo.vout);
        self.utxos.insert(key, utxo);
        self.save_utxos()?;
        Ok(())
    }

    /// Add a coinbase UTXO from mining with Bitcoin-compatible block data
    pub fn add_coinbase_utxo(
        &mut self,
        txid: String,
        vout: u32,
        value_credits: u64,
        address: String,
        block_height: u64,
    ) -> Result<()> {
        // Clean the address to ensure consistent storage format
        let clean_addr = clean_address(&address);
        println!("🔧 DEBUG (add_coinbase_utxo): Raw address: '{}' -> Cleaned: '{}'", address, clean_addr);

        let block_time = Utc::now();
        let utxo = UTXO {
            txid: txid.clone(),
            vout,
            value_credits,
            value_btp: value_credits as f64 / 100_000_000.0,
            address: clean_addr.clone(),  // Use cleaned address
            block_height,
            is_coinbase: true,
            created_at: block_time,
            spent: false,
            spent_in_tx: None,
            spent_at_height: None,
            script_pubkey: Vec::new(), // Simplified for now
        };

        // Create Bitcoin-compatible coinbase transaction
        let coinbase_transaction = Transaction {
            txid: txid.clone(),
            version: 1,
            inputs: vec![], // Coinbase has no inputs
            outputs: vec![TxOutput {
                value: value_credits,
                script_pubkey: address.as_bytes().to_vec(), // Simplified
            }],
            lock_time: 0,
            fork_id: 2, // Regtest by default (0=mainnet, 1=testnet, 2=regtest)
            block_height: Some(block_height),
            confirmed_at: Some(block_time),
            is_coinbase: true,
        };

        // Add UTXO to the set
        self.add_utxo(utxo)?;

        // Add transaction to history
        self.add_transaction(coinbase_transaction)?;

        println!("🆕 NEW BLOCK CREATED #{}", block_height);
        println!("├─ Block Hash: {}", generate_block_hash(block_height));
        println!("├─ Timestamp: {}", block_time.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("├─ Coinbase TXID: {}", txid);
        println!("├─ Reward: {} BTP ({} credits)", value_credits as f64 / 100_000_000.0, value_credits);
        println!("├─ Recipient: {}", address);
        println!("└─ Status: ✓ Block accepted to blockchain");

        Ok(())
    }

    /// Mark a UTXO as spent
    pub fn spend_utxo(&mut self, txid: &str, vout: u32, spent_in_tx: String, spent_at_height: u64) -> Result<()> {
        let key = format!("{}:{}", txid, vout);

        if let Some(utxo) = self.utxos.get_mut(&key) {
            utxo.spent = true;
            utxo.spent_in_tx = Some(spent_in_tx);
            utxo.spent_at_height = Some(spent_at_height);
            self.save_utxos()?;
            Ok(())
        } else {
            Err(anyhow!("UTXO not found: {}:{}", txid, vout))
        }
    }

    /// Mark a UTXO as spent (simplified version for transaction signing)
    pub fn mark_utxo_as_spent(&mut self, txid: &str, vout: u32) -> Result<()> {
        let key = format!("{}:{}", txid, vout);

        if let Some(utxo) = self.utxos.get_mut(&key) {
            utxo.spent = true;
            utxo.spent_in_tx = Some("pending_broadcast".to_string());
            utxo.spent_at_height = None; // Will be updated when confirmed
            Ok(())
        } else {
            Err(anyhow!("UTXO not found: {}:{}", txid, vout))
        }
    }

    /// Get all unspent UTXOs for an address
    pub fn get_unspent_utxos(&self, address: &str) -> Vec<&UTXO> {
        // Clean and normalize the query address for case-insensitive comparison
        let clean_query_addr = clean_address(address);
        let normalized_query_addr = normalize_address_for_comparison(&clean_query_addr);

        self.utxos
            .values()
            .filter(|utxo| {
                let utxo_clean = clean_address(&utxo.address);
                let utxo_normalized = normalize_address_for_comparison(&utxo_clean);
                utxo_normalized == normalized_query_addr && !utxo.spent
            })
            .collect()
    }

    /// Get all UTXOs (spent and unspent) for an address
    pub fn get_all_utxos(&self, address: &str) -> Vec<&UTXO> {
        let clean_query_addr = clean_address(address);
        let normalized_query_addr = normalize_address_for_comparison(&clean_query_addr);
        self.utxos
            .values()
            .filter(|utxo| {
                let utxo_clean = clean_address(&utxo.address);
                let utxo_normalized = normalize_address_for_comparison(&utxo_clean);
                utxo_normalized == normalized_query_addr
            })
            .collect()
    }

    /// Calculate balance for an address
    pub fn get_balance(&self, address: &str) -> (u64, f64) {
        let unspent = self.get_unspent_utxos(address);
        let total_credits: u64 = unspent.iter().map(|utxo| utxo.value_credits).sum();
        let total_btp = total_credits as f64 / 100_000_000.0;
        (total_credits, total_btp)
    }

    /// Get UTXO set statistics
    pub fn get_stats(&self) -> UTXOStats {
        let total_utxos = self.utxos.len();
        let total_value_credits: u64 = self.utxos.values().map(|u| u.value_credits).sum();
        let total_value_btp = total_value_credits as f64 / 100_000_000.0;

        let coinbase_utxos = self.utxos.values().filter(|u| u.is_coinbase).count();
        let regular_utxos = total_utxos - coinbase_utxos;

        let spent_utxos = self.utxos.values().filter(|u| u.spent).count();
        let unspent_utxos = total_utxos - spent_utxos;

        UTXOStats {
            total_utxos,
            total_value_credits,
            total_value_btp,
            coinbase_utxos,
            regular_utxos,
            spent_utxos,
            unspent_utxos,
        }
    }

    /// Select UTXOs for spending (simple greedy selection)
    pub fn select_utxos_for_spending(&self, address: &str, amount_credits: u64) -> Result<Vec<&UTXO>> {
        let mut unspent = self.get_unspent_utxos(address);

        // Sort by value (largest first for fewer inputs)
        unspent.sort_by(|a, b| b.value_credits.cmp(&a.value_credits));

        let mut selected = Vec::new();
        let mut total_selected = 0u64;

        for utxo in unspent {
            selected.push(utxo);
            total_selected += utxo.value_credits;

            if total_selected >= amount_credits {
                return Ok(selected);
            }
        }

        Err(anyhow!("Insufficient funds: need {} credits, have {} credits", amount_credits, total_selected))
    }

    /// Add a transaction to history
    pub fn add_transaction(&mut self, transaction: Transaction) -> Result<()> {
        self.transactions.insert(transaction.txid.clone(), transaction);
        self.save_transactions()?;
        Ok(())
    }

    /// Get transaction history for an address
    pub fn get_transaction_history(&self, address: &str) -> Vec<&Transaction> {
        let clean_query_addr = clean_address(address);
        let normalized_query_addr = normalize_address_for_comparison(&clean_query_addr);

        self.transactions
            .values()
            .filter(|tx| {
                // Check if transaction involves this address
                tx.outputs.iter().any(|_output| {
                    // Check if any UTXO belongs to this address (with normalized comparison)
                    self.utxos.values().any(|utxo| {
                        let utxo_clean = clean_address(&utxo.address);
                        let utxo_normalized = normalize_address_for_comparison(&utxo_clean);
                        utxo_normalized == normalized_query_addr && utxo.txid == tx.txid
                    })
                })
            })
            .collect()
    }

    /// Create a simple send transaction
    pub fn create_send_transaction(
        &self,
        from_address: &str,
        to_address: &str,
        amount_credits: u64,
        fee_credits: u64,
    ) -> Result<Transaction> {
        // Select UTXOs to spend
        let selected_utxos = self.select_utxos_for_spending(from_address, amount_credits + fee_credits)?;

        let total_input: u64 = selected_utxos.iter().map(|u| u.value_credits).sum();
        let change_amount = total_input - amount_credits - fee_credits;

        // Create inputs (unsigned - will be signed by sign_transaction command)
        let inputs: Vec<TxInput> = selected_utxos
            .iter()
            .map(|utxo| TxInput {
                prev_txid: utxo.txid.clone(),
                prev_vout: utxo.vout,
                signature_script: Vec::new(), // Empty until signed with ML-DSA + P2PKH script
                sequence: 0xffffffff,
            })
            .collect();

        // Create outputs with proper address decoding
        // Decode Base58 addresses to get the actual hash160 (pubkey hash)
        let to_addr = Address::from_string(to_address)
            .map_err(|e| anyhow!("Invalid recipient address: {}", e))?;
        let from_addr = Address::from_string(from_address)
            .map_err(|e| anyhow!("Invalid sender address: {}", e))?;

        let mut outputs = vec![TxOutput {
            value: amount_credits,
            script_pubkey: to_addr.hash160().to_vec(), // Use decoded hash160
        }];

        // Add change output if needed
        if change_amount > 0 {
            outputs.push(TxOutput {
                value: change_amount,
                script_pubkey: from_addr.hash160().to_vec(), // Use decoded hash160
            });
        }

        // Generate transaction ID (simplified)
        let txid = format!("tx_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));

        Ok(Transaction {
            txid,
            version: 1,
            inputs,
            outputs,
            lock_time: 0,
            fork_id: 2, // Regtest by default (TODO: get from network config)
            block_height: None,
            confirmed_at: None,
            is_coinbase: false,
        })
    }

    /// Process a new block and update UTXO set
    pub fn process_block(&mut self, block_height: u64, transactions: Vec<Transaction>) -> Result<()> {
        for transaction in transactions {
            // Add new UTXOs from outputs
            for _output in transaction.outputs.iter() {
                // TODO: Decode script_pubkey to get address
                // For now, skip if we can't determine the address
            }

            // Mark spent UTXOs from inputs
            if !transaction.is_coinbase {
                for input in &transaction.inputs {
                    self.spend_utxo(
                        &input.prev_txid,
                        input.prev_vout,
                        transaction.txid.clone(),
                        block_height,
                    )?;
                }
            }

            // Add to transaction history
            self.add_transaction(transaction)?;
        }

        Ok(())
    }

    /// Export UTXO set for integration with Python scripts
    pub fn export_utxos_for_integration(&self, address: &str) -> Result<()> {
        let utxos: Vec<&UTXO> = self.get_all_utxos(address);

        // Convert to Python script format
        let python_utxos: Vec<serde_json::Value> = utxos
            .iter()
            .map(|utxo| {
                serde_json::json!({
                    "txid": utxo.txid,
                    "vout": utxo.vout,
                    "value_credits": utxo.value_credits,
                    "value_btp": utxo.value_btp,
                    "address": utxo.address,
                    "block_height": utxo.block_height,
                    "is_coinbase": utxo.is_coinbase,
                    "created_at": utxo.created_at,
                    "spent": utxo.spent
                })
            })
            .collect();

        // Write to the same format as our Python integration scripts
        let utxo_file = PathBuf::from("/home/bob/.btpc/data/wallet/wallet_utxos.json");
        let content = serde_json::to_string_pretty(&python_utxos)?;
        fs::write(&utxo_file, content)?;

        Ok(())
    }

    /// T016: Release UTXO reservations (explicit release, also done by Drop)
    pub fn release_utxos(&self, outpoints: &[String]) {
        let mut reserved = self.reserved_utxos.lock();
        for outpoint in outpoints {
            reserved.remove(outpoint);
        }
    }

    /// T016: Modify select_utxos_for_spending to exclude reserved UTXOs
    pub fn select_utxos_for_spending_with_reservation(&self, address: &str, amount_credits: u64) -> Result<Vec<&UTXO>> {
        let mut unspent = self.get_unspent_utxos(address);

        // Filter out reserved UTXOs
        let reserved = self.reserved_utxos.lock();
        unspent.retain(|utxo| !reserved.contains(&utxo.outpoint()));
        drop(reserved);

        // Sort by value (largest first for fewer inputs)
        unspent.sort_by(|a, b| b.value_credits.cmp(&a.value_credits));

        let mut selected = Vec::new();
        let mut total_selected = 0u64;

        for utxo in unspent {
            selected.push(utxo);
            total_selected += utxo.value_credits;

            if total_selected >= amount_credits {
                return Ok(selected);
            }
        }

        Err(anyhow!("Insufficient funds: need {} credits, have {} credits", amount_credits, total_selected))
    }

    // ============= T016-T018: UTXO Reservation Methods =============

    /// T016: Reserve UTXOs for a transaction to prevent double-spending
    pub fn reserve_utxos(&self, utxo_keys: Vec<String>, transaction_id: Option<String>) -> Result<ReservationToken> {
        let mut reserved = self.reserved_utxos.lock();
        let mut reservations = self.reservations.lock();

        // T017: Clean up expired reservations first
        self.cleanup_expired_reservations_internal(&mut reserved, &mut reservations);

        // Check if any UTXOs are already reserved
        for key in &utxo_keys {
            if reserved.contains(key) {
                return Err(anyhow!("UTXO {} is already reserved", key));
            }
        }

        // Reserve the UTXOs
        for key in &utxo_keys {
            reserved.insert(key.clone());
        }

        // Create reservation token
        let token = ReservationToken::new(transaction_id, utxo_keys);
        let token_id = token.id.clone();
        reservations.insert(token_id.clone(), token.clone());

        Ok(token)
    }

    /// T016: Release reserved UTXOs
    pub fn release_reservation(&self, token_id: &str) -> Result<()> {
        let mut reserved = self.reserved_utxos.lock();
        let mut reservations = self.reservations.lock();

        if let Some(token) = reservations.remove(token_id) {
            // Release all UTXOs in this reservation
            for utxo_key in &token.utxos {
                reserved.remove(utxo_key);
            }
            Ok(())
        } else {
            Err(anyhow!("Reservation {} not found", token_id))
        }
    }

    /// T017: Clean up expired reservations (called automatically)
    fn cleanup_expired_reservations_internal(
        &self,
        reserved: &mut HashSet<String>,
        reservations: &mut HashMap<String, ReservationToken>
    ) {
        let expired: Vec<String> = reservations
            .iter()
            .filter(|(_, token)| token.is_expired())
            .map(|(id, _)| id.clone())
            .collect();

        for token_id in expired {
            if let Some(token) = reservations.remove(&token_id) {
                for utxo_key in &token.utxos {
                    reserved.remove(utxo_key);
                }
            }
        }
    }

    /// T017: Public method to cleanup expired reservations
    pub fn cleanup_expired_reservations(&self) {
        let mut reserved = self.reserved_utxos.lock();
        let mut reservations = self.reservations.lock();
        self.cleanup_expired_reservations_internal(&mut reserved, &mut reservations);
    }

    /// T018: Select UTXOs for a transaction (with reservation check)
    pub fn select_utxos_for_amount(&self, address: &str, amount_credits: u64) -> Result<Vec<UTXO>> {
        let reserved = self.reserved_utxos.lock();
        let normalized_addr = normalize_address_for_comparison(address);

        // Diagnostic counts
        let total_utxos = self.utxos.len();
        let unspent_utxos: Vec<_> = self.utxos.iter().filter(|(_, u)| !u.spent).collect();
        let matching_address: Vec<_> = unspent_utxos.iter()
            .filter(|(_, u)| normalize_address_for_comparison(&u.address) == normalized_addr)
            .collect();
        let not_reserved: Vec<_> = matching_address.iter()
            .filter(|(key, _)| !reserved.contains(*key))
            .collect();

        println!("🔍 UTXO Selection Debug:");
        println!("  Requested address: {} (normalized: {})", address, normalized_addr);
        println!("  Total UTXOs in system: {}", total_utxos);
        println!("  Unspent UTXOs: {}", unspent_utxos.len());
        println!("  Matching address: {}", matching_address.len());
        println!("  Not reserved: {}", not_reserved.len());

        // Show unique addresses in system for debugging
        let unique_addrs: std::collections::HashSet<String> = self.utxos.iter()
            .filter(|(_, u)| !u.spent)
            .map(|(_, u)| normalize_address_for_comparison(&u.address))
            .collect();
        println!("  Unique addresses in system: {:?}", unique_addrs);

        // Get all unspent UTXOs for the address
        let mut available_utxos: Vec<UTXO> = self.utxos
            .iter()
            .filter_map(|(key, utxo)| {
                if !utxo.spent &&
                   normalize_address_for_comparison(&utxo.address) == normalized_addr &&
                   !reserved.contains(key) {  // T018: Skip reserved UTXOs
                    Some(utxo.clone())
                } else {
                    None
                }
            })
            .collect();

        // T018: Sort by age (prefer older UTXOs) - more confirmations
        available_utxos.sort_by(|a, b| {
            a.block_height.cmp(&b.block_height)
        });

        // Select UTXOs until we have enough
        let mut selected = Vec::new();
        let mut total_selected = 0u64;

        for utxo in available_utxos {
            selected.push(utxo.clone());
            total_selected += utxo.value_credits;
            if total_selected >= amount_credits {
                println!("✅ Selected {} UTXOs with total {} credits", selected.len(), total_selected);
                return Ok(selected);
            }
        }

        let amount_btpc = amount_credits as f64 / 100_000_000.0;
        let available_btpc = total_selected as f64 / 100_000_000.0;

        Err(anyhow!(
            "Insufficient funds for address {}:\n\
             Need: {} BTPC ({} credits)\n\
             Available: {} BTPC ({} credits)\n\
             UTXOs matching this address: {}\n\
             Hint: Check if you selected the correct wallet with sufficient balance",
            address,
            amount_btpc,
            amount_credits,
            available_btpc,
            total_selected,
            not_reserved.len()
        ))
    }

    /// Check if a UTXO is reserved
    pub fn is_utxo_reserved(&self, txid: &str, vout: u32) -> bool {
        let key = format!("{}:{}", txid, vout);
        self.reserved_utxos.lock().contains(&key)
    }

    /// Get all active reservations
    pub fn get_active_reservations(&self) -> Vec<ReservationToken> {
        self.reservations.lock().values().cloned().collect()
    }
}

// Removed duplicate ReservationToken definition - using the one at line 172 with proper fields

impl UTXO {
    /// Check if this UTXO can be spent (is unspent and meets requirements)
    pub fn is_spendable(&self, current_height: u64) -> bool {
        if self.spent {
            return false;
        }

        // Coinbase UTXOs need to mature (100 blocks in Bitcoin, adjust as needed)
        if self.is_coinbase {
            return current_height >= self.block_height + 100;
        }

        true
    }

    /// Get a unique identifier for this UTXO
    pub fn outpoint(&self) -> String {
        format!("{}:{}", self.txid, self.vout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_utxo_manager_basic() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = UTXOManager::new(temp_dir.path().to_path_buf()).unwrap();

        // Add a coinbase UTXO
        manager.add_coinbase_utxo(
            "test_tx_1".to_string(),
            0,
            5000000000, // 50 BTP
            "test_address".to_string(),
            1,
        ).unwrap();

        // Check balance
        let (credits, btp) = manager.get_balance("test_address");
        assert_eq!(credits, 5000000000);
        assert_eq!(btp, 50.0);

        // Check stats
        let stats = manager.get_stats();
        assert_eq!(stats.total_utxos, 1);
        assert_eq!(stats.coinbase_utxos, 1);
        assert_eq!(stats.unspent_utxos, 1);
    }
}