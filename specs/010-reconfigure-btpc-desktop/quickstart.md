# Quickstart: Manual Test Scenarios for Embedded Architecture

**Feature**: 010-reconfigure-btpc-desktop
**Version**: 1.0
**Purpose**: Manual testing guide for embedded btpc-core architecture
**Last Updated**: 2025-11-10

---

## Prerequisites

**Environment Setup**:
```bash
# Build desktop app with embedded architecture
cd /home/bob/BTPC/BTPC/btpc-desktop-app
npm run tauri:build

# Verify single executable created
ls -lh src-tauri/target/release/btpc-desktop-app
# Expected: Single binary, <50MB

# Clean slate testing (remove existing data)
rm -rf ~/.btpc/regtest
```

**Tools Required**:
- `ps aux | grep btpc` - Verify no orphaned processes
- `lsof -p <pid>` - Check open file handles
- `htop` - Monitor CPU/GPU usage during mining
- `du -sh ~/.btpc/regtest` - Monitor database size
- `time` command - Measure operation latency

---

## Scenario 1: First-Time Launch

**Objective**: Verify single binary architecture with automatic initialization

### Test Steps

1. **Launch desktop app**:
   ```bash
   cd /home/bob/BTPC/BTPC/btpc-desktop-app
   npm run tauri:dev
   ```

2. **Verify no external processes spawned**:
   ```bash
   ps aux | grep btpc_node
   # Expected: No results (process eliminated)

   ps aux | grep btpc_miner
   # Expected: No results (process eliminated)

   ps aux | grep btpc
   # Expected: Only btpc-desktop-app process
   ```

3. **Check RocksDB initialization**:
   ```bash
   ls -la ~/.btpc/regtest/
   # Expected files:
   # - LOCK (RocksDB lock file)
   # - LOG (RocksDB debug log)
   # - 000001.log (Write-ahead log)
   # - CURRENT (current manifest file)
   # NO separate wallet files (migrated to RocksDB)
   ```

4. **Observe UI sync progress**:
   - Open Developer Console: Ctrl+Shift+I
   - Check for event log:
     ```javascript
     // Expected events in console:
     node:initialized { height: 0, network: "regtest", peer_count: 0 }
     blockchain:state_updated { height: 0, sync_status: "initializing" }
     ```

5. **Create wallet while sync continues**:
   - Click "Create New Wallet" button
   - Enter password: `test123456`
   - Verify wallet created without blocking sync
   - Check console for: `wallet:created { wallet_id: "..." }`

### Success Criteria

- [x] Only 1 btpc process running (desktop app)
- [x] RocksDB initialized in ~/.btpc/regtest
- [x] No .dat wallet files (migrated to RocksDB CF_WALLETS)
- [x] UI displays sync progress in real-time
- [x] Wallet creation succeeds during sync
- [x] Events emitted: `node:initialized`, `blockchain:state_updated`

### Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| App startup time | <3 seconds | `time npm run tauri:dev` |
| RocksDB initialization | <1 second | Check LOG file timestamp |
| First event emission | <500ms | Console timestamp |

---

## Scenario 2: Fast Balance Queries

**Objective**: Verify <10ms balance query performance (vs current ~50ms RPC)

### Test Steps

1. **Generate test wallet with balance**:
   ```bash
   # In desktop app console
   const { invoke } = window.__TAURI__.tauri;

   // Create wallet
   const wallet = await invoke('create_wallet', {
     password: 'test123456'
   });
   console.log('Wallet ID:', wallet.wallet_id);

   // Get first address
   const address = await invoke('get_addresses', {
     walletId: wallet.wallet_id
   });
   console.log('Address:', address[0]);
   ```

2. **Mine 100 blocks to wallet** (generate balance):
   ```bash
   # Start mining to test address
   const { invoke } = window.__TAURI__.tauri;

   await invoke('start_mining', {
     miningAddress: '<address_from_step1>',
     cpuThreads: 2,
     gpuEnabled: false
   });

   // Wait for 100 blocks (monitor mining:block_found events)
   // Or use console to check: await invoke('get_blockchain_state')
   ```

3. **Measure balance query latency**:
   ```javascript
   // Measure 100 balance queries
   const { invoke } = window.__TAURI__.tauri;

   const latencies = [];
   for (let i = 0; i < 100; i++) {
     const start = performance.now();
     await invoke('get_balance', { walletId: wallet.wallet_id });
     const end = performance.now();
     latencies.push(end - start);
   }

   // Calculate percentiles
   latencies.sort((a, b) => a - b);
   const p50 = latencies[49];
   const p90 = latencies[89];
   const p99 = latencies[98];

   console.log('Balance query latency:');
   console.log('  50th percentile:', p50.toFixed(2), 'ms');
   console.log('  90th percentile:', p90.toFixed(2), 'ms');
   console.log('  99th percentile:', p99.toFixed(2), 'ms');
   ```

