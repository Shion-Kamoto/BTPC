//! Merkle tree implementation for BTPC transaction verification
//!
//! Provides efficient verification of transaction inclusion in blocks.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{blockchain::Transaction, crypto::Hash};

/// Calculate the merkle root of a list of transactions
pub fn calculate_merkle_root(transactions: &[Transaction]) -> Result<Hash, MerkleError> {
    if transactions.is_empty() {
        return Err(MerkleError::EmptyInput);
    }

    // Get transaction hashes
    let mut hashes: Vec<Hash> = transactions.iter().map(|tx| tx.hash()).collect();

    // Special case for single transaction (Bitcoin-compatible)
    if hashes.len() == 1 {
        return Ok(Hash::double_sha512(hashes[0].as_slice()));
    }

    // Build merkle tree bottom-up
    while hashes.len() > 1 {
        let mut next_level = Vec::new();

        // Process pairs of hashes
        for chunk in hashes.chunks(2) {
            let combined_hash = if chunk.len() == 2 {
                // Hash pair
                hash_pair(&chunk[0], &chunk[1])
            } else {
                // Odd number - duplicate last hash (Bitcoin behavior)
                hash_pair(&chunk[0], &chunk[0])
            };
            next_level.push(combined_hash);
        }

        hashes = next_level;
    }

    Ok(hashes[0])
}

/// Hash a pair of merkle tree nodes
fn hash_pair(left: &Hash, right: &Hash) -> Hash {
    let mut combined = Vec::with_capacity(128); // 2 * 64 bytes
    combined.extend_from_slice(left.as_slice());
    combined.extend_from_slice(right.as_slice());
    Hash::double_sha512(&combined)
}

/// Merkle tree for efficient transaction verification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleTree {
    /// Root hash of the tree
    pub root: Hash,
    /// Tree levels (bottom to top)
    levels: Vec<Vec<Hash>>,
}

impl MerkleTree {
    /// Build a merkle tree from transactions
    pub fn from_transactions(transactions: &[Transaction]) -> Result<Self, MerkleError> {
        if transactions.is_empty() {
            return Err(MerkleError::EmptyInput);
        }

        let mut levels = Vec::new();

        // Level 0: transaction hashes
        let mut current_level: Vec<Hash> = transactions.iter().map(|tx| tx.hash()).collect();
        levels.push(current_level.clone());

        // Build tree levels
        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in current_level.chunks(2) {
                let combined_hash = if chunk.len() == 2 {
                    hash_pair(&chunk[0], &chunk[1])
                } else {
                    hash_pair(&chunk[0], &chunk[0]) // Duplicate last hash
                };
                next_level.push(combined_hash);
            }

            levels.push(next_level.clone());
            current_level = next_level;
        }

        let root = current_level[0];

        Ok(MerkleTree { root, levels })
    }

    /// Get the merkle root
    pub fn root(&self) -> Hash {
        self.root
    }

    /// Get tree depth
    pub fn depth(&self) -> usize {
        self.levels.len()
    }

    /// Generate merkle proof for transaction at given index
    pub fn generate_proof(&self, tx_index: usize) -> Result<MerkleProof, MerkleError> {
        if self.levels.is_empty() {
            return Err(MerkleError::EmptyTree);
        }

        if tx_index >= self.levels[0].len() {
            return Err(MerkleError::InvalidIndex);
        }

        let mut proof_hashes = Vec::new();
        let mut proof_directions = Vec::new();
        let mut current_index = tx_index;

        // Generate proof by collecting sibling hashes at each level
        for level in 0..self.levels.len() - 1 {
            let sibling_index = if current_index % 2 == 0 {
                // Current is left child, sibling is right
                if current_index + 1 < self.levels[level].len() {
                    current_index + 1
                } else {
                    current_index // Duplicate case
                }
            } else {
                // Current is right child, sibling is left
                current_index - 1
            };

            proof_hashes.push(self.levels[level][sibling_index]);
            proof_directions.push(current_index % 2 == 0); // true = left, false = right

            current_index /= 2;
        }

        Ok(MerkleProof {
            root: self.root,
            transaction_hash: self.levels[0][tx_index],
            proof_hashes,
            proof_directions,
        })
    }

    /// Verify that a transaction is included in the tree
    pub fn verify_inclusion(&self, tx_hash: &Hash) -> bool {
        // Find transaction in bottom level
        if let Some(index) = self.levels[0].iter().position(|h| h == tx_hash) {
            if let Ok(proof) = self.generate_proof(index) {
                return proof.verify();
            }
        }
        false
    }
}

