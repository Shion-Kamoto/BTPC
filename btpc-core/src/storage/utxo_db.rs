// UTXO set storage implementation
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    blockchain::{utxo::UTXO, OutPoint},
    crypto::Hash,
    storage::database::Database,
};

/// UTXO database for storing unspent transaction outputs
#[derive(Debug)]
pub struct UtxoDb {
    db: Arc<Database>,
}

/// UTXO database interface
pub trait UTXODatabase {
    /// Store a UTXO
    fn store_utxo(&mut self, utxo: &crate::blockchain::utxo::UTXO) -> Result<(), UTXODbError>;
    /// Get a UTXO by outpoint
    fn get_utxo(
        &self,
        outpoint: &crate::blockchain::OutPoint,
    ) -> Result<Option<crate::blockchain::utxo::UTXO>, UTXODbError>;
    /// Remove a UTXO
    fn remove_utxo(&mut self, outpoint: &crate::blockchain::OutPoint) -> Result<(), UTXODbError>;

    /// Apply UTXO changes atomically (Issue #5: Race Conditions)
    /// Removes spent UTXOs and adds new UTXOs in single atomic batch
    fn apply_utxo_batch(
        &mut self,
        to_remove: &[&crate::blockchain::OutPoint],
        to_add: &[&crate::blockchain::utxo::UTXO],
    ) -> Result<(), UTXODbError>;
}

impl UtxoDb {
    /// Create a new UTXO database with the given underlying database
    pub fn new(db: Arc<Database>) -> Self {
        UtxoDb { db }
    }

    /// Get statistics about the UTXO set
    pub fn get_stats(&self) -> UtxoSetStats {
        let stats = self.db.get_statistics();
        UtxoSetStats {
            total_utxos: stats.total_keys,
            total_value: 0, // Would need to iterate all UTXOs to compute this
            db_size: stats.total_size,
        }
    }

