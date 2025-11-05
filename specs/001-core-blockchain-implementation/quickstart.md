# Quickstart Guide: BTPC Core Blockchain Implementation

**Feature**: Core Blockchain Implementation
**Branch**: `001-core-blockchain-implementation`
**Phase**: 1 - Design & Contracts
**Generated**: 2025-09-30

## Overview

This quickstart guide provides test scenarios for validating the BTPC quantum-resistant blockchain implementation. All scenarios are derived from the feature specification user stories and serve as integration test validation steps.

## Prerequisites

- Rust 1.75+ development environment
- BTPC core blockchain implementation
- Test network configuration
- ML-DSA signature verification capability

## Test Scenarios

### Scenario 1: Genesis Block Creation and Network Bootstrap

**Objective**: Validate genesis block creation with quantum-resistant parameters

**Steps**:
1. Initialize new BTPC network in regtest mode
2. Generate genesis block with 32.375 BTPC initial reward
3. Verify SHA-512 hash validation and ML-DSA signature structure
4. Confirm network parameters match constitutional requirements

**Expected Results**:
- Genesis block hash follows SHA-512 format (64-byte hash)
- Initial block reward equals 32.375 BTPC (3,237,500,000 base units)
- Block timestamp within acceptable variance
- Network accepts genesis block as chain start

**Validation Commands**:
```bash
# Start regtest network
btpc_node --network=regtest --generate-genesis

# Verify genesis block
btpc_cli getblock 0
btpc_cli getblockchaininfo

# Expected: height=0, reward=32.375, hash=SHA-512
```

### Scenario 2: ML-DSA Transaction Creation and Validation

**Objective**: Validate quantum-resistant transaction flow with ML-DSA signatures

**Steps**:
1. Generate ML-DSA-65 key pair for test wallet
2. Create transaction spending genesis block reward
3. Sign transaction with ML-DSA private key
4. Broadcast transaction to network
5. Verify signature validation within 1.5ms requirement

**Expected Results**:
- ML-DSA public key: 1,952 bytes (3,904 hex characters)
- ML-DSA signature: 3,309 bytes (6,618 hex characters)
- Signature verification time < 1.5ms
- Transaction accepted by network nodes

**Validation Commands**:
```bash
# Generate wallet and address
btpc_wallet create --name=test --network=regtest
btpc_wallet address new --name=test --label="Genesis Recipient"

# Create and sign transaction
btpc_wallet transaction create --to=<address> --amount=1.0 --fee=0.001
btpc_wallet transaction sign --hex=<unsigned_tx> --passphrase=<pass>

# Broadcast and verify
btpc_cli sendrawtransaction <signed_tx>
btpc_cli getrawtransaction <txid> true
```

### Scenario 3: Linear Decay Reward Calculation

**Objective**: Validate linear decay economics over multiple blocks

**Steps**:
1. Mine first 100 blocks in regtest mode
2. Calculate expected rewards using linear decay formula
3. Verify actual block rewards match mathematical precision
4. Test reward calculation at key milestones (year 1, year 12, year 24)

**Expected Results**:
- Block 1 reward: 32.375 BTPC
- Block 52,560 (year 1): ~31.025 BTPC
- Block 630,720 (year 12): ~16.1875 BTPC
- Block 1,261,440 (year 24): ~0.5 BTPC (tail emission)
- Mathematical precision to 8 decimal places

**Validation Commands**:
```bash
# Generate blocks and check rewards
for i in {1..100}; do
  btpc_cli generate 1
  reward=$(btpc_cli getblock $(btpc_cli getblockhash $i) | jq .reward)
  echo "Block $i: $reward BTPC"
done

# Verify linear decay formula
btpc_cli validaterewards --start=1 --end=1261440 --sample=1000
```

### Scenario 4: Quantum Computer Attack Simulation

**Objective**: Demonstrate ML-DSA signature security against quantum attacks

