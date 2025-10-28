// SHA-512 proof-of-work implementation
use anyhow::Result;
use thiserror::Error;

/// Proof of work structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofOfWork {
    nonce: u64,
}

impl ProofOfWork {
    /// Create new proof of work with nonce
    pub fn new(nonce: u64) -> Self {
        ProofOfWork { nonce }
    }

    /// Mine a block to find valid proof of work
    ///
    /// Iterates through the 32-bit nonce space to find a hash that meets the target difficulty.
    /// At high difficulty levels, the entire nonce space (4,294,967,295 attempts) may be
    /// insufficient to find a valid block.
    ///
    /// # Nonce Exhaustion Handling (Issue #16)
    ///
    /// When all nonces are exhausted without finding a valid proof, this function returns
    /// `PoWError::NonceExhausted`. Miners should handle this by:
    ///
    /// 1. **Update timestamp**: Increment block timestamp (respecting MIN_BLOCK_TIME = 60s)
    /// 2. **Modify coinbase**: Change coinbase extra nonce field (when implemented)
    /// 3. **Update merkle root**: Any coinbase change will update the merkle root
    /// 4. **Retry mining**: Call mine() again with the updated header
    ///
    /// # Example Mining Loop
    ///
    /// ```ignore
    /// loop {
    ///     match ProofOfWork::mine(&header, &target) {
    ///         Ok(proof) => {
    ///             // Found valid proof!
    ///             break;
    ///         },
    ///         Err(PoWError::NonceExhausted) => {
    ///             // Exhausted nonce space - update header and retry
    ///             header.timestamp += 1;  // Respecting timing constraints
    ///             // Optionally: Update coinbase extra nonce
    ///             // Recalculate merkle root if coinbase changed
    ///             continue;
    ///         },
    ///         Err(e) => return Err(e),
    ///     }
    /// }
    /// ```
    ///
    /// This approach is Bitcoin-compatible and doesn't require structural changes to BlockHeader.
    pub fn mine(
        header: &crate::blockchain::BlockHeader,
        target: &MiningTarget,
    ) -> Result<Self, PoWError> {
        use rand::Rng;

        let mut mining_header = header.clone();

        // Start from random nonce to avoid miners competing on same nonces (Issue #10)
        let start_nonce = rand::thread_rng().gen::<u32>();
        let mut nonce = start_nonce;

        // Try different nonce values until we find a valid hash
        // Check from start_nonce to u32::MAX, then wrap around to start_nonce
        loop {
            mining_header.nonce = nonce;
            let hash = mining_header.hash();

            if hash.meets_target(&target.as_hash()) {
                return Ok(ProofOfWork {
                    nonce: nonce as u64,
                });
            }

            // Periodically check if we should abort (for real implementation)
            if nonce % 100_000 == 0 {
                // In real implementation, check for shutdown signal
            }

            // Increment with wrapping
            nonce = nonce.wrapping_add(1);

            // If we've wrapped around back to start, we've exhausted all nonces
            if nonce == start_nonce {
                break;
            }
        }

        // Exhausted all 4 billion nonces without finding valid proof
        // Caller should update timestamp or merkle root and retry (Issue #7)
        Err(PoWError::NonceExhausted)
    }

    /// Verify proof of work
    pub fn verify(
        header: &crate::blockchain::BlockHeader,
        proof: &ProofOfWork,
        target: &MiningTarget,
    ) -> bool {
        let mut verification_header = header.clone();
        verification_header.nonce = proof.nonce as u32;

        let hash = verification_header.hash();
        hash.meets_target(&target.as_hash())
    }

    /// Batch verify multiple proofs
    pub fn batch_verify(
        headers: &[crate::blockchain::BlockHeader],
        proofs: &[ProofOfWork],
        target: &MiningTarget,
    ) -> Result<Vec<bool>, PoWError> {
        if headers.len() != proofs.len() {
            return Err(PoWError::InvalidProof);
        }

        let mut results = Vec::with_capacity(headers.len());
        for (header, proof) in headers.iter().zip(proofs.iter()) {
            let is_valid = Self::verify(header, proof, target);
            results.push(is_valid);
        }

        Ok(results)
    }

    /// Get the nonce value
    pub fn nonce(&self) -> u64 {
        self.nonce
    }

    /// Validate block proof of work
    pub fn validate_block_pow(
        block: &crate::blockchain::Block,
        target: &crate::consensus::DifficultyTarget,
    ) -> Result<(), PoWError> {
        // Convert DifficultyTarget to MiningTarget
        let mining_target = MiningTarget::from_bytes(*target.as_bytes());

        // Extract proof from block header nonce
        let proof = ProofOfWork::new(block.header.nonce as u64);

        // Verify the proof
        if Self::verify(&block.header, &proof, &mining_target) {
            Ok(())
        } else {
            Err(PoWError::InvalidProof)
        }
    }

