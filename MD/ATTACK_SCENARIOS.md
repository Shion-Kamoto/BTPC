# BTPC Attack Scenarios - Educational Analysis


**Purpose:** Detailed attack scenarios to understand vulnerabilities
**Audience:** Development team, security reviewers
**Classification:** INTERNAL - Do not distribute publicly before fixes

---

## Scenario 1: The Signature Bypass Attack (CRITICAL)

**Vulnerability:** Missing ML-DSA signature verification
**Difficulty:** TRIVIAL
**Impact:** Complete theft of funds

### Attack Steps

1. **Reconnaissance**
```bash
# Attacker scans blockchain for high-value UTXOs
btpc-cli listunspent | jq '.[] | select(.value > 100000000)'
# Finds UTXO worth 1000 BTPC at txid abc123:0
```

2. **Craft Malicious Transaction**
```rust
// Attacker creates transaction spending victim's UTXO
let stolen_utxo = OutPoint {
    txid: Hash::from_hex("abc123..."),
    vout: 0,
};

let malicious_tx = Transaction {
    version: 1,
    inputs: vec![TransactionInput {
        previous_output: stolen_utxo,
        script_sig: Script::new(),  // Empty/invalid signature!
        sequence: 0xffffffff,
    }],
    outputs: vec![TransactionOutput {
        value: 100_000_000_000,  // 1000 BTPC
        script_pubkey: Script::pay_to_pubkey_hash(attacker_address),
    }],
    lock_time: 0,
};
```

3. **Submit to Network**
```bash
# Transaction accepted because signatures not checked
btpc-cli sendrawtransaction <malicious_tx_hex>
# Returns: txid xyz789... (SUCCESS!)
```

4. **Wait for Confirmation**
```bash
# Miner includes transaction in block
# Validation passes (no signature check)
# Attacker now controls victim's 1000 BTPC
```

### Why It Works
- `storage_validation.rs:165-166` has TODO comment instead of verification
- No signature check in `validate_transaction_with_utxos()`
- Validation only checks UTXO exists, not ownership

### Impact
- **Any UTXO can be stolen by anyone**
- Total loss of funds
- Network unusable for value transfer