**Steps**:
1. Create test transaction with ML-DSA signature
2. Attempt signature forgery using classical methods
3. Simulate quantum algorithm attacks (Shor's algorithm simulation)
4. Verify ML-DSA signatures remain secure

**Expected Results**:
- Classical signature forgery attempts fail
- Quantum simulation cannot break ML-DSA-65 signatures
- Transaction authorization requires valid private key
- No false positive signature validations

**Validation Commands**:
```bash
# Create test transaction
btpc_wallet transaction create --to=<addr> --amount=1.0

# Attempt forgery simulation
btpc_test_harness quantum-attack-sim --signature=<sig> --pubkey=<pub>

# Expected: All forgery attempts fail
btpc_test_harness classical-attack-sim --signature=<sig> --pubkey=<pub>
```

### Scenario 5: Network Hashrate Drop and Recovery

**Objective**: Validate difficulty adjustment during network stress

**Steps**:
1. Establish stable mining with normal hashrate
2. Simulate 75% hashrate drop over 1000 blocks
3. Wait for next difficulty adjustment (block 2016)
4. Verify difficulty reduces proportionally
5. Confirm 10-minute block time restoration

**Expected Results**:
- Initial block time: ~10 minutes average
- During hashrate drop: ~40 minutes average
- After difficulty adjustment: ~10 minutes restored
- Adjustment factor within constitutional ±15% variance

**Validation Commands**:
```bash
# Monitor initial state
btpc_cli getblockchaininfo | jq .difficulty

# Simulate hashrate drop
btpc_miner_sim --hashrate-factor=0.25 --blocks=1000

# Check adjustment at block 2016
btpc_cli getblock $(btpc_cli getblockhash 2016) | jq .difficulty
btpc_cli validatetimings --start=2016 --end=2100
```

### Scenario 6: Invalid Signature Rejection

**Objective**: Verify network properly rejects invalid ML-DSA signatures

**Steps**:
1. Create valid transaction with correct ML-DSA signature
2. Modify signature bytes to create invalid signature
3. Attempt to broadcast invalid transaction
4. Verify network rejection with specific error codes
5. Confirm no propagation to peer nodes

**Expected Results**:
- Valid transaction accepted and propagated
- Invalid signature transaction rejected immediately
- Error code indicates signature validation failure
- No network propagation of invalid transaction
- Block validation within 10ms requirement

**Validation Commands**:
```bash
# Create valid transaction
valid_tx=$(btpc_wallet transaction create --to=<addr> --amount=1.0)

# Create invalid signature version
invalid_tx=$(echo $valid_tx | btpc_test_harness corrupt-signature)

# Attempt broadcast
btpc_cli sendrawtransaction $invalid_tx
# Expected: Error - Invalid ML-DSA signature

# Verify no propagation
btpc_cli getrawmempool | grep $(btpc_test_harness get-txid $invalid_tx)
# Expected: Not found
```

### Scenario 7: Multi-Network Mode Support

**Objective**: Validate mainnet, testnet, and regtest compatibility

**Steps**:
1. Start nodes in each network mode (mainnet, testnet, regtest)
2. Verify distinct genesis blocks and parameters
3. Test cross-network transaction rejection
4. Confirm network isolation and proper identification

**Expected Results**:
- Each network has unique genesis block hash
- Mainnet/testnet/regtest transactions are isolated
- Network identification headers prevent cross-contamination
- P2P protocol correctly identifies network type

**Validation Commands**:
```bash
# Start different networks
btpc_node --network=mainnet --datadir=./mainnet &
btpc_node --network=testnet --datadir=./testnet &
btpc_node --network=regtest --datadir=./regtest &

# Verify isolation
btpc_cli --network=mainnet getblockchaininfo | jq .chain
btpc_cli --network=testnet getblockchaininfo | jq .chain
btpc_cli --network=regtest getblockchaininfo | jq .chain

# Test cross-network rejection
testnet_tx=$(btpc_cli --network=testnet createrawtransaction [...])
btpc_cli --network=mainnet sendrawtransaction $testnet_tx
# Expected: Network mismatch error
```

## Performance Validation

### Signature Performance Test
```bash
# Test ML-DSA signature generation time
btpc_benchmark signature-generation --count=1000 --target=2ms

# Test ML-DSA signature verification time
btpc_benchmark signature-verification --count=1000 --target=1.5ms
```

### Block Validation Performance Test
```bash
# Test block validation time for 1MB blocks
btpc_benchmark block-validation --size=1MB --target=10ms --iterations=100
```

### Throughput Test
```bash
# Test transaction throughput
btpc_benchmark throughput --target=1000tps --duration=60s
```

### RPC Latency Test
```bash
# Test RPC response times
btpc_benchmark rpc-latency --target=200ms --methods=all --iterations=1000
```

## Integration Test Suite

### Automated Test Execution
```bash
# Run complete test suite
cargo test --release --workspace -- --test-threads=1

# Run specific test categories
cargo test blockchain_validation
cargo test mldsa_signature_tests
cargo test linear_decay_economics
cargo test difficulty_adjustment
cargo test network_protocol

# Performance regression tests
cargo bench --workspace
```

### Continuous Integration Validation
```bash
# Constitution compliance check
btpc_test_harness constitutional-compliance

# Security audit simulation
btpc_test_harness security-audit --mldsa-validation --timing-attacks

# Memory safety validation
cargo miri test --workspace
```

## Expected Outcomes

### Success Criteria
- ✅ All genesis block parameters match constitutional requirements
- ✅ ML-DSA signatures generated and verified within performance targets
- ✅ Linear decay rewards calculated with mathematical precision
- ✅ Network properly handles hashrate variations and difficulty adjustment
- ✅ Invalid signatures rejected with appropriate error handling
- ✅ Multi-network isolation and compatibility verified
- ✅ Performance benchmarks meet constitutional standards

### Performance Targets Met
- ✅ Signature generation: <2ms
- ✅ Signature verification: <1.5ms
- ✅ Block validation: <10ms per 1MB block
- ✅ Transaction throughput: >1000 TPS
- ✅ RPC latency: <200ms p95

### Constitutional Compliance
- ✅ Quantum-resistant ML-DSA signatures throughout
- ✅ SHA-512 proof-of-work implementation
- ✅ Linear decay economics (not halving)
- ✅ Bitcoin-compatible structure maintained
- ✅ Rust memory safety principles followed
- ✅ >90% test coverage achieved

---

**Status**: ✅ Complete - All test scenarios defined
**Next**: Update agent context file and finalize Phase 1