# Tasks: Embed btpc-core as In-Process Library

**Feature**: 010-reconfigure-btpc-desktop
**Branch**: `010-reconfigure-btpc-desktop`
**Plan**: [plan.md](plan.md)

## Task Execution Order

Tasks are numbered and must be executed in order unless marked with `[P]` for parallel execution.

### Phase 3.1: Setup & Dependencies
- T001: Verify btpc-core library architecture
- T002: Update Cargo.toml dependencies

### Phase 3.2: Contract Tests (TDD - MUST FAIL FIRST) ⚠️
- T003 [P]: Write embedded node initialization test
- T004 [P]: Write blockchain state query test
- T005 [P]: Write sync progress test
- T006 [P]: Write mining start/stop test
- T007 [P]: Write mining stats test
- T008 [P]: Write blockchain:block_added event test
- T009 [P]: Write mining:hashrate_updated event test

### Phase 3.3: Core Implementation (Make Tests Pass)
- T010: Implement UnifiedDatabase module
- T011: Implement EmbeddedNode wrapper
- T012: Implement embedded_node_commands module
- T013: Implement MiningThreadPool module
- T014: Implement mining_commands module (no external process)

### Phase 3.4: Integration & Modification
- T015: Modify main.rs (remove process spawning, init embedded node)
- T016: Modify wallet_manager.rs (use UnifiedDatabase)
- T017: Modify transaction_commands.rs (direct mempool access)
- T018: Modify utxo_manager.rs (query UnifiedDatabase)

### Phase 3.5: Code Removal (Parallel)
- T019 [P]: Remove btpc_integration.rs
- T020 [P]: Remove process_manager.rs
- T021 [P]: Remove rpc_client.rs
- T022 [P]: Remove sync_service.rs

### Phase 3.6: Integration Testing
- T023: Test first-time launch (single process)
- T024: Benchmark balance queries (<10ms)
- T025: Test transaction creation (no RPC)
- T026: Test mining operations (thread lifecycle)
- T027: Test graceful shutdown
- T028: Test migration from multi-process

### Phase 3.7: Validation & Polish
- T029: Run cargo test --workspace
- T030: Run cargo bench (performance)
- T031: Execute quickstart.md manual tests
- T032: Measure code reduction
- T033: Verify constitutional compliance

---

## Task Details

### T001: Verify btpc-core library architecture
**File**: `/home/bob/BTPC/BTPC/btpc-core/`
**Type**: Setup
**Dependencies**: None

**Objective**: Verify btpc-core supports in-process initialization (designed for embedding).

**Steps**:
1. Read `btpc-core/src/lib.rs` - check for public Node/Blockchain initialization API
2. Read `bins/btpc_node/src/main.rs` - understand how btpc_node uses btpc-core
3. Verify RocksDB column families are exposed: CF_BLOCKS, CF_TRANSACTIONS, CF_UTXOS, CF_METADATA
4. Check if `btpc_core::storage::Database` supports custom data directory paths
5. Verify `btpc_core::network::P2PManager` can be initialized programmatically

**Success Criteria**:
- ✅ btpc-core exports public initialization API
- ✅ RocksDB column families are documented and accessible
- ✅ No blockers for in-process embedding found

---

### T002: Update Cargo.toml dependencies
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/Cargo.toml`
**Type**: Setup
**Dependencies**: T001

**Objective**: Add btpc-core as direct dependency (if not already present), ensure rayon for CPU mining.

**Steps**:
1. Add `btpc-core = { path = "../../btpc-core" }` to `[dependencies]` (if missing)
2. Verify `rayon` is present for CPU mining thread pool (add if missing: `rayon = "1.8"`)
3. Verify `tokio` has `full` features for async runtime
4. Check `ocl = "0.19"` is present (GPU mining - existing)
5. Run `cargo check --manifest-path btpc-desktop-app/src-tauri/Cargo.toml` to verify dependencies resolve

**Success Criteria**:
- ✅ btpc-core dependency added/verified
- ✅ rayon dependency present
- ✅ `cargo check` passes with no dependency errors

---

### T003 [P]: Write embedded node initialization test
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/tests/embedded_node_tests.rs`
**Type**: Contract Test (TDD)
**Dependencies**: None (parallel with T004-T009)

**Objective**: Write failing test for `init_embedded_node` Tauri command (from embedded-node-api.yaml).

**Test Requirements**:
1. Create test function `test_init_embedded_node()`
2. Call hypothetical `init_embedded_node(network: "regtest", data_dir: "/tmp/test_btpc")` Tauri command
3. Assert response contains:
   - `initialized: true`
   - `blockchain_height: 0` (genesis block)
   - `p2p_listening: true`
   - `sync_status: "syncing"`