    /// Iterate over all UTXOs with optional prefix filtering
    pub fn iter_utxos(&self) -> impl Iterator<Item = Result<(OutPoint, UTXO), UTXODbError>> + '_ {
        self.db.iter_prefix(b"utxo:").filter_map(|result| {
            match result {
                Ok((key, value)) => {
                    // Extract outpoint from key and deserialize UTXO from value
                    match Self::decode_utxo_entry(&key, &value) {
                        Ok((outpoint, utxo)) => Some(Ok((outpoint, utxo))),
                        Err(e) => Some(Err(e)),
                    }
                }
                Err(e) => Some(Err(UTXODbError::DatabaseError(e.to_string()))),
            }
        })
    }

    /// Encode an outpoint as a database key
    fn encode_utxo_key(outpoint: &OutPoint) -> Vec<u8> {
        let mut key = Vec::with_capacity(5 + 32 + 4); // "utxo:" + txid (first 32 bytes) + vout
        key.extend_from_slice(b"utxo:");
        // Use only the first 32 bytes of the 64-byte hash for storage efficiency
        key.extend_from_slice(&outpoint.txid.as_bytes()[..32]);
        key.extend_from_slice(&outpoint.vout.to_le_bytes());
        key
    }

    /// Decode a database entry into outpoint and UTXO
    fn decode_utxo_entry(key: &[u8], value: &[u8]) -> Result<(OutPoint, UTXO), UTXODbError> {
        if !key.starts_with(b"utxo:") || key.len() != 41 {
            // 5 + 32 + 4
            return Err(UTXODbError::InvalidUTXOData(
                "Invalid key format".to_string(),
            ));
        }

        // Extract txid and vout from key
        let txid_bytes: [u8; 32] = key[5..37]
            .try_into()
            .map_err(|_| UTXODbError::InvalidUTXOData("Invalid txid length".to_string()))?;
        let vout = u32::from_le_bytes(
            key[37..41]
                .try_into()
                .map_err(|_| UTXODbError::InvalidUTXOData("Invalid vout".to_string()))?,
        );

        // Convert 32-byte txid to 64-byte hash by padding with zeros
        let mut hash_bytes = [0u8; 64];
        hash_bytes[0..32].copy_from_slice(&txid_bytes);

        let outpoint = OutPoint {
            txid: Hash::from_bytes(hash_bytes),
            vout,
        };

        // Deserialize UTXO from value
        let utxo: UTXO = serde_json::from_slice(value)
            .map_err(|e| UTXODbError::InvalidUTXOData(format!("Deserialization failed: {}", e)))?;

        Ok((outpoint, utxo))
    }

    /// Store multiple UTXOs atomically
    pub fn store_utxos(&mut self, utxos: &[&UTXO]) -> Result<(), UTXODbError> {
        let pairs: Result<Vec<_>, UTXODbError> = utxos
            .iter()
            .map(|utxo| {
                let key = Self::encode_utxo_key(&utxo.outpoint);
                let value = serde_json::to_vec(utxo).map_err(|e| {
                    UTXODbError::InvalidUTXOData(format!("Serialization failed: {}", e))
                })?;
                Ok((key, value))
            })
            .collect();

        let pairs = pairs?;
        let key_value_refs: Vec<(&[u8], &[u8])> = pairs
            .iter()
            .map(|(k, v)| (k.as_slice(), v.as_slice()))
            .collect();

        self.db
            .put_batch(&key_value_refs)
            .map_err(|e| UTXODbError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Remove multiple UTXOs atomically
    pub fn remove_utxos(&mut self, outpoints: &[&OutPoint]) -> Result<(), UTXODbError> {
        let keys: Vec<Vec<u8>> = outpoints
            .iter()
            .map(|outpoint| Self::encode_utxo_key(outpoint))
            .collect();

        let key_refs: Vec<&[u8]> = keys.iter().map(|k| k.as_slice()).collect();

        self.db
            .delete_batch(&key_refs)
            .map_err(|e| UTXODbError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Flush pending UTXO database writes to disk
    pub fn flush(&self) -> Result<(), UTXODbError> {
        self.db
            .flush()
            .map_err(|e| UTXODbError::DatabaseError(e.to_string()))
    }

    /// Compact the UTXO database
    pub fn compact(&self) -> Result<(), UTXODbError> {
        self.db
            .compact()
            .map_err(|e| UTXODbError::DatabaseError(e.to_string()))
    }
}

/// UTXO set statistics
#[derive(Debug, Clone)]
pub struct UtxoSetStats {
    /// Total number of UTXOs
    pub total_utxos: u64,
    /// Total value of all UTXOs (in base units)
    pub total_value: u64,
    /// Database size in bytes
    pub db_size: u64,
}

impl UTXODatabase for UtxoDb {
    fn store_utxo(&mut self, utxo: &UTXO) -> Result<(), UTXODbError> {
        let key = Self::encode_utxo_key(&utxo.outpoint);
        let value = serde_json::to_vec(utxo)
            .map_err(|e| UTXODbError::InvalidUTXOData(format!("Serialization failed: {}", e)))?;

        self.db
            .put(&key, &value)
            .map_err(|e| UTXODbError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    fn get_utxo(&self, outpoint: &OutPoint) -> Result<Option<UTXO>, UTXODbError> {
        let key = Self::encode_utxo_key(outpoint);

        match self.db.get(&key) {
            Ok(Some(value)) => {
                let utxo: UTXO = serde_json::from_slice(&value).map_err(|e| {
                    UTXODbError::InvalidUTXOData(format!("Deserialization failed: {}", e))
                })?;
                Ok(Some(utxo))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(UTXODbError::DatabaseError(e.to_string())),
        }
    }

    fn remove_utxo(&mut self, outpoint: &OutPoint) -> Result<(), UTXODbError> {
        let key = Self::encode_utxo_key(outpoint);

        // Check if UTXO exists before removal
        match self.db.exists(&key) {
            Ok(true) => {
                self.db
                    .delete(&key)
                    .map_err(|e| UTXODbError::DatabaseError(e.to_string()))?;
                Ok(())
            }
            Ok(false) => Err(UTXODbError::UTXONotFound),
            Err(e) => Err(UTXODbError::DatabaseError(e.to_string())),
        }
    }

    fn apply_utxo_batch(
        &mut self,
        to_remove: &[&OutPoint],
        to_add: &[&UTXO],
    ) -> Result<(), UTXODbError> {
        // Encode keys for removal
        let remove_keys: Vec<Vec<u8>> = to_remove
            .iter()
            .map(|outpoint| Self::encode_utxo_key(outpoint))
            .collect();

        // Encode key-value pairs for addition
        let add_pairs: Result<Vec<_>, UTXODbError> = to_add
            .iter()
            .map(|utxo| {
                let key = Self::encode_utxo_key(&utxo.outpoint);
                let value = serde_json::to_vec(utxo).map_err(|e| {
                    UTXODbError::InvalidUTXOData(format!("Serialization failed: {}", e))
                })?;
                Ok((key, value))
            })
            .collect();
        let add_pairs = add_pairs?;

        // Apply atomic batch (remove + add)
        let remove_refs: Vec<&[u8]> = remove_keys.iter().map(|k| k.as_slice()).collect();
        let add_refs: Vec<(&[u8], &[u8])> = add_pairs
            .iter()
            .map(|(k, v)| (k.as_slice(), v.as_slice()))
            .collect();

        self.db
            .write_batch(&remove_refs, &add_refs)
            .map_err(|e| UTXODbError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

/// UTXO database errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum UTXODbError {
    #[error("Database operation failed: {0}")]
    DatabaseError(String),
    #[error("UTXO not found")]
    UTXONotFound,
    #[error("Invalid UTXO data: {0}")]
    InvalidUTXOData(String),
}
