//! Transaction structures and operations for BTPC
//!
//! BTPC uses Bitcoin-compatible transaction structure with ML-DSA signatures.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    blockchain::constants::*,
    crypto::{Hash, Script},
};

/// A BTPC transaction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction version
    pub version: u32,
    /// Transaction inputs
    pub inputs: Vec<TransactionInput>,
    /// Transaction outputs
    pub outputs: Vec<TransactionOutput>,
    /// Lock time (block height or timestamp)
    pub lock_time: u32,
    /// Fork ID for replay protection (Issue #6)
    /// 0 = Mainnet, 1 = Testnet, 2 = Regtest
    pub fork_id: u8,
}

impl Transaction {
    /// Create a new transaction
    pub fn new() -> Self {
        Transaction {
            version: 1,
            inputs: Vec::new(),
            outputs: Vec::new(),
            lock_time: 0,
            fork_id: 0, // Default to mainnet
        }
    }

    /// Create a coinbase transaction for mining
    ///
    /// # Arguments
    /// * `reward` - Block reward in satoshis
    /// * `recipient_hash` - Hash of recipient public key
    ///
    /// # Note
    /// Creates transaction with fork_id=0 (mainnet). Caller must update fork_id
    /// for testnet (1) or regtest (2) networks.
    pub fn coinbase(reward: u64, recipient_hash: Hash) -> Self {
        let coinbase_input = TransactionInput {
            previous_output: OutPoint {
                txid: Hash::zero(),
                vout: 0xffffffff,
            },
            script_sig: Script::new(), // Will be filled with coinbase data
            sequence: 0xffffffff,
        };

        let coinbase_output = TransactionOutput {
            value: reward,
            script_pubkey: Script::pay_to_pubkey_hash(recipient_hash),
        };

        Transaction {
            version: 1,
            inputs: vec![coinbase_input],
            outputs: vec![coinbase_output],
            lock_time: 0,
            fork_id: 0, // Default to mainnet (caller should update for testnet=1 or regtest=2)
        }
    }