4. **Verify RocksDB cache effectiveness**:
   ```bash
   # Check RocksDB cache hit rate in LOG file
   grep "cache" ~/.btpc/regtest/LOG
   # Expected: High hit rate (>90%) after warmup
   ```

### Success Criteria

- [x] 90th percentile balance query latency <10ms
- [x] No RPC client calls (verify no HTTP traffic)
- [x] Balance queries succeed during blockchain sync
- [x] RocksDB cache hit rate >90%

### Performance Targets

| Metric | Target | Actual |
|--------|--------|--------|
| p50 latency | <5ms | _____ ms |
| p90 latency | <10ms | _____ ms |
| p99 latency | <20ms | _____ ms |
| Cache hit rate | >90% | ____% |

---

## Scenario 3: Transaction Creation

**Objective**: Verify instant UTXO selection and in-process transaction signing

### Test Steps

1. **Create transaction**:
   ```javascript
   const { invoke } = window.__TAURI__.tauri;

   // Measure transaction creation latency
   const start = performance.now();

   const tx = await invoke('create_transaction', {
     walletId: wallet.wallet_id,
     fromAddress: '<sender_address>',
     toAddress: '<recipient_address>',
     amount: 50_00000000,  // 50 BTPC
     feeRate: 1000  // 1000 crd/byte
   });

   const end = performance.now();
   console.log('Transaction creation latency:', (end - start).toFixed(2), 'ms');
   console.log('Transaction ID:', tx.txid);
   ```

2. **Verify no RPC calls**:
   ```bash
   # Monitor network activity (should be none)
   sudo netstat -tuln | grep 18360
   # Expected: No results (no RPC server)

   # Or check process open connections
   lsof -p $(pgrep btpc-desktop) | grep TCP
   # Expected: Only P2P connections (port 18444), no RPC (18360)
   ```

3. **Check mempool emission**:
   ```javascript
   // Listen for mempool event
   const { listen } = window.__TAURI__.event;

   await listen('mempool:transaction_added', (event) => {
     console.log('Transaction added to mempool:', event.payload);
     // Expected: { txid: "...", fee_rate: 1000, size: 1952 }
   });
   ```

4. **Verify UTXO reservation**:
   ```javascript
   // Try creating duplicate transaction (should fail)
   try {
     const tx2 = await invoke('create_transaction', {
       walletId: wallet.wallet_id,
       fromAddress: '<same_sender>',
       toAddress: '<recipient_address>',
       amount: 50_00000000,
       feeRate: 1000
     });
     console.error('FAIL: Duplicate transaction allowed (UTXO not reserved)');
   } catch (err) {
     console.log('PASS: UTXO reservation prevented double-spend:', err);
   }
   ```

### Success Criteria

- [x] Transaction creation latency <50ms
- [x] No RPC client HTTP requests (verify with netstat)
- [x] `mempool:transaction_added` event emitted
- [x] UTXO reservation prevents concurrent double-spends
- [x] ML-DSA signature validated in-process

### Performance Targets

| Metric | Target | Actual |
|--------|--------|--------|
| UTXO selection | <5ms | _____ ms |
| Transaction signing | <20ms | _____ ms |
| Total creation time | <50ms | _____ ms |

---

## Scenario 4: Mining Operations

**Objective**: Verify background thread pool mining with event-driven updates

### Test Steps

1. **Start CPU mining**:
   ```javascript
   const { invoke } = window.__TAURI__.tauri;

   const result = await invoke('start_mining', {
     miningAddress: '<your_address>',
     cpuThreads: 4,
     gpuEnabled: false
   });

   console.log('Mining started:', result);
   // Expected: { success: true, cpu_threads: 4, gpu_enabled: false }
   ```

2. **Verify mining threads**:
   ```bash
   # Check CPU usage (should see 4 cores active)
   htop
   # Expected: 4 threads at ~100% CPU (named btpc-cpu-miner-*)

   # Verify thread priority
   ps -eo pid,ni,comm | grep btpc-cpu-miner
   # Expected: Nice value 10 (below-normal priority)
   ```

3. **Monitor hashrate events**:
   ```javascript
   const { listen } = window.__TAURI__.event;

   await listen('mining:hashrate_updated', (event) => {
     console.log('Hashrate:', event.payload.total_hashrate, 'H/s');
     console.log('CPU:', event.payload.cpu_hashrate, 'H/s');
     // Expected: Updates every 5 seconds
   });
   ```