    /// Validate block before mining to avoid wasting hashpower (Issue #13)
    ///
    /// Checks that the block is valid for mining, including:
    /// - Block size doesn't exceed MAX_BLOCK_SIZE
    /// - Block structure is valid
    ///
    /// Call this BEFORE starting mining to ensure you don't waste hashpower
    /// mining an invalid block that will be rejected by the network.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Validate before mining
    /// ProofOfWork::validate_before_mining(&block)?;
    ///
    /// // Now mine with confidence
    /// let proof = ProofOfWork::mine(&block.header, &target)?;
    /// ```
    pub fn validate_before_mining(block: &crate::blockchain::Block) -> Result<(), PoWError> {
        // Check block size (Issue #13: prevent mining oversized blocks)
        if block.is_oversized() {
            return Err(PoWError::BlockOversized {
                actual: block.size(),
                max: crate::blockchain::constants::MAX_BLOCK_SIZE,
            });
        }

        // Validate basic block structure
        block
            .validate_structure()
            .map_err(|e| PoWError::InvalidBlockStructure(format!("{}", e)))?;

        Ok(())
    }

    /// Mine a block with pre-validation (Issue #13)
    ///
    /// Convenience function that validates the block before mining to prevent
    /// wasting hashpower on invalid blocks. This is the recommended way to mine.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Validates and mines in one call
    /// let proof = ProofOfWork::mine_validated(&block, &target)?;
    /// ```
    pub fn mine_validated(
        block: &crate::blockchain::Block,
        target: &MiningTarget,
    ) -> Result<Self, PoWError> {
        // Validate before mining (Issue #13)
        Self::validate_before_mining(block)?;

        // Now mine the validated block
        Self::mine(&block.header, target)
    }
}

/// Mining target for difficulty
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiningTarget {
    target: [u8; 64], // SHA-512 target
}

impl MiningTarget {
    /// Create mining target from difficulty
    pub fn from_difficulty(difficulty: crate::consensus::difficulty::Difficulty) -> Self {
        // Convert Difficulty to DifficultyTarget, then extract target bytes
        let difficulty_target = crate::consensus::DifficultyTarget::from_bits(difficulty.bits());
        MiningTarget {
            target: *difficulty_target.as_bytes()
        }
    }

    /// Get target as bytes
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.target
    }

    /// Get target as Hash (for comparison)
    pub fn as_hash(&self) -> [u8; 64] {
        self.target
    }

    /// Create easy target for testing
    pub fn easy_target() -> Self {
        let mut target = [0u8; 64];
        target[0] = 0x7f; // Very easy target with high value
        for i in 1..64 {
            target[i] = 0xff;
        }
        MiningTarget { target }
    }

    /// Create medium difficulty target for testing
    /// Harder than easy_target but still feasible for unit tests
    pub fn medium_target() -> Self {
        let mut target = [0u8; 64];
        // Target requires hash to start with ~8 zero bits
        // 0x00ff... means first byte must be 0x00, second can be up to 0xff
        target[0] = 0x00;
        target[1] = 0xff;
        for i in 2..64 {
            target[i] = 0xff;
        }
        MiningTarget { target }
    }

    /// Create from target bytes
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        MiningTarget { target: bytes }
    }
}

/// Proof of work errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PoWError {
    #[error("Mining failed")]
    MiningFailed,
    #[error("Nonce space exhausted - caller should update timestamp or merkle root and retry")]
    NonceExhausted,
    #[error("Invalid proof of work")]
    InvalidProof,
    #[error("Target calculation failed")]
    TargetCalculationFailed,
    #[error("Block oversized: {actual} bytes exceeds max {max} bytes")]
    BlockOversized { actual: usize, max: usize },
    #[error("Invalid block structure: {0}")]
    InvalidBlockStructure(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{blockchain::BlockHeader, crypto::Hash};

    #[test]
    fn test_mining_target_creation() {
        let target = MiningTarget::easy_target();
        assert!(target.as_bytes()[0] == 0x7f);
        assert!(target.as_bytes()[1] == 0xff);
    }

    #[test]
    fn test_proof_of_work_creation() {
        let proof = ProofOfWork::new(42);
        assert_eq!(proof.nonce(), 42);
    }

    #[test]
    fn test_proof_verification() {
        let header = BlockHeader::create_test_header();
        // Use medium target for this test to ensure wrong headers fail
        // easy_target is too permissive (0x7fff...) - wrong headers often still pass
        let target = MiningTarget::medium_target();

        // Mine a proof for the header
        if let Ok(proof) = ProofOfWork::mine(&header, &target) {
            // Verify the proof
            assert!(ProofOfWork::verify(&header, &proof, &target));

            // Wrong header should fail (change timestamp instead of nonce,
            // since verify() overwrites the nonce with proof.nonce)
            let mut wrong_header = header.clone();
            wrong_header.timestamp = header.timestamp + 1;
            assert!(!ProofOfWork::verify(&wrong_header, &proof, &target));
        }
    }

    #[test]
    fn test_batch_verification() {
        let headers = vec![
            BlockHeader::create_test_header(),
            BlockHeader::create_test_header(),
        ];
        let proofs = vec![ProofOfWork::new(0), ProofOfWork::new(1)];
        let target = MiningTarget::easy_target();

        let results = ProofOfWork::batch_verify(&headers, &proofs, &target).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_batch_verification_size_mismatch() {
        let headers = vec![BlockHeader::create_test_header()];
        let proofs = vec![ProofOfWork::new(0), ProofOfWork::new(1)];
        let target = MiningTarget::easy_target();

        assert!(ProofOfWork::batch_verify(&headers, &proofs, &target).is_err());
    }
}