    /// Check if this is a coinbase transaction
    pub fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1
            && self.inputs[0].previous_output.txid == Hash::zero()
            && self.inputs[0].previous_output.vout == 0xffffffff
    }

    /// Calculate the transaction hash (txid)
    pub fn hash(&self) -> Hash {
        let serialized = self.serialize();
        Hash::double_sha512(&serialized)
    }

    /// Get transaction size in bytes
    pub fn size(&self) -> usize {
        self.serialize().len()
    }

    /// Check if transaction exceeds size limits
    pub fn is_oversized(&self) -> bool {
        self.size() > MAX_TRANSACTION_SIZE
    }

    /// Validate basic transaction structure
    pub fn validate_structure(&self) -> Result<(), TransactionError> {
        // Check version
        if self.version == 0 {
            return Err(TransactionError::InvalidVersion);
        }

        // Check inputs
        if self.inputs.is_empty() {
            return Err(TransactionError::NoInputs);
        }

        if self.inputs.len() > MAX_TRANSACTION_INPUTS {
            return Err(TransactionError::TooManyInputs);
        }

        // Check outputs
        if self.outputs.is_empty() {
            return Err(TransactionError::NoOutputs);
        }

        if self.outputs.len() > MAX_TRANSACTION_OUTPUTS {
            return Err(TransactionError::TooManyOutputs);
        }

        // Check size
        if self.is_oversized() {
            return Err(TransactionError::TransactionTooLarge);
        }

        // Validate outputs
        for output in &self.outputs {
            output.validate()?;
        }

        // Validate inputs (basic structure only)
        for input in &self.inputs {
            input.validate()?;
        }

        // Check for duplicate inputs (prevent double-spending)
        for i in 0..self.inputs.len() {
            for j in (i + 1)..self.inputs.len() {
                if self.inputs[i].previous_output == self.inputs[j].previous_output {
                    return Err(TransactionError::DuplicateInput);
                }
            }
        }

        Ok(())
    }

    /// Calculate total input value (requires UTXO set)
    pub fn calculate_input_value(
        &self,
        utxo_set: &crate::blockchain::UTXOSet,
    ) -> Result<u64, TransactionError> {
        let mut total = 0u64;

        for input in &self.inputs {
            if self.is_coinbase() {
                continue; // Coinbase has no real inputs
            }

            let utxo = utxo_set
                .get_utxo(&input.previous_output)
                .ok_or(TransactionError::UTXONotFound)?;

            total = total
                .checked_add(utxo.output.value)
                .ok_or(TransactionError::ValueOverflow)?;
        }

        Ok(total)
    }

    /// Calculate total output value
    pub fn calculate_output_value(&self) -> Result<u64, TransactionError> {
        let mut total = 0u64;

        for output in &self.outputs {
            total = total
                .checked_add(output.value)
                .ok_or(TransactionError::ValueOverflow)?;
        }

        Ok(total)
    }

    /// Calculate transaction fee (input_value - output_value)
    pub fn calculate_fee(
        &self,
        utxo_set: &crate::blockchain::UTXOSet,
    ) -> Result<u64, TransactionError> {
        if self.is_coinbase() {
            return Ok(0); // Coinbase transactions don't pay fees
        }

        let input_value = self.calculate_input_value(utxo_set)?;
        let output_value = self.calculate_output_value()?;

        if input_value < output_value {
            return Err(TransactionError::InsufficientInputs);
        }

        Ok(input_value - output_value)
    }

    /// Serialize transaction to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Version (4 bytes, little endian)
        bytes.extend_from_slice(&self.version.to_le_bytes());

        // Input count (variable length integer)
        Self::write_varint(&mut bytes, self.inputs.len() as u64);

        // Inputs
        for input in &self.inputs {
            input.serialize_into(&mut bytes);
        }

        // Output count (variable length integer)
        Self::write_varint(&mut bytes, self.outputs.len() as u64);

        // Outputs
        for output in &self.outputs {
            output.serialize_into(&mut bytes);
        }

        // Lock time (4 bytes, little endian)
        bytes.extend_from_slice(&self.lock_time.to_le_bytes());

        // Fork ID (1 byte) - Issue #6: Replay Protection
        bytes.push(self.fork_id);

        bytes
    }

    /// Serialize transaction for signature verification
    /// This version excludes signature scripts to prevent signatures from signing themselves
    pub fn serialize_for_signature(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Version (4 bytes, little endian)
        bytes.extend_from_slice(&self.version.to_le_bytes());

        // Input count (variable length integer)
        Self::write_varint(&mut bytes, self.inputs.len() as u64);

        // Inputs (with empty script_sig for signature verification)
        for input in &self.inputs {
            // Previous output hash
            bytes.extend_from_slice(input.previous_output.txid.as_slice());
            // Previous output index
            bytes.extend_from_slice(&input.previous_output.vout.to_le_bytes());
            // Empty script (zero length)
            Self::write_varint(&mut bytes, 0);
            // Sequence
            bytes.extend_from_slice(&input.sequence.to_le_bytes());
        }

        // Output count (variable length integer)
        Self::write_varint(&mut bytes, self.outputs.len() as u64);

        // Outputs
        for output in &self.outputs {
            output.serialize_into(&mut bytes);
        }

        // Lock time (4 bytes, little endian)
        bytes.extend_from_slice(&self.lock_time.to_le_bytes());

        // Fork ID (1 byte) - Issue #6: Replay Protection
        // Commits signature to specific network (mainnet/testnet/regtest)
        bytes.push(self.fork_id);

        bytes
    }

    /// Deserialize transaction from bytes
    pub fn deserialize(bytes: &[u8]) -> Result<Self, TransactionError> {
        let mut cursor = 0;

        // Version
        if bytes.len() < 4 {
            return Err(TransactionError::InvalidSerialization);
        }
        let version = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        cursor += 4;

        // Input count
        let (input_count, varint_size) = Self::read_varint(&bytes[cursor..])?;
        cursor += varint_size;

        // Inputs
        let mut inputs = Vec::new();
        for _ in 0..input_count {
            let (input, input_size) = TransactionInput::deserialize_from(&bytes[cursor..])?;
            inputs.push(input);
            cursor += input_size;
        }

        // Output count
        let (output_count, varint_size) = Self::read_varint(&bytes[cursor..])?;
        cursor += varint_size;

        // Outputs
        let mut outputs = Vec::new();
        for _ in 0..output_count {
            let (output, output_size) = TransactionOutput::deserialize_from(&bytes[cursor..])?;
            outputs.push(output);
            cursor += output_size;
        }

        // Lock time
        if bytes.len() < cursor + 4 {
            return Err(TransactionError::InvalidSerialization);
        }
        let lock_time = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]);
        cursor += 4;

        // Fork ID (Issue #6: Replay Protection)
        let fork_id = if bytes.len() > cursor {
            bytes[cursor]
        } else {
            0 // Legacy transactions default to mainnet
        };

        Ok(Transaction {
            version,
            inputs,
            outputs,
            lock_time,
            fork_id,
        })
    }

    // Helper functions for variable-length integers
    fn write_varint(bytes: &mut Vec<u8>, value: u64) {
        if value < 0xfd {
            bytes.push(value as u8);
        } else if value <= 0xffff {
            bytes.push(0xfd);
            bytes.extend_from_slice(&(value as u16).to_le_bytes());
        } else if value <= 0xffffffff {
            bytes.push(0xfe);
            bytes.extend_from_slice(&(value as u32).to_le_bytes());
        } else {
            bytes.push(0xff);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }

    fn read_varint(bytes: &[u8]) -> Result<(u64, usize), TransactionError> {
        if bytes.is_empty() {
            return Err(TransactionError::InvalidSerialization);
        }

        match bytes[0] {
            0..=0xfc => Ok((bytes[0] as u64, 1)),
            0xfd => {
                if bytes.len() < 3 {
                    return Err(TransactionError::InvalidSerialization);
                }
                let value = u16::from_le_bytes([bytes[1], bytes[2]]) as u64;
                Ok((value, 3))
            }
            0xfe => {
                if bytes.len() < 5 {
                    return Err(TransactionError::InvalidSerialization);
                }
                let value = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as u64;
                Ok((value, 5))
            }
            0xff => {
                if bytes.len() < 9 {
                    return Err(TransactionError::InvalidSerialization);
                }
                let value = u64::from_le_bytes([
                    bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
                ]);
                Ok((value, 9))
            }
        }
    }

    /// Create a test transfer transaction
    pub fn create_test_transfer(amount: u64, recipient: Hash) -> Self {
        let input = TransactionInput {
            previous_output: OutPoint {
                txid: Hash::random(),
                vout: 0,
            },
            script_sig: Script::new(),
            sequence: 0xffffffff,
        };

        let output = TransactionOutput {
            value: amount,
            script_pubkey: Script::new_p2pkh(&recipient.as_bytes()[..20]),
        };

        Transaction {
            version: 1,
            inputs: vec![input],
            outputs: vec![output],
            lock_time: 0,
            fork_id: 0, // Test txs default to mainnet
        }
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

/// A transaction input
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionInput {
    /// Reference to previous transaction output
    pub previous_output: OutPoint,
    /// Script signature (unlock script)
    pub script_sig: Script,
    /// Sequence number
    pub sequence: u32,
}

impl TransactionInput {
    /// Validate transaction input structure
    pub fn validate(&self) -> Result<(), TransactionError> {
        // Basic validation - more comprehensive validation happens during script execution
        if self.script_sig.size() > MAX_TRANSACTION_SIZE / 2 {
            return Err(TransactionError::ScriptTooLarge);
        }

        Ok(())
    }

    /// Serialize input to bytes
    pub fn serialize_into(&self, bytes: &mut Vec<u8>) {
        // Previous output hash (32 bytes for SHA-512)
        bytes.extend_from_slice(self.previous_output.txid.as_slice());

        // Previous output index (4 bytes, little endian)
        bytes.extend_from_slice(&self.previous_output.vout.to_le_bytes());

        // Script signature length and data
        let script_bytes = self.script_sig.serialize();
        Transaction::write_varint(bytes, script_bytes.len() as u64);
        bytes.extend_from_slice(&script_bytes);

        // Sequence (4 bytes, little endian)
        bytes.extend_from_slice(&self.sequence.to_le_bytes());
    }

    /// Deserialize input from bytes
    pub fn deserialize_from(bytes: &[u8]) -> Result<(Self, usize), TransactionError> {
        let mut cursor = 0;

        // Previous output hash (64 bytes for SHA-512)
        if bytes.len() < 64 {
            return Err(TransactionError::InvalidSerialization);
        }
        let txid = Hash::from_slice(&bytes[cursor..cursor + 64])
            .map_err(|_| TransactionError::InvalidSerialization)?;
        cursor += 64;

        // Previous output index
        if bytes.len() < cursor + 4 {
            return Err(TransactionError::InvalidSerialization);
        }
        let vout = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]);
        cursor += 4;

        // Script signature length
        let (script_len, varint_size) = Transaction::read_varint(&bytes[cursor..])?;
        cursor += varint_size;

        // Script signature data
        if bytes.len() < cursor + script_len as usize {
            return Err(TransactionError::InvalidSerialization);
        }
        let script_sig = Script::deserialize(&bytes[cursor..cursor + script_len as usize])
            .map_err(|_| TransactionError::InvalidSerialization)?;
        cursor += script_len as usize;

        // Sequence
        if bytes.len() < cursor + 4 {
            return Err(TransactionError::InvalidSerialization);
        }
        let sequence = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]);
        cursor += 4;

        let input = TransactionInput {
            previous_output: OutPoint { txid, vout },
            script_sig,
            sequence,
        };

        Ok((input, cursor))
    }
}

