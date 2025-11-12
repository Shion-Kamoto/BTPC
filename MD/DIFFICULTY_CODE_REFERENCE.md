# BTPC Difficulty System - Code Reference Guide

## File Locations & Code Sections

### 1. Difficulty Target Calculation
**File**: `/home/bob/BTPC/BTPC/btpc-core/src/consensus/difficulty.rs`

#### Special Case: 0x1d000000 (lines 272-296)
```rust
// Bitcoin compact bits: 0x1d000000
// Exponent: 0x1d = 29
// Mantissa: 0x000000 = 0
// Target = 0 means minimum non-zero value at position
// For 64-byte SHA-512 hash, position from left = 64 - 29 = 35

if bits == 0x1d000000 {
    let byte_pos = 64usize.saturating_sub(29);  // = 35
    if byte_pos < 64 {
        // First 35 bytes must be 0x00
        for i in 0..byte_pos {
            target[i] = 0x00;
        }
        // Byte 35 gets minimal non-zero value
        target[byte_pos] = 0x01;
        // Remaining bytes can be 0xff
        for i in (byte_pos + 1)..64 {
            target[i] = 0xff;
        }
    }
    return target;
}
```

**Result**: `[00...00 (×35), 01, ff...ff (×28)]`
**Requires**: First 35 bytes = 0x00 (280 leading zero bits)
**Probability**: ~1 in 2^280 (essentially impossible)

#### Special Case: 0x1d00ffff (lines 323-333)
```rust
// Special case for mainnet minimum difficulty 0x1d00ffff
// Exponent: 0x1d = 29
// Mantissa: 0x00ffff

if bits == 0x1d00ffff {
    target[0] = 0x00;
    target[1] = 0xff;
    target[2] = 0xff;
    for i in 3..64 {
        target[i] = 0xff;
    }
    return target;
}
```

**Result**: `[00, ff, ff, ff...ff (×61)]`
**Requires**: First byte = 0x00, rest can be up to 0xff
**Probability**: ~1 in 256 (very easy)

#### Special Case: 0x1d00000f (lines 298-310)
```rust
// Legacy regtest minimum difficulty 0x1d00000f (very easy, instant mining)
if bits == 0x1d008fff {
    // Fill with 0xff (easy target - high values are easier to meet)
    for i in 0..64 {
        target[i] = 0xff;
    }
    // Place the mantissa bytes at the calculated position
    // Position 35 onwards (64 - 29 = 35)
    target[35] = 0x00;
    target[36] = 0x8f;
    target[37] = 0xff;
    return target;
}
```

#### General Formula (lines 335-360)
```rust
// Bitcoin compact format: mantissa * 256^(exponent-3)
// For SHA-512 (64 bytes), we need to map this correctly

let exponent = (bits >> 24) as i32;
let mantissa = bits & 0x00ffffff;

// The exponent tells us how many bytes the number occupies
let position = exponent - 3;

if position >= 0 && (position as usize) < 64 {
    let start_pos = (64 - exponent) as usize;
    
    if start_pos < 64 {
        // Place mantissa bytes (big-endian)
        target[start_pos] = ((mantissa >> 16) & 0xff) as u8;
        if start_pos + 1 < 64 {
            target[start_pos + 1] = ((mantissa >> 8) & 0xff) as u8;
        }
        if start_pos + 2 < 64 {
            target[start_pos + 2] = (mantissa & 0xff) as u8;
        }
    }
}
```

### 2. Hash Validation
**File**: `/home/bob/BTPC/BTPC/btpc-core/src/consensus/difficulty.rs` (lines 154-157)

```rust
/// Check if a hash meets this difficulty target
pub fn validates_hash(&self, hash: &Hash) -> bool {
    hash.as_bytes() <= &self.target  // Byte-by-byte comparison
}
```

**Logic**: Hash is valid if `hash <= target` (treating both as big-endian byte arrays)

### 3. Network Minimum Difficulties
**File**: `/home/bob/BTPC/BTPC/btpc-core/src/consensus/difficulty.rs` (lines 463-470)

```rust
pub fn minimum_for_network(network: crate::Network) -> Self {
    match network {
        crate::Network::Mainnet => 
            DifficultyTarget::from_bits(0x1d00ffff),
        crate::Network::Testnet => 
            DifficultyTarget::from_bits(0x1d0fffff), // Different!
        crate::Network::Regtest => 
            DifficultyTarget::from_bits(0x1d00000f), // Also Different!
    }
}
```

### 4. Consensus Parameters
**File**: `/home/bob/BTPC/BTPC/btpc-core/src/consensus/mod.rs` (lines 98-134)

#### Mainnet (lines 98-108)
```rust
pub fn mainnet() -> Self {
    ConsensusParams {
        network: Network::Mainnet,
        genesis_hash: Hash::zero(),
        min_difficulty_target: DifficultyTarget::from_bits(0x1d00ffff),
        max_difficulty_target: DifficultyTarget::from_bits(0x207fffff),
        allow_min_difficulty_blocks: false,
        pow_limit: Self::mainnet_pow_limit(),
        reward_params: RewardParams::mainnet(),
    }
}
```

