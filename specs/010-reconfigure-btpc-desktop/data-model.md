# Data Model: Embedded btpc-core In-Process Architecture

**Feature**: 010-reconfigure-btpc-desktop
**Version**: 1.0
**Status**: Phase 1 Design
**Created**: 2025-11-10

---

## Overview

This document defines the key entities for the embedded blockchain architecture, their relationships, thread-safety patterns, and state transitions. All entities transition from multi-process IPC architecture to in-process shared memory with Arc-based concurrency.

---

## Core Entities

### 1. EmbeddedNode

**Purpose**: Represents the in-process blockchain node (replaces external btpc_node binary)

**Fields**:
```rust
struct EmbeddedNode {
    /// Blockchain state (blocks, headers, UTXO set)
    blockchain: Arc<RwLock<Blockchain>>,

    /// Unconfirmed transaction pool
    mempool: Arc<RwLock<Mempool>>,

    /// P2P connection manager
    p2p_manager: Arc<P2PManager>,

    /// Consensus validation engine
    consensus: Arc<ConsensusValidator>,

    /// Shared RocksDB instance
    database: Arc<UnifiedDatabase>,

    /// Current sync status
    sync_state: Arc<RwLock<SyncState>>,

    /// Network configuration (Mainnet/Testnet/Regtest)
    network: NetworkType,

    /// Node initialization timestamp
    started_at: SystemTime,

    /// Shutdown signal receiver
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
}
```

**Types**:
- `blockchain: Arc<RwLock<Blockchain>>` - Read-heavy (balance queries, UTXO lookup), write-rare (new block)
- `mempool: Arc<RwLock<Mempool>>` - Moderate writes (new tx), reads (mining template)
- `p2p_manager: Arc<P2PManager>` - Internal Mutex for peer list, async message passing
- `consensus: Arc<ConsensusValidator>` - Stateless (pure functions), no locking needed
- `database: Arc<UnifiedDatabase>` - RocksDB provides internal synchronization
- `sync_state: Arc<RwLock<SyncState>>` - Frequent reads (UI updates), infrequent writes (every block)
- `network: NetworkType` - Immutable after initialization (enum: Mainnet, Testnet, Regtest)
- `shutdown_rx: tokio::sync::watch::Receiver<bool>` - Async signal, no explicit locking

**Relationships**:
- **Shared by**: WalletManager (UTXO queries), MiningThreadPool (block submission), UIEventEmitter (state updates)
- **Owns**: UnifiedDatabase (exclusive access via Arc), Mempool, Blockchain state
- **Communicates with**: P2P peers (external network), Tauri frontend (events)

**Validation Rules**:
1. Node MUST initialize database before accepting commands (state: Initializing → Ready)
2. Blockchain height MUST be monotonically increasing (validated by consensus)
3. UTXO set MUST be consistent with blockchain state (validated on block add)
4. Mempool transactions MUST have valid ML-DSA signatures (pre-validation before insertion)
5. Network type MUST NOT change after initialization (immutable configuration)
6. Shutdown signal MUST cancel all background tasks within 30 seconds

**Thread-Safety Patterns**:
- **Read-heavy workload**: Use `RwLock` for blockchain/mempool (multiple concurrent readers)
- **Write operations**: Acquire write lock only during block insertion (infrequent, <1 per 10 minutes)
- **Async operations**: All methods return `impl Future` for non-blocking execution
- **Atomic updates**: Database writes batched via RocksDB WriteBatch (ACID guarantees)

**State Transitions**:
```
Initializing (loading RocksDB, validating genesis)
    ↓
Syncing (P2P active, downloading blocks)
    ↓
Synced (at network tip, processing new blocks)
    ↓ (on shutdown signal)
ShuttingDown (flushing state, closing connections)
    ↓
Stopped (all resources released)
```

**Lifecycle**:
1. **Initialization**: Created in Tauri `setup()`, managed as `tauri::State<NodeState>`
2. **Background tasks**: P2P sync runs on Tokio runtime (`tauri::async_runtime::spawn`)
3. **Shutdown**: Triggered by Tauri `on_exit`, ordered component shutdown (30s timeout)

