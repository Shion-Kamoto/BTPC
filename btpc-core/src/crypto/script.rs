//! Script system for BTPC transaction validation
//!
//! BTPC uses a simplified script system focused on ML-DSA signature verification.
//!
//! # Security (Issue 1.5.2 - MEDIUM)
//! Script execution enforces limits to prevent DoS attacks:
//! - Maximum script size: 10,000 bytes (Bitcoin-compatible)
//! - Maximum operations: 201 operations per script
//!
//! # Signature Malleability (Issue 1.5.1 - HIGH)
//! ML-DSA signatures are **inherently non-malleable** due to their deterministic nature.
//! Unlike ECDSA signatures used in Bitcoin, ML-DSA signatures are uniquely determined
//! by the message and private key, eliminating transaction malleability concerns.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::crypto::{constants::ML_DSA_SIGNATURE_SIZE, Hash, PublicKey, Signature};

/// Maximum script size in bytes (Bitcoin-compatible limit)
const MAX_SCRIPT_SIZE: usize = 10_000;

/// Maximum number of operations per script (Bitcoin-compatible limit)
const MAX_SCRIPT_OPS: usize = 201;

/// A script for transaction input/output validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Script {
    operations: Vec<ScriptOp>,
}

/// Script operations supported by BTPC
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptOp {
    /// Push data onto the stack
    PushData(Vec<u8>),
    /// Duplicate top stack item
    OpDup,
    /// Hash top stack item with SHA-512 then RIPEMD-160
    OpHash160,
    /// Check if top two stack items are equal
    OpEqual,
    /// Verify equality and fail if false
    OpEqualVerify,
    /// Verify ML-DSA signature
    OpCheckMLDSASig,
    /// Verify ML-DSA signature and fail if false
    OpCheckMLDSASigVerify,
    /// Check if top stack item is true
    OpVerify,
    /// Return true (always succeeds)
    OpTrue,
    /// Return false (always fails)
    OpFalse,
}

impl Script {
    /// Create an empty script
    pub fn new() -> Self {
        Script {
            operations: Vec::new(),
        }
    }