/// Merkle proof for transaction inclusion
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Root hash of the merkle tree
    pub root: Hash,
    /// Hash of the transaction being proved
    pub transaction_hash: Hash,
    /// Sibling hashes along the path to root
    pub proof_hashes: Vec<Hash>,
    /// Directions for each step (true = left, false = right)
    pub proof_directions: Vec<bool>,
}

impl MerkleProof {
    /// Verify the merkle proof
    pub fn verify(&self) -> bool {
        if self.proof_hashes.len() != self.proof_directions.len() {
            return false;
        }

        let mut current_hash = self.transaction_hash;

        // Reconstruct path to root
        for (sibling_hash, is_left) in self.proof_hashes.iter().zip(&self.proof_directions) {
            current_hash = if *is_left {
                // Current is left, sibling is right
                hash_pair(&current_hash, sibling_hash)
            } else {
                // Current is right, sibling is left
                hash_pair(sibling_hash, &current_hash)
            };
        }

        current_hash == self.root
    }

    /// Get proof size in bytes
    pub fn size(&self) -> usize {
        64 + 64 + (self.proof_hashes.len() * 64) + self.proof_directions.len()
    }

    /// Serialize proof to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Root hash
        bytes.extend_from_slice(self.root.as_slice());

        // Transaction hash
        bytes.extend_from_slice(self.transaction_hash.as_slice());

        // Number of proof elements
        bytes.extend_from_slice(&(self.proof_hashes.len() as u32).to_le_bytes());

        // Proof hashes and directions
        for (hash, direction) in self.proof_hashes.iter().zip(&self.proof_directions) {
            bytes.extend_from_slice(hash.as_slice());
            bytes.push(if *direction { 1 } else { 0 });
        }

        bytes
    }

    /// Deserialize proof from bytes
    pub fn deserialize(bytes: &[u8]) -> Result<Self, MerkleError> {
        if bytes.len() < 132 {
            // Minimum: 64 + 64 + 4 bytes
            return Err(MerkleError::InvalidSerialization);
        }

        let mut cursor = 0;

        // Root hash
        let root = Hash::from_slice(&bytes[cursor..cursor + 64])
            .map_err(|_| MerkleError::InvalidSerialization)?;
        cursor += 64;

        // Transaction hash
        let transaction_hash = Hash::from_slice(&bytes[cursor..cursor + 64])
            .map_err(|_| MerkleError::InvalidSerialization)?;
        cursor += 64;

        // Number of proof elements
        let proof_count = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        cursor += 4;

        // Proof hashes and directions
        let mut proof_hashes = Vec::new();
        let mut proof_directions = Vec::new();

        for _ in 0..proof_count {
            if cursor + 65 > bytes.len() {
                return Err(MerkleError::InvalidSerialization);
            }

            let hash = Hash::from_slice(&bytes[cursor..cursor + 64])
                .map_err(|_| MerkleError::InvalidSerialization)?;
            cursor += 64;

            let direction = bytes[cursor] != 0;
            cursor += 1;

            proof_hashes.push(hash);
            proof_directions.push(direction);
        }

        Ok(MerkleProof {
            root,
            transaction_hash,
            proof_hashes,
            proof_directions,
        })
    }
}

/// Error types for merkle tree operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MerkleError {
    /// Empty input
    EmptyInput,
    /// Empty tree
    EmptyTree,
    /// Invalid transaction index
    InvalidIndex,
    /// Invalid serialization
    InvalidSerialization,
    /// Hash mismatch
    HashMismatch,
}

impl fmt::Display for MerkleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MerkleError::EmptyInput => write!(f, "Empty transaction list"),
            MerkleError::EmptyTree => write!(f, "Empty merkle tree"),
            MerkleError::InvalidIndex => write!(f, "Invalid transaction index"),
            MerkleError::InvalidSerialization => write!(f, "Invalid merkle proof serialization"),
            MerkleError::HashMismatch => write!(f, "Merkle hash mismatch"),
        }
    }
}

impl std::error::Error for MerkleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_transaction_merkle_root() {
        let tx = Transaction::coinbase(1000000, Hash::random());
        let root = calculate_merkle_root(&[tx.clone()]).unwrap();

