# CRITICAL: BIP39 Seed Phrase Recovery Is Broken

**Date**: 2025-11-06
**Severity**: 🔴 **CRITICAL SECURITY ISSUE**
**Status**: ❌ **BROKEN - WALLET RECOVERY WILL FAIL**

---

## Executive Summary

**BTPC displays 24-word BIP39 seed phrases to users for wallet recovery, but the underlying ML-DSA key generation is NOT deterministic.** This means:

- ✅ User writes down 24-word seed phrase
- ❌ User tries to recover wallet with same seed phrase
- ❌ **Different keys are generated** → **Cannot access funds**

This is a **showstopper bug** that breaks the fundamental promise of wallet recovery.

---

## Current Implementation Analysis

### 1. Frontend Promises Seed Phrase Recovery

**File**: `btpc-desktop-app/ui/wallet-manager.html:362`
```html
<h3>Recovery Seed Phrase (24 words)</h3>
<div id="recovery-seed-phrase">...</div>
<button>Copy Seed Phrase</button>
```

**User Experience**:
1. User creates wallet
2. UI displays 24-word BIP39 mnemonic
3. User writes it down for "recovery"
4. User believes this seed will restore their wallet

### 2. Backend Implementation Is NON-Deterministic

**File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs:549-642`

```rust
pub async fn import_wallet_from_mnemonic(...) {
    // Parse BIP39 mnemonic (✅ works correctly)
    let mnemonic = bip39::Mnemonic::parse_in_normalized(...)?;

    // Derive seed from mnemonic (✅ works correctly - BIP39 standard)
    let seed = mnemonic.to_seed("");

    // Hash seed to 32 bytes (✅ deterministic)
    let mut hasher = Sha512::new();
    hasher.update(&seed[..32]);
    let key_material = hasher.finalize()[..32];

    // ❌ PROBLEM: Generate ML-DSA key from seed
    let private_key = PrivateKey::from_seed(&key_material)?;
    //                          ^^^^^^^^^^^
    //                          This is NOT deterministic!
}
```

### 3. The Root Cause: PrivateKey::from_seed() Uses Randomness

**File**: `btpc-core/src/crypto/keys.rs:112-138`

```rust
pub fn from_seed(seed: &[u8; 32]) -> Result<Self, KeyError> {
    // ❌ IGNORES THE SEED - Uses system randomness instead!
    let keypair = DilithiumKeypair::generate();  // <-- OS random

    // Only stores seed for later, doesn't use it for generation
    Ok(PrivateKey {
        key_bytes,
        public_key_bytes,
        seed: Some(*seed),  // Stored but not used for deterministic generation
        keypair: Some(keypair),
    })
}
```

**Documentation Admits This** (lines 82-105):
```rust
/// # IMPORTANT LIMITATION
/// Due to pqc_dilithium v0.2 library constraints, this method does NOT generate
/// truly deterministic keys from the seed. The seed is stored for future use
/// (enabling on-demand keypair regeneration for signing), but the initial keypair
/// uses system randomness.
```

---

## Why This Is Critical

### Scenario 1: User Loses Device
1. User creates wallet, gets 24-word seed phrase
2. User loses phone/computer
3. User tries to recover with seed phrase
4. **Different keys generated** → Funds inaccessible forever

### Scenario 2: Multi-Device Wallet
1. User enters same seed on two devices
2. **Two different wallets created**
3. Funds sent to address A cannot be spent by device B

### Scenario 3: False Security
- Users think their funds are backed up
- They might not backup the actual wallet file (.dat)
- **Lose wallet file = lose funds permanently**

---

## Technical Root Cause

### The pqc_dilithium v0.2.0 Library Limitation

```rust
// pqc_dilithium v0.2.0 public API
impl Keypair {
    pub fn generate() -> Self {
        // Uses OS randomness via rand crate
        // NO public API for seeded generation
    }
}
```

The library has **internal** functions for seeded generation (`crypto_sign_keypair()`) but they're **not exposed** in the public API.

---

## Industry Best Practices (from /ref research)

### 1. BIP32/BIP39 Standard (Bitcoin/Ethereum)
```
Mnemonic (24 words)
  ↓ PBKDF2
Seed (512 bits)
  ↓ HMAC-SHA512 with hardening