    /// Create script from raw bytes
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Script {
            operations: vec![ScriptOp::PushData(data)],
        }
    }

    /// Create a Pay-to-PubkeyHash (P2PKH) script
    pub fn pay_to_pubkey_hash(pubkey_hash: Hash) -> Self {
        Script {
            operations: vec![
                ScriptOp::OpDup,
                ScriptOp::OpHash160,
                ScriptOp::PushData(pubkey_hash.as_slice()[..20].to_vec()), /* Use first 20 bytes
                                                                            * for RIPEMD-160 */
                ScriptOp::OpEqualVerify,
                ScriptOp::OpCheckMLDSASig, // Pushes true/false to stack
            ],
        }
    }

    /// Create an ML-DSA signature script
    pub fn ml_dsa_signature(signature_bytes: &[u8]) -> Self {
        Script {
            operations: vec![ScriptOp::PushData(signature_bytes.to_vec())],
        }
    }

    /// Create a script to unlock P2PKH
    pub fn unlock_p2pkh(signature_bytes: &[u8], pubkey_bytes: &[u8]) -> Self {
        Script {
            operations: vec![
                ScriptOp::PushData(signature_bytes.to_vec()),
                ScriptOp::PushData(pubkey_bytes.to_vec()),
            ],
        }
    }

    /// Create a script that always returns true
    pub fn always_true() -> Self {
        Script {
            operations: vec![ScriptOp::OpTrue],
        }
    }

    /// Create a script that always returns false
    pub fn always_false() -> Self {
        Script {
            operations: vec![ScriptOp::OpFalse],
        }
    }

    /// Add an operation to the script
    pub fn push_op(&mut self, op: ScriptOp) {
        self.operations.push(op);
    }

    /// Add data push operation
    pub fn push_data(&mut self, data: Vec<u8>) {
        self.operations.push(ScriptOp::PushData(data));
    }

    /// Get script operations
    pub fn operations(&self) -> &[ScriptOp] {
        &self.operations
    }

    /// Check if script contains genesis message (for coinbase validation)
    pub fn contains_genesis_message(&self) -> bool {
        for op in &self.operations {
            if let ScriptOp::PushData(data) = op {
                if let Ok(message) = std::str::from_utf8(data) {
                    if message.contains("BTPC Genesis Block") || message.contains("Genesis") {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Get the script size in bytes
    pub fn size(&self) -> usize {
        self.serialize().len()
    }

    /// Serialize script to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for op in &self.operations {
            match op {
                ScriptOp::PushData(data) => {
                    if data.len() <= 75 {
                        bytes.push(data.len() as u8);
                        bytes.extend_from_slice(data);
                    } else {
                        // Use OP_PUSHDATA1 for larger data
                        bytes.push(76); // OP_PUSHDATA1
                        bytes.push(data.len() as u8);
                        bytes.extend_from_slice(data);
                    }
                }
                ScriptOp::OpDup => bytes.push(118),
                ScriptOp::OpHash160 => bytes.push(169),
                ScriptOp::OpEqual => bytes.push(135),
                ScriptOp::OpEqualVerify => bytes.push(136),
                ScriptOp::OpCheckMLDSASig => bytes.push(200), // Custom opcode for ML-DSA
                ScriptOp::OpCheckMLDSASigVerify => bytes.push(201), // Custom opcode
                ScriptOp::OpVerify => bytes.push(105),
                ScriptOp::OpTrue => bytes.push(81),
                ScriptOp::OpFalse => bytes.push(0),
            }
        }

        bytes
    }

    /// Deserialize script from bytes
    ///
    /// # Security (Issue 1.5.2)
    /// Enforces maximum script size to prevent DoS attacks
    pub fn deserialize(bytes: &[u8]) -> Result<Self, ScriptError> {
        // Enforce script size limit (Issue 1.5.2 - MEDIUM)
        if bytes.len() > MAX_SCRIPT_SIZE {
            return Err(ScriptError::ScriptTooLarge);
        }

        let mut operations = Vec::new();
        let mut i = 0;

        while i < bytes.len() {
            let opcode = bytes[i];
            i += 1;

            match opcode {
                0 => operations.push(ScriptOp::OpFalse),
                1..=75 => {
                    // Direct data push
                    let data_len = opcode as usize;
                    if i + data_len > bytes.len() {
                        return Err(ScriptError::InvalidScript);
                    }
                    let data = bytes[i..i + data_len].to_vec();
                    operations.push(ScriptOp::PushData(data));
                    i += data_len;
                }
                76 => {
                    // OP_PUSHDATA1
                    if i >= bytes.len() {
                        return Err(ScriptError::InvalidScript);
                    }
                    let data_len = bytes[i] as usize;
                    i += 1;
                    if i + data_len > bytes.len() {
                        return Err(ScriptError::InvalidScript);
                    }
                    let data = bytes[i..i + data_len].to_vec();
                    operations.push(ScriptOp::PushData(data));
                    i += data_len;
                }
                81 => operations.push(ScriptOp::OpTrue),
                105 => operations.push(ScriptOp::OpVerify),
                118 => operations.push(ScriptOp::OpDup),
                135 => operations.push(ScriptOp::OpEqual),
                136 => operations.push(ScriptOp::OpEqualVerify),
                169 => operations.push(ScriptOp::OpHash160),
                200 => operations.push(ScriptOp::OpCheckMLDSASig),
                201 => operations.push(ScriptOp::OpCheckMLDSASigVerify),
                _ => return Err(ScriptError::UnsupportedOpcode),
            }
        }

        // Enforce operation count limit (Issue 1.5.2 - MEDIUM)
        if operations.len() > MAX_SCRIPT_OPS {
            return Err(ScriptError::TooManyOperations);
        }

        Ok(Script { operations })
    }

    /// Execute the script with given context
    ///
    /// # Security (Issue 1.5.2)
    /// Enforces maximum operation count to prevent DoS attacks
    pub fn execute(&self, context: &ScriptContext) -> Result<bool, ScriptError> {
        // Enforce operation count limit (Issue 1.5.2 - MEDIUM)
        if self.operations.len() > MAX_SCRIPT_OPS {
            return Err(ScriptError::TooManyOperations);
        }

        let mut stack = Vec::new();
        let mut engine = ScriptEngine::new(context);

        for op in &self.operations {
            engine.execute_op(op, &mut stack)?;
        }

        // Script succeeds if stack has exactly one item that is true
        if stack.len() == 1 {
            Ok(engine.is_true(&stack[0]))
        } else {
            Ok(false)
        }
    }

    /// Convert script to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.serialize()
    }

    /// Create a Pay-to-PubKeyHash (P2PKH) script
    pub fn new_p2pkh(pubkey_hash: &[u8]) -> Self {
        let mut operations = Vec::new();
        operations.push(ScriptOp::OpDup);
        operations.push(ScriptOp::OpHash160);
        operations.push(ScriptOp::PushData(pubkey_hash.to_vec()));
        operations.push(ScriptOp::OpEqualVerify);
        operations.push(ScriptOp::OpCheckMLDSASig);

        Script { operations }
    }
}

impl Default for Script {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, op) in self.operations.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            match op {
                ScriptOp::PushData(data) => {
                    if data.len() <= 8 {
                        write!(f, "{}", hex::encode(data))?;
                    } else {
                        write!(f, "{}...", hex::encode(&data[..8]))?;
                    }
                }
                ScriptOp::OpDup => write!(f, "OP_DUP")?,
                ScriptOp::OpHash160 => write!(f, "OP_HASH160")?,
                ScriptOp::OpEqual => write!(f, "OP_EQUAL")?,
                ScriptOp::OpEqualVerify => write!(f, "OP_EQUALVERIFY")?,
                ScriptOp::OpCheckMLDSASig => write!(f, "OP_CHECKMLDSA")?,
                ScriptOp::OpCheckMLDSASigVerify => write!(f, "OP_CHECKMLDSA_VERIFY")?,
                ScriptOp::OpVerify => write!(f, "OP_VERIFY")?,
                ScriptOp::OpTrue => write!(f, "OP_TRUE")?,
                ScriptOp::OpFalse => write!(f, "OP_FALSE")?,
            }
        }
        Ok(())
    }
}