4. Test MUST FAIL initially (implementation doesn't exist yet)

**Implementation Notes**:
- Use Tauri test harness or mock Tauri app context
- If command doesn't exist, test should fail to compile or panic
- Document expected response schema per embedded-node-api.yaml

**Success Criteria**:
- ✅ Test file created
- ✅ Test compiles
- ✅ Test FAILS (command not implemented yet)

---

### T004 [P]: Write blockchain state query test
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/tests/embedded_node_tests.rs`
**Type**: Contract Test (TDD)
**Dependencies**: None (parallel with T003, T005-T009)

**Objective**: Write failing test for `get_blockchain_state` command (Arc<RwLock> access verification).

**Test Requirements**:
1. Create test function `test_get_blockchain_state()`
2. Assume node initialized (mock or prerequisite)
3. Call `get_blockchain_state()` Tauri command
4. Assert response contains:
   - `current_height: u64`
   - `best_hash: [u8; 64]` (SHA-512)
   - `sync_status: "syncing" | "synced"`
   - `peer_count: u32`
5. Test latency: Assert response time <10ms (performance target)

**Success Criteria**:
- ✅ Test written
- ✅ Test FAILS (command doesn't exist)
- ✅ Performance assertion documented (<10ms)

---

### T005 [P]: Write sync progress test
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/tests/embedded_node_tests.rs`
**Type**: Contract Test (TDD)
**Dependencies**: None (parallel)

**Objective**: Write failing test for `get_sync_progress` command (atomic height reads).

**Test Requirements**:
1. Create test function `test_get_sync_progress()`
2. Mock blockchain at height 500 syncing to 1000
3. Call `get_sync_progress()` Tauri command
4. Assert response contains:
   - `current_height: 500`
   - `target_height: 1000`
   - `percentage: 50.0`
   - `estimated_time_remaining: Option<u64>` (seconds)

**Success Criteria**:
- ✅ Test written
- ✅ Test FAILS (command doesn't exist)

---

### T006 [P]: Write mining start/stop test
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/tests/mining_thread_pool_tests.rs`
**Type**: Contract Test (TDD)
**Dependencies**: None (parallel)

**Objective**: Write failing test for `start_mining` and `stop_mining` commands (thread pool lifecycle).

**Test Requirements**:
1. Create test function `test_start_stop_mining()`
2. Call `start_mining(threads: 2, gpu: false, address: "test_addr")` command
3. Assert response: `{ started: true, thread_count: 2, gpu_enabled: false }`
4. Verify thread pool spawned (check process threads with num_cpus crate or similar)
5. Call `stop_mining()` command
6. Assert response: `{ stopped: true, uptime_seconds: u64 }`
7. Verify threads terminated within 1 second (graceful cancellation)

**Success Criteria**:
- ✅ Test written
- ✅ Test FAILS (commands don't exist)
- ✅ Thread lifecycle verified (spawn + cancel)

---

### T007 [P]: Write mining stats test
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/tests/mining_thread_pool_tests.rs`
**Type**: Contract Test (TDD)
**Dependencies**: None (parallel)

**Objective**: Write failing test for `get_mining_stats` command (atomic counter reads).

**Test Requirements**:
1. Create test function `test_get_mining_stats()`
2. Assume mining active (mock MiningThreadPool with AtomicU64 counters)
3. Call `get_mining_stats()` command
4. Assert response contains:
   - `hashrate: f64` (MH/s)
   - `total_hashes: u64`
   - `uptime_seconds: u64`
   - `blocks_found: u64`
   - `threads_active: u32`

**Success Criteria**:
- ✅ Test written
- ✅ Test FAILS (command doesn't exist)
- ✅ Atomic counter usage documented

---

### T008 [P]: Write blockchain:block_added event test
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/tests/events_tests.rs`
**Type**: Contract Test (TDD)
**Dependencies**: None (parallel)

**Objective**: Write failing test for `blockchain:block_added` event emission (from events-api.yaml).

**Test Requirements**:
1. Create test function `test_blockchain_block_added_event()`
2. Mock embedded node adding new block (height 1001)
3. Set up Tauri event listener for `blockchain:block_added`
4. Trigger block addition
5. Assert event emitted with payload:
   - `height: 1001`
   - `hash: [u8; 64]`
   - `timestamp: u64`
   - `transactions: u32` (count)

**Success Criteria**:
- ✅ Test written
- ✅ Test FAILS (event not emitted)
- ✅ Event payload schema verified per events-api.yaml

---

### T009 [P]: Write mining:hashrate_updated event test
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/tests/events_tests.rs`
**Type**: Contract Test (TDD)
**Dependencies**: None (parallel)

**Objective**: Write failing test for `mining:hashrate_updated` event (5-second interval from events-api.yaml).

**Test Requirements**:
1. Create test function `test_mining_hashrate_updated_event()`
2. Mock MiningThreadPool with active mining
3. Set up Tauri event listener for `mining:hashrate_updated`
4. Wait 5 seconds (or fast-forward time in mock)
5. Assert event emitted with payload:
   - `hashrate: f64` (MH/s)
   - `threads: u32`
   - `gpu_enabled: bool`
6. Verify event emits every 5 seconds (test multiple emissions)

**Success Criteria**:
- ✅ Test written
- ✅ Test FAILS (event not emitted)
- ✅ 5-second interval verified

---

### T010: Implement UnifiedDatabase module
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/unified_database.rs`
**Type**: Core Implementation
**Dependencies**: T001, T002, Tests T003-T009 MUST BE FAILING

**Objective**: Create UnifiedDatabase wrapper managing single RocksDB instance (from data-model.md).

**Implementation**:
1. Create `pub struct UnifiedDatabase { db: Arc<Database> }` wrapping btpc-core's RocksDB
2. Implement `pub fn open(data_dir: &Path) -> Result<Self>`:
   - Open RocksDB at `data_dir/unified.db`
   - Verify existing column families: CF_BLOCKS, CF_TRANSACTIONS, CF_UTXOS, CF_METADATA
   - Create CF_WALLETS if missing (for encrypted wallet data)
   - Set cache size to 512MB (desktop-optimized per research.md)
3. Implement `pub fn get_column_family(&self, name: &str) -> Option<&ColumnFamily>`
4. Implement `pub fn close(&self) -> Result<()>` (flush WAL before close)
5. Wrap in `Arc` for shared ownership (Thread-safety: RocksDB provides internal sync)

**Success Criteria**:
- ✅ Module compiles
- ✅ Opens existing btpc-core RocksDB successfully
- ✅ CF_WALLETS column family created if missing
- ✅ Arc<Database> shareable across threads

---

### T011: Implement EmbeddedNode wrapper
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/embedded_node.rs`
**Type**: Core Implementation
**Dependencies**: T010 (needs UnifiedDatabase)

**Objective**: Create EmbeddedNode struct wrapping btpc-core's Node (from data-model.md).

**Implementation**:
1. Create `pub struct EmbeddedNode`:
   ```rust
   pub struct EmbeddedNode {
       node: Arc<RwLock<btpc_core::Node>>,
       database: Arc<UnifiedDatabase>,
       runtime: tokio::runtime::Runtime, // For P2P async ops
   }
   ```
2. Implement `pub fn initialize(network: Network, data_dir: PathBuf) -> Result<Arc<Self>>`:
   - Open UnifiedDatabase
   - Initialize btpc_core::Node with database
   - Start P2P manager on background Tokio runtime
   - Start blockchain sync
   - Return Arc<EmbeddedNode>
3. Implement `pub fn get_blockchain_state(&self) -> BlockchainState`:
   - Acquire read lock: `self.node.read().unwrap()`
   - Query current height (AtomicU64), best hash, sync status
   - Return state (no RPC, direct access)
4. Implement `pub fn shutdown(&self) -> Result<()>`:
   - Stop P2P connections
   - Flush mempool
   - Flush RocksDB WAL
   - Zeroize any keys in memory
   - Shutdown Tokio runtime

**Success Criteria**:
- ✅ EmbeddedNode compiles
- ✅ Initializes btpc_core::Node successfully
- ✅ Arc<RwLock<>> allows concurrent reads
- ✅ Shutdown is graceful (ordered: P2P → mempool → DB → runtime)

---

### T012: Implement embedded_node_commands module
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/embedded_node_commands.rs`
**Type**: Core Implementation
**Dependencies**: T011 (needs EmbeddedNode)

**Objective**: Create Tauri commands for embedded node operations (from embedded-node-api.yaml).

**Implementation**:
1. Create `#[tauri::command]` functions:
   - `pub async fn init_embedded_node(network: String, data_dir: String, state: tauri::State<'_, Arc<Mutex<Option<Arc<EmbeddedNode>>>>>) -> Result<InitResponse>`
     - Parse network ("mainnet", "testnet", "regtest")
     - Call EmbeddedNode::initialize()
     - Store in Tauri state
     - Return InitResponse { initialized: true, blockchain_height, p2p_listening, sync_status }
   - `pub async fn get_blockchain_state(state: tauri::State<'_, Arc<Mutex<Option<Arc<EmbeddedNode>>>>>) -> Result<BlockchainState>`
     - Get EmbeddedNode from state
     - Call node.get_blockchain_state() (direct, <10ms)
     - Return state
   - `pub async fn get_sync_progress(state: tauri::State<'_, Arc<Mutex<Option<Arc<EmbeddedNode>>>>>) -> Result<SyncProgress>`
     - Query sync manager
     - Calculate percentage: current_height / target_height * 100
   - `pub async fn shutdown_node(state: tauri::State<'_, Arc<Mutex<Option<Arc<EmbeddedNode>>>>>) -> Result<()>`
     - Call node.shutdown()
     - Remove from state

**Success Criteria**:
- ✅ All commands compile
- ✅ Tests T003-T005 now PASS
- ✅ Performance: get_blockchain_state <10ms (verify with benchmark)

---

### T013: Implement MiningThreadPool module
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/mining_thread_pool.rs`
**Type**: Core Implementation
**Dependencies**: T011 (needs EmbeddedNode for block template)

**Objective**: Create background mining thread pool (from data-model.md).

**Implementation**:
1. Create `pub struct MiningThreadPool`:
   ```rust
   pub struct MiningThreadPool {
       cpu_pool: Option<rayon::ThreadPool>,
       gpu_miner: Option<Arc<btpc_miner::GpuMiner>>, // Reuse existing GPU miner code
       stats: Arc<MiningStats>, // AtomicU64 counters
       cancel_flag: Arc<AtomicBool>,
   }

   pub struct MiningStats {
       pub total_hashes: AtomicU64,
       pub blocks_found: AtomicU64,
       pub start_time: Instant,
   }
   ```
2. Implement `pub fn start(threads: usize, gpu: bool, address: String, node: Arc<EmbeddedNode>) -> Result<Arc<Self>>`:
   - Create rayon::ThreadPoolBuilder with `threads` threads, below-normal priority
   - If gpu: Initialize GpuMiner from bins/btpc_miner/src/gpu_miner.rs
   - Set cancel_flag to false
   - Spawn mining loop:
     - Get block template from node (direct call, no RPC)
     - Mine with CPU pool (rayon) and/or GPU
     - If valid block found: Submit to node directly
     - Update stats (AtomicU64::fetch_add)
     - Check cancel_flag every iteration
3. Implement `pub fn stop(&self) -> Result<()>`:
   - Set cancel_flag to true
   - Wait up to 1 second for threads to exit
   - If timeout: Force terminate (acceptable for mining threads)
4. Implement `pub fn get_stats(&self) -> MiningStatsSnapshot`:
   - Read AtomicU64 counters (load with Ordering::Relaxed)
   - Calculate hashrate: total_hashes / elapsed_seconds / 1_000_000.0
   - Return snapshot

**Success Criteria**:
- ✅ MiningThreadPool compiles
- ✅ Rayon thread pool spawns with correct thread count
- ✅ GPU miner integrates (reuses existing bins/btpc_miner code)
- ✅ Graceful cancellation within 1 second
- ✅ Tests T006-T007 now PASS

---

### T014: Implement mining_commands module (no external process)
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/mining_commands.rs`
**Type**: Core Implementation
**Dependencies**: T013 (needs MiningThreadPool)

**Objective**: Modify existing mining_commands.rs to use MiningThreadPool instead of spawning btpc_miner binary.

**Implementation**:
1. **Remove** all process spawning code (calls to btpc_integration::spawn_miner)
2. Create `#[tauri::command]` functions:
   - `pub async fn start_mining(threads: usize, gpu: bool, address: String, node_state: State<Arc<Mutex<Option<Arc<EmbeddedNode>>>>>, mining_state: State<Arc<Mutex<Option<Arc<MiningThreadPool>>>>>) -> Result<StartMiningResponse>`
     - Get EmbeddedNode from state
     - Call MiningThreadPool::start()
     - Store pool in mining_state
     - Emit event: `mining:started` with { threads, gpu, address }
     - Return { started: true, thread_count: threads, gpu_enabled: gpu }
   - `pub async fn stop_mining(mining_state: State<Arc<Mutex<Option<Arc<MiningThreadPool>>>>>) -> Result<StopMiningResponse>`
     - Get pool from state
     - Call pool.stop() (graceful cancellation)
     - Emit event: `mining:stopped` with { uptime_seconds }
     - Remove from state
     - Return { stopped: true, uptime_seconds }
   - `pub async fn get_mining_stats(mining_state: State<Arc<Mutex<Option<Arc<MiningThreadPool>>>>>) -> Result<MiningStats>`
     - Get pool from state
     - Call pool.get_stats() (atomic reads, <1ms)
     - Return stats
3. Add background task to emit `mining:hashrate_updated` event every 5 seconds:
   - Spawn tokio task when mining starts
   - Query pool.get_stats() every 5 seconds
   - Emit event with hashrate, threads, gpu_enabled

**Success Criteria**:
- ✅ All mining commands compile
- ✅ No external process spawning code remains
- ✅ Tests T006-T007 now PASS
- ✅ Event T009 now PASS (hashrate_updated every 5s)

---

### T015: Modify main.rs (remove process spawning, init embedded node)
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/main.rs`
**Type**: Integration (Modification)
**Dependencies**: T012, T014 (needs embedded_node_commands, mining_commands)

**Objective**: Remove all process spawning, initialize EmbeddedNode on app startup.

**Steps**:
1. **Remove** imports and usage of:
   - `process_manager::ProcessManager`
   - `btpc_integration::spawn_node`, `btpc_integration::spawn_miner`
   - Any stdout parsing logic for mining events
2. Add Tauri state for EmbeddedNode:
   ```rust
   #[tauri::main]
   fn main() {
       tauri::Builder::default()
           .manage(Arc::new(Mutex::new(None::<Arc<EmbeddedNode>>))) // Node state
           .manage(Arc::new(Mutex::new(None::<Arc<MiningThreadPool>>))) // Mining state
           .invoke_handler(tauri::generate_handler![
               embedded_node_commands::init_embedded_node,
               embedded_node_commands::get_blockchain_state,
               embedded_node_commands::get_sync_progress,
               embedded_node_commands::shutdown_node,
               mining_commands::start_mining,
               mining_commands::stop_mining,
               mining_commands::get_mining_stats,
               // ... other existing commands
           ])
           .setup(|app| {
               // Optional: Auto-initialize node on app startup
               // Or wait for user action (init_embedded_node command)
               Ok(())
           })
           .run(tauri::generate_context!())
           .expect("error while running tauri application");
   }
   ```
3. Add shutdown hook to gracefully stop node on app close:
   ```rust
   .on_window_event(|event| {
       if let tauri::WindowEvent::Destroyed = event.event() {
           // Call shutdown_node() to clean up
       }
   })
   ```

**Success Criteria**:
- ✅ main.rs compiles
- ✅ No process spawning code remains
- ✅ Tauri commands registered
- ✅ App starts without external processes

---

### T016: Modify wallet_manager.rs (use UnifiedDatabase)
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/wallet_manager.rs`
**Type**: Integration (Modification)
**Dependencies**: T010 (needs UnifiedDatabase)

**Objective**: Remove separate UTXO RocksDB, query UnifiedDatabase directly.

**Steps**:
1. **Remove** field `utxo_db: Arc<RocksDB>` from WalletManager struct
2. Add field `unified_db: Arc<UnifiedDatabase>`
3. Modify `fn load_wallets()`:
   - Instead of reading .dat files from `~/.btpc/wallets/`
   - Query CF_WALLETS column family from UnifiedDatabase
   - Deserialize encrypted wallet data
4. Modify `fn save_wallet()`:
   - Instead of writing .dat file
   - Serialize wallet data
   - Write to CF_WALLETS column family in UnifiedDatabase
5. Modify `fn get_balance()`:
   - Instead of querying separate utxo_db
   - Query CF_UTXOS from UnifiedDatabase directly
   - Filter UTXOs by wallet's public keys
6. One-time migration:
   - On first load, check if old .dat files exist
   - If yes: Migrate to CF_WALLETS, delete .dat files
   - Emit migration complete event

**Success Criteria**:
- ✅ wallet_manager.rs compiles
- ✅ No separate RocksDB instance opened
- ✅ Wallets load/save from UnifiedDatabase
- ✅ Migration from .dat files works

---

### T017: Modify transaction_commands.rs (direct mempool access)
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/transaction_commands.rs`
**Type**: Integration (Modification)
**Dependencies**: T011 (needs EmbeddedNode)

**Objective**: Replace RPC calls with direct mempool access (no RPC overhead).

**Steps**:
1. **Remove** all `rpc_client::send_raw_transaction()` calls
2. Modify `create_transaction` command:
   - After building and signing transaction
   - Instead of calling RPC: Get EmbeddedNode from Tauri state
   - Call `node.add_transaction_to_mempool(signed_tx)` directly
   - Validate transaction (ML-DSA signature, UTXO existence)
   - If valid: Add to mempool, emit `wallet:transaction_created` event
   - If invalid: Return error immediately (no RPC roundtrip)
3. Modify `get_transaction_status`:
   - Instead of RPC call: Query node's mempool directly
   - Check if tx in mempool (pending) or in blockchain (confirmed)
4. Measure latency:
   - Direct mempool access should be <5ms (vs ~30ms RPC)

**Success Criteria**:
- ✅ transaction_commands.rs compiles
- ✅ No RPC client usage remains
- ✅ Transactions broadcast to mempool directly
- ✅ Latency improvement verified (<5ms vs ~30ms)

---

### T018: Modify utxo_manager.rs (query UnifiedDatabase)
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/utxo_manager.rs`
**Type**: Integration (Modification)
**Dependencies**: T010 (needs UnifiedDatabase)

**Objective**: Remove separate UTXO database, query UnifiedDatabase's CF_UTXOS directly.

**Steps**:
1. **Remove** field `utxo_db: Arc<RocksDB>` from UTXOManager struct
2. Add field `unified_db: Arc<UnifiedDatabase>`
3. Modify `fn select_utxos_for_amount()`:
   - Instead of querying separate DB: Query CF_UTXOS from UnifiedDatabase
   - Filter by wallet's public keys
   - Select UTXOs totaling >= amount
   - Return selected UTXOs (<5ms target, direct DB query)
4. Modify `fn add_utxo()` and `fn remove_utxo()`:
   - Write to CF_UTXOS in UnifiedDatabase
   - No separate database writes
5. **Remove** sync_service.rs dependency:
   - UTXO updates now come from embedded node's blockchain events
   - Listen for `blockchain:block_added` event
   - Update UTXO set when new blocks added

**Success Criteria**:
- ✅ utxo_manager.rs compiles
- ✅ No separate UTXO database
- ✅ UTXO selection <5ms (direct query)
- ✅ sync_service.rs no longer needed

---

### T019 [P]: Remove btpc_integration.rs
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/btpc_integration.rs`
**Type**: Code Removal
**Dependencies**: T015 (main.rs modified first)

**Objective**: Delete btpc_integration.rs module (binary management obsolete).

**Steps**:
1. Verify no imports of `btpc_integration` remain in codebase:
   ```bash
   grep -r "use.*btpc_integration" btpc-desktop-app/src-tauri/src/
   grep -r "btpc_integration::" btpc-desktop-app/src-tauri/src/
   ```
2. If no references found: Delete file `btpc-desktop-app/src-tauri/src/btpc_integration.rs`
3. Remove `mod btpc_integration;` declaration from main.rs or lib.rs

**Success Criteria**:
- ✅ File deleted
- ✅ No compile errors
- ✅ ~377 lines removed (from architectural analysis)

---

### T020 [P]: Remove process_manager.rs
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/process_manager.rs`
**Type**: Code Removal
**Dependencies**: T015 (main.rs modified first)

**Objective**: Delete process_manager.rs module (~459 lines).

**Steps**:
1. Verify no imports of `process_manager` remain:
   ```bash
   grep -r "use.*process_manager" btpc-desktop-app/src-tauri/src/
   grep -r "ProcessManager" btpc-desktop-app/src-tauri/src/
   ```
2. If no references: Delete `btpc-desktop-app/src-tauri/src/process_manager.rs`
3. Remove `mod process_manager;` declaration

**Success Criteria**:
- ✅ File deleted
- ✅ No compile errors
- ✅ ~459 lines removed

---

### T021 [P]: Remove rpc_client.rs
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/rpc_client.rs`
**Type**: Code Removal
**Dependencies**: T017 (transaction_commands.rs modified first)

**Objective**: Delete rpc_client.rs module (~424 lines).

**Steps**:
1. Verify no RPC client usage remains:
   ```bash
   grep -r "use.*rpc_client" btpc-desktop-app/src-tauri/src/
   grep -r "RpcClient" btpc-desktop-app/src-tauri/src/
   grep -r "send_raw_transaction" btpc-desktop-app/src-tauri/src/
   ```
2. If no references: Delete `btpc-desktop-app/src-tauri/src/rpc_client.rs`
3. Remove `mod rpc_client;` declaration

**Success Criteria**:
- ✅ File deleted
- ✅ No compile errors
- ✅ ~424 lines removed

---

### T022 [P]: Remove sync_service.rs
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/sync_service.rs`
**Type**: Code Removal
**Dependencies**: T018 (utxo_manager.rs modified first)

**Objective**: Delete sync_service.rs module (~410 lines).

**Steps**:
1. Verify no sync_service usage remains:
   ```bash
   grep -r "use.*sync_service" btpc-desktop-app/src-tauri/src/
   grep -r "SyncService\|BlockchainSyncService" btpc-desktop-app/src-tauri/src/
   ```
2. If no references: Delete `btpc-desktop-app/src-tauri/src/sync_service.rs`
3. Remove `mod sync_service;` declaration

**Success Criteria**:
- ✅ File deleted
- ✅ No compile errors
- ✅ ~410 lines removed

---

### T023: Test first-time launch (single process)
**File**: Manual test (from quickstart.md Scenario 1)
**Type**: Integration Test
**Dependencies**: T015 (main.rs modified)

**Objective**: Verify app launches as single process with no external binaries.

**Test Steps**:
1. Build desktop app: `cd btpc-desktop-app && npm run tauri:build`
2. Kill any running btpc processes: `pkill -f btpc`
3. Launch app: `./btpc-desktop-app/src-tauri/target/release/btpc-desktop-app`
4. Verify process count:
   ```bash
   ps aux | grep btpc | grep -v grep
   # Should show ONLY 1 process (the desktop app)
   ```
5. Check app UI:
   - Node status shows "Initializing" or "Syncing"
   - No errors about missing btpc_node binary
6. Check logs: No "spawning btpc_node" messages
7. Close app: Verify process terminates cleanly (`ps aux | grep btpc` shows 0)

**Success Criteria**:
- ✅ Single process only (no btpc_node, no btpc_miner)
- ✅ App initializes embedded node
- ✅ No spawn errors
- ✅ Graceful shutdown (0 processes after close)

---

### T024: Benchmark balance queries (<10ms)
**File**: Manual test (from quickstart.md Scenario 2)
**Type**: Performance Test
**Dependencies**: T016 (wallet_manager.rs uses UnifiedDatabase)

**Objective**: Measure balance query latency, verify <10ms target (vs ~50ms RPC).

**Test Steps**:
1. Create test wallet with known balance (e.g., 100 BTPC)
2. Add JavaScript benchmark to wallet UI:
   ```javascript
   async function benchmarkBalanceQuery() {
       const iterations = 100;
       const times = [];
       for (let i = 0; i < iterations; i++) {
           const start = performance.now();
           await invoke('get_balance', { walletId: 'test_wallet' });
           const end = performance.now();
           times.push(end - start);
       }
       const avg = times.reduce((a, b) => a + b) / times.length;
       const p50 = times.sort()[Math.floor(times.length / 2)];
       const p99 = times.sort()[Math.floor(times.length * 0.99)];
       console.log(`Balance query - Avg: ${avg}ms, P50: ${p50}ms, P99: ${p99}ms`);
   }
   ```
3. Run benchmark
4. Compare to baseline (RPC-based):
   - Expected RPC: P50 ~50ms, P99 ~100ms
   - Expected direct: P50 <10ms, P99 <20ms

**Success Criteria**:
- ✅ P50 latency <10ms (5x improvement)
- ✅ P99 latency <20ms
- ✅ No RPC overhead

---

### T025: Test transaction creation (no RPC)
**File**: Manual test (from quickstart.md Scenario 3)
**Type**: Integration Test
**Dependencies**: T017 (transaction_commands.rs uses direct mempool)

**Objective**: Verify transaction creation works without RPC calls.

**Test Steps**:
1. Create 2 wallets: Wallet A (100 BTPC), Wallet B (0 BTPC)
2. Monitor network traffic to localhost:18360:
   ```bash
   sudo tcpdump -i lo port 18360 -A
   ```
3. Create transaction: Send 50 BTPC from Wallet A to Wallet B
4. Verify:
   - tcpdump shows NO traffic (no RPC calls)
   - Transaction appears in mempool instantly (<50ms)
   - UTXO selection completed <5ms
   - Signing completed <10ms (ML-DSA)
5. Check logs: No "RPC call to sendrawtransaction" messages

**Success Criteria**:
- ✅ No RPC traffic detected
- ✅ Transaction creation <50ms total
- ✅ UTXO selection <5ms
- ✅ ML-DSA signing <10ms

---

### T026: Test mining operations (thread lifecycle)
**File**: Manual test (from quickstart.md Scenario 4)
**Type**: Integration Test
**Dependencies**: T014 (mining_commands.rs uses MiningThreadPool)

**Objective**: Verify CPU/GPU mining operates in background threads, no external process.

**Test Steps**:
1. Launch desktop app
2. Navigate to Mining page
3. Start CPU mining:
   - Click "Start CPU Mining" (4 threads)
   - Verify:
     ```bash
     ps -eLf | grep btpc-desktop-app | wc -l
     # Should show increased thread count (main + 4 mining threads)
     ```
   - Check UI: Hashrate updates every 5 seconds
   - Check UI responsiveness: Click around, should be <100ms
4. Start GPU mining (if available):
   - Click "Start GPU Mining"
   - Verify: No separate btpc_miner process spawned
   - Check logs: "GPU mining enabled" message
5. Stop mining:
   - Click "Stop Mining"
   - Verify: Threads terminate within 1 second
   - Verify: `ps -eLf` shows thread count reduced

**Success Criteria**:
- ✅ Mining threads spawn in desktop app process
- ✅ No external btpc_miner process
- ✅ UI remains responsive (<100ms updates)
- ✅ Graceful thread cancellation within 1 second
- ✅ Hashrate events emit every 5 seconds

---

### T027: Test graceful shutdown
**File**: Manual test (from quickstart.md Scenario 5)
**Type**: Integration Test
**Dependencies**: T015 (main.rs has shutdown hook)

**Objective**: Verify app shuts down cleanly with active sync + mining, no orphaned processes.

**Test Steps**:
1. Launch desktop app with regtest network
2. Start blockchain sync (let it reach ~100 blocks)
3. Start CPU mining (4 threads)
4. While both active, close app:
   - Click window close button OR
   - Send SIGTERM: `kill -TERM <pid>`
5. Monitor shutdown:
   - Check logs for shutdown sequence: "Stopping mining... Closing P2P... Flushing DB... Shutdown complete"
   - Timeout: Should complete within 30 seconds
6. Verify cleanup:
   ```bash
   ps aux | grep btpc | grep -v grep
   # Should show 0 processes
   ```
7. Restart app:
   - Check blockchain resumes from last persisted block (~100)
   - No "corrupted database" errors

**Success Criteria**:
- ✅ Graceful shutdown within 30 seconds
- ✅ Ordered shutdown: Mining → P2P → Mempool → DB → Runtime
- ✅ 0 orphaned processes
- ✅ No data corruption (RocksDB WAL ensures consistency)
- ✅ App restarts successfully

---

### T028: Test migration from multi-process
**File**: Manual test (from quickstart.md Scenario 6)
**Type**: Integration Test
**Dependencies**: T016 (wallet_manager.rs has migration code)

**Objective**: Verify upgrade from old multi-process version preserves wallets and blockchain.

**Test Steps**:
1. Set up "old" environment:
   - Ensure old btpc_node and btpc_miner binaries exist in `~/.btpc/bin/`
   - Create test wallet using old version (if available) or manually create .dat file
   - Populate blockchain data in `~/.btpc/data/node/blockchain/`
2. Install new version (single binary)
3. Launch new desktop app
4. Verify migration:
   - Check logs: "Migrating wallet from .dat to RocksDB..."
   - Check: Wallet balance matches old version
   - Check: Blockchain height matches (no re-sync required)
   - Check: Old .dat files deleted after successful migration
5. Verify old binaries no longer used:
   - `ps aux | grep btpc_node` shows nothing
   - `ps aux | grep btpc_miner` shows nothing

**Success Criteria**:
- ✅ Wallet .dat files migrated to CF_WALLETS
- ✅ Blockchain data preserved (height matches)
- ✅ Old binaries no longer executed
- ✅ No data loss

---

### T029: Run cargo test --workspace
**File**: N/A (command execution)
**Type**: Validation
**Dependencies**: All implementation tasks (T010-T018)

**Objective**: Run all unit and integration tests to verify implementation correctness.

**Steps**:
1. Run from repository root:
   ```bash
   cargo test --workspace
   ```
2. Verify all tests pass:
   - Contract tests (T003-T009) should PASS
   - Any existing btpc-core tests should still PASS
   - Desktop app tests should PASS
3. If failures:
   - Identify which task's implementation is incorrect
   - Fix and re-run tests

**Success Criteria**:
- ✅ All tests PASS
- ✅ No regressions in btpc-core
- ✅ Contract tests verify API contracts

---

### T030: Run cargo bench (performance)
**File**: N/A (command execution)
**Type**: Validation
**Dependencies**: T029 (tests passing)

**Objective**: Run performance benchmarks to verify targets met.

**Steps**:
1. Run benchmarks:
   ```bash
   cargo bench --workspace
   ```
2. Verify performance targets:
   - Balance queries: <10ms (from T024 manual benchmark)
   - Transaction creation: <50ms
   - UTXO selection: <5ms
   - Block validation: <100ms (btpc-core existing)
   - ML-DSA signature verification: <10ms (btpc-core existing)
3. Compare to baseline (RPC-based architecture):
   - Balance query improvement: ~5x (50ms → 10ms)
   - Transaction creation improvement: ~2x (100ms → 50ms)

**Success Criteria**:
- ✅ All performance targets met
- ✅ Benchmarks show improvement over RPC-based architecture
- ✅ No performance regressions in btpc-core

---

### T031: Execute quickstart.md manual tests
**File**: `/home/bob/BTPC/BTPC/specs/010-reconfigure-btpc-desktop/quickstart.md`
**Type**: Validation
**Dependencies**: T023-T028 (individual scenario tests)

**Objective**: Execute all 6 quickstart scenarios end-to-end, verify success criteria.

**Steps**:
1. Follow quickstart.md instructions for each scenario:
   - Scenario 1: First-time launch (T023)
   - Scenario 2: Fast balance queries (T024)
   - Scenario 3: Transaction creation (T025)
   - Scenario 4: Mining operations (T026)
   - Scenario 5: Graceful shutdown (T027)
   - Scenario 6: Migration (T028)
2. For each scenario, check all success criteria boxes
3. Document any failures or deviations
4. Take screenshots for documentation

**Success Criteria**:
- ✅ All 6 scenarios PASS
- ✅ All success criteria met
- ✅ No critical issues found

---

### T032: Measure code reduction
**File**: N/A (analysis)
**Type**: Validation
**Dependencies**: T019-T022 (removal tasks complete)

**Objective**: Verify ~1500 lines code reduction target met.

**Steps**:
1. Count lines removed:
   ```bash
   # Files deleted:
   wc -l btpc_integration.rs process_manager.rs rpc_client.rs sync_service.rs
   # Expected: ~377 + 459 + 424 + 410 = ~1670 lines removed
   ```
2. Count lines added:
   ```bash
   # Files created:
   wc -l embedded_node.rs embedded_node_commands.rs mining_thread_pool.rs unified_database.rs
   # Expected: ~800 lines added
   ```
3. Count modified lines:
   ```bash
   git diff --stat main.rs wallet_manager.rs transaction_commands.rs utxo_manager.rs mining_commands.rs
   # Expected: ~500 lines changed
   ```
4. Calculate net change: Removed (~1670) - Added (~800) - Changed (~500) = ~370 net reduction
5. **Note**: Primary win is complexity reduction (no IPC, no process management), not just LOC

**Success Criteria**:
- ✅ ~1300+ lines removed (process management, RPC client)
- ✅ ~800 lines added (embedded wrappers)
- ✅ Net reduction: ~500+ lines
- ✅ **Massive complexity reduction** (no IPC, no stdout parsing, no process lifecycle)

---

### T033: Verify constitutional compliance
**File**: N/A (checklist)
**Type**: Validation
**Dependencies**: T029 (tests passing)

**Objective**: Verify implementation complies with BTPC Constitution and Article XI.

**Checklist**:

**Article I (Security)**:
- ✅ All cryptographic operations use btpc-core (ML-DSA, SHA-512 unchanged)
- ✅ Single process reduces IPC attack surface
- ✅ Key zeroization on shutdown (verify in T027 logs)
- ✅ No hardcoded secrets

**Article II (Quantum Resistance)**:
- ✅ ML-DSA signatures unchanged (btpc-core direct usage)
- ✅ SHA-512 PoW unchanged
- ✅ No cryptographic protocol changes

**Article III (TDD)**:
- ✅ Contract tests written before implementation (T003-T009)
- ✅ Tests failed initially, then passed after implementation

**Article XI (Desktop App)**:
- ✅ Section 11.1: Backend state is single source of truth (Arc<RwLock<EmbeddedNode>>)
- ✅ Section 11.2: Backend-first validation (transaction_commands validates before mempool add)
- ✅ Section 11.3: Event-driven architecture (blockchain:*, mining:*, wallet:* events)
- ✅ Section 11.6: Event cleanup on shutdown (verify in shutdown hook)

**Performance Targets**:
- ✅ Balance queries <10ms (from T024)
- ✅ Transaction creation <50ms (from T025)
- ✅ UTXO selection <5ms
- ✅ Block validation <100ms (btpc-core preserved)

**Success Criteria**:
- ✅ All constitutional requirements met
- ✅ No violations introduced
- ✅ Article XI patterns verified
- ✅ Performance targets achieved

---

## Dependencies

**Setup Phase**:
- T001 (verify btpc-core) blocks all other tasks

**Test Phase (all parallel)**:
- T003-T009 can run in parallel (different test files)

**Implementation Dependencies**:
- T010 (UnifiedDatabase) blocks T011 (EmbeddedNode needs DB)
- T011 (EmbeddedNode) blocks T012 (commands need node), T013 (mining needs node)
- T012 (node commands) blocks T015 (main.rs registers commands)
- T013 (MiningThreadPool) blocks T014 (mining commands need pool)
- T010 (UnifiedDatabase) blocks T016 (wallet_manager needs DB), T018 (utxo_manager needs DB)
- T011 (EmbeddedNode) blocks T017 (transaction_commands needs mempool access)

**Removal Dependencies**:
- T015 (main.rs modified) blocks T019 (btpc_integration removal)
- T015 (main.rs modified) blocks T020 (process_manager removal)
- T017 (transaction_commands modified) blocks T021 (rpc_client removal)
- T018 (utxo_manager modified) blocks T022 (sync_service removal)

**Integration Test Dependencies**:
- T023-T028 require all implementation tasks (T010-T018) complete

**Validation Dependencies**:
- T029 (cargo test) requires all implementation complete
- T030 (cargo bench) requires T029 passing
- T031 (quickstart) requires T023-T028 complete
- T032 (code reduction) requires T019-T022 removal complete
- T033 (constitutional compliance) requires T029 passing

---

## Parallel Execution Examples

**Test Phase (all parallel):**
```bash
# Execute T003-T009 together (7 independent test files):
# Note: These are example commands - actual execution would use your task runner

Task "Write embedded node initialization test" (T003)
Task "Write blockchain state query test" (T004)
Task "Write sync progress test" (T005)
Task "Write mining start/stop test" (T006)
Task "Write mining stats test" (T007)
Task "Write blockchain:block_added event test" (T008)
Task "Write mining:hashrate_updated event test" (T009)
```

**Removal Phase (all parallel):**
```bash
# Execute T019-T022 together (4 independent file deletions):
Task "Remove btpc_integration.rs" (T019)
Task "Remove process_manager.rs" (T020)
Task "Remove rpc_client.rs" (T021)
Task "Remove sync_service.rs" (T022)
```

**No Parallelization (Sequential)**:
- T010-T014: Core implementation has dependencies (DB → Node → Commands → Mining)
- T015-T018: Modifications touch related state, must be sequential

---

## Notes

- **[P]** tasks = different files, no dependencies, can run in parallel
- **TDD Mandatory**: Tests (T003-T009) MUST be written and MUST FAIL before implementation (T010-T014)
- **Commit frequently**: After each task with descriptive message
- **Performance verification**: Benchmark at each step (T024, T030)
- **Constitutional compliance**: Article XI patterns enforced (T033)
- **No Docker**: All testing uses native binary (per spec constraint)

---

**Template Version**: 1.1 (BTPC-specific)
**Status**: ✅ READY FOR EXECUTION
**Estimated Total**: 33 numbered tasks
**Parallel Opportunities**: 7 test tasks [P], 4 removal tasks [P]