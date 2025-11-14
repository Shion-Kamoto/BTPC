# Chain Reorganization Implementation Plan

## Overview
Implement chain reorganization (reorg) to handle competing blockchain forks and select the chain with the most cumulative proof-of-work.

## Current Limitation
The blockchain currently only accepts blocks that connect directly to the current tip:
```rust
// btpc-core/src/blockchain/chain.rs:80-82
if block.header.prev_hash != self.tip_hash {
    return Err(ChainError::DoesNotConnect);
}
```

This prevents the node from:
1. Accepting valid blocks that fork from earlier heights
2. Switching to a longer/heavier chain when discovered
3. Recovering from temporary network partitions

## Implementation Requirements

### 1. Chain State Storage
**File**: `btpc-core/src/blockchain/chain_state.rs` (NEW)

- Store multiple competing branches (not just main chain)
- Track cumulative work for each branch
- Maintain orphan block pool for blocks arriving out of order

```rust
pub struct ChainState {
    /// All blocks by hash
    all_blocks: HashMap<Hash, BlockMetadata>,

    /// Active branches (tip hash -> cumulative work)
    branches: HashMap<Hash, ChainWork>,

    /// Main chain (best cumulative work)
    main_chain_tip: Hash,

    /// Orphan blocks (blocks without parent)
    orphan_pool: HashMap<Hash, Block>,
}

pub struct BlockMetadata {
    block: Block,
    height: u32,
    cumulative_work: ChainWork,
    parent_hash: Hash,
    children: Vec<Hash>,
}
```

### 2. Cumulative Work Calculation
**File**: `btpc-core/src/consensus/chain_work.rs` (NEW)

- Calculate cumulative proof-of-work for a chain
- Compare chains to find "heaviest" (most work)

```rust
pub struct ChainWork {
    work: BigUint,
}

impl ChainWork {
    pub fn from_target(target: &MiningTarget) -> Self;
    pub fn add(&self, other: &ChainWork) -> ChainWork;
    pub fn is_greater_than(&self, other: &ChainWork) -> bool;
}
```

### 3. Reorganization Logic
**File**: `btpc-core/src/blockchain/reorganize.rs` (NEW)

```rust
pub struct Reorganizer {
    blockchain_db: Arc<RwLock<BlockchainDb>>,
    utxo_db: Arc<RwLock<UtxoDb>>,
}

impl Reorganizer {
    /// Attempt to reorganize to a new chain tip
    pub fn try_reorganize(&mut self, new_tip: Hash) -> Result<ReorgResult, ReorgError> {
        // 1. Find common ancestor
        let fork_point = self.find_fork_point(&new_tip)?;

        // 2. Disconnect blocks from old chain
        let old_blocks = self.disconnect_blocks_from(fork_point)?;

        // 3. Connect blocks to new chain
        let new_blocks = self.connect_blocks_to(new_tip)?;

        // 4. Update UTXO set
        self.update_utxos(&old_blocks, &new_blocks)?;

        Ok(ReorgResult {
            fork_point,
            disconnected_count: old_blocks.len(),
            connected_count: new_blocks.len(),
        })
    }

    fn find_fork_point(&self, tip: &Hash) -> Result<Hash, ReorgError>;
    fn disconnect_blocks_from(&mut self, fork_point: Hash) -> Result<Vec<Block>, ReorgError>;
    fn connect_blocks_to(&mut self, new_tip: Hash) -> Result<Vec<Block>, ReorgError>;
    fn update_utxos(&mut self, old: &[Block], new: &[Block]) -> Result<(), ReorgError>;
}
```

### 4. Block Reception Handler
**File**: `bins/btpc_node/src/main.rs` (MODIFY)

Update the `start_peer_event_handler()` method:

```rust
PeerEvent::BlockReceived { from, block } => {
    let block_hash = block.hash();

    // Try to add to main chain
    match blockchain_mut.store_block(&block) {
        Ok(_) => {
            println!("✅ Block {} added to main chain", block_hash.to_hex());
            *block_height.write().await += 1;
        }
        Err(BlockchainDbError::DoesNotConnect) => {
            // Block doesn't connect to tip - check if it creates a better fork
            let reorganizer = Reorganizer::new(blockchain_db, utxo_db);

            match reorganizer.try_reorganize(block_hash) {
                Ok(reorg_result) => {
                    println!("🔄 Chain reorganization:");
                    println!("   Fork point: height {}", reorg_result.fork_point);
                    println!("   Disconnected: {} blocks", reorg_result.disconnected_count);
                    println!("   Connected: {} blocks", reorg_result.connected_count);

                    // Broadcast reorganization to peers
                    for block in reorg_result.new_blocks {
                        peer_manager.broadcast_block(&block).await;
                    }
                }
                Err(ReorgError::NotEnoughWork) => {
                    println!("📦 Block {} stored as side chain (insufficient work)", block_hash.to_hex());
                }
                Err(e) => {
                    eprintln!("❌ Reorganization failed: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to store block: {}", e);
        }
    }
}
```

