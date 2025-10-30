//! Blockchain synchronization logic
//!
//! Handles syncing with other peers to maintain blockchain consensus.

use std::collections::VecDeque;

use thiserror::Error;

use crate::{
    blockchain::{Block, BlockHeader},
    crypto::Hash,
    network::{NetworkError, ProtocolError},
    storage::BlockchainDatabase,
};

/// Blockchain synchronization manager
pub struct SyncManager {
    /// Current sync state
    state: SyncState,
    /// Download queue for blocks
    download_queue: VecDeque<Hash>,
    /// Maximum blocks to download in parallel
    max_parallel_downloads: usize,
    /// Currently downloading blocks
    downloading: std::collections::HashSet<Hash>,
}

/// Synchronization states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncState {
    /// Not syncing
    Idle,
    /// Syncing headers
    SyncingHeaders,
    /// Syncing blocks
    SyncingBlocks,
    /// Fully synced
    Synced,
}

/// Sync errors
#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("Invalid block: {0}")]
    InvalidBlock(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Sync timeout")]
    Timeout,
}

impl SyncManager {
    /// Create a new sync manager
    pub fn new(max_parallel_downloads: usize) -> Self {
        SyncManager {
            state: SyncState::Idle,
            download_queue: VecDeque::new(),
            max_parallel_downloads,
            downloading: std::collections::HashSet::new(),
        }
    }

    /// Get current sync state
    pub fn state(&self) -> &SyncState {
        &self.state
    }

    /// Check if we are fully synced
    pub fn is_synced(&self) -> bool {
        self.state == SyncState::Synced
    }

    /// Start initial sync
    pub fn start_sync(&mut self) -> Result<(), SyncError> {
        // For a local testnet with only genesis block and no peers,
        // we can immediately transition to synced state
        // In a real deployment with peers, this would start header sync
        if self.download_queue.is_empty() && self.downloading.is_empty() {
            // No blocks to download, mark as synced
            self.state = SyncState::Synced;
        } else {
            // Start header sync if there are blocks to download
            self.state = SyncState::SyncingHeaders;
        }
        Ok(())
    }

    /// Process received headers
    pub fn process_headers(
        &mut self,
        headers: Vec<BlockHeader>,
        db: &dyn BlockchainDatabase,
    ) -> Result<Vec<Hash>, SyncError> {
        let mut needed_blocks = Vec::new();

        for header in headers {
            let block_hash = header.hash();

            // Check if we already have this block
            match db.get_header(&block_hash) {
                Ok(Some(_)) => continue, // Already have this block
                Ok(None) => {
                    // Need to download this block
                    needed_blocks.push(block_hash);
                    self.download_queue.push_back(block_hash);
                }
                Err(e) => return Err(SyncError::Database(e.to_string())),
            }
        }

        if !needed_blocks.is_empty() {
            self.state = SyncState::SyncingBlocks;
        }

        Ok(needed_blocks)
    }

    /// Process received block
    pub fn process_block(
        &mut self,
        block: Block,
        db: &mut dyn BlockchainDatabase,
    ) -> Result<(), SyncError> {
        let block_hash = block.hash();

        // Remove from downloading set
        self.downloading.remove(&block_hash);

        // Validate block (basic checks)
        if !self.validate_block(&block) {
            return Err(SyncError::InvalidBlock(
                "Block validation failed".to_string(),
            ));
        }

        // Store block in database
        db.store_block(&block)
            .map_err(|e| SyncError::Database(e.to_string()))?;

        // Update sync state
        if self.download_queue.is_empty() && self.downloading.is_empty() {
            self.state = SyncState::Synced;
        }

        Ok(())
    }

    /// Get next blocks to download
    pub fn get_next_downloads(&mut self) -> Vec<Hash> {
        let mut downloads = Vec::new();

        while downloads.len() < self.max_parallel_downloads && !self.download_queue.is_empty() {
            if let Some(hash) = self.download_queue.pop_front() {
                if !self.downloading.contains(&hash) {
                    self.downloading.insert(hash);
                    downloads.push(hash);
                }
            }
        }

        downloads
    }

    /// Handle download timeout for a block
    pub fn handle_timeout(&mut self, block_hash: &Hash) {
        self.downloading.remove(block_hash);
        self.download_queue.push_front(*block_hash);
    }

