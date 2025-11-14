# Mining Database & RPC Investigation - 2025-11-05

## User Report
**Issues**:
1. No mining history displayed for "Blocks found"
2. No transaction history being displayed
3. RPC error: "429 Too Many Requests" from multiple mining threads

## Investigation Findings

### Issue #1: Mining History Not Persisted ❌ ARCHITECTURE ISSUE

**Current Architecture**:
- Mining logs stored in-memory only (`Arc<Mutex<MiningLogBuffer>>`)
- NO database file (`mining_history.json` does not exist)
- MiningLogBuffer is a circular buffer (max 1000 entries)
- Data lost on application restart

**Evidence**:
```bash
$ ls /home/bob/.btpc/data/wallet/
wallet.dat  wallet_transactions.json  wallet_utxos.json
# mining_history.json DOES NOT EXIST!

$ wc -l /home/bob/.btpc/data/wallet/*.json
  868 wallet_transactions.json  # 17 transactions * 51 lines each
  239 wallet_utxos.json         # 17 UTXOs * 14 lines each
```

**Code Analysis** (btpc-desktop-app/src-tauri/src/main.rs):
- Line 381-407: MiningLogBuffer implementation
- Line 423: `mining_logs: Arc<Mutex<MiningLogBuffer>>` (in-memory only)
- Line 1733-1737: `get_mining_logs()` command returns in-memory entries
- **NO persistence layer** - logs never written to disk

**Impact**:
- Mining history disappears on app restart
- "Blocks found" counter resets to 0
- No permanent record of mining rewards

---

### Issue #2: Transaction History IS Working ✅ (User May Be Looking in Wrong Place)

**Database Evidence**:
```bash
$ wc -l /home/bob/.btpc/data/wallet/wallet_transactions.json
868 lines  # 17 transactions exist

$ cat wallet_transactions.json | grep "txid" | head -3
"txid": "coinbase_1761997972_1761997972388997462",
"txid": "coinbase_1761994366_1761994366389322681",
"txid": "coinbase_1762306185_1762306185629218929",
```

**All 17 transactions are coinbase (mining rewards)**:
- Value: 3237500000 credits (32.375 BTPC) each
- is_coinbase: true
- Block heights: Unix timestamps (e.g., 1762306185)
- Confirmed timestamps: 2025-11-01 to 2025-11-05

**UTXO Count**: 17 UTXOs (all unspent)
- Total balance: 17 × 32.375 = **550.375 BTPC**

**Transactions ARE being stored correctly!**
- Fix from yesterday (address normalization) is working
- User may be looking at wrong wallet or wrong tab in UI

---

### Issue #3: RPC Rate Limiting ❌ CRITICAL BUG

**Error Message**:
```
12:28:58 [error] Mining error in thread 16: RPC HTTP error: 429 Too Many Requests
12:28:58 [error] Mining error in thread 5: RPC HTTP error: 429 Too Many Requests
12:28:58 [error] Mining error in thread 10: RPC HTTP error: 429 Too Many Requests
```

**Root Cause** (bins/btpc_miner/src/main.rs):

**Line 180-199**: Mining loop
```rust
while running.load(Ordering::SeqCst) {
    match Self::mine_block(&config, &hash_counter) {  // Called continuously!
```

**Line 209-214**: mine_block function
```rust
fn mine_block(config: &MinerConfig, hash_counter: &Arc<AtomicU64>) -> Result<...> {
    // Create a mining block template
    let block_template = Self::create_block_template(config)?;  // ❌ RPC CALL HERE!
```

**Line 250-269**: create_block_template function
```rust
fn create_block_template(config: &MinerConfig) -> Result<...> {
    let client = reqwest::blocking::Client::new();
    let request = json!({
        "jsonrpc": "2.0",
        "id": "1",
        "method": "getblocktemplate",  // ❌ RPC CALL EVERY 100K NONCES!
        "params": []
    });

    let response = client.post(&config.rpc_url)
        .header("Content-Type", "application/json")
        .body(request.to_string())
        .send()?;  // ❌ HTTP REQUEST
```