### 5. Database Schema Changes
**File**: `btpc-core/src/storage/blockchain_db.rs` (MODIFY)

Add column families:
- `block_metadata`: Store cumulative work and parent/child links
- `chain_branches`: Track active fork tips
- `orphan_blocks`: Temporarily store blocks without parents

```rust
// New methods
fn store_block_metadata(&mut self, hash: &Hash, metadata: &BlockMetadata) -> Result<()>;
fn get_cumulative_work(&self, hash: &Hash) -> Result<ChainWork>;
fn get_branch_tips(&self) -> Result<Vec<Hash>>;
fn store_orphan(&mut self, block: Block) -> Result<()>;
fn get_orphans_by_parent(&self, parent_hash: &Hash) -> Result<Vec<Block>>;
```

### 6. Testing Requirements

**File**: `tests/integration/chain_reorg_test.rs` (NEW)

```rust
#[tokio::test]
async fn test_simple_reorganization() {
    // 1. Mine main chain: Genesis -> A -> B -> C
    // 2. Mine side chain: Genesis -> A -> D -> E -> F (longer)
    // 3. Verify node switches to D -> E -> F chain
    // 4. Verify UTXOs updated correctly
}

#[tokio::test]
async fn test_reorg_with_double_spend() {
    // 1. Mine chain with transaction TX1
    // 2. Mine competing chain with conflicting TX2
    // 3. Verify only one transaction is valid after reorg
}

#[tokio::test]
async fn test_orphan_block_handling() {
    // 1. Receive block at height 5 before blocks 1-4
    // 2. Verify stored as orphan
    // 3. Receive missing blocks 1-4
    // 4. Verify orphan connects and chain extends
}
```

## Implementation Steps

1. **Phase 1**: Chain work calculation (1-2 hours)
   - Implement `ChainWork` struct and cumulative work calculation
   - Add unit tests for work comparison

2. **Phase 2**: Multi-branch storage (2-3 hours)
   - Extend database schema with new column families
   - Implement block metadata storage
   - Add orphan block pool

3. **Phase 3**: Reorganization logic (3-4 hours)
   - Implement fork point detection
   - Implement block disconnect/reconnect
   - Handle UTXO set updates during reorg

4. **Phase 4**: Node integration (1-2 hours)
   - Update block reception handler
   - Add reorg event notifications
   - Test with actual P2P network

5. **Phase 5**: Testing & validation (2-3 hours)
   - Write integration tests
   - Test with testnet deployment
   - Handle edge cases

**Total Estimated Time**: 9-14 hours

## Risks & Mitigation

### Risk: UTXO Set Corruption
- **Mitigation**: Use database transactions for atomic reorg operations
- **Fallback**: Maintain UTXO snapshot before each reorg

### Risk: Performance on Deep Reorgs
- **Mitigation**: Limit max reorg depth (e.g., 100 blocks)
- **Monitoring**: Log reorg depth and duration

### Risk: Double-Spend Attacks
- **Mitigation**: Require confirmations before considering transactions final
- **Best Practice**: 6+ confirmations for large-value transactions

## Success Criteria

✅ Node accepts blocks that fork from earlier heights
✅ Node switches to chain with most cumulative work
✅ UTXO set remains consistent after reorganization
✅ Orphan blocks processed when missing parents arrive
✅ All integration tests pass
✅ Testnet demonstrates successful reorg scenarios

## Related Files

- `btpc-core/src/blockchain/chain.rs` - Current linear chain implementation
- `btpc-core/src/consensus/pow.rs` - Proof-of-work validation
- `btpc-core/src/storage/blockchain_db.rs` - Block storage
- `btpc-core/src/storage/utxo_db.rs` - UTXO management
- `bins/btpc_node/src/main.rs` - Block reception handling

## References

- Bitcoin Core reorg implementation: `src/validation.cpp:ActivateBestChain()`
- Ethereum fork choice: GHOST protocol
- [Bitcoin Wiki: Chain Reorganization](https://en.bitcoin.it/wiki/Chain_Reorganization)
