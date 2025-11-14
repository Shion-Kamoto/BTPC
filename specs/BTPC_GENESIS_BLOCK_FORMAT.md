# BTPC Genesis Block Format

> ⚠️ **IMPORTANT NOTE**: The genesis block message shown in this specification is a placeholder for development purposes. The actual mainnet genesis message will be updated with a relevant news headline before official network launch.

## Overview
BTPC uses a Bitcoin-compatible block structure with quantum-resistant cryptography. The key differences are:
- **Hash Algorithm**: SHA-512 (64 bytes) instead of SHA-256 (32 bytes)
- **Signatures**: ML-DSA (Dilithium5) instead of ECDSA
- **Header Size**: 144 bytes instead of 80 bytes
- **Genesis Reward**: 32.375 BTPC (3,237,500,000 credits)
- **Smallest Unit**: 1 credit = 0.00000001 BTPC

## Mainnet Genesis Block Data

```
GetHash()      = 0x[to be calculated after mining]
hashMerkleRoot = 0x[calculated from coinbase transaction]

txNew.vin[0].scriptSig     = [timestamp(8)] + [difficulty_target(32)] + [message_length(1)] + "The Times 2025/01/01 Bitcoin Testnet Post-Quantum Chain Launch"
txNew.vout[0].nValue       = 3237500000  (32.37500000 BTPC)
txNew.vout[0].scriptPubKey = OP_DUP OP_HASH160 0x0000000000000000000000000000000000000000 OP_EQUALVERIFY OP_CHECKSIG

block.nVersion = 1
block.nTime    = 1735689600  (2025-01-01 00:00:00 UTC)
block.nBits    = 0x1d00ffff  (mainnet difficulty)
block.nNonce   = [to be determined during mining]
```

## Block Structure

```
CBlock(
    hash=[64-byte SHA-512 hash],
    ver=1,
    hashPrevBlock=0000000000000000000000000000000000000000000000000000000000000000
                  0000000000000000000000000000000000000000000000000000000000000000,
    hashMerkleRoot=[64-byte SHA-512 merkle root],
    nTime=1735689600,
    nBits=0x1d00ffff,
    nNonce=[TBD],
    vtx=1
)
  CTransaction(
      hash=[64-byte SHA-512 transaction hash],
      ver=1,
      vin.size=1,
      vout.size=1,
      nLockTime=0
  )
    CTxIn(
        COutPoint(0000...0000, 0xFFFFFFFF),
        coinbase=[timestamp + target + message]
    )
    CTxOut(
        nValue=32.37500000,
        scriptPubKey=OP_DUP OP_HASH160 0x00000000... OP_EQUALVERIFY OP_CHECKSIG
    )
  vMerkleTree: [64-byte hash]
```

## Detailed Field Descriptions

### Block Header (144 bytes total)

| Field | Size | Type | Value | Description |
|-------|------|------|-------|-------------|
| version | 4 bytes | u32 (LE) | 1 | Block version |
| prev_hash | 64 bytes | SHA-512 | 0x000...000 | Previous block hash (all zeros for genesis) |
| merkle_root | 64 bytes | SHA-512 | [calculated] | Merkle root of transactions |
| timestamp | 4 bytes | u32 (LE) | 1735689600 | Unix timestamp (2025-01-01 00:00:00 UTC) |
| bits | 4 bytes | u32 (LE) | 0x1d00ffff | Difficulty target (compact format) |
| nonce | 4 bytes | u32 (LE) | [TBD] | Proof-of-work nonce |

**Note**: Internal timestamp is u64 but serialized as u32 for Bitcoin compatibility

### Coinbase Transaction

#### Input (scriptSig structure):
```
[8 bytes]  - timestamp (u64, little-endian): 1735689600
[32 bytes] - difficulty target (first 32 bytes of 64-byte target)
[1 byte]   - message length: 71 (0x47)
[71 bytes] - message: "The Times 2025/01/01 Bitcoin Testnet Post-Quantum Chain Launch"
```

**Total scriptSig size**: 112 bytes