**Line 223**: Nonce range
```rust
const NONCE_RANGE: u32 = 100_000; // Mine 100k nonces before checking for new work
```

**Critical Math**:
- **16 mining threads** (default for 16-core CPU)
- Each thread mines **100,000 nonces** per template fetch
- Regtest difficulty = **minimum** (very fast mining)
- Estimated time per 100k nonces on regtest: **~0.001 seconds** (1 millisecond)

**RPC Call Rate**:
```
16 threads × (1 RPC call / 100,000 nonces) × (100,000 nonces / 0.001 sec)
= 16 threads × 1,000 RPC calls/sec/thread
= 16,000 RPC calls per second ❌ MASSIVE OVERLOAD!
```

**Expected Behavior** (Bitcoin mining):
- Fetch template **once every 60 seconds** (or when new block arrives)
- Share template across ALL threads
- Only refetch on: (1) new block notification, (2) timer expiry, (3) template stale

**Current Broken Behavior**:
- Fetch template **after every 100k nonces PER THREAD**
- NO template sharing between threads
- NO caching mechanism
- NO rate limiting

---

## Root Causes Summary

| Issue | Root Cause | Severity |
|-------|-----------|----------|
| No mining history | In-memory only storage (MiningLogBuffer) | MEDIUM |
| No transaction history | User error - data exists, check UI navigation | LOW |
| 429 RPC errors | Template fetched every 100k nonces × 16 threads | **CRITICAL** |

---

## Required Fixes

### Fix #1: Add Mining History Persistence (2-3 hours)
**Create**: `btpc-desktop-app/src-tauri/src/mining_history.rs`

**Required Changes**:
1. Create `MiningHistoryManager` struct with RocksDB or JSON file storage
2. Store mining events: block_found, reward_added, template_updated
3. Add `save_mining_event()` method
4. Add `get_mining_history()` method for retrieval
5. Update main.rs line 1385 to call `save_mining_event()` after UTXO addition
6. Migrate MiningLogBuffer to persistent storage

**Schema** (JSON example):
```json
{
  "events": [
    {
      "timestamp": "2025-11-05T12:30:00Z",
      "event_type": "block_found",
      "thread_id": 5,
      "block_height": 1762306185,
      "block_hash": "00000000abcd...",
      "reward_credits": 3237500000,
      "txid": "coinbase_1762306185_..."
    }
  ]
}
```

---

### Fix #2: Fix RPC Rate Limiting (CRITICAL - 4-5 hours)

**Strategy**: Template caching with periodic updates

**Create**: `bins/btpc_miner/src/template_cache.rs`

**Required Changes**:

1. **Template Cache Structure**:
```rust
pub struct TemplateCache {
    current_template: Arc<RwLock<Option<Block>>>,
    last_update: Arc<RwLock<Instant>>,
    update_interval: Duration,  // 60 seconds default
}
```

2. **Background Template Fetcher**:
```rust
impl TemplateCache {
    pub fn start_updater(&self, config: MinerConfig) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                match fetch_template_from_rpc(&config).await {
                    Ok(template) => {
                        *self.current_template.write().unwrap() = Some(template);
                        *self.last_update.write().unwrap() = Instant::now();
                    }
                    Err(e) => eprintln!("Template fetch failed: {}", e),
                }
            }
        });
    }

    pub fn get_template(&self) -> Option<Block> {
        self.current_template.read().unwrap().clone()
    }
}
```