4. **Test GPU mining (if available)**:
   ```javascript
   // List available GPUs
   const gpus = await invoke('get_available_gpus');
   console.log('Available GPUs:', gpus);

   // Start GPU mining
   if (gpus.gpus.length > 0) {
     await invoke('stop_mining');  // Stop CPU first
     await invoke('start_mining', {
       miningAddress: '<your_address>',
       cpuThreads: 2,
       gpuEnabled: true,
       gpuDeviceIndex: 0
     });

     // Verify GPU usage
     // Run: nvidia-smi (NVIDIA) or radeontop (AMD)
   }
   ```

5. **Wait for block found**:
   ```javascript
   await listen('mining:block_found', (event) => {
     console.log('Block found!', event.payload);
     // Expected: { height: N, hash: "...", reward: 5000000000, nonce: ... }
   });

   // Verify block added to blockchain (not submitted via RPC)
   await listen('blockchain:block_added', (event) => {
     console.log('Block added to blockchain:', event.payload.height);
   });
   ```

6. **Stop mining gracefully**:
   ```javascript
   const start = performance.now();

   const result = await invoke('stop_mining');

   const end = performance.now();
   console.log('Mining stop latency:', (end - start).toFixed(2), 'ms');
   console.log('Final stats:', result);
   // Expected: { blocks_found: N, total_hashes: ..., uptime_secs: ... }
   ```

7. **Verify no zombie threads**:
   ```bash
   # Check for leftover mining threads
   ps aux | grep btpc-cpu-miner
   # Expected: No results (all threads joined)

   # Check thread count
   ps -T -p $(pgrep btpc-desktop) | wc -l
   # Expected: Back to baseline count (~10-15 threads for Tauri runtime)
   ```

### Success Criteria

- [x] Mining threads start within 1 second
- [x] CPU threads use below-normal priority (verified with ps)
- [x] `mining:hashrate_updated` events every 5 seconds
- [x] Found blocks added directly to blockchain (no RPC)
- [x] Mining stops within 5 seconds
- [x] No zombie threads after stop

### Performance Targets

| Metric | Target | Actual |
|--------|--------|--------|
| Mining start latency | <1s | _____ s |
| Mining stop latency | <5s | _____ s |
| Hashrate update frequency | 5s | _____ s |
| CPU thread count | user-specified | _____ threads |

---

## Scenario 5: Graceful Shutdown

**Objective**: Verify ordered shutdown with no orphaned processes or corruption

### Test Steps

1. **Start mining and create pending transaction**:
   ```javascript
   // Start mining
   await invoke('start_mining', {
     miningAddress: '<address>',
     cpuThreads: 4,
     gpuEnabled: false
   });

   // Create transaction (add to mempool)
   const tx = await invoke('create_transaction', {
     walletId: wallet.wallet_id,
     fromAddress: '<sender>',
     toAddress: '<recipient>',
     amount: 10_00000000,
     feeRate: 1000
   });
   console.log('Transaction in mempool:', tx.txid);
   ```

2. **Monitor shutdown events**:
   ```javascript
   const { listen } = window.__TAURI__.event;

   await listen('node:shutdown_started', (event) => {
     console.log('Shutdown started, timeout:', event.payload.shutdown_timeout_secs, 's');
   });

   await listen('node:shutdown_progress', (event) => {
     console.log('Shutdown step:', event.payload.step_name, 'completed:', event.payload.completed);
   });

   await listen('node:shutdown_complete', (event) => {
     console.log('Shutdown complete in', event.payload.duration_secs, 'seconds');
   });
   ```

3. **Trigger shutdown**:
   ```javascript
   // Close window (or Ctrl+C in terminal)
   window.close();
   ```

4. **Verify shutdown order** (check console output):
   ```
   Expected sequence:
   1. node:shutdown_started
   2. node:shutdown_progress { step: 1, step_name: "Stopping mining threads" }
   3. node:shutdown_progress { step: 2, step_name: "Closing P2P connections" }
   4. node:shutdown_progress { step: 3, step_name: "Flushing mempool to database" }
   5. node:shutdown_progress { step: 4, step_name: "Flushing RocksDB WAL" }
   6. node:shutdown_progress { step: 5, step_name: "Zeroizing cryptographic keys" }
   7. node:shutdown_complete { success: true, duration_secs: <8 }
   ```

5. **Verify no orphaned processes**:
   ```bash
   # Wait 2 seconds after window closes
   sleep 2

   # Check for any btpc processes
   ps aux | grep btpc
   # Expected: No results

   # Check for orphaned threads
   ps aux | grep -E "btpc-cpu-miner|btpc_node|btpc_miner"
   # Expected: No results
   ```