/// A transaction output
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionOutput {
    /// Value in satoshis
    pub value: u64,
    /// Script public key (lock script)
    pub script_pubkey: Script,
}

impl TransactionOutput {
    /// Validate transaction output
    pub fn validate(&self) -> Result<(), TransactionError> {
        // Check value bounds
        if self.value == 0 {
            return Err(TransactionError::ZeroValue);
        }

        if self.value > MAX_OUTPUT_VALUE {
            return Err(TransactionError::ValueTooLarge);
        }

        // Check script size
        if self.script_pubkey.size() > MAX_TRANSACTION_SIZE / 2 {
            return Err(TransactionError::ScriptTooLarge);
        }

        Ok(())
    }

    /// Serialize output to bytes
    pub fn serialize_into(&self, bytes: &mut Vec<u8>) {
        // Value (8 bytes, little endian)
        bytes.extend_from_slice(&self.value.to_le_bytes());

        // Script public key length and data
        let script_bytes = self.script_pubkey.serialize();
        Transaction::write_varint(bytes, script_bytes.len() as u64);
        bytes.extend_from_slice(&script_bytes);
    }

    /// Deserialize output from bytes
    pub fn deserialize_from(bytes: &[u8]) -> Result<(Self, usize), TransactionError> {
        let mut cursor = 0;

        // Value
        if bytes.len() < 8 {
            return Err(TransactionError::InvalidSerialization);
        }
        let value = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        cursor += 8;

        // Script public key length
        let (script_len, varint_size) = Transaction::read_varint(&bytes[cursor..])?;
        cursor += varint_size;

        // Script public key data
        if bytes.len() < cursor + script_len as usize {
            return Err(TransactionError::InvalidSerialization);
        }
        let script_pubkey = Script::deserialize(&bytes[cursor..cursor + script_len as usize])
            .map_err(|_| TransactionError::InvalidSerialization)?;
        cursor += script_len as usize;

        let output = TransactionOutput {
            value,
            script_pubkey,
        };

        Ok((output, cursor))
    }
}