---

### 2. UnifiedDatabase

**Purpose**: Single RocksDB instance shared between blockchain and wallet (eliminates duplication)

**Fields**:
```rust
struct UnifiedDatabase {
    /// RocksDB instance handle
    db: Arc<rocksdb::DB>,

    /// Column family handles
    cf_blocks: ColumnFamily,
    cf_transactions: ColumnFamily,
    cf_utxos: ColumnFamily,
    cf_metadata: ColumnFamily,
    cf_wallets: ColumnFamily,  // NEW: wallet data storage

    /// Cache sizing configuration
    cache_size_mb: usize,  // Default: 512MB for desktop

    /// Write-ahead log enabled
    wal_enabled: bool,  // Always true for crash recovery
}
```

**Types**:
- `db: Arc<rocksdb::DB>` - Thread-safe by RocksDB design, cloneable Arc
- `cf_blocks: ColumnFamily` - Immutable reference to column family handle
- `cf_wallets: ColumnFamily` - NEW column family (migration from .dat files)
- `cache_size_mb: usize` - Read-only after initialization
- `wal_enabled: bool` - Configuration flag (always true for embedded node)

**Column Family Schemas**:

**CF_BLOCKS**:
- Key: `block_height (u64 BE bytes)` → Value: `Block (bincode serialized)`
- Key: `block_hash (SHA-512 digest)` → Value: `BlockHeader (bincode)`

**CF_TRANSACTIONS**:
- Key: `txid (SHA-512 digest)` → Value: `(block_height: u64, tx_index: u32)`

**CF_UTXOS**:
- Key: `(txid, output_index)` → Value: `UTXO { amount: u64, script_pubkey: Vec<u8>, height: u64 }`

**CF_METADATA**:
- Key: `"chain_tip"` → Value: `block_hash (SHA-512 digest)`
- Key: `"height"` → Value: `u64 BE bytes`
- Key: `"network"` → Value: `NetworkType (enum u8)`

**CF_WALLETS** (NEW):
- Key: `wallet:{wallet_id}` → Value: `WalletData (existing struct from wallet_serde.rs)`
- Key: `wallet:{wallet_id}:key:{address}` → Value: `KeyEntry (encrypted private key)`
- Key: `wallet:{wallet_id}:metadata` → Value: `{ name: String, created_at: u64, version: u8 }`

**Relationships**:
- **Owned by**: EmbeddedNode (exclusive database ownership)
- **Accessed by**: WalletManager (CF_WALLETS reads/writes), MiningThreadPool (CF_UTXOS reads)
- **Replaces**: Separate wallet .dat files + desktop app's duplicate UTXO database

**Validation Rules**:
1. CF_WALLETS key format MUST match `wallet:{uuid}:*` pattern
2. WalletData MUST have version field for migration compatibility
3. Private keys MUST be AES-256-GCM encrypted before storage
4. UTXO entries MUST reference valid transaction in CF_TRANSACTIONS
5. Blockchain height in CF_METADATA MUST match max height in CF_BLOCKS

