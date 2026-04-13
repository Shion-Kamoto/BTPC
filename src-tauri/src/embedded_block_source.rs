//! `BlockSource` implementation backing the P2P peer manager.
//!
//! Wraps `UnifiedDatabase` so the peer manager can answer `getheaders` and
//! `getdata` requests from its read-only view of the chain. Injected into
//! `SimplePeerManager` via `set_block_source` during `start_sync`.

use btpc_core::{
    blockchain::{Block, BlockHeader},
    crypto::Hash,
    network::BlockSource,
};

use crate::unified_database::UnifiedDatabase;

/// Thin read-only adapter from `UnifiedDatabase` to the `BlockSource` trait.
pub struct EmbeddedBlockSource {
    db: UnifiedDatabase,
}

impl EmbeddedBlockSource {
    pub fn new(db: UnifiedDatabase) -> Self {
        EmbeddedBlockSource { db }
    }
}

impl BlockSource for EmbeddedBlockSource {
    fn find_fork_point(&self, locator: &[Hash]) -> u32 {
        // Locator is newest → oldest. Return the height of the first hash
        // we know; fall back to 0 (genesis) if none match.
        for hash in locator {
            let hash_bytes = hash.as_bytes();
            if let Ok(Some(height)) = self.db.get_height_by_hash(hash_bytes) {
                return height;
            }
        }
        0
    }

    fn headers_after(&self, start_height: u32, limit: usize, stop: &Hash) -> Vec<BlockHeader> {
        let mut out = Vec::with_capacity(limit.min(2048));
        let stop_is_zero = stop.as_bytes().iter().all(|b| *b == 0);

        // Walk forward by height. We rely on `get_block(height)` returning
        // `Ok(None)` once we go past the tip.
        let mut height = start_height.saturating_add(1);
        while out.len() < limit {
            match self.db.get_block(height) {
                Ok(Some(block)) => {
                    let header = block.header.clone();
                    let reached_stop = !stop_is_zero && block.hash() == *stop;
                    out.push(header);
                    if reached_stop {
                        break;
                    }
                }
                _ => break,
            }
            height = height.saturating_add(1);
            if height == u32::MAX {
                break;
            }
        }
        out
    }

    fn get_block_by_hash(&self, hash: &Hash) -> Option<Block> {
        self.db
            .get_block_by_hash_bytes(hash.as_bytes())
            .ok()
            .flatten()
    }
}
