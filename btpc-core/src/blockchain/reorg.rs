//! Blockchain reorganization logic
//!
//! Implements chain reorganization (reorg) to ensure nodes always follow
//! the chain with the most accumulated proof-of-work.

use std::collections::VecDeque;

use crate::{
    blockchain::{Block, BlockError},
    consensus::DifficultyTarget,
    crypto::Hash,
    storage::{BlockchainDatabase, BlockchainDbError},
};

/// Result of comparing two chains
#[derive(Debug, Clone, PartialEq)]
pub enum ChainComparison {
    /// Current chain has more work
    CurrentChainBetter,
    /// New chain has more work - reorganization needed
    NewChainBetter {
        /// Common ancestor block hash
        fork_point: Hash,
        /// Blocks to disconnect (in reverse order)
        disconnect: Vec<Hash>,
        /// Blocks to connect (in forward order)
        connect: Vec<Hash>,
    },
    /// Chains have equal work
    Equal,
}

/// Calculate accumulated work for a chain up to a specific block
///
/// This traverses from the specified block back to genesis, summing work.
pub fn calculate_chain_work(
    block_hash: &Hash,
    db: &dyn BlockchainDatabase,
) -> Result<f64, BlockchainDbError> {
    let mut current_hash = *block_hash;
    let mut total_work = 0.0;

    loop {
        // Get the block header
        let header = match db.get_header(&current_hash)? {
            Some(h) => h,
            None => return Ok(total_work), // Block not found, return accumulated work
        };

        // Add this block's work
        let target = DifficultyTarget::from_bits(header.bits);
        total_work += target.work_integer() as f64;

        // Check if we've reached genesis
        if header.prev_hash == Hash::zero() {
            break;
        }

        // Move to previous block
        current_hash = header.prev_hash;
    }

    Ok(total_work)
}

/// Find the common ancestor (fork point) between two chains
///
/// Returns the hash of the most recent common ancestor block.
pub fn find_fork_point(
    hash1: &Hash,
    hash2: &Hash,
    db: &dyn BlockchainDatabase,
) -> Result<Hash, BlockchainDbError> {
    // Build path from hash1 to genesis
    let mut path1 = Vec::new();
    let mut current = *hash1;

    while current != Hash::zero() {
        path1.push(current);
        match db.get_header(&current)? {
            Some(header) => current = header.prev_hash,
            None => break,
        }
    }

    // Traverse from hash2 backwards until we find a block in path1
    current = *hash2;
    while current != Hash::zero() {
        if path1.contains(&current) {
            return Ok(current); // Found common ancestor
        }

        match db.get_header(&current)? {
            Some(header) => current = header.prev_hash,
            None => break,
        }
    }

    // If no common ancestor found, return genesis
    Ok(Hash::zero())
}

/// Compare two competing chains and determine if reorganization is needed
///
/// Returns ChainComparison indicating whether to reorganize and what blocks
/// to disconnect/connect.
pub fn compare_chains(
    current_tip: &Hash,
    new_tip: &Hash,
    db: &dyn BlockchainDatabase,
) -> Result<ChainComparison, BlockchainDbError> {
    // Calculate work for both chains
    let current_work = calculate_chain_work(current_tip, db)?;
    let new_work = calculate_chain_work(new_tip, db)?;

    // If current chain has more work, no reorg needed
    if current_work > new_work {
        return Ok(ChainComparison::CurrentChainBetter);
    }

    // If chains have equal work, no reorg
    if (current_work - new_work).abs() < 0.0001 {
        return Ok(ChainComparison::Equal);
    }

    // New chain has more work - find fork point
    let fork_point = find_fork_point(current_tip, new_tip, db)?;

    // Build list of blocks to disconnect (current chain from tip to fork)
    let mut disconnect = Vec::new();
    let mut current = *current_tip;

    while current != fork_point {
        disconnect.push(current);
        match db.get_header(&current)? {
            Some(header) => current = header.prev_hash,
            None => break,
        }
    }

    // Build list of blocks to connect (new chain from fork to new tip)
    let mut connect = Vec::new();
    current = *new_tip;

    while current != fork_point {
        connect.push(current);
        match db.get_header(&current)? {
            Some(header) => current = header.prev_hash,
            None => break,
        }
    }

    // Reverse connect list so it's in forward order (from fork to tip)
    connect.reverse();

    Ok(ChainComparison::NewChainBetter {
        fork_point,
        disconnect,
        connect,
    })
}

/// Get the chain of block hashes from a given tip back to a specific ancestor
pub fn get_chain_to_ancestor(
    tip: &Hash,
    ancestor: &Hash,
    db: &dyn BlockchainDatabase,
) -> Result<Vec<Hash>, BlockchainDbError> {
    let mut chain = Vec::new();
    let mut current = *tip;

    while current != *ancestor && current != Hash::zero() {
        chain.push(current);
        match db.get_header(&current)? {
            Some(header) => current = header.prev_hash,
            None => break,
        }
    }

    Ok(chain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        blockchain::Block,
        storage::{database::*, BlockchainDb},
    };
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_db() -> (BlockchainDb, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_config = DatabaseConfig::test();
        let database = Arc::new(Database::open(temp_dir.path(), db_config).unwrap());
        (BlockchainDb::new(database), temp_dir)
    }

    #[test]
    fn test_calculate_chain_work() {
        let (mut db, _temp) = create_test_db();

        // Store genesis
        let genesis = Block::create_genesis_block();
        let genesis_hash = genesis.hash();
        db.store_block(&genesis).unwrap();

        // Calculate work should succeed
        let work = calculate_chain_work(&genesis_hash, &db).unwrap();
        assert!(work > 0.0);
    }

    #[test]
    fn test_find_fork_point_same_block() {
        let (mut db, _temp) = create_test_db();

        let genesis = Block::create_genesis_block();
        let genesis_hash = genesis.hash();
        db.store_block(&genesis).unwrap();

        // Fork point of same block should be itself
        let fork = find_fork_point(&genesis_hash, &genesis_hash, &db).unwrap();
        assert_eq!(fork, genesis_hash);
    }
}