### Mitigation
Implement ML-DSA signature verification (see Fix #1)

---

## Scenario 2: The Time-Warp Attack (CRITICAL)

**Vulnerability:** Missing median-time-past validation
**Difficulty:** MODERATE (requires 51% hashrate)
**Impact:** Difficulty manipulation, rapid mining

### Attack Steps

1. **Preparation (Day 0)**
```
Attacker controls 51% of network hashrate
Waits for block height = 2016 (first difficulty adjustment)
Current difficulty: 1000 (takes ~10 minutes per block)
Target: Reduce difficulty to 250 (4x easier)
```

2. **Phase 1: Fast-Forward Time (Blocks 1-2015)**
```
For each block from 1 to 2015:
  - Set timestamp = current_time + 7200 seconds (MAX_FUTURE)
  - Mine block
  - Broadcast immediately

Timeline:
  Block 1: timestamp = Jan 1, 2025 02:00:00 (2 hours in future)
  Block 2: timestamp = Jan 1, 2025 04:00:00 (4 hours in future)
  ...
  Block 2015: timestamp = Jan 168, 2025 06:00:00 (168 days in future)

Network accepts all blocks (within MAX_FUTURE_BLOCK_TIME)
```

3. **Phase 2: Compress Time (Block 2016)**
```
Block 2016 (difficulty adjustment):
  - Timestamp = last_block.timestamp + 61 seconds (MIN_BLOCK_TIME)
  - Timestamp = Jan 168, 2025 06:01:01

Difficulty calculation:
  first_block = Block 1 (Jan 1, 2025 00:00:00)
  last_block = Block 2016 (Jan 168, 2025 06:01:01)
  actual_timespan = 14,522,461 seconds (~168 days)
  expected_timespan = 1,209,600 seconds (~14 days)
  ratio = actual_timespan / expected = 12.0

But MAX_DIFFICULTY_ADJUSTMENT = 4.0 (clamped)
New difficulty = old_difficulty / 4.0 = 1000 / 4 = 250
```

4. **Phase 3: Exploit Low Difficulty**
```
Blocks 2017-4031:
  - Attacker mines at 4x speed
  - Each block takes ~2.5 minutes instead of 10
  - Mines 2016 blocks in ~3.5 days instead of 14 days
  - Earns 4x more block rewards
  - Other miners can't compete
```

5. **Phase 4: Repeat**
```
At block 4032 (next adjustment):
  - Repeat time-warp attack
  - Reduce difficulty to 62.5 (another 4x)
  - Now mining at 16x original speed
  - Continue until difficulty near zero
```

### Why It Works
- No median-time-past validation
- Only checks `timestamp > prev_block.timestamp`
- Allows timestamps 2 hours in future
- 51% miner controls all timestamps

### Impact
- Difficulty reduces to near-zero
- Attacker mines all blocks
- Network centralization
- Inflation attack (excess rewards)

### Real-World Example
- Bitcoin's Time-Warp Bug (fixed in Bitcoin Core)
- Several altcoins exploited before fix

### Mitigation
Implement median-time-past validation (see Fix #3)

---

## Scenario 3: The Integer Overflow Attack (CRITICAL)

**Vulnerability:** Unchecked arithmetic in reward calculation
**Difficulty:** MODERATE (requires specific conditions)
**Impact:** Consensus split, node crashes

### Attack Steps

1. **Setup**
```
Network at very high block height (near u32::MAX)
height = 4,294,967,200 (near u32::MAX = 4,294,967,295)
```

2. **Craft Block at Overflow Height**
```rust
// In rewards.rs calculate_block_reward_with_params()
let height = 4_294_967_200u32;
let params = RewardParams::mainnet();

// Line 46: This calculation overflows
let current_decrease = (height as f64) * decrease_per_block;
// height * decrease_per_block overflows f64 mantissa
// Result: NaN or Infinity

// Line 48: Conversion to u64 fails
let current_reward = (params.initial_reward as f64) - current_decrease;
let final_reward = current_reward.max(params.tail_emission as f64) as u64;
// final_reward = 0 or random value (undefined behavior)
```

3. **Submit Block**
```
Different nodes calculate different rewards:
- Node A (x86_64 Linux): reward = 0
- Node B (ARM macOS): reward = 50_000_000
- Node C (RISC-V): reward = 18_446_744_073_709_551_615 (u64::MAX)
```

4. **Consensus Split**
```
Node A: Rejects block (reward too low)
Node B: Accepts block (reward correct)
Node C: Accepts block (different reward)

Network splits into 3 chains:
- Chain A: Nodes rejecting block
- Chain B: Nodes accepting with reward X
- Chain C: Nodes accepting with reward Y

Network permanently forked
```

### Why It Works
- f64 has only 53 bits of mantissa precision
- Large u32 * f64 loses precision
- f64 → u64 conversion undefined for NaN/Infinity
- Different platforms handle differently

### Impact
- Consensus split at high block heights
- Network fragmentation
- Loss of decentralization
- Trust in protocol destroyed

### Mitigation
Use only integer arithmetic (see Fix #4)

---

## Scenario 4: The Race Condition Attack (HIGH)

**Vulnerability:** Non-atomic UTXO updates
**Difficulty:** MODERATE (requires timing)
**Impact:** UTXO corruption, double-spend

### Attack Steps

1. **Setup**
```
Attacker runs miner node
Creates two conflicting transactions:
  TX1: Spends UTXO A → Output B
  TX2: Spends UTXO A → Output C (double-spend)
```

2. **Mining**
```
Attacker mines two blocks simultaneously:
  Block N: Contains TX1
  Block N': Contains TX2 (same height, different hash)
```

3. **Broadcast Timing**
```python
# Thread 1: Broadcast Block N to half of network
import socket
import time

def broadcast_block_n():
    for peer in network_peers[:len(network_peers)//2]:
        peer.send(block_n)
        time.sleep(0.001)  # Precise timing

# Thread 2: Broadcast Block N' to other half
def broadcast_block_n_prime():
    for peer in network_peers[len(network_peers)//2:]:
        peer.send(block_n_prime)
        time.sleep(0.001)

# Execute simultaneously
import threading
t1 = threading.Thread(target=broadcast_block_n)
t2 = threading.Thread(target=broadcast_block_n_prime)
t1.start()
t2.start()
```

4. **Race Condition Trigger**
```rust
// In storage_validation.rs apply_transaction()

// Node A processes TX1:
let mut utxo_db = self.utxo_db.write().unwrap();
utxo_db.remove_utxo(&utxo_a)?;  // Removes UTXO A
// LOCK RELEASED HERE

// Node B processes TX2 (simultaneously):
let mut utxo_db = self.utxo_db.write().unwrap();
utxo_db.remove_utxo(&utxo_a)?;  // Tries to remove UTXO A again
// UTXO A already removed - error or corruption

// OR: Crash during TX1 processing between remove and add
utxo_db.remove_utxo(&utxo_a)?;  // Removed
// <-- PROCESS CRASHES HERE
utxo_db.store_utxo(&utxo_b)?;  // Never executed

// Result: UTXO A removed but B never added
// Funds permanently lost
```

5. **Outcome**
```
Case 1: UTXO set inconsistency
  - Some nodes have UTXO A removed
  - Some nodes have UTXO A still present
  - Consensus split

Case 2: Permanent fund loss
  - UTXO A removed but outputs never added
  - Funds vanish from circulation

Case 3: Double-spend success
  - Both TX1 and TX2 processed
  - UTXO A spent twice
  - Network corruption
```

### Why It Works
- No atomic transaction guarantees
- Lock released between operations
- No rollback on failure
- Process crash leaves inconsistent state

### Impact
- UTXO corruption
- Fund loss
- Consensus splits
- Unreliable transaction processing

### Mitigation
Use RocksDB WriteBatch for atomicity (see Fix #5)

---

## Scenario 5: The Cross-Fork Replay Attack (HIGH)

**Vulnerability:** No transaction replay protection
**Difficulty:** EASY
**Impact:** Unintended fund loss across forks

### Attack Steps

1. **Fork Creation**
```
Timeline:
  Block 1000: BTPC mainnet (Chain A)
  |
  ├─ Block 1001: Protocol upgrade (controversial)
  |
  Split:
    Chain A (Pro-upgrade): Block 1001a
    Chain B (Anti-upgrade): Block 1001b

Both chains have same history up to block 1000
User Alice has 100 BTPC in UTXO at block 1000
```

2. **User Transaction on Chain A**
```
Alice creates transaction on Chain A:
  TX_Alice:
    Input: UTXO from block 1000 (100 BTPC)
    Output: 100 BTPC to merchant (buying goods)

Transaction included in Chain A block 1050
Alice receives goods from merchant
```

3. **Attacker Replay on Chain B**
```python
# Attacker copies TX_Alice from Chain A
tx_bytes = chain_a.get_transaction("TX_Alice").serialize()

# Submits to Chain B
chain_b.submit_transaction(tx_bytes)

# Result: Transaction valid on Chain B too!
# - Same UTXO exists (from before fork)
# - Same signature (no fork ID check)
# - Same outputs

Chain B includes TX_Alice in block 1075
```

4. **Double Payment**
```
Outcome:
  - Alice paid merchant on Chain A (intended)
  - Alice also paid merchant on Chain B (unintended)
  - Alice lost 100 BTPC on both chains
  - Merchant received 200 BTPC total

If Alice wanted to spend on Chain B differently, too bad!
Attacker controls her Chain B funds via replay.
```

### Real-World Scenario
```
Post-Fork Market:
  - Chain A (BTPC-Classic) = $10 per coin
  - Chain B (BTPC-Quantum) = $15 per coin

Alice wants to:
  - Sell BTPC-Classic (Chain A) for $10
  - Keep BTPC-Quantum (Chain B) worth $15

But attacker replays sell transaction to Chain B:
  - Alice loses valuable Chain B coins
  - Forced to sell at $10 instead of $15
  - Lost value: $5 * 100 = $500
```

### Why It Works
- No fork identifier in signatures
- Transactions are identical across forks
- Both chains accept same signatures
- No way to make transaction fork-specific

### Impact
- Users lose funds on unintended chains
- Exchanges suffer losses
- Fork adoption inhibited
- Market manipulation possible

### Mitigation
Add fork ID to signature hash (see Fix #6)

---

## Scenario 6: The Selfish Mining Attack (MEDIUM)

**Vulnerability:** No block withholding protection
**Difficulty:** HARD (requires 25%+ hashrate)
**Impact:** Unfair mining rewards

### Attack Steps

1. **Setup**
```
Attacker controls 30% of network hashrate
Honest miners: 70%
Normal distribution: Attacker gets 30% of rewards
Goal: Get >30% via strategy
```

2. **Phase 1: Secret Chain**
```
Attacker mines blocks privately:
  Public chain:  ... → Block 100 → Block 101
  Secret chain:  ... → Block 100 → Block 100'

Attacker found Block 100' but doesn't broadcast
Continues mining on top of 100'
```

3. **Phase 2: Extend Lead**
```
Public:  ... → 100 → 101 → 102
Secret:  ... → 100 → 100' → 101'

If honest miner finds 102:
  - Public chain extends
  - Secret chain still hidden

If attacker finds 101':
  - Secret chain now 2 blocks ahead
  - Still hidden
```

4. **Phase 3: Selfish Reveal**
```
Scenario A: Secret chain has lead of 2
  Public:  ... → 100 → 101 → 102
  Secret:  ... → 100 → 100' → 101' → 102'

  Attacker broadcasts secret chain
  Secret chain has more work (3 blocks vs 2)
  Network reorganizes to secret chain
  Blocks 101, 102 orphaned
  Attacker keeps all rewards from 100', 101', 102'

Scenario B: Secret chain only 1 ahead
  Public:  ... → 100 → 101
  Secret:  ... → 100 → 100'

  Honest miner about to find 102:
  Attacker immediately broadcasts 100'
  Network splits temporarily
  Attacker mines on 100', honest on 101
  Whoever finds next block wins
  ~50/50 split benefits attacker
```

5. **Profit Calculation**
```
Traditional honest mining (30% hashrate):
  Expected reward: 30% of blocks

Selfish mining strategy:
  - Wastes honest miners' work
  - Causes reorganizations
  - Gets disproportionate rewards
  Expected reward: ~33-35% of blocks

Extra profit: +3-5% blocks at others' expense
```

### Why It Works
- No penalty for withholding blocks
- No timestamp requirements between broadcasts
- Network accepts longest chain blindly
- Honest miners waste work on orphaned blocks

### Impact
- Unfair reward distribution
- Centralization pressure
- 51% attack easier to achieve
- Trust in fairness damaged

### Mitigation Difficulty
- Hard to prevent without protocol changes
- Requires network-wide coordination
- Consider uncle block rewards (Ethereum-style)
- Implement timestamp freshness requirements

---

## Scenario 7: The Timestamp Manipulation Attack (MEDIUM)

**Vulnerability:** Weak timestamp validation
**Difficulty:** MODERATE
**Impact:** Difficulty manipulation, block withholding

### Attack Steps

1. **Setup**
```
Attacker controls 40% hashrate
Current difficulty: 1000
Goal: Manipulate difficulty downward
```

2. **Phase 1: Set Future Timestamps**
```python
import time

current_time = time.time()
future_time = current_time + 7200  # MAX_FUTURE (2 hours)

for block in range(1, 2016):
    block.timestamp = future_time
    mine_block(block)
    broadcast_block(block)

# All blocks have timestamp 2 hours in future
# But within MAX_FUTURE_BLOCK_TIME, so accepted
```

3. **Phase 2: Adjustment Block**
```
Block 2016:
  timestamp = first_block.timestamp + 60 seconds

Calculation:
  first = Block 1 (Jan 1, 2025 14:00:00)
  last = Block 2016 (Jan 1, 2025 14:01:00)
  timespan = 60 seconds (!!!)

Expected = 1,209,600 seconds
Actual = 60 seconds
Ratio = 60 / 1,209,600 = 0.000049

Clamped to MIN_DIFFICULTY_ADJUSTMENT = 0.25
New difficulty = 1000 * 0.25 = 250
```

4. **Outcome**
```
Difficulty reduced to 25% in one adjustment
Attacker now mines 4x faster
40% hashrate becomes effective 160% (dominates network)
Can now execute 51% attacks
```

### Why It Works
- MAX_FUTURE_BLOCK_TIME allows 2-hour jumps
- MIN_BLOCK_TIME only requires 1-minute spacing
- No median-time-past validation
- Timestamps under miner control

### Impact
- Artificial difficulty reduction
- Centralization
- Enables 51% attacks
- Network instability

### Mitigation
Median-time-past + stricter timestamp rules (see Fix #3)

---

## Scenario 8: The TOCTOU Bug (MEDIUM)

**Vulnerability:** Time-of-check-time-of-use in validation
**Difficulty:** HARD (requires precise timing)
**Impact:** Double-spend, consensus split

### Attack Steps

1. **Setup**
```rust
// Two threads validating same block simultaneously
// Thread A: validate_block_context()
// Thread B: modify blockchain state
```

2. **Timing Sequence**
```
Time T0:
  Thread A: let blockchain_db = self.blockchain_db.read().unwrap();
  Thread A: let prev_block = blockchain_db.get_block(&prev_hash)?;
  [LOCK RELEASED]

Time T1:
  Thread B: let mut blockchain_db = self.blockchain_db.write().unwrap();
  Thread B: blockchain_db.add_block(competing_block)?;
  Thread B: blockchain_db.update_chain_tip(new_tip)?;
  [LOCK RELEASED]

Time T2:
  Thread A: // Using stale prev_block from T0
  Thread A: if block.timestamp <= prev_block.timestamp { error }
  Thread A: // prev_block is old - new block might be valid
  Thread A: validate_difficulty(block, &prev_block);  // Wrong!
```

3. **Exploit**
```
Result:
  - Thread A validates against old state
  - Thread B changed blockchain state
  - Validation inconsistent
  - Block accepted that should be rejected
  - Or: Block rejected that should be accepted

Leads to:
  - Different nodes accept different blocks
  - Consensus split
  - Network fragmentation
```

### Why It Works
- Locks released between check and use
- No atomic validation sequence
- Concurrent modifications allowed
- Stale data used for critical decisions

### Impact
- Consensus instability
- Double-spend opportunities
- Network splits
- Unreliable validation

### Mitigation
Hold locks for entire validation (see Fix #5)

---

## Defense Recommendations

### Immediate (Week 1)
1. ✅ Signature verification - Blocks scenario #1
2. ✅ Constant-time hash comparison - Prevents side-channels
3. ✅ Median-time-past - Blocks scenarios #2, #7
4. ✅ Checked arithmetic - Blocks scenario #3

### Short-term (Month 1)
5. ✅ Atomic operations - Blocks scenarios #4, #8
6. ✅ Replay protection - Blocks scenario #5
7. ✅ Nonce exhaustion handling
8. ✅ Strict difficulty validation - Mitigates scenario #7

### Long-term (Month 2+)
- Comprehensive fuzzing
- Game theory analysis (scenario #6)
- Economic incentive modeling
- Formal verification
- External security audit

---

## Responsible Disclosure

**DO NOT:**
- ❌ Publicly disclose these scenarios before fixes
- ❌ Attempt attacks on mainnet (if deployed)
- ❌ Share with malicious actors

**DO:**
- ✅ Report new attack vectors to development team
- ✅ Help test fixes
- ✅ Participate in bug bounty (when available)

---

**Last Updated:** 2025-10-11
**Classification:** INTERNAL - Security Sensitive
**Disclosure:** After all critical fixes deployed