        // Single transaction root should be double hash of transaction
        let expected = Hash::double_sha512(tx.hash().as_slice());
        assert_eq!(root, expected);
    }

    #[test]
    fn test_multiple_transaction_merkle_root() {
        let transactions = vec![
            Transaction::coinbase(1000000, Hash::random()),
            Transaction::create_test_transfer(500000, Hash::random()),
            Transaction::create_test_transfer(300000, Hash::random()),
        ];

        let root = calculate_merkle_root(&transactions).unwrap();

        // Different transaction sets should have different roots
        let mut different_transactions = transactions.clone();
        different_transactions.pop();
        let different_root = calculate_merkle_root(&different_transactions).unwrap();

        assert_ne!(root, different_root);
    }

    #[test]
    fn test_merkle_tree_construction() {
        let transactions = vec![
            Transaction::coinbase(1000000, Hash::random()),
            Transaction::create_test_transfer(500000, Hash::random()),
            Transaction::create_test_transfer(300000, Hash::random()),
            Transaction::create_test_transfer(200000, Hash::random()),
        ];

        let tree = MerkleTree::from_transactions(&transactions).unwrap();

        // Root should match direct calculation
        let expected_root = calculate_merkle_root(&transactions).unwrap();
        assert_eq!(tree.root(), expected_root);

        // Tree should have correct depth (4 transactions -> 3 levels)
        assert_eq!(tree.depth(), 3);
    }

    #[test]
    fn test_merkle_proof_generation_and_verification() {
        let transactions = vec![
            Transaction::coinbase(1000000, Hash::random()),
            Transaction::create_test_transfer(500000, Hash::random()),
            Transaction::create_test_transfer(300000, Hash::random()),
        ];

        let tree = MerkleTree::from_transactions(&transactions).unwrap();

        // Generate proof for second transaction
        let proof = tree.generate_proof(1).unwrap();

        // Proof should verify
        assert!(proof.verify());

        // Proof should have correct transaction hash
        assert_eq!(proof.transaction_hash, transactions[1].hash());

        // Proof should have correct root
        assert_eq!(proof.root, tree.root());
    }

    #[test]
    fn test_merkle_proof_serialization() {
        let transactions = vec![
            Transaction::coinbase(1000000, Hash::random()),
            Transaction::create_test_transfer(500000, Hash::random()),
        ];

        let tree = MerkleTree::from_transactions(&transactions).unwrap();
        let proof = tree.generate_proof(0).unwrap();

        // Serialize and deserialize
        let serialized = proof.serialize();
        let deserialized = MerkleProof::deserialize(&serialized).unwrap();

        assert_eq!(proof, deserialized);
        assert!(deserialized.verify());
    }

    #[test]
    fn test_transaction_inclusion_verification() {
        let transactions = vec![
            Transaction::coinbase(1000000, Hash::random()),
            Transaction::create_test_transfer(500000, Hash::random()),
            Transaction::create_test_transfer(300000, Hash::random()),
        ];

        let tree = MerkleTree::from_transactions(&transactions).unwrap();

        // All transactions should be verifiable as included
        for tx in &transactions {
            assert!(tree.verify_inclusion(&tx.hash()));
        }

        // Random transaction should not be included
        let random_tx = Transaction::create_test_transfer(999999, Hash::random());
        assert!(!tree.verify_inclusion(&random_tx.hash()));
    }

    #[test]
    fn test_empty_transaction_list() {
        let empty_transactions: Vec<Transaction> = vec![];

        // Should fail to create tree from empty list
        assert!(MerkleTree::from_transactions(&empty_transactions).is_err());
        assert!(calculate_merkle_root(&empty_transactions).is_err());
    }

    #[test]
    fn test_invalid_proof_index() {
        let transactions = vec![Transaction::coinbase(1000000, Hash::random())];

        let tree = MerkleTree::from_transactions(&transactions).unwrap();

        // Should fail to generate proof for invalid index
        assert!(tree.generate_proof(1).is_err());
        assert!(tree.generate_proof(100).is_err());
    }

    #[test]
    fn test_odd_number_of_transactions() {
        // Test Bitcoin's behavior with odd number of transactions
        let transactions = vec![
            Transaction::coinbase(1000000, Hash::random()),
            Transaction::create_test_transfer(500000, Hash::random()),
            Transaction::create_test_transfer(300000, Hash::random()),
            Transaction::create_test_transfer(200000, Hash::random()),
            Transaction::create_test_transfer(100000, Hash::random()),
        ];

        let tree = MerkleTree::from_transactions(&transactions).unwrap();
        let root = tree.root();

        // Should be able to generate proofs for all transactions
        for i in 0..transactions.len() {
            let proof = tree.generate_proof(i).unwrap();
            assert!(proof.verify());
            assert_eq!(proof.root, root);
        }
    }

    #[test]
    fn test_hash_pair_function() {
        let hash1 = Hash::from_int(12345);
        let hash2 = Hash::from_int(54321);

        let combined1 = hash_pair(&hash1, &hash2);
        let combined2 = hash_pair(&hash1, &hash2);

        // Should be deterministic
        assert_eq!(combined1, combined2);

        // Different order should give different result
        let combined3 = hash_pair(&hash2, &hash1);
        assert_ne!(combined1, combined3);
    }
}