**Thread-Safety Patterns**:
- RocksDB provides internal synchronization (no external locking needed)
- WriteBatch API for atomic multi-column writes (e.g., block + UTXO updates)
- Column families are independently lockable (wallet writes don't block blockchain reads)

**Migration Strategy**:
```
On first launch with embedded architecture:
1. Check for ~/.btpc/wallets/*.dat files
2. For each .dat file:
   a. Decrypt using existing WalletData::from_encrypted_file()
   b. Insert into CF_WALLETS with key pattern wallet:{wallet_id}
   c. Verify RocksDB write succeeded
   d. Rename .dat to .dat.migrated (backup)
3. Set metadata key "wallets_migrated" = "true"
```

---

### 3. MiningThreadPool

**Purpose**: Background mining operation (replaces external btpc_miner process)

**Fields**:
```rust
struct MiningThreadPool {
    /// CPU mining thread pool (Rayon)
    cpu_pool: Option<rayon::ThreadPool>,

    /// GPU mining handle (OpenCL)
    gpu_miner: Option<Arc<GpuMiner>>,

    /// Mining enabled flag
    running: Arc<AtomicBool>,

    /// Current mining address (receives block rewards)
    mining_address: Arc<RwLock<String>>,

    /// Hashrate statistics
    hashrate_stats: Arc<RwLock<HashrateStats>>,

    /// Blocks found counter
    blocks_found: Arc<AtomicU64>,

    /// Block template cache (refreshed every 30s or on new block)
    template_cache: Arc<RwLock<Option<BlockTemplate>>>,

    /// Reference to embedded node (for block submission)
    node_ref: Arc<RwLock<EmbeddedNode>>,

    /// Tauri app handle (for event emission)
    app_handle: tauri::AppHandle,
}
```

**Types**:
- `cpu_pool: Option<rayon::ThreadPool>` - Created on start_mining(), None when stopped
- `gpu_miner: Option<Arc<GpuMiner>>` - Lazy initialization (only if GPU available)
- `running: Arc<AtomicBool>` - Lock-free flag for thread cancellation
- `mining_address: Arc<RwLock<String>>` - User-configurable, validated on update
- `hashrate_stats: Arc<RwLock<HashrateStats>>` - Updated every 5 seconds, read by UI
- `blocks_found: Arc<AtomicU64>` - Monotonic counter, persists to RocksDB on block found
- `template_cache: Arc<RwLock<Option<BlockTemplate>>>` - Refreshed on blockchain state change
- `node_ref: Arc<RwLock<EmbeddedNode>>` - Direct reference (no RPC), read-only access
- `app_handle: tauri::AppHandle` - Cloneable handle for event emission (thread-safe)

**Operations**:

**start_mining(threads: usize, use_gpu: bool)**:
1. Validate mining address is set and valid
2. Create Rayon pool with `num_cpus.max(threads).min(num_cpus - 2)` threads
3. Set thread priority to below-normal (prevent UI starvation)
4. If use_gpu: Initialize OpenCL miner with device selection
5. Set running flag to true
6. Spawn mining loop on background threads
7. Emit `mining:started` event to UI

**stop_mining(timeout: Duration)**:
1. Set running flag to false
2. Wait for CPU threads to finish current batch (max timeout)
3. If GPU active: Flush OpenCL queue, abort kernels
4. Join thread pool (Rayon handles graceful shutdown)
5. Emit `mining:stopped` event with final statistics

**submit_block(block: Block)**:
1. Acquire node write lock
2. Validate block via consensus validator
3. If valid: Insert into blockchain, update UTXO set
4. Emit `mining:block_found` event with block details
5. Increment blocks_found counter
6. Refresh template cache immediately

**Events Emitted**:
- `mining:started` - { threads: usize, gpu_enabled: bool, address: String }
- `mining:stopped` - { blocks_found: u64, total_hashes: u64, uptime_secs: u64 }
- `mining:hashrate_updated` - { cpu_hashrate: f64, gpu_hashrate: f64, total: f64 } (every 5s)
- `mining:block_found` - { height: u64, hash: String, reward: u64 }

**Validation Rules**:
1. Mining address MUST be valid BTPC address before starting
2. Thread count MUST be in range [1, num_cpus]
3. GPU mining MUST gracefully fail if OpenCL unavailable (continue CPU-only)
4. Block template MUST be refreshed if older than 30 seconds
5. Found blocks MUST pass full consensus validation before submission

**Thread-Safety Patterns**:
- **Atomic operations**: Use AtomicBool for running flag, AtomicU64 for counters
- **Thread cancellation**: Check running flag every hash batch iteration
- **Resource cleanup**: Rayon pool drop automatically joins threads
- **Event emission**: AppHandle is thread-safe, can emit from mining threads

**Resource Limits**:
- CPU threads: Default `num_cpus - 2` (leave cores for UI/node)
- GPU VRAM: Check available memory before kernel launch (fail gracefully)
- Template cache: Max size 10KB (single BlockTemplate struct)
- Hashrate buffer: Last 100 samples (rolling window, ~500 bytes)

---

### 4. SharedBlockchainState

**Purpose**: Current blockchain tip and sync progress (single source of truth, Article XI)

**Fields**:
```rust
struct SharedBlockchainState {
    /// Current blockchain height
    height: u64,

    /// Best block hash (chain tip)
    best_block_hash: String,  // Hex-encoded SHA-512

    /// Sync status
    sync_status: SyncStatus,

    /// Peer count
    peer_count: usize,

    /// Estimated sync percentage (0-100)
    sync_percentage: f64,

    /// Last state update timestamp
    last_updated: SystemTime,
}

enum SyncStatus {
    Initializing,      // Loading database
    Syncing,           // Downloading blocks from peers
    Synced,            // At network tip
    Disconnected,      // No peer connections
}
```

**Types**:
- `height: u64` - Monotonically increasing, persisted in RocksDB CF_METADATA
- `best_block_hash: String` - 128-char hex string (64-byte SHA-512 digest)
- `sync_status: SyncStatus` - Enum (4 states), determines UI display
- `peer_count: usize` - Updated by P2P manager on connect/disconnect
- `sync_percentage: f64` - Calculated: `(local_height / network_height) * 100.0`
- `last_updated: SystemTime` - For staleness detection (warn if >60s old)

**Backend Storage**:
- **Rust**: `Arc<RwLock<SharedBlockchainState>>` in Tauri managed state
- **RocksDB**: height + best_block_hash persisted in CF_METADATA (reconstructed on startup)

**Frontend Access**:
- **Query**: Tauri command `get_blockchain_state()` - Returns snapshot of state
- **Updates**: Tauri event `blockchain:state_updated` - Emitted on every block add

**State Transitions**:
```
Initializing (loading RocksDB, height = 0)
    ↓
Disconnected (no peers found)
    ↓
Syncing (peer_count > 0, height < network_tip)
    ↓
Synced (height == network_tip, processing new blocks)
    ↓ (on peer disconnect)
Disconnected (peer_count == 0, height unchanged)
```

**Validation Rules**:
1. Height MUST NOT decrease (only increase or stay same)
2. Sync percentage MUST be in range [0.0, 100.0]
3. best_block_hash MUST be valid hex string (128 chars for SHA-512)
4. SyncStatus MUST match peer_count (if 0 peers → Disconnected)
5. State updates MUST trigger event emission within 200ms (Article XI requirement)

**Thread-Safety Patterns**:
- **Read-heavy**: UI polls state frequently, use RwLock for concurrent reads
- **Write-rare**: Updated only on new block (~10 minutes), atomic write lock
- **Event emission**: Triggered after releasing write lock (avoid holding lock during emit)

**Persistence**:
- On every block add: Write height + best_block_hash to CF_METADATA (batched with block write)
- On startup: Read from CF_METADATA, set sync_status to Initializing → Syncing

---

### 5. TransactionMempool

**Purpose**: Unconfirmed transaction pool (embedded in-process, no RPC)

**Fields**:
```rust
struct TransactionMempool {
    /// Pending transactions (txid → transaction)
    transactions: HashMap<String, Transaction>,

    /// Fee priority queue (for mining template generation)
    fee_priority: BinaryHeap<PriorityTx>,

    /// Transaction arrival timestamps (for eviction)
    arrival_times: HashMap<String, SystemTime>,

    /// DoS protection counters (address → tx_count)
    dos_counters: HashMap<String, usize>,

    /// Maximum mempool size (bytes)
    max_size_bytes: usize,  // Default: 50MB

    /// Current mempool size (bytes)
    current_size: usize,
}

struct PriorityTx {
    txid: String,
    fee_rate: f64,  // crd per byte
    timestamp: SystemTime,
}
```

**Types**:
- `transactions: HashMap<String, Transaction>` - Keyed by hex txid
- `fee_priority: BinaryHeap<PriorityTx>` - Max-heap (highest fee first)
- `arrival_times: HashMap<String, SystemTime>` - For FIFO eviction when full
- `dos_counters: HashMap<String, usize>` - Reset every 10 minutes
- `max_size_bytes: usize` - Configurable, default 50MB (desktop has RAM)
- `current_size: usize` - Sum of serialized transaction sizes

**Operations**:

**add_transaction(tx: Transaction)**:
1. Validate ML-DSA signature (constant-time)
2. Check UTXO availability (read from UnifiedDatabase)
3. Calculate fee rate (fee / tx_size_bytes)
4. If mempool full: Evict lowest fee transaction
5. Insert into transactions HashMap
6. Push to fee_priority heap
7. Increment dos_counter for sender address
8. Emit `mempool:transaction_added` event

**remove_transaction(txid: &str)**:
1. Remove from transactions HashMap
2. Rebuild fee_priority heap (O(n) operation, acceptable for small mempool)
3. Remove from arrival_times
4. Decrement current_size

**get_block_template(max_txs: usize)**:
1. Pop max_txs transactions from fee_priority heap
2. Validate no double-spends (check UTXO availability)
3. Return Vec<Transaction> ordered by fee

**Validation Rules**:
1. Transactions MUST have valid ML-DSA signatures before insertion
2. Transaction inputs MUST reference unspent UTXOs (checked in UnifiedDatabase)
3. Mempool MUST evict lowest fee transactions when max_size_bytes exceeded
4. DoS counter MUST reject addresses exceeding 1000 txs per 10 minutes
5. Transactions older than 72 hours MUST be evicted automatically

**Thread-Safety Patterns**:
- **RwLock wrapper**: `Arc<RwLock<TransactionMempool>>` for concurrent access
- **Write operations**: add/remove transactions (acquire write lock)
- **Read operations**: get_block_template (acquire read lock, no mutation)
- **Heap rebuilding**: Deferred until next get_block_template (amortized cost)

**Event Emissions**:
- `mempool:transaction_added` - { txid: String, fee_rate: f64, size: usize }
- `mempool:transaction_removed` - { txid: String, reason: String } (confirmed/evicted/invalid)
- `mempool:size_updated` - { count: usize, bytes: usize } (emitted every 30s)

**Persistence**:
- On shutdown: Flush mempool to CF_METADATA as `mempool_snapshot` key
- On startup: Load from `mempool_snapshot`, re-validate all transactions (some may have stale UTXOs)

---

### 6. WalletManager

**Purpose**: Existing entity, modified to use UnifiedDatabase (removes separate wallet files)

**Changes from Current Implementation**:

**Fields Removed**:
- `utxo_database: Arc<RocksDB>` - REMOVED (now uses node's UnifiedDatabase)
- `wallet_file_path: PathBuf` - REMOVED (now stored in RocksDB CF_WALLETS)

**Fields Added**:
```rust
struct WalletManager {
    // Existing fields preserved...

    /// Reference to unified database (NEW)
    unified_db: Arc<UnifiedDatabase>,

    /// Wallet cache (in-memory, loaded from RocksDB on startup)
    wallet_cache: Arc<RwLock<HashMap<String, WalletData>>>,
}
```

**Modified Operations**:

**load_wallet(wallet_id: &str) -> Result<WalletData>**:
- OLD: Read from ~/.btpc/wallets/{wallet_id}.dat file
- NEW: Read from `unified_db.get(CF_WALLETS, wallet:{wallet_id})`
- Migration: If key not found, check for .dat file and migrate

**save_wallet(wallet_data: &WalletData) -> Result<()>**:
- OLD: Encrypt and write to .dat file
- NEW: Serialize and write to `unified_db.put(CF_WALLETS, wallet:{wallet_id}, data)`
- Backup: On first write, create .dat.backup (one-time safety measure)

**get_balance(wallet_id: &str) -> Result<u64>**:
- OLD: Query separate UTXO database via RPC
- NEW: Query `unified_db.scan(CF_UTXOS)` for addresses in wallet
- Performance: <10ms target (direct RocksDB access, no HTTP overhead)

**Preservation**:
- Encryption format UNCHANGED (AES-256-GCM + Argon2id)
- BIP39 recovery UNCHANGED (seed derivation preserved)
- Multi-wallet support UNCHANGED (multiple wallet_id keys in RocksDB)
- Transaction signing UNCHANGED (ML-DSA signature generation)

**Validation Rules**:
1. Wallet keys in CF_WALLETS MUST have version field for migration compatibility
2. Private keys MUST be encrypted before storage (no plaintext in RocksDB)
3. Wallet cache MUST be invalidated on external database write (prevent stale reads)
4. UTXO queries MUST check fork_id field (regtest/testnet/mainnet separation)

---

## Thread-Safety Summary

| Entity | Primary Lock | Access Pattern | Contention Risk |
|--------|--------------|----------------|-----------------|
| EmbeddedNode | Arc<RwLock<>> | Read-heavy (balance queries) | LOW (99% reads) |
| UnifiedDatabase | Arc (internal RocksDB locks) | Mixed read/write | LOW (column isolation) |
| MiningThreadPool | AtomicBool + channels | Write-rare (start/stop) | NONE (atomic ops) |
| SharedBlockchainState | Arc<RwLock<>> | Read-heavy (UI polling) | LOW (write every 10min) |
| TransactionMempool | Arc<RwLock<>> | Moderate write (new tx) | MEDIUM (rebuild heap) |
| WalletManager | Arc<RwLock<>> | Read-heavy (balance checks) | LOW (caching) |

**Deadlock Prevention**:
- Lock ordering: Always acquire node lock → database lock → wallet lock (never reverse)
- Lock duration: Release locks before emitting Tauri events (avoid blocking UI)
- Async operations: Use `tokio::spawn` for long-running tasks (don't hold locks across awaits)

---

## Performance Targets

| Operation | Current (RPC) | Target (In-Process) | Improvement |
|-----------|---------------|---------------------|-------------|
| Balance query | 30-50ms | <10ms | 3-5x faster |
| UTXO lookup | 40-60ms | <20ms | 2-3x faster |
| Transaction submission | 20-30ms | <5ms | 4-6x faster |
| Block validation | 80-100ms | <100ms | Same (bottleneck is crypto) |

**Measurement Criteria**:
- All measurements at 90th percentile latency
- Test environment: 1000 blocks, 100 UTXOs, 10 pending transactions
- Success: 90% of operations meet target latency

---

## State Transition Diagram

```
Application Lifecycle:

[App Startup]
    ↓
[Initialize UnifiedDatabase] (load RocksDB, migrate .dat files)
    ↓
[Create EmbeddedNode] (load blockchain state, initialize P2P)
    ↓
[Start Background Tasks] (P2P sync, mempool cleanup, mining if configured)
    ↓
[Ready State] (accept Tauri commands, emit events)
    ↓
[User Initiates Shutdown]
    ↓
[Stop Mining Threads] (cancel gracefully, 5s timeout)
    ↓
[Close P2P Connections] (send disconnect messages, 5s timeout)
    ↓
[Flush Mempool to RocksDB] (persist pending transactions, 10s timeout)
    ↓
[Flush RocksDB WAL] (ensure durability, 5s timeout)
    ↓
[Zeroize Cryptographic Keys] (wallet keys, instant)
    ↓
[App Exit]
```

---

## Validation Checklist

### Data Integrity
- [x] All UTXO references validated against CF_TRANSACTIONS
- [x] Blockchain height monotonically increasing
- [x] Wallet keys encrypted before storage
- [x] Mempool transactions signature-validated before insertion
- [x] RocksDB WAL enabled for crash recovery

### Thread Safety
- [x] All shared state wrapped in Arc<RwLock<>> or atomic types
- [x] Lock ordering documented (prevents deadlocks)
- [x] No locks held during async operations
- [x] Event emissions after lock release

### Performance
- [x] Read-heavy workloads use RwLock (not Mutex)
- [x] Database writes batched via WriteBatch
- [x] Caching for frequent queries (blockchain state, wallet balance)
- [x] Background tasks use below-normal priority

### Article XI Compliance
- [x] Backend is single source of truth (RocksDB + Arc state)
- [x] Frontend queries backend before displaying state
- [x] Events emitted on all state changes
- [x] No localStorage for authoritative data

---

**Document Version**: 1.0
**Last Updated**: 2025-11-10
**Status**: Ready for API Contract Design (Phase 1)