/// Reference to a transaction output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutPoint {
    /// Transaction hash
    pub txid: Hash,
    /// Output index
    pub vout: u32,
}

impl OutPoint {
    /// Create a new outpoint
    pub fn new(txid: Hash, vout: u32) -> Self {
        OutPoint { txid, vout }
    }

    /// Create null outpoint (for coinbase)
    pub fn null() -> Self {
        OutPoint {
            txid: Hash::zero(),
            vout: 0xffffffff,
        }
    }

    /// Check if this is a null outpoint
    pub fn is_null(&self) -> bool {
        self.txid == Hash::zero() && self.vout == 0xffffffff
    }
}

impl fmt::Display for OutPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.txid, self.vout)
    }
}

/// Error types for transaction operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionError {
    /// Invalid transaction version
    InvalidVersion,
    /// No transaction inputs
    NoInputs,
    /// No transaction outputs
    NoOutputs,
    /// Too many inputs
    TooManyInputs,
    /// Too many outputs
    TooManyOutputs,
    /// Transaction too large
    TransactionTooLarge,
    /// Zero value output
    ZeroValue,
    /// Value too large
    ValueTooLarge,
    /// Value overflow
    ValueOverflow,
    /// Script too large
    ScriptTooLarge,
    /// Duplicate input
    DuplicateInput,
    /// UTXO not found
    UTXONotFound,
    /// Insufficient inputs
    InsufficientInputs,
    /// Invalid serialization
    InvalidSerialization,
    /// Script execution error
    ScriptError(crate::crypto::ScriptError),
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionError::InvalidVersion => write!(f, "Invalid transaction version"),
            TransactionError::NoInputs => write!(f, "Transaction has no inputs"),
            TransactionError::NoOutputs => write!(f, "Transaction has no outputs"),
            TransactionError::TooManyInputs => write!(f, "Too many transaction inputs"),
            TransactionError::TooManyOutputs => write!(f, "Too many transaction outputs"),
            TransactionError::TransactionTooLarge => write!(f, "Transaction too large"),
            TransactionError::ZeroValue => write!(f, "Output has zero value"),
            TransactionError::ValueTooLarge => write!(f, "Output value too large"),
            TransactionError::ValueOverflow => write!(f, "Value overflow"),
            TransactionError::ScriptTooLarge => write!(f, "Script too large"),
            TransactionError::DuplicateInput => write!(f, "Duplicate transaction input"),
            TransactionError::UTXONotFound => write!(f, "UTXO not found"),
            TransactionError::InsufficientInputs => write!(f, "Insufficient input value"),
            TransactionError::InvalidSerialization => {
                write!(f, "Invalid transaction serialization")
            }
            TransactionError::ScriptError(e) => write!(f, "Script error: {}", e),
        }
    }
}

