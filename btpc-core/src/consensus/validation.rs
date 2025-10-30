// Block and chain validation rules implementation
use anyhow::Result;
use thiserror::Error;

use crate::blockchain::{Block, Transaction};

/// Block validator for BTPC blockchain
#[derive(Debug)]
pub struct BlockValidator {
    // Validation configuration if needed
}

impl BlockValidator {
    /// Create a new block validator
    pub fn new() -> Self {
        BlockValidator {}
    }

    /// Validate a complete block
    pub fn validate_block(&self, block: &Block) -> Result<(), ValidationError> {
        // Validate basic structure first
        block.validate_structure().map_err(ValidationError::Block)?;

        // Validate header
        self.validate_header(&block.header)?;

        // Validate merkle root
        self.validate_merkle_root(block)?;

        // Validate block size
        self.validate_block_size(block)?;

        // Validate proof of work
        self.validate_proof_of_work(block)?;

        // Validate transactions
        self.validate_transactions(block)?;

        Ok(())
    }

    /// Validate block header
    pub fn validate_header(
        &self,
        header: &crate::blockchain::BlockHeader,
    ) -> Result<(), ValidationError> {
        use crate::consensus::constants::MIN_BLOCK_VERSION;

        // Validate block version
        if header.version < MIN_BLOCK_VERSION {
            return Err(ValidationError::InvalidBlockVersion);
        }

        // Delegate to header's built-in validation
        header
            .validate()
            .map_err(|_| ValidationError::InvalidBlockHeader)?;

        // Additional validation can be added here if needed
        Ok(())
    }

    /// Validate merkle root
    pub fn validate_merkle_root(&self, block: &Block) -> Result<(), ValidationError> {
        use crate::blockchain::merkle::calculate_merkle_root;

        let calculated_root = calculate_merkle_root(&block.transactions)
            .map_err(|_| ValidationError::InvalidMerkleRoot)?;

        if calculated_root != block.header.merkle_root {
            return Err(ValidationError::InvalidMerkleRoot);
        }

        Ok(())
    }

    /// Validate block size
    pub fn validate_block_size(&self, block: &Block) -> Result<(), ValidationError> {
        if block.is_oversized() {
            return Err(ValidationError::BlockTooLarge);
        }
        Ok(())
    }

    /// Validate proof of work
    pub fn validate_proof_of_work(&self, block: &Block) -> Result<(), ValidationError> {
        use crate::consensus::{pow::ProofOfWork, DifficultyTarget};

        // Convert block bits to difficulty target
        let target = DifficultyTarget::from_bits(block.header.bits);

        // Validate using PoW module
        ProofOfWork::validate_block_pow(block, &target)
            .map_err(|_| ValidationError::InvalidProofOfWork)?;

        Ok(())
    }

    /// Validate transactions in block
    pub fn validate_transactions(&self, block: &Block) -> Result<(), ValidationError> {
        // Validate each transaction's structure
        for transaction in &block.transactions {
            transaction
                .validate_structure()
                .map_err(|_| ValidationError::InvalidTransaction)?;
        }

        // Validate coinbase transaction rules (already checked in block.validate_structure())
        // Additional transaction validation would go here (signatures, etc.)

        Ok(())
    }
}

impl Default for BlockValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction validator for BTPC transactions
#[derive(Debug)]
pub struct TransactionValidator {
    // Validation configuration if needed
}

impl TransactionValidator {
    /// Create a new transaction validator
    pub fn new() -> Self {
        TransactionValidator {}
    }

    /// Validate a transaction
    pub fn validate_transaction(&self, transaction: &Transaction) -> Result<(), ValidationError> {
        use crate::consensus::constants::MIN_TRANSACTION_VERSION;

        // Validate transaction version
        if transaction.version < MIN_TRANSACTION_VERSION {
            return Err(ValidationError::InvalidTransaction);
        }

        // Validate transaction structure
        transaction
            .validate_structure()
            .map_err(|_| ValidationError::InvalidTransaction)?;

        // Additional validation would include:
        // - Signature verification (requires UTXO set)
        // - Input/output value validation
        // - Script execution
        // For now, we rely on the transaction's built-in validation

        Ok(())
    }
}

impl Default for TransactionValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation error types
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("Invalid merkle root")]
    InvalidMerkleRoot,
    #[error("Invalid proof of work")]
    InvalidProofOfWork,
    #[error("Block too large")]
    BlockTooLarge,
    #[error("Invalid transaction")]
    InvalidTransaction,
    #[error("Invalid block header")]
    InvalidBlockHeader,
    #[error("Invalid timestamp")]
    InvalidTimestamp,
    #[error("Invalid difficulty")]
    InvalidDifficulty,
    #[error("Invalid block version")]
    InvalidBlockVersion,
    #[error("Block error: {0}")]
    Block(crate::blockchain::BlockError),
    #[error("Transaction error: {0}")]
    Transaction(crate::blockchain::TransactionError),
}

impl From<crate::blockchain::BlockError> for ValidationError {
    fn from(err: crate::blockchain::BlockError) -> Self {
        ValidationError::Block(err)
    }
}

impl From<crate::blockchain::TransactionError> for ValidationError {
    fn from(err: crate::blockchain::TransactionError) -> Self {
        ValidationError::Transaction(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::Block;

    #[test]
    fn test_block_validator_creation() {
        let validator = BlockValidator::new();
        assert!(validator
            .validate_header(&crate::blockchain::BlockHeader::create_test_header())
            .is_ok());
    }

    #[test]
    fn test_transaction_validator_creation() {
        let validator = TransactionValidator::new();
        let transaction = crate::blockchain::Transaction::create_test_transfer(
            1000000,
            crate::crypto::Hash::random(),
        );
        assert!(validator.validate_transaction(&transaction).is_ok());
    }

    #[test]
    fn test_complete_block_validation() {
        let validator = BlockValidator::new();
        let mut test_block = Block::create_test_block();

        // Mine the block to get valid PoW
        use crate::consensus::pow::{ProofOfWork, MiningTarget};
        let target = MiningTarget::easy_target();
        if let Ok(proof) = ProofOfWork::mine(&test_block.header, &target) {
            test_block.header.nonce = proof.nonce() as u32;
            // Update bits to match the easy target (bits should encode the same difficulty)
            // For now, use a bits value that corresponds to easy mining
            test_block.header.bits = 0x207fffff; // Already set, but keeping it explicit
        }

        // Should pass all validations for a properly constructed test block with valid PoW
        assert!(validator.validate_block(&test_block).is_ok());
    }

    #[test]
    fn test_validation_error_conversion() {
        let block_error = crate::blockchain::BlockError::BlockTooLarge;
        let validation_error = ValidationError::from(block_error);

        match validation_error {
            ValidationError::Block(_) => (),
            _ => panic!("Expected Block variant"),
        }
    }

    #[test]
    fn test_block_size_validation() {
        let validator = BlockValidator::new();
        let test_block = Block::create_test_block();

        // Normal block should pass size validation
        assert!(validator.validate_block_size(&test_block).is_ok());
    }

    #[test]
    fn test_header_validation() {
        let validator = BlockValidator::new();
        let header = crate::blockchain::BlockHeader::create_test_header();

        // Valid test header should pass
        assert!(validator.validate_header(&header).is_ok());
    }
}