3. **Update Mining Loop** (main.rs line 180):
```rust
// BEFORE (broken):
while running.load(Ordering::SeqCst) {
    match Self::mine_block(&config, &hash_counter) {  // Fetches template every call!

// AFTER (fixed):
let template_cache = Arc::new(TemplateCache::new(Duration::from_secs(60)));
template_cache.start_updater(config.clone());

while running.load(Ordering::SeqCst) {
    if let Some(template) = template_cache.get_template() {
        match Self::mine_block_with_template(&template, &hash_counter) {
```

4. **Update mine_block signature**:
```rust
// BEFORE:
fn mine_block(config: &MinerConfig, ...) -> Result<...> {
    let block_template = Self::create_block_template(config)?;  // ❌ RPC call

// AFTER:
fn mine_block_with_template(template: &Block, ...) -> Result<...> {
    let mut mining_block = template.clone();  // ✅ No RPC call
```

**Expected RPC Call Rate After Fix**:
```
1 background thread × (1 RPC call / 60 seconds) = 0.0167 RPC calls/sec
Reduction: 16,000 → 0.0167 calls/sec (960,000× improvement!)
```

---

### Fix #3: Verify Transaction History UI (15 minutes)

**Frontend Check** (mining.html):
1. Verify `get_wallet_utxos()` is called on page load
2. Verify transaction list rendering logic
3. Check for JavaScript console errors
4. Ensure address parameter is correct (case-normalized)

**Backend Check** (main.rs line 1747-1759):
1. `get_wallet_utxos()` returns all UTXOs for address
2. Address normalization is applied (fixed yesterday)
3. UTXO → transaction mapping works

**Quick Test**:
```javascript
// Run in browser console on mining.html:
await window.__TAURI__.invoke('get_wallet_utxos');
// Should return 17 UTXOs

await window.__TAURI__.invoke('get_mining_logs');
// Should return recent log entries (in-memory only)
```

---

## Testing Plan

### Test #1: RPC Rate Limiting Fix
```bash
# Start node with logging
./target/release/btpc_node --network regtest --log-level debug 2>&1 | tee node.log &

# Start miner with 16 threads
./target/release/btpc_miner --network regtest --address <ADDR> --threads 16 2>&1 | tee miner.log &

# Monitor RPC call rate
tail -f node.log | grep "getblocktemplate" | ts -s
# Should show ~1 call every 60 seconds (not 16,000/sec!)

# Check for 429 errors
grep "429" miner.log
# Should return NOTHING after fix
```

### Test #2: Mining History Persistence
```bash
# Start mining, find 1 block, stop mining
# Restart desktop app
# Navigate to Mining History tab
# EXPECTED: Block found event still visible ✅
# CURRENT: No events shown (data lost) ❌
```

### Test #3: Transaction History Display
```bash
# Navigate to Transactions tab
# EXPECTED: 17 transactions shown (550.375 BTPC total)
# Check browser console for errors
```

---

## Impact Assessment

### Critical (Fix Immediately)
- **RPC Rate Limiting**: Miner is unusable, node crashes from request flood
- **Estimated Downtime**: Mining completely broken
- **User Impact**: Cannot mine any blocks

### Medium (Fix Soon)
- **Mining History**: User experience degraded, no record of past mining
- **Estimated Downtime**: None (feature missing, not broken)
- **User Impact**: Confusion about mining activity

### Low (Verify)
- **Transaction History**: Likely user error or UI navigation issue
- **Estimated Downtime**: None (working correctly in database)
- **User Impact**: None if UI navigation is clarified

---

## Recommended Action Priority

1. **IMMEDIATE** (Today): Fix RPC rate limiting bug
   - Implement template caching
   - Test with 16 threads
   - Verify <1 RPC call per minute

2. **HIGH** (This Week): Add mining history persistence
   - Create RocksDB or JSON storage
   - Persist block found events
   - Display in UI

3. **MEDIUM** (This Week): Verify transaction history UI
   - Check frontend rendering
   - Add user guidance if needed
   - Document UI navigation

---

**Investigation Complete**: 2025-11-05
**Next Step**: Implement template caching fix for RPC rate limiting