#### Output (scriptPubKey structure):
```
[1 byte]   - OP_DUP (0x76)
[1 byte]   - OP_HASH160 (0xa9)
[1 byte]   - pubkey_hash length (20)
[20 bytes] - pubkey_hash (0x0000000000000000000000000000000000000000 - burn address)
[1 byte]   - OP_EQUALVERIFY (0x88)
[1 byte]   - OP_CHECKSIG (0xac)
```

**Total scriptPubKey size**: 25 bytes

### Transaction Serialization

```
[4 bytes]     - version (1)
[varint]      - input count (1)
  [64 bytes]  - previous tx hash (0x000...000)
  [4 bytes]   - previous output index (0xFFFFFFFF for coinbase)
  [varint]    - scriptSig length (112)
  [112 bytes] - scriptSig data
  [4 bytes]   - sequence (0xFFFFFFFF)
[varint]      - output count (1)
  [8 bytes]   - value (3237500000 = 32.375 BTPC)
  [varint]    - scriptPubKey length (25)
  [25 bytes]  - scriptPubKey data
[4 bytes]     - lock_time (0)
```

## Key Differences from Bitcoin

### 1. Hash Size (SHA-512 vs SHA-256)
- **Bitcoin**: 32-byte hashes (SHA-256)
- **BTPC**: 64-byte hashes (SHA-512)
- **Impact**: Block headers are 144 bytes vs Bitcoin's 80 bytes

### 2. Signature Algorithm
- **Bitcoin**: ECDSA with secp256k1 (compact signatures)
- **BTPC**: ML-DSA (Dilithium5) - quantum-resistant
- **Impact**: Larger transaction signatures (post-quantum security)

### 3. Coinbase Message
- **Bitcoin**: "The Times 03/Jan/2009 Chancellor on brink of second bailout for banks"
- **BTPC**: "The Times 2025/01/01 Bitcoin Testnet Post-Quantum Chain Launch"

### 4. Genesis Reward
- **Bitcoin**: 50 BTC (5,000,000,000 satoshis)
- **BTPC**: 32.375 BTPC (3,237,500,000 credits)

### 5. Timestamp
- **Bitcoin**: 1231006505 (January 3, 2009 18:15:05 GMT)
- **BTPC**: 1735689600 (January 1, 2025 00:00:00 UTC)

## Network Variants

### Mainnet
```
timestamp: 1735689600
message: "The Times 2025/01/01 Bitcoin Testnet Post-Quantum Chain Launch"
        **NOTE: This message will be updated before official mainnet launch**
bits: 0x1d00ffff (standard Bitcoin mainnet difficulty)
reward: 3237500000 (32.375 BTPC)
```

### Testnet
```
timestamp: 1735689600 (same as mainnet for deterministic testing)
message: "BTPC Testnet Genesis Block - Post-Quantum Bitcoin"
bits: 0x1d00ffff
reward: 3237500000 (32.375 BTPC)
```

### Regtest
```
timestamp: [current time]
message: "BTPC Regtest Genesis Block"
bits: 0x207fffff (very easy difficulty for development)
reward: 3237500000 (32.375 BTPC)
```

## Mining the Genesis Block

The genesis block must be mined by finding a nonce value such that:
```
double_sha512(block_header) <= target
```

Where:
- `double_sha512(data) = sha512(sha512(data))`
- `target` is derived from the `bits` field (compact representation)

### Mining Process
1. Serialize the block header (144 bytes)
2. Compute `hash = sha512(sha512(header))`
3. Compare hash against difficulty target
4. If hash > target, increment nonce and repeat
5. If hash <= target, genesis block is found

### Expected Mining Time
- **Mainnet** (0x1d00ffff): ~10 minutes to hours (depending on hardware)
- **Testnet** (0x1d00ffff): Same as mainnet
- **Regtest** (0x207fffff): Instant to seconds

## Verification Checklist

To verify a BTPC genesis block:

- [ ] Block header is exactly 144 bytes
- [ ] Previous hash is all zeros (64 bytes)
- [ ] Timestamp matches network configuration
- [ ] Bits field matches expected difficulty
- [ ] Contains exactly one transaction (coinbase)
- [ ] Coinbase input references null outpoint (txid=0x000...000, vout=0xFFFFFFFF)
- [ ] Coinbase output value is 3,237,500,000 credits (32.375 BTPC)
- [ ] Merkle root matches calculated merkle root of coinbase tx
- [ ] Block hash meets difficulty target (double SHA-512)
- [ ] ScriptSig contains timestamp, difficulty, and message
- [ ] ScriptPubKey is valid P2PKH format (burn address for genesis)

## Code References

Implementation files:
- Genesis logic: `btpc-core/src/blockchain/genesis.rs`
- Block structure: `btpc-core/src/blockchain/block.rs`
- Transaction structure: `btpc-core/src/blockchain/transaction.rs`
- Hash functions: `btpc-core/src/crypto/hash.rs`
- Mining tool: `bins/genesis_tool/`

## Example Usage

### Creating Genesis Block (Rust)
```rust
use btpc_core::blockchain::genesis::{GenesisConfig, GenesisCreator};

// Create mainnet genesis configuration
let config = GenesisConfig::mainnet();
let creator = GenesisCreator::new(config);

// Create and mine genesis block
let genesis_block = creator.mine_genesis_block();

println!("Genesis Hash: {}", hex::encode(genesis_block.hash().as_bytes()));
println!("Merkle Root: {}", hex::encode(genesis_block.header.merkle_root.as_bytes()));
println!("Nonce: {}", genesis_block.header.nonce);
```

### Validating Genesis Block
```rust
use btpc_core::blockchain::genesis::{GenesisConfig, GenesisCreator};

let config = GenesisConfig::mainnet();
let creator = GenesisCreator::new(config);

// Validate a genesis block
match creator.validate_genesis_block(&block) {
    Ok(_) => println!("✓ Valid genesis block"),
    Err(e) => println!("✗ Invalid: {}", e),
}
```

## Binary Tools

### Generate Genesis Block
```bash
# Build genesis tool
cargo build --release --bin genesis_tool

# Generate mainnet genesis
./target/release/genesis_tool --network mainnet --output genesis-mainnet.json

# Generate testnet genesis
./target/release/genesis_tool --network testnet --output genesis-testnet.json

# Generate regtest genesis
./target/release/genesis_tool --network regtest --output genesis-regtest.json
```

### Export Genesis Block
```bash
# Export as JSON
./target/release/genesis_tool --export genesis-mainnet.json

# Export as hex
./target/release/genesis_tool --export-hex genesis-mainnet.hex

# Verify genesis block
./target/release/genesis_tool --verify genesis-mainnet.json
```

## Security Considerations

1. **Burn Address**: The genesis coinbase output uses a burn address (all zeros), making the genesis reward unspendable. This is intentional and matches common practice.

2. **Deterministic Generation**: The genesis block must be deterministic for all nodes to agree on the chain start. Only the nonce varies during mining.

3. **Timestamp Validation**: Nodes must validate that the genesis timestamp matches the expected value for the network.

4. **Hash Verification**: The genesis block hash must meet the difficulty target and be hardcoded into the software for verification.

5. **Message Verification**: The coinbase message serves as proof of the chain's creation time and cannot be altered without changing the entire block hash.

## Future Considerations

- **ML-DSA Integration**: Future versions may include ML-DSA signatures in the genesis transaction for additional verification
- **Multi-signature Genesis**: Potential for multi-party genesis creation with multiple public keys
- **Genesis State**: Possible pre-allocation of funds to specific addresses (not currently implemented)

## Appendix: Hexadecimal Format

The genesis block header in hexadecimal format (144 bytes):
```
[version: 4 bytes]
01 00 00 00

[prev_hash: 64 bytes]
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00

[merkle_root: 64 bytes - calculated]
[...merkle root bytes...]

[timestamp: 4 bytes]
00 70 6d 67  (1735689600 in little-endian)

[bits: 4 bytes]
ff ff 00 1d  (0x1d00ffff in little-endian)

[nonce: 4 bytes]
[...nonce bytes - to be determined...]
```

---

**Document Version**: 1.0
**Last Updated**: 2025-10-07
**Status**: Specification Complete - Pending Mining