Master Private Key
  ↓ Hierarchical Derivation (m/44'/0'/0'/0/0)
Child Private Keys
```

**Key Property**: Same mnemonic → Same keys (always)

### 2. AWS KMS Key Derivation (NIST SP 800-108)
- Uses **HKDF** (HMAC-based Key Derivation Function)
- Counter mode with SHA-256
- Fully deterministic: same input → same output

### 3. Post-Quantum Key Derivation
For ML-DSA (Dilithium), deterministic generation requires:
- **SHAKE256** (ML-DSA's native PRF) or
- **HKDF-SHA512** for extracting entropy
- Feed deterministic bytes into ML-DSA key expansion

---

## Solution Requirements

### Must-Have Properties
1. ✅ **Determinism**: Same BIP39 seed → Same ML-DSA keypair (always)
2. ✅ **Security**: No reduction in cryptographic strength
3. ✅ **Standards**: Follow NIST FIPS 204 (ML-DSA) guidelines
4. ✅ **Compatibility**: BIP39 mnemonic input (24 words)

### Solution Options

#### Option A: Use Alternative ML-DSA Library ⭐ **RECOMMENDED**
**Library**: `pqcrypto-mldsa` (from rustpq/pqcrypto)
- ✅ More mature implementation
- ✅ Better API design
- ⚠️ Need to verify seeded generation support

**Implementation**:
```rust
// Pseudocode
use pqcrypto_mldsa::mldsa65;
use sha3::Shake256;

pub fn from_seed_deterministic(seed: &[u8; 32]) -> Result<Self, KeyError> {
    // Use SHAKE256 (ML-DSA's native PRF) to expand seed
    let mut shake = Shake256::default();
    shake.update(seed);
    shake.update(b"BTPC-ML-DSA-v1"); // Domain separation

    let mut rng_seed = [0u8; 48]; // ML-DSA needs 48 bytes
    shake.finalize_xof().read(&mut rng_seed);

    // Generate keypair deterministically
    let keypair = mldsa65::keypair_from_seed(&rng_seed)?;

    Ok(PrivateKey { ... })
}
```

#### Option B: Custom Dilithium Implementation
- ❌ High effort (months of work)
- ❌ Security risk (crypto is hard)
- ❌ Not recommended

#### Option C: Fork pqc_dilithium and Expose Internal API
- ⚠️ Medium effort
- ⚠️ Maintenance burden
- ⚠️ Last resort option

---

## Recommended Action Plan

### Phase 1: Research & Validation (2-3 hours)
1. ✅ Investigate pqcrypto-mldsa library capabilities
2. ✅ Verify it supports deterministic key generation
3. ✅ Check NIST FIPS 204 compliance
4. ✅ Review security implications

### Phase 2: Implementation (RED-GREEN-REFACTOR, 6-8 hours)
1. **RED Phase** (2 hours):
   ```rust
   #[test]
   fn test_deterministic_key_generation_from_bip39() {
       let mnemonic = "abandon abandon ... art";
       let seed1 = derive_seed_from_mnemonic(mnemonic);
       let seed2 = derive_seed_from_mnemonic(mnemonic);

       let key1 = PrivateKey::from_seed_deterministic(&seed1).unwrap();
       let key2 = PrivateKey::from_seed_deterministic(&seed2).unwrap();

       // MUST be equal
       assert_eq!(key1.to_bytes(), key2.to_bytes());
       assert_eq!(key1.public_key().to_bytes(), key2.public_key().to_bytes());
   }
   ```

2. **GREEN Phase** (3-4 hours):
   - Replace `pqc_dilithium` with `pqcrypto-mldsa`
   - Implement deterministic key derivation
   - Update `PrivateKey::from_seed()` implementation
   - Ensure all tests pass

3. **REFACTOR Phase** (1-2 hours):
   - Clean up code
   - Add comprehensive documentation
   - Security audit of derivation path

### Phase 3: Migration Strategy (2-3 hours)
**Problem**: Existing wallets were created with random keys

**Solution**:
1. Add wallet version field to metadata
2. Mark existing wallets as "v1 (non-deterministic)"
3. New wallets are "v2 (BIP39 deterministic)"
4. Show migration warning to users:
   ```
   ⚠️ Your wallet was created with an older version.
   For proper seed phrase recovery, please:
   1. Transfer funds to a new v2 wallet
   2. Backup the new 24-word seed phrase
   ```

### Phase 4: Testing (3-4 hours)
1. Unit tests for deterministic generation
2. Integration tests for wallet recovery
3. Manual testing:
   - Create wallet with seed A
   - Delete wallet
   - Recover with seed A
   - Verify same address/keys
4. Test cross-device recovery

---

## Estimated Effort

| Task | Effort |
|------|--------|
| Research pqcrypto-mldsa | 2-3 hours |
| Implement deterministic derivation | 6-8 hours |
| Migration strategy | 2-3 hours |
| Testing & validation | 3-4 hours |
| **TOTAL** | **13-18 hours** |

---

## Risk Assessment

### Current State Risks
- 🔴 **HIGH**: Users cannot recover wallets from seed phrases
- 🔴 **HIGH**: False sense of security (they think backup works)
- 🔴 **HIGH**: Funds could be permanently lost
- 🟡 **MEDIUM**: Reputational damage when users discover this

### Migration Risks
- 🟡 **MEDIUM**: Breaking change for existing wallets
- 🟢 **LOW**: New implementation is standard cryptography
- 🟢 **LOW**: Can support both v1 and v2 wallets side-by-side

---

## Constitutional Compliance

### Article II: Technical Specifications
- ✅ Still uses ML-DSA (Dilithium) signatures
- ✅ Still uses SHA-512 hashing
- ⚠️ Implementation detail change (deterministic vs random generation)

**Verdict**: ✅ **COMPLIANT** - This fixes a bug, doesn't change the cryptographic algorithm

### Article VI.3: TDD Methodology
- ✅ **MUST** follow RED-GREEN-REFACTOR
- ✅ Write tests first (RED)
- ✅ Implement minimum code (GREEN)
- ✅ Refactor for quality (REFACTOR)

**Verdict**: ✅ **REQUIRED** - Perfect use case for TDD

---

## Next Steps

1. **IMMEDIATE**: Stop advertising seed phrase recovery until fixed
   - Update UI to say "Backup wallet file" instead
   - Remove "Seed Phrase (24 words)" messaging

2. **SHORT TERM** (next 1-2 sessions):
   - Research pqcrypto-mldsa library
   - Implement deterministic key derivation
   - Follow TDD methodology strictly

3. **MEDIUM TERM**:
   - Migrate existing users
   - Comprehensive testing
   - Document recovery procedures

---

## References

- NIST FIPS 204: ML-DSA Standard
- BIP39: Mnemonic code for generating deterministic keys
- BIP32: Hierarchical Deterministic Wallets
- NIST SP 800-108: Key Derivation Functions
- pqcrypto-mldsa: https://github.com/rustpq/pqcrypto

---

**CRITICAL**: This issue must be resolved before any production deployment or public release.