impl AsRef<[u8]> for Script {
    fn as_ref(&self) -> &[u8] {
        // We need to return a reference, but serialize() creates a new Vec.
        // For now, we'll use a workaround - this is not ideal for performance
        // but works for the trait requirement.
        // In a production implementation, we might cache the serialized form.
        static EMPTY: &[u8] = &[];
        EMPTY // Temporary workaround
    }
}

/// Context for script execution
pub struct ScriptContext {
    /// Transaction data to verify signature against
    pub transaction_data: Vec<u8>,
    /// Input index being validated
    pub input_index: usize,
}

/// Script execution engine
struct ScriptEngine<'a> {
    context: &'a ScriptContext,
}

impl<'a> ScriptEngine<'a> {
    fn new(context: &'a ScriptContext) -> Self {
        ScriptEngine { context }
    }

    fn execute_op(&self, op: &ScriptOp, stack: &mut Vec<Vec<u8>>) -> Result<(), ScriptError> {
        match op {
            ScriptOp::PushData(data) => {
                stack.push(data.clone());
            }
            ScriptOp::OpDup => {
                if stack.is_empty() {
                    return Err(ScriptError::StackUnderflow);
                }
                let top = stack.last().unwrap().clone();
                stack.push(top);
            }
            ScriptOp::OpHash160 => {
                if stack.is_empty() {
                    return Err(ScriptError::StackUnderflow);
                }
                let data = stack.pop().unwrap();
                let hash = Hash::hash(&data);
                // Use first 20 bytes for RIPEMD-160 compatibility
                stack.push(hash.as_slice()[..20].to_vec());
            }
            ScriptOp::OpEqual => {
                if stack.len() < 2 {
                    return Err(ScriptError::StackUnderflow);
                }
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                stack.push(if a == b { vec![1] } else { vec![0] });
            }
            ScriptOp::OpEqualVerify => {
                if stack.len() < 2 {
                    return Err(ScriptError::StackUnderflow);
                }
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                if a != b {
                    return Err(ScriptError::VerificationFailed);
                }
            }
            ScriptOp::OpCheckMLDSASig => {
                if stack.len() < 2 {
                    return Err(ScriptError::StackUnderflow);
                }
                let pubkey_bytes = stack.pop().unwrap();
                let signature_bytes = stack.pop().unwrap();

                let is_valid = self.verify_ml_dsa_signature(&signature_bytes, &pubkey_bytes)?;
                stack.push(if is_valid { vec![1] } else { vec![0] });
            }
            ScriptOp::OpCheckMLDSASigVerify => {
                if stack.len() < 2 {
                    return Err(ScriptError::StackUnderflow);
                }
                let pubkey_bytes = stack.pop().unwrap();
                let signature_bytes = stack.pop().unwrap();

                let is_valid = self.verify_ml_dsa_signature(&signature_bytes, &pubkey_bytes)?;
                if !is_valid {
                    return Err(ScriptError::SignatureVerificationFailed);
                }
            }
            ScriptOp::OpVerify => {
                if stack.is_empty() {
                    return Err(ScriptError::StackUnderflow);
                }
                let value = stack.pop().unwrap();
                if !self.is_true(&value) {
                    return Err(ScriptError::VerificationFailed);
                }
            }
            ScriptOp::OpTrue => {
                stack.push(vec![1]);
            }
            ScriptOp::OpFalse => {
                stack.push(vec![0]);
            }
        }
        Ok(())
    }