impl std::error::Error for TransactionError {}

impl From<crate::crypto::ScriptError> for TransactionError {
    fn from(err: crate::crypto::ScriptError) -> Self {
        TransactionError::ScriptError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation() {
        let tx = Transaction::new();
        assert_eq!(tx.version, 1);
        assert!(tx.inputs.is_empty());
        assert!(tx.outputs.is_empty());
        assert_eq!(tx.lock_time, 0);
    }

    #[test]
    fn test_coinbase_transaction() {
        let reward = 3_237_500_000;
        let recipient = Hash::from_int(12345);
        let coinbase = Transaction::coinbase(reward, recipient);

        assert!(coinbase.is_coinbase());
        assert_eq!(coinbase.inputs.len(), 1);
        assert_eq!(coinbase.outputs.len(), 1);
        assert_eq!(coinbase.outputs[0].value, reward);
    }

    #[test]
    fn test_transaction_hash() {
        let tx = Transaction::new();
        let hash1 = tx.hash();
        let hash2 = tx.hash();

        // Hash should be deterministic
        assert_eq!(hash1, hash2);

        // Different transactions should have different hashes
        let mut tx2 = tx.clone();
        tx2.version = 2;
        assert_ne!(tx.hash(), tx2.hash());
    }

    #[test]
    fn test_transaction_serialization() {
        let recipient = Hash::from_int(12345);
        let tx = Transaction::create_test_transfer(1000000, recipient);

        let serialized = tx.serialize();
        let deserialized = Transaction::deserialize(&serialized).unwrap();

        assert_eq!(tx, deserialized);
    }

    #[test]
    fn test_transaction_validation() {
        let recipient = Hash::from_int(12345);
        let mut tx = Transaction::create_test_transfer(1000000, recipient);

        // Valid transaction should pass
        assert!(tx.validate_structure().is_ok());

        // Remove inputs - should fail
        tx.inputs.clear();
        assert!(tx.validate_structure().is_err());

        // Add inputs back, remove outputs - should fail
        tx.inputs.push(TransactionInput {
            previous_output: OutPoint::new(Hash::from_int(1), 0),
            script_sig: Script::new(),
            sequence: 0xffffffff,
        });
        tx.outputs.clear();
        assert!(tx.validate_structure().is_err());
    }

    #[test]
    fn test_outpoint() {
        let txid = Hash::from_int(12345);
        let outpoint = OutPoint::new(txid, 0);

        assert_eq!(outpoint.txid, txid);
        assert_eq!(outpoint.vout, 0);
        assert!(!outpoint.is_null());

        let null_outpoint = OutPoint::null();
        assert!(null_outpoint.is_null());
    }

    #[test]
    fn test_value_calculations() {
        // This test would require a mock UTXO set
        // For now, just test the output value calculation
        let recipient = Hash::from_int(12345);
        let mut tx = Transaction::create_test_transfer(1000000, recipient);

        // Add another output
        tx.outputs.push(TransactionOutput {
            value: 500000,
            script_pubkey: Script::pay_to_pubkey_hash(Hash::from_int(54321)),
        });

        let total_output = tx.calculate_output_value().unwrap();
        assert_eq!(total_output, 1500000);
    }

    #[test]
    fn test_varint_encoding() {
        let mut bytes = Vec::new();

        // Test different varint sizes
        Transaction::write_varint(&mut bytes, 252);
        Transaction::write_varint(&mut bytes, 253);
        Transaction::write_varint(&mut bytes, 65535);
        Transaction::write_varint(&mut bytes, 65536);

        // Test reading back
        let mut cursor = 0;
        let (val1, size1) = Transaction::read_varint(&bytes[cursor..]).unwrap();
        cursor += size1;
        let (val2, size2) = Transaction::read_varint(&bytes[cursor..]).unwrap();
        cursor += size2;
        let (val3, size3) = Transaction::read_varint(&bytes[cursor..]).unwrap();
        cursor += size3;
        let (val4, _size4) = Transaction::read_varint(&bytes[cursor..]).unwrap();

        assert_eq!(val1, 252);
        assert_eq!(val2, 253);
        assert_eq!(val3, 65535);
        assert_eq!(val4, 65536);
    }
}