    /// Get sync progress as percentage
    pub fn progress(&self) -> f32 {
        match self.state {
            SyncState::Idle => 0.0,
            SyncState::SyncingHeaders => 25.0,
            SyncState::SyncingBlocks => {
                let total = self.download_queue.len() + self.downloading.len();
                if total == 0 {
                    100.0
                } else {
                    let completed = self.downloading.len();
                    25.0 + (75.0 * completed as f32 / total as f32)
                }
            }
            SyncState::Synced => 100.0,
        }
    }

    /// Basic block validation (preliminary check)
    ///
    /// Note: This is a lightweight validation for sync manager.
    /// Full validation (PoW, signatures, consensus rules) is performed by
    /// StorageBlockValidator in the integrated_sync layer.
    ///
    /// This function performs basic sanity checks:
    /// - Block must have at least 1 transaction (coinbase)
    /// - Timestamp must be reasonable (not too far in future)
    fn validate_block(&self, block: &Block) -> bool {
        // Must have at least coinbase transaction
        if block.transactions.is_empty() {
            return false;
        }

        // Timestamp shouldn't be more than 2 hours in the future
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        const TWO_HOURS: u64 = 2 * 60 * 60;
        if block.header.timestamp > current_time + TWO_HOURS {
            return false;
        }

        // Basic checks passed - full validation happens in StorageBlockValidator
        true
    }
}

/// Locator for finding common block
pub fn create_block_locator(
    tip_hash: Hash,
    db: &dyn BlockchainDatabase,
) -> Result<Vec<Hash>, SyncError> {
    let mut locator = Vec::new();
    let mut current_hash = tip_hash;
    let mut step = 1;

    // Add the tip
    locator.push(current_hash);

    // Walk backwards with exponentially increasing steps
    loop {
        // Get the header for current hash
        let header = match db.get_header(&current_hash) {
            Ok(Some(header)) => header,
            Ok(None) => break, // Reached genesis or orphaned chain
            Err(e) => return Err(SyncError::Database(e.to_string())),
        };

        // Add to locator
        locator.push(current_hash);

        // Move to previous block
        current_hash = header.prev_hash;

        // Increase step exponentially after the first 10 blocks
        if locator.len() > 10 {
            step *= 2;
        }

        // Skip 'step' number of blocks
        for _ in 0..step {
            let header = match db.get_header(&current_hash) {
                Ok(Some(header)) => header,
                Ok(None) => break, // Reached genesis
                Err(e) => return Err(SyncError::Database(e.to_string())),
            };
            current_hash = header.prev_hash;
        }

        // Stop if we've collected enough locators
        if locator.len() >= 500 {
            break;
        }
    }

    Ok(locator)
}

/// Inventory manager for efficient data exchange
pub struct InventoryManager {
    /// Known inventory items
    known_items: std::collections::HashSet<Hash>,
    /// Maximum items to track
    max_items: usize,
}

impl InventoryManager {
    /// Create a new inventory manager
    pub fn new(max_items: usize) -> Self {
        InventoryManager {
            known_items: std::collections::HashSet::new(),
            max_items,
        }
    }

    /// Add item to known inventory
    pub fn add_item(&mut self, hash: Hash) {
        if self.known_items.len() >= self.max_items {
            // Simple eviction: clear half the items
            let items_to_remove: Vec<Hash> = self
                .known_items
                .iter()
                .take(self.max_items / 2)
                .cloned()
                .collect();

            for item in items_to_remove {
                self.known_items.remove(&item);
            }
        }

        self.known_items.insert(hash);
    }

    /// Check if item is known
    pub fn is_known(&self, hash: &Hash) -> bool {
        self.known_items.contains(hash)
    }

    /// Filter out known items from a list
    pub fn filter_unknown(&self, items: Vec<Hash>) -> Vec<Hash> {
        items
            .into_iter()
            .filter(|hash| !self.is_known(hash))
            .collect()
    }

    /// Get number of known items
    pub fn count(&self) -> usize {
        self.known_items.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_manager_creation() {
        let sync_manager = SyncManager::new(10);
        assert_eq!(sync_manager.state(), &SyncState::Idle);
        assert!(sync_manager.is_synced() == false);
    }

    #[test]
    fn test_sync_progress() {
        let sync_manager = SyncManager::new(10);
        assert_eq!(sync_manager.progress(), 0.0);
    }

    #[test]
    fn test_inventory_manager() {
        let mut inv_manager = InventoryManager::new(100);
        let hash = Hash::zero();

        assert!(!inv_manager.is_known(&hash));
        inv_manager.add_item(hash);
        assert!(inv_manager.is_known(&hash));
        assert_eq!(inv_manager.count(), 1);
    }
}