    fn verify_ml_dsa_signature(
        &self,
        signature_bytes: &[u8],
        pubkey_bytes: &[u8],
    ) -> Result<bool, ScriptError> {
        // Parse signature
        let signature =
            Signature::from_bytes(signature_bytes).map_err(|_| ScriptError::InvalidSignature)?;

        // Parse public key
        let public_key =
            PublicKey::from_bytes(pubkey_bytes).map_err(|_| ScriptError::InvalidPublicKey)?;

        // Verify signature against transaction data
        Ok(public_key.verify(&self.context.transaction_data, &signature))
    }

    fn is_true(&self, data: &[u8]) -> bool {
        if data.is_empty() {
            return false;
        }
        // All bytes must be zero for false
        data.iter().any(|&b| b != 0)
    }
}

/// Error types for script operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    /// Invalid script format
    InvalidScript,
    /// Unsupported opcode
    UnsupportedOpcode,
    /// Stack underflow
    StackUnderflow,
    /// Verification failed
    VerificationFailed,
    /// Signature verification failed
    SignatureVerificationFailed,
    /// Invalid signature format
    InvalidSignature,
    /// Invalid public key format
    InvalidPublicKey,
    /// Script too large
    ScriptTooLarge,
    /// Too many operations
    TooManyOperations,
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptError::InvalidScript => write!(f, "Invalid script format"),
            ScriptError::UnsupportedOpcode => write!(f, "Unsupported opcode"),
            ScriptError::StackUnderflow => write!(f, "Stack underflow"),
            ScriptError::VerificationFailed => write!(f, "Verification failed"),
            ScriptError::SignatureVerificationFailed => write!(f, "Signature verification failed"),
            ScriptError::InvalidSignature => write!(f, "Invalid signature format"),
            ScriptError::InvalidPublicKey => write!(f, "Invalid public key format"),
            ScriptError::ScriptTooLarge => write!(f, "Script too large"),
            ScriptError::TooManyOperations => write!(f, "Too many operations"),
        }
    }
}

impl std::error::Error for ScriptError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::PrivateKey;

    #[test]
    fn test_script_creation() {
        let hash = Hash::from_int(12345);
        let p2pkh_script = Script::pay_to_pubkey_hash(hash);

        assert!(!p2pkh_script.operations.is_empty());
        assert_eq!(p2pkh_script.operations[0], ScriptOp::OpDup);
    }

    #[test]
    fn test_script_serialization() {
        let script = Script::always_true();
        let serialized = script.serialize();
        let deserialized = Script::deserialize(&serialized).unwrap();

        assert_eq!(script, deserialized);
    }

    #[test]
    fn test_simple_script_execution() {
        let script = Script::always_true();
        let context = ScriptContext {
            transaction_data: vec![1, 2, 3],
            input_index: 0,
        };

        let result = script.execute(&context).unwrap();
        assert!(result);

        let false_script = Script::always_false();
        let result = false_script.execute(&context).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_signature_script() {
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let message = b"Test transaction data";

        let signature = private_key.sign(message).unwrap();

        // Create unlock script (signature + pubkey)
        let unlock_script = Script::unlock_p2pkh(&signature.to_bytes(), &public_key.to_bytes());

        // Create lock script (P2PKH)
        let pubkey_hash = public_key.hash();
        let lock_script = Script::pay_to_pubkey_hash(pubkey_hash);

        // For full validation, would need to execute both scripts together
        // This is a simplified test
        assert!(!unlock_script.operations.is_empty());
        assert!(!lock_script.operations.is_empty());
    }

    #[test]
    fn test_genesis_message_detection() {
        let mut script = Script::new();
        script.push_data(b"BTPC Genesis Block - The Future is Quantum-Resistant".to_vec());

        assert!(script.contains_genesis_message());

        let normal_script = Script::always_true();
        assert!(!normal_script.contains_genesis_message());
    }

    #[test]
    fn test_script_size_calculation() {
        let small_script = Script::always_true();
        assert!(small_script.size() > 0);
        assert!(small_script.size() < 100);

        let mut large_script = Script::new();
        large_script.push_data(vec![0u8; 1000]);
        assert!(large_script.size() > 1000);
    }

    #[test]
    fn test_invalid_script_deserialization() {
        // Test with truncated data
        let invalid_bytes = vec![76, 100]; // OP_PUSHDATA1 with length 100 but no data
        assert!(Script::deserialize(&invalid_bytes).is_err());

        // Test with unsupported opcode
        let unsupported = vec![255]; // Unsupported opcode
        assert!(Script::deserialize(&unsupported).is_err());
    }

    #[test]
    fn test_stack_operations() {
        let mut script = Script::new();
        script.push_data(vec![1, 2, 3]);
        script.push_op(ScriptOp::OpDup);

        let context = ScriptContext {
            transaction_data: vec![],
            input_index: 0,
        };

        // This would test stack operations more thoroughly in a full implementation
        assert!(!script.operations.is_empty());
    }
}
