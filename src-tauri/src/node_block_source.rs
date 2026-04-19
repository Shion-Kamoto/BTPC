//! `UnifiedDatabaseBlockSource` — `BlockSource` impl backed by the node's
//! `UnifiedDatabase` (RocksDB).
//!
//! Phase 8 (003-testnet-p2p-hardening, 2026-04-20, T116): wiring the flat
//! tree's `IntegratedSyncManager` + `SimplePeerManager` to the node-side
//! block store so a catching-up peer can receive headers and blocks from us.
//!
//! Design:
//! - Read-only view; no background tasks, no locks held across awaits.
//! - All lookups reuse the existing `UnifiedDatabase` schema:
//!     * `b"height:" + block_hash (64B)` → height (u32 LE)  (DEFAULT CF)
//!     * `b"block:"  + height (u32 LE)`  → block_hash (64B) (DEFAULT CF)
//!     * `b"block:"  + block_hash (64B)` → serialized Block (CF_BLOCKS)
//! - No new column families or migrations.

use std::sync::Arc;

use btpc_core::{
    blockchain::{Block, BlockHeader},
    crypto::Hash,
    network::block_source::BlockSource,
};

use crate::unified_database::{UnifiedDatabase, CF_BLOCKS};

/// Production `BlockSource` impl reading from the embedded node's
/// `UnifiedDatabase`. Cheap to clone (`Arc` under the hood).
pub struct UnifiedDatabaseBlockSource {
    db: Arc<UnifiedDatabase>,
}

impl UnifiedDatabaseBlockSource {
    pub fn new(db: Arc<UnifiedDatabase>) -> Self {
        Self { db }
    }

    /// Look up the stored height for a given block hash using the
    /// `height:` index. Returns `None` if the block is not in our chain.
    fn height_of(&self, hash: &Hash) -> Option<u32> {
        let mut key = Vec::with_capacity(7 + 64);
        key.extend_from_slice(b"height:");
        key.extend_from_slice(hash.as_bytes());
        match self.db.inner().get(&key).ok().flatten() {
            Some(bytes) if bytes.len() == 4 => Some(u32::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3],
            ])),
            _ => None,
        }
    }

    /// Fetch a block at a given height via the `block:<height>` index and
    /// the `CF_BLOCKS` column family.
    fn block_at_height(&self, height: u32) -> Option<Block> {
        let hash_bytes = match self.db.get_block_hash_at_height(height).ok().flatten() {
            Some(b) => b,
            None => return None,
        };
        self.block_by_raw_hash(&hash_bytes)
    }

    fn block_by_raw_hash(&self, hash_bytes: &[u8]) -> Option<Block> {
        if hash_bytes.len() != 64 {
            return None;
        }
        let blocks_cf = self.db.cf_handle(CF_BLOCKS)?;
        let mut block_key = Vec::with_capacity(6 + 64);
        block_key.extend_from_slice(b"block:");
        block_key.extend_from_slice(hash_bytes);
        let block_bytes = self.db.inner().get_cf(&blocks_cf, &block_key).ok().flatten()?;
        Block::deserialize(&block_bytes).ok()
    }
}

impl BlockSource for UnifiedDatabaseBlockSource {
    /// Walk the locator newest → oldest. The first hash we recognise is the
    /// fork point; return its height. If none match, return 0 (genesis) so
    /// the caller starts from block 1.
    fn find_fork_point(&self, locator: &[Hash]) -> u32 {
        for h in locator {
            if let Some(height) = self.height_of(h) {
                return height;
            }
        }
        0
    }

    /// Return up to `limit` headers starting at `start_height + 1`, stopping
    /// early when `stop` is encountered (inclusive) or the tip is reached.
    fn headers_after(&self, start_height: u32, limit: usize, stop: &Hash) -> Vec<BlockHeader> {
        let mut out: Vec<BlockHeader> = Vec::with_capacity(limit.min(2000));
        let zero = Hash::from_bytes([0u8; 64]);
        let mut height = start_height.saturating_add(1);
        while out.len() < limit {
            let block = match self.block_at_height(height) {
                Some(b) => b,
                None => break, // reached tip
            };
            let hash = block.hash();
            out.push(block.header);
            if stop != &zero && &hash == stop {
                break;
            }
            height = match height.checked_add(1) {
                Some(h) => h,
                None => break,
            };
        }
        out
    }

    fn get_block_by_hash(&self, hash: &Hash) -> Option<Block> {
        self.block_by_raw_hash(hash.as_bytes())
    }
}
