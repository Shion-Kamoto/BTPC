//! BlockSource trait — the read-only view of the blockchain that the
//! peer manager needs to answer `getheaders` and `getdata` requests.
//!
//! Ported from btpc-desktop-app/btpc-core/src/network/block_source.rs
//! for the live flat tree (T053, US3).
//!
//! Public API surface:
//!   - `BlockSource` trait: `find_fork_point`, `headers_after`, `get_block_by_hash`
//!
//! This trait is implemented by the embedding application (e.g. the desktop
//! app's `EmbeddedNode`) and injected into `SimplePeerManager` via
//! `set_block_source`. Keeping it as a trait means `btpc-core` has no
//! dependency on any particular storage backend.

use crate::{
    blockchain::{Block, BlockHeader},
    crypto::Hash,
};

/// Read-only blockchain view used by the P2P layer.
///
/// All methods are synchronous and called from async tokio tasks, so
/// implementations must not block for long periods. Typical implementations
/// will be thin wrappers around the local block database.
pub trait BlockSource: Send + Sync {
    /// Given a Bitcoin-style block locator (hashes ordered newest -> oldest),
    /// return the height of the most recent hash we know about.
    ///
    /// Returns `0` (genesis) if none of the locator hashes are known, which
    /// tells the caller to start sending headers from block 1.
    fn find_fork_point(&self, locator: &[Hash]) -> u32;

    /// Return up to `limit` block headers starting at height `start_height + 1`.
    ///
    /// Iteration stops early if a header with hash `stop` is encountered
    /// (that header is included). If `stop` is the zero hash, the iteration
    /// continues until `limit` headers have been collected or the tip is
    /// reached.
    fn headers_after(&self, start_height: u32, limit: usize, stop: &Hash) -> Vec<BlockHeader>;

    /// Fetch a full block by its hash. Returns `None` if not known.
    fn get_block_by_hash(&self, hash: &Hash) -> Option<Block>;
}