#### Testnet (lines 111-121)
```rust
pub fn testnet() -> Self {
    ConsensusParams {
        network: Network::Testnet,
        genesis_hash: Hash::zero(),
        min_difficulty_target: DifficultyTarget::from_bits(0x1d000000),
        max_difficulty_target: DifficultyTarget::from_bits(0x207fffff),
        allow_min_difficulty_blocks: true,
        pow_limit: Self::testnet_pow_limit(),
        reward_params: RewardParams::testnet(),
    }
}
```

#### Regtest (lines 124-134)
```rust
pub fn regtest() -> Self {
    ConsensusParams {
        network: Network::Regtest,
        genesis_hash: Hash::zero(),
        min_difficulty_target: DifficultyTarget::from_bits(0x1d000000),
        max_difficulty_target: DifficultyTarget::from_bits(0x207fffff),
        allow_min_difficulty_blocks: true,
        pow_limit: Self::regtest_pow_limit(),
        reward_params: RewardParams::regtest(),
    }
}
```

**CONFLICT**: 
- `consensus/mod.rs` says regtest uses `0x1d000000`
- `consensus/difficulty.rs:468` says regtest uses `0x1d00000f`
- Both are impossible anyway!

### 5. Mining Implementation
**File**: `/home/bob/BTPC/BTPC/btpc-core/src/consensus/pow.rs` (lines 55-96)

```rust
pub fn mine(
    header: &crate::blockchain::BlockHeader,
    target: &MiningTarget,
) -> Result<Self, PoWError> {
    use rand::Rng;
    
    let mut mining_header = header.clone();
    let start_nonce = rand::thread_rng().gen::<u32>();
    let mut nonce = start_nonce;
    
    // Try different nonce values until we find a valid hash
    loop {
        mining_header.nonce = nonce;
        let hash = mining_header.hash();
        
        if hash.meets_target(&target.as_hash()) {  // LINE 73: Hash comparison
            return Ok(ProofOfWork {
                nonce: nonce as u64,
            });
        }
        
        nonce = nonce.wrapping_add(1);
        
        // If we've wrapped around back to start, we've exhausted all nonces
        if nonce == start_nonce {
            break;
        }
    }
    
    // Exhausted all 4 billion nonces without finding valid proof
    Err(PoWError::NonceExhausted)
}
```

**Problem**: With 0x1d000000/0x1d00000f, the loop exhausts 4 billion nonces without finding valid hash

### 6. Test Cases
**File**: `/home/bob/BTPC/BTPC/btpc-core/tests/pow_validation.rs` (lines 69-93)

```rust
#[test]
fn test_pow_difficulty_target_contract() {
    let target_easy = DifficultyTarget::from_bits(0x207fffff);
    let target_hard = DifficultyTarget::from_bits(0x1d00ffff);
    
    // Easy target should have larger numeric value (easier to meet)
    assert!(
        target_easy.is_easier_than(&target_hard),
        "Easy target must be easier than hard target"
    );
    
    // Test that targets can validate hashes correctly
    let easy_hash = Hash::from_hex(
        "0000ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    ).expect("Valid easy hash");
    
    assert!(
        target_easy.validates_hash(&easy_hash),
        "Easy target should validate easy hash"
    );
    assert!(
        !target_hard.validates_hash(&easy_hash),
        "Hard target should not validate easy hash"
    );
}
```

## Compact Bits Decoding Reference

### Format: 0xAABBCCDD
- **AA** = Exponent (decimal: subtract 0x10 from hex for rough byte count)
- **BBCCDD** = Mantissa (significand)

### Examples:

| Bits | Exponent | Mantissa | Target | Notes |
|------|----------|----------|--------|-------|
| 0x207fffff | 0x20 (32) | 0x7fffff | [7f, ff, ff...] | Easiest (test) |
| 0x1d00ffff | 0x1d (29) | 0x00ffff | [00, ff, ff...] | Bitcoin mainnet |
| 0x1d008000 | 0x1d (29) | 0x008000 | [00, 80, 00...] | **Recommended** |
| 0x1d001000 | 0x1d (29) | 0x001000 | [00, 10, 00...] | Harder variant |
| 0x1d0fffff | 0x1d (29) | 0x0fffff | [0f, ff, ff...] | Test (legacy) |
| 0x1d000000 | 0x1d (29) | 0x000000 | [00, 00...35x, 01, ff...] | **IMPOSSIBLE** |
| 0x1d00000f | 0x1d (29) | 0x00000f | [00, 00...35x, 01, ff...] | **IMPOSSIBLE** |

## Difficulty Progression (Within 0x1d Range)

To find working 5-minute difficulty, test this sequence:

```
Start: 0x1d00ffff (blocks per second - reference point)
Try:   0x1d00ff00 (reduce byte[1] from ff to ff, byte[2] from ff to 00)
       0x1d00fe00
       0x1d00fd00
       ...
       0x1d00a000
       0x1d008000  ← Recommended starting point
       0x1d004000
       0x1d002000
Stop:  When reaching ~5 minute block times
```

Each step reduces target by ~1 byte value (small change).