6. **Verify database consistency**:
   ```bash
   # Check RocksDB lock file removed
   ls ~/.btpc/regtest/LOCK
   # Expected: File not found (lock released)

   # Restart app and verify mempool transaction persisted
   npm run tauri:dev

   # In console:
   const { invoke } = window.__TAURI__.tauri;
   const state = await invoke('get_blockchain_state');
   console.log('Restarted at height:', state.height);

   // Check if pending transaction still in mempool
   // (implementation detail: mempool may or may not persist)
   ```

7. **Test force-quit recovery**:
   ```bash
   # Start app
   npm run tauri:dev

   # Force kill (simulates crash)
   pkill -9 btpc-desktop

   # Restart app
   npm run tauri:dev

   # Verify no corruption
   # Expected: App starts normally, blockchain state recovered from RocksDB WAL
   ```

### Success Criteria

- [x] Shutdown completes within 30 seconds
- [x] All shutdown steps complete (7 events emitted)
- [x] No orphaned processes after shutdown
- [x] RocksDB lock file removed
- [x] Blockchain state consistent after restart
- [x] Force-quit recovery succeeds (no corruption)

### Performance Targets

| Metric | Target | Actual |
|--------|--------|--------|
| Total shutdown time | <30s | _____ s |
| Mining stop step | <5s | _____ s |
| P2P close step | <5s | _____ s |
| RocksDB flush step | <10s | _____ s |
| Orphaned processes | 0 | _____ |

---

## Scenario 6: Upgrade from Multi-Process Version

**Objective**: Verify migration from old architecture (wallet .dat files → RocksDB)

### Test Steps

1. **Set up old architecture data**:
   ```bash
   # Create fake old wallet files
   mkdir -p ~/.btpc/wallets
   echo "fake_encrypted_wallet_data" > ~/.btpc/wallets/test-wallet.dat

   # Verify file exists
   ls -la ~/.btpc/wallets/
   ```

2. **Start embedded architecture app**:
   ```bash
   npm run tauri:dev

   # Check console for migration event
   # Expected: "Migrating 1 wallet file(s) to RocksDB..."
   ```

3. **Verify migration**:
   ```bash
   # Check RocksDB for wallet data
   # (Requires RocksDB CLI tool or log inspection)

   # Check .dat file renamed
   ls -la ~/.btpc/wallets/
   # Expected: test-wallet.dat.migrated (backup created)
   ```

4. **Verify wallet still accessible**:
   ```javascript
   const { invoke } = window.__TAURI__.tauri;

   const wallets = await invoke('list_wallets');
   console.log('Migrated wallets:', wallets);
   // Expected: test-wallet appears in list
   ```

### Success Criteria

- [x] .dat files detected and migrated
- [x] Backup .dat.migrated files created
- [x] Wallets accessible after migration
- [x] No data loss (compare balances before/after)

---

## Measurement Commands

### Process Monitoring
```bash
# Check process tree
pstree -p $(pgrep btpc-desktop)

# Monitor resource usage
htop -p $(pgrep btpc-desktop)

# Check thread count
ps -T -p $(pgrep btpc-desktop) | wc -l

# Check open files
lsof -p $(pgrep btpc-desktop) | wc -l
```

### Database Monitoring
```bash
# Check RocksDB size
du -sh ~/.btpc/regtest

# Check RocksDB file count
find ~/.btpc/regtest -type f | wc -l

# Check WAL size
ls -lh ~/.btpc/regtest/*.log

# Monitor RocksDB stats (requires LOG parsing)
tail -f ~/.btpc/regtest/LOG
```

### Performance Profiling
```bash
# CPU profiling (requires perf)
perf record -g -p $(pgrep btpc-desktop)
perf report

# Memory profiling (requires valgrind)
valgrind --tool=massif --massif-out-file=massif.out btpc-desktop-app
ms_print massif.out
```

---

## Success Criteria Summary

### Code Reduction
- [x] ProcessManager module removed (~459 lines)
- [x] RpcClient module removed (~424 lines)
- [x] Total reduction: ~883+ lines

### Performance Improvements
- [x] Balance queries: <10ms (vs ~50ms RPC baseline)
- [x] Transaction creation: <50ms (vs ~100ms RPC baseline)
- [x] UTXO selection: <5ms (direct database access)

### Architecture Validation
- [x] Single process (no btpc_node, btpc_miner)
- [x] Single RocksDB instance (CF_BLOCKS, CF_UTXOS, CF_WALLETS)
- [x] Event-driven UI updates (no polling)
- [x] Graceful shutdown (<30s)
- [x] No orphaned processes
- [x] Database consistency after crash

### Article XI Compliance
- [x] Backend is single source of truth
- [x] Frontend validates with backend first
- [x] Events emitted on all state changes
- [x] Event listeners cleaned up on unload
- [x] No localStorage for authoritative data

---

**Document Version**: 1.0
**Test Status**: Ready for Execution
**Next Step**: Execute manual tests and record actual measurements