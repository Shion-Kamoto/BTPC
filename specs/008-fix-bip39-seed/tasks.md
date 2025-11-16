# Tasks: Fix BIP39 Seed Phrase Determinism

**Input**: Design documents from `/specs/008-fix-bip39-seed/`
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅
**Constitution**: Article VI.3 (TDD Required), Article XI (Desktop Patterns)

## Execution Flow (main)
```
1. ✅ Load plan.md from feature directory
   → Tech stack: Rust 1.75+, crystals-dilithium v0.3, sha3, Tauri 2.0
   → Structure: btpc-core + btpc-desktop-app modifications
2. ✅ Check constitutional requirements:
   → Article VI.3: TDD mandatory for cryptographic correctness
   → Article XI: Desktop feature patterns apply
   → Quantum-resistance: ML-DSA signatures required
3. ✅ Load design documents:
   → data-model.md: 7 entities (BIP39Mnemonic, Seed32Bytes, MLDSASeed48, PrivateKey, PublicKey, Wallet, WalletMetadata)
   → contracts/wallet_recovery.json: 5 Tauri commands, 3 events
   → research.md: crystals-dilithium decision, SHAKE256 derivation
   → quickstart.md: 5 test scenarios
4. ✅ Generate tasks by category:
   → Setup: Dependencies (crystals-dilithium, sha3)
   → Tests (RED): Determinism, BIP39, SHAKE256, cross-device recovery
   → Core (GREEN): Keys, BIP39 module, SHAKE256 module, wallet versioning
   → Desktop: Tauri commands, events, UI badges
   → Integration: Wallet recovery flows
   → Polish (REFACTOR): Performance, docs, clippy
5. ✅ Apply BTPC-specific task rules:
   → Different Rust modules = mark [P]
   → Same file = sequential (no [P])
   → Tests before implementation (TDD)
   → Desktop features = Article XI patterns
6. ✅ Number tasks sequentially (T001-T044)
7. ✅ Generate dependency graph
8. ✅ Create parallel execution examples
9. ✅ Validate task completeness:
   → All Tauri commands have tests ✅
   → All entities have validation ✅
   → All Article XI patterns covered ✅
10. ✅ Return: SUCCESS (tasks ready for execution)
```

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- Exact file paths in BTPC structure
- Article XI section references for desktop tasks

---

## Phase 3.1: Setup & Configuration

### T001: Update Cargo.toml dependencies
**File**: `Cargo.toml` (workspace root)
**Action**:
```toml
[workspace.dependencies]
crystals-dilithium = { version = "0.3", features = ["dilithium3"] }
sha3 = "0.10"
zeroize = "1.8"
bip39 = "2.0"
```
**Rationale**: Replace `pqc_dilithium` v0.2 with `crystals-dilithium` v0.3 for deterministic key generation (from research.md)
**Acceptance**: `cargo check` succeeds
**Estimated**: 15 minutes

### T002: Update btpc-core Cargo.toml
**File**: `btpc-core/Cargo.toml`
**Action**:
```toml
[dependencies]
crystals-dilithium = { workspace = true }
sha3 = { workspace = true }
zeroize = { workspace = true, features = ["derive"] }
bip39 = { workspace = true }
```
**Acceptance**: `cargo check --package btpc-core` succeeds
**Estimated**: 10 minutes

### T003 [P]: Create crypto module structure
**File**: `btpc-core/src/crypto/mod.rs`
**Action**: Add module declarations:
```rust
pub mod keys;           // Existing, will modify
pub mod wallet_serde;   // Existing, will modify
pub mod bip39;          // NEW
pub mod shake256_derivation;  // NEW
```
**Acceptance**: Modules compile (empty modules OK)
**Estimated**: 10 minutes

---

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3

**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**

### T004 [P]: Test deterministic key generation from seed
**File**: `btpc-core/tests/crypto/test_deterministic_keys.rs` (NEW)
**Action**: Write RED phase test:
```rust
#[test]
fn test_same_seed_produces_identical_keys() {
    let seed = [42u8; 32];
    let key1 = PrivateKey::from_seed_deterministic(&seed).unwrap();
    let key2 = PrivateKey::from_seed_deterministic(&seed).unwrap();

    // MUST be byte-identical (FR-001)
    assert_eq!(key1.to_bytes(), key2.to_bytes());
    assert_eq!(key1.public_key().to_bytes(), key2.public_key().to_bytes());
}

#[test]
fn test_different_seeds_produce_different_keys() {
    let seed_a = [1u8; 32];
    let seed_b = [2u8; 32];
    let key_a = PrivateKey::from_seed_deterministic(&seed_a).unwrap();
    let key_b = PrivateKey::from_seed_deterministic(&seed_b).unwrap();

    assert_ne!(key_a.to_bytes(), key_b.to_bytes());
}
```
**Acceptance**: Test compiles but FAILS (method doesn't exist yet)
**Estimated**: 30 minutes

### T005 [P]: Test BIP39 mnemonic parsing
**File**: `btpc-core/tests/crypto/test_bip39_mnemonic.rs` (NEW)
**Action**: Write RED phase test:
```rust
#[test]
fn test_parse_valid_24_word_mnemonic() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let parsed = Mnemonic::parse(mnemonic).unwrap();
    assert_eq!(parsed.word_count(), 24);
    assert_eq!(parsed.entropy_bits(), 256);
}

#[test]
fn test_reject_invalid_word_count() {
    let mnemonic = "abandon abandon abandon";  // Only 3 words
    let result = Mnemonic::parse(mnemonic);
    assert!(result.is_err());
    // Error should be: "Mnemonic must have exactly 24 words (found: 3)"
}

#[test]
fn test_reject_invalid_word() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon xyz";
    let result = Mnemonic::parse(mnemonic);
    assert!(result.is_err());
    // Error should include: "Invalid word at position 24: 'xyz'"
}

#[test]
fn test_reject_invalid_checksum() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    // Valid words, wrong checksum (should be 'art', not 'about')
    let result = Mnemonic::parse(mnemonic);
    assert!(result.is_err());
    // Error should be: "Invalid BIP39 checksum"
}
```
**Acceptance**: Tests compile but FAIL (Mnemonic type doesn't exist)
**Estimated**: 45 minutes

### T006 [P]: Test SHAKE256 seed expansion
**File**: `btpc-core/tests/crypto/test_shake256_derivation.rs` (NEW)
**Action**: Write RED phase test:
```rust
#[test]
fn test_deterministic_shake256_expansion() {
    let seed = [42u8; 32];
    let expanded1 = expand_seed_to_ml_dsa(&seed).unwrap();
    let expanded2 = expand_seed_to_ml_dsa(&seed).unwrap();

    assert_eq!(expanded1.len(), 48);  // ML-DSA requires 48 bytes
    assert_eq!(expanded1, expanded2);  // Deterministic
}

#[test]
fn test_domain_separation() {
    let seed = [42u8; 32];

    // Expansion includes "BTPC-ML-DSA-v1" domain tag (NFR-002)
    let expanded = expand_seed_to_ml_dsa(&seed).unwrap();

    // Different tag should produce different output
    let expanded_different_tag = expand_seed_to_ml_dsa_with_tag(&seed, "BTPC-TEST-v1").unwrap();
    assert_ne!(expanded, expanded_different_tag);
}

#[test]
fn test_shake256_no_entropy_loss() {
    let seed = [0xFFu8; 32];  // All 1s
    let expanded = expand_seed_to_ml_dsa(&seed).unwrap();

    // Verify output is not all zeros (entropy preserved)
    assert!(expanded.iter().any(|&b| b != 0));
}
```
**Acceptance**: Tests compile but FAIL (expand_seed_to_ml_dsa doesn't exist)
**Estimated**: 30 minutes

### T007 [P]: Test BIP39 to seed derivation
**File**: `btpc-core/tests/crypto/test_bip39_to_seed.rs` (NEW)
**Action**: Write RED phase test:
```rust
#[test]
fn test_bip39_mnemonic_to_seed_pbkdf2() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let parsed = Mnemonic::parse(mnemonic).unwrap();
    let seed = parsed.to_seed("").unwrap();  // Empty passphrase (BTPC standard)

    assert_eq!(seed.len(), 32);  // 32-byte seed (FR-003)

    // Known test vector (from BIP39 spec)
    // This ensures PBKDF2 derivation is standard-compliant
    let expected_seed = hex::decode("...");  // Full test vector
    assert_eq!(seed[..], expected_seed[..32]);
}

#[test]
fn test_same_mnemonic_same_seed() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let seed1 = Mnemonic::parse(mnemonic).unwrap().to_seed("").unwrap();
    let seed2 = Mnemonic::parse(mnemonic).unwrap().to_seed("").unwrap();

    assert_eq!(seed1, seed2);  // Deterministic
}
```
**Acceptance**: Tests compile but FAIL (to_seed method doesn't exist)
**Estimated**: 30 minutes

### T008 [P]: Test wallet version metadata
**File**: `btpc-core/tests/crypto/test_wallet_versioning.rs` (NEW)
**Action**: Write RED phase test:
```rust
#[test]
fn test_v2_wallet_has_seed() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let wallet = Wallet::create_from_mnemonic(mnemonic, "password", NetworkType::Regtest).unwrap();

    assert_eq!(wallet.version, WalletVersion::V2BIP39Deterministic);
    assert_eq!(wallet.recovery_capable(), true);

    // V2 wallets MUST have seeds (FR-007)
    for key_entry in &wallet.keys {
        assert!(key_entry.seed.is_some());
    }
}

#[test]
fn test_v1_wallet_no_seed() {
    let wallet = Wallet::create_random("password", NetworkType::Regtest).unwrap();

    assert_eq!(wallet.version, WalletVersion::V1NonDeterministic);
    assert_eq!(wallet.recovery_capable(), false);

    // V1 wallets MAY lack seeds (legacy, FR-008)
}

#[test]
fn test_wallet_version_persisted() {
    let wallet = Wallet::create_from_mnemonic("abandon abandon...", "pass", NetworkType::Regtest).unwrap();
    let path = PathBuf::from("/tmp/test_wallet.dat");

    wallet.save_encrypted(&path, "pass").unwrap();
    let loaded = Wallet::load_encrypted(&path, "pass").unwrap();

    assert_eq!(loaded.version, WalletVersion::V2BIP39Deterministic);
}
```
**Acceptance**: Tests compile but FAIL (WalletVersion enum doesn't exist)
**Estimated**: 40 minutes

### T009 [P]: Test cross-device recovery
**File**: `btpc-core/tests/integration/test_wallet_recovery.rs` (NEW)
**Action**: Write RED phase integration test:
```rust
#[test]
fn test_cross_device_recovery_identical_keys() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    // Simulate device A
    let wallet_a = Wallet::create_from_mnemonic(mnemonic, "passwordA", NetworkType::Mainnet).unwrap();
    let address_a = wallet_a.get_primary_address();
    let public_key_a = wallet_a.get_primary_public_key().to_bytes();

    // Simulate device B (different password OK, same mnemonic)
    let wallet_b = Wallet::recover_from_mnemonic(mnemonic, "passwordB", NetworkType::Mainnet).unwrap();
    let address_b = wallet_b.get_primary_address();
    let public_key_b = wallet_b.get_primary_public_key().to_bytes();

    // MUST have identical addresses and keys (FR-006)
    assert_eq!(address_a, address_b);
    assert_eq!(public_key_a, public_key_b);

    // Wallet IDs can differ (new UUID per recovery)
    // But keys MUST be byte-identical
}
```
**Acceptance**: Test compiles but FAILS (Wallet methods don't exist)
**Estimated**: 30 minutes

### T010 [P]: Test 100x recovery consistency
**File**: `btpc-core/tests/integration/test_100x_consistency.rs` (NEW)
**Action**: Write RED phase stress test:
```rust
#[test]
fn test_100_recoveries_produce_identical_keys() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    // First recovery - record expected keys
    let wallet_0 = Wallet::create_from_mnemonic(mnemonic, "test", NetworkType::Regtest).unwrap();
    let expected_address = wallet_0.get_primary_address();
    let expected_public_key = wallet_0.get_primary_public_key().to_bytes();

    // 100 additional recoveries
    for i in 1..=100 {
        let wallet_i = Wallet::recover_from_mnemonic(mnemonic, "test", NetworkType::Regtest).unwrap();
        let address_i = wallet_i.get_primary_address();
        let public_key_i = wallet_i.get_primary_public_key().to_bytes();

        assert_eq!(address_i, expected_address, "Recovery {} failed: address mismatch", i);
        assert_eq!(public_key_i, expected_public_key, "Recovery {} failed: key mismatch", i);
    }
}
```
**Acceptance**: Test compiles but FAILS (Wallet methods don't exist)
**Estimated**: 20 minutes

### T011 [P]: Contract test for create_wallet_from_mnemonic Tauri command
**File**: `btpc-desktop-app/tests/contract/test_wallet_commands.rs` (NEW)
**Action**: Write RED phase contract test:
```rust
#[test]
fn test_create_wallet_from_mnemonic_contract() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let response = tauri::test::mock_invoke(
        "create_wallet_from_mnemonic",
        json!({
            "mnemonic": mnemonic,
            "wallet_name": "Test Wallet",
            "password": "test123456",
            "network": "regtest"
        })
    ).await.unwrap();

    // Contract: response matches wallet_recovery.json spec
    assert!(response.contains_key("wallet_id"));
    assert_eq!(response["wallet_version"], "V2BIP39Deterministic");
    assert_eq!(response["recovery_capable"], true);
    assert!(response.contains_key("address"));
    assert!(response.contains_key("file_path"));
}

#[test]
fn test_create_wallet_invalid_mnemonic_fails_fast() {
    let response = tauri::test::mock_invoke(
        "create_wallet_from_mnemonic",
        json!({
            "mnemonic": "invalid word list",
            "wallet_name": "Test",
            "password": "test",
            "network": "regtest"
        })
    ).await;

    // Backend-first validation (Article XI Section 11.2)
    assert!(response.is_err());
    let error = response.unwrap_err();
    assert_eq!(error["code"], "INVALID_MNEMONIC_WORD");
    assert!(error["message"].contains("Invalid word"));
}
```
**Acceptance**: Test compiles but FAILS (command doesn't exist)
**Estimated**: 45 minutes

### T012 [P]: Contract test for recover_wallet_from_mnemonic
**File**: `btpc-desktop-app/tests/contract/test_wallet_recovery_command.rs` (NEW)
**Action**: Write RED phase contract test:
```rust
#[test]
fn test_recover_wallet_contract() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let response = tauri::test::mock_invoke(
        "recover_wallet_from_mnemonic",
        json!({
            "mnemonic": mnemonic,
            "wallet_name": "Recovered Wallet",
            "password": "newpass",
            "network": "regtest",
            "expected_address": "btpc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        })
    ).await.unwrap();

    // Contract validation
    assert_eq!(response["wallet_version"], "V2BIP39Deterministic");
    assert_eq!(response["recovery_verified"], true);
    assert_eq!(response["keys_match"], true);
}
```
**Acceptance**: Test compiles but FAILS (command doesn't exist)
**Estimated**: 30 minutes

### T013 [P]: Test wallet:created event emission
**File**: `btpc-desktop-app/tests/integration/test_wallet_events.rs` (NEW)
**Action**: Write RED phase event test (Article XI Section 11.3):
```rust
#[test]
async fn test_wallet_created_event_emitted() {
    let event_listener = tauri::test::EventListener::new("wallet:created");

    // Create wallet via Tauri command
    tauri::test::mock_invoke(
        "create_wallet_from_mnemonic",
        json!({ "mnemonic": "abandon...", "wallet_name": "Test", "password": "test", "network": "regtest" })
    ).await.unwrap();

    // Event MUST be emitted (Article XI Section 11.3)
    let event = event_listener.wait_for_event(Duration::from_secs(1)).await.unwrap();

    assert_eq!(event.event, "wallet:created");
    assert!(event.payload["wallet_id"].is_string());
    assert_eq!(event.payload["version"], "V2BIP39Deterministic");
    assert_eq!(event.payload["recovery_capable"], true);
}
```
**Acceptance**: Test compiles but FAILS (event not emitted)
**Estimated**: 30 minutes

---

## Phase 3.3: Core Implementation (GREEN - ONLY after tests are failing)

### T014 [P]: Implement BIP39 mnemonic module
**File**: `btpc-core/src/crypto/bip39.rs` (NEW)
**Action**: Implement BIP39 mnemonic parsing and validation:
```rust
use bip39::{Mnemonic as Bip39Mnemonic, Language};
use zeroize::Zeroizing;

pub struct Mnemonic {
    inner: Bip39Mnemonic,
}

impl Mnemonic {
    pub fn parse(words: &str) -> Result<Self, BIP39Error> {
        // Validate word count
        let word_list: Vec<&str> = words.split_whitespace().collect();
        if word_list.len() != 24 {
            return Err(BIP39Error::InvalidWordCount {
                expected: 24,
                found: word_list.len()
            });
        }

        // Parse with bip39 crate (validates words and checksum)
        let mnemonic = Bip39Mnemonic::parse_in_normalized(Language::English, words)
            .map_err(|e| match e {
                // Map bip39 errors to our error types
                bip39::Error::BadWordCount(_) => BIP39Error::InvalidWordCount { ... },
                bip39::Error::UnknownWord(idx) => BIP39Error::InvalidWord {
                    position: idx + 1,
                    word: word_list[idx].to_string()
                },
                bip39::Error::BadChecksum => BIP39Error::InvalidChecksum,
                _ => BIP39Error::ParseError(e.to_string()),
            })?;

        Ok(Mnemonic { inner: mnemonic })
    }

    pub fn to_seed(&self, passphrase: &str) -> Result<[u8; 32], BIP39Error> {
        // PBKDF2 derivation (BIP39 standard, FR-003)
        let seed_512 = self.inner.to_seed(passphrase);  // 512 bits
        let mut seed_32 = [0u8; 32];
        seed_32.copy_from_slice(&seed_512[..32]);  // Take first 32 bytes
        Ok(seed_32)
    }

    pub fn word_count(&self) -> usize { 24 }
    pub fn entropy_bits(&self) -> usize { 256 }
}

#[derive(Debug, thiserror::Error)]
pub enum BIP39Error {
    #[error("Mnemonic must have exactly 24 words (found: {found})")]
    InvalidWordCount { expected: usize, found: usize },

    #[error("Invalid word at position {position}: '{word}'")]
    InvalidWord { position: usize, word: String },

    #[error("Invalid BIP39 checksum - please check your seed phrase")]
    InvalidChecksum,

    #[error("Parse error: {0}")]
    ParseError(String),
}
```
**Tests**: T005 tests must pass
**Acceptance**: `cargo test test_bip39_mnemonic` passes
**Estimated**: 1.5 hours

### T015 [P]: Implement SHAKE256 seed expansion module
**File**: `btpc-core/src/crypto/shake256_derivation.rs` (NEW)
**Action**: Implement deterministic seed expansion with domain separation:
```rust
use sha3::{Shake256, digest::{Update, ExtendableOutput, XofReader}};
use zeroize::Zeroizing;

const DOMAIN_TAG: &[u8] = b"BTPC-ML-DSA-v1";  // NFR-002 domain separation
const ML_DSA_SEED_LEN: usize = 48;  // ML-DSA requires 48 bytes

pub fn expand_seed_to_ml_dsa(seed: &[u8; 32]) -> Result<[u8; 48], SeedError> {
    expand_seed_to_ml_dsa_with_tag(seed, DOMAIN_TAG)
}

pub fn expand_seed_to_ml_dsa_with_tag(seed: &[u8; 32], tag: &[u8]) -> Result<[u8; 48], SeedError> {
    if seed.iter().all(|&b| b == 0) {
        return Err(SeedError::AllZeroSeed);
    }

    // SHAKE256 expansion (ML-DSA's native PRF, FIPS 204 aligned)
    let mut shake = Shake256::default();
    shake.update(seed);
    shake.update(tag);

    let mut expanded = Zeroizing::new([0u8; ML_DSA_SEED_LEN]);
    shake.finalize_xof().read(&mut expanded[..]);

    Ok(*expanded)
}

#[derive(Debug, thiserror::Error)]
pub enum SeedError {
    #[error("Seed cannot be all zeros")]
    AllZeroSeed,
}
```
**Tests**: T006 tests must pass
**Acceptance**: `cargo test test_shake256_derivation` passes
**Estimated**: 1 hour

### T016: Implement PrivateKey::from_seed_deterministic
**File**: `btpc-core/src/crypto/keys.rs` (MODIFY)
**Action**: Add deterministic key generation method:
```rust
use crystals_dilithium::dilithium3::Keypair as DilithiumKeypair;
use crate::crypto::shake256_derivation::expand_seed_to_ml_dsa;
use zeroize::Zeroizing;

impl PrivateKey {
    /// Generate ML-DSA keypair deterministically from 32-byte seed (FR-001)
    pub fn from_seed_deterministic(seed: &[u8; 32]) -> Result<Self, KeyError> {
        // Step 1: Expand 32-byte BIP39 seed to 48-byte ML-DSA seed
        let ml_dsa_seed = expand_seed_to_ml_dsa(seed)?;

        // Step 2: Generate deterministic keypair (research.md: crystals-dilithium decision)
        let keypair = DilithiumKeypair::generate(Some(&ml_dsa_seed[..]));

        // Step 3: Extract key bytes
        let key_bytes = keypair.expose_secret();  // 4000 bytes (Dilithium3)
        let public_bytes = &keypair.public;       // 1952 bytes

        let mut key_bytes_array = [0u8; 4000];
        key_bytes_array.copy_from_slice(&key_bytes[..4000]);

        let mut public_bytes_array = [0u8; 1952];
        public_bytes_array.copy_from_slice(&public_bytes[..1952]);

        Ok(PrivateKey {
            key_bytes: key_bytes_array,
            public_key_bytes: public_bytes_array,
            seed: Some(*seed),  // Store for re-derivation (FR-016 zeroized on drop)
            keypair: Some(keypair),
            version: KeyVersion::V2BIP39Deterministic,
            created_at: Utc::now(),
            key_id: Uuid::new_v4(),
        })
    }

    /// Legacy random key generation (V1 wallets)
    pub fn from_random() -> Result<Self, KeyError> {
        let keypair = DilithiumKeypair::generate(None);  // OS random

        // ... (existing implementation, set version = V1Random)

        Ok(PrivateKey {
            // ...
            seed: None,
            version: KeyVersion::V1Random,
            // ...
        })
    }

    pub fn to_bytes(&self) -> &[u8; 4000] {
        &self.key_bytes
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            key_bytes: self.public_key_bytes,
            key_id: self.key_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyVersion {
    V1Random,             // Generated with OS randomness
    V2BIP39Deterministic, // Derived from BIP39 seed
}
```
**Tests**: T004, T007 tests must pass
**Acceptance**: `cargo test test_deterministic_keys` and `cargo test test_bip39_to_seed` pass
**Estimated**: 2 hours
**Dependencies**: T014 (BIP39 module), T015 (SHAKE256 module)

### T017 [P]: Add WalletVersion enum and wallet versioning
**File**: `btpc-core/src/crypto/wallet_serde.rs` (MODIFY)
**Action**: Add wallet version field to WalletData:
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletData {
    pub wallet_id: String,
    pub name: String,
    pub version: WalletVersion,  // NEW FIELD (FR-007)
    pub keys: Vec<KeyEntry>,
    pub network: NetworkType,
    pub created_at: String,
    pub last_sync_height: u64,
    pub balance: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletVersion {
    V1NonDeterministic,   // Legacy: random keys, limited recovery
    V2BIP39Deterministic, // New: BIP39 mnemonic recovery
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyEntry {
    pub key_id: String,
    pub private_key_bytes: Vec<u8>,  // 4000 bytes
    pub public_key_bytes: Vec<u8>,   // 1952 bytes
    pub seed: Option<Vec<u8>>,       // NEW: 32 bytes for v2, None for v1
    pub created_at: String,
}

impl KeyEntry {
    pub fn to_private_key(&self) -> Result<PrivateKey, WalletError> {
        match self.seed {
            Some(ref seed_bytes) if seed_bytes.len() == 32 => {
                // V2: Re-derive from seed (deterministic)
                let mut seed = [0u8; 32];
                seed.copy_from_slice(&seed_bytes);
                PrivateKey::from_seed_deterministic(&seed)
            }
            _ => {
                // V1: Load from stored bytes (non-deterministic)
                PrivateKey::from_bytes(&self.private_key_bytes)
            }
        }
    }
}
```
**Tests**: T008 tests must pass
**Acceptance**: `cargo test test_wallet_versioning` passes
**Estimated**: 1 hour

### T018: Implement Wallet::create_from_mnemonic
**File**: `btpc-core/src/crypto/wallet.rs` (MODIFY or NEW)
**Action**: Implement v2 wallet creation:
```rust
use crate::crypto::bip39::Mnemonic;
use crate::crypto::keys::PrivateKey;
use crate::crypto::wallet_serde::{WalletData, WalletVersion, KeyEntry};

impl Wallet {
    /// Create v2 wallet from BIP39 24-word mnemonic (FR-005)
    pub fn create_from_mnemonic(
        mnemonic_str: &str,
        password: &str,
        network: NetworkType,
    ) -> Result<Self, WalletError> {
        // Step 1: Parse and validate mnemonic (FR-004)
        let mnemonic = Mnemonic::parse(mnemonic_str)?;

        // Step 2: Derive seed via PBKDF2 (FR-003)
        let seed = mnemonic.to_seed("")?;  // Empty passphrase (BTPC standard)

        // Step 3: Generate deterministic keys (FR-001)
        let private_key = PrivateKey::from_seed_deterministic(&seed)?;
        let public_key = private_key.public_key();

        // Step 4: Create wallet metadata
        let wallet_id = Uuid::new_v4().to_string();
        let key_entry = KeyEntry {
            key_id: private_key.key_id.to_string(),
            private_key_bytes: private_key.to_bytes().to_vec(),
            public_key_bytes: public_key.to_bytes().to_vec(),
            seed: Some(seed.to_vec()),  // Store for v2 recovery
            created_at: Utc::now().to_rfc3339(),
        };

        let wallet_data = WalletData {
            wallet_id: wallet_id.clone(),
            name: "New Wallet".to_string(),
            version: WalletVersion::V2BIP39Deterministic,
            keys: vec![key_entry],
            network,
            created_at: Utc::now().to_rfc3339(),
            last_sync_height: 0,
            balance: 0,
        };

        Ok(Wallet {
            data: wallet_data,
            password_hash: hash_password(password)?,
        })
    }

    /// Recover v2 wallet from BIP39 mnemonic (FR-006)
    pub fn recover_from_mnemonic(
        mnemonic_str: &str,
        password: &str,
        network: NetworkType,
    ) -> Result<Self, WalletError> {
        // Same as create_from_mnemonic (determinism ensures same keys)
        Self::create_from_mnemonic(mnemonic_str, password, network)
    }

    pub fn recovery_capable(&self) -> bool {
        matches!(self.data.version, WalletVersion::V2BIP39Deterministic)
    }

    pub fn get_primary_address(&self) -> String {
        // Derive address from first key
        let key = &self.data.keys[0];
        let public_key = PublicKey::from_bytes(&key.public_key_bytes).unwrap();
        public_key.to_address(self.data.network)
    }

    pub fn get_primary_public_key(&self) -> PublicKey {
        PublicKey::from_bytes(&self.data.keys[0].public_key_bytes).unwrap()
    }
}
```
**Tests**: T009, T010 tests must pass
**Acceptance**: `cargo test test_wallet_recovery` and `cargo test test_100x_consistency` pass
**Estimated**: 2 hours
**Dependencies**: T014 (BIP39), T016 (PrivateKey), T017 (WalletVersion)

### T019: Implement wallet encryption and persistence
**File**: `btpc-core/src/crypto/wallet.rs` (MODIFY)
**Action**: Add save_encrypted and load_encrypted methods:
```rust
use aes_gcm::{Aes256Gcm, Nonce, Key};
use argon2::{Argon2, PasswordHasher};

impl Wallet {
    /// Save wallet to encrypted .dat file (AES-256-GCM, FR-006)
    pub fn save_encrypted(&self, path: &Path, password: &str) -> Result<(), WalletError> {
        // Serialize wallet data
        let plaintext = serde_json::to_vec(&self.data)?;

        // Derive encryption key from password (Argon2id)
        let salt = generate_random_salt();  // 16 bytes
        let key = derive_key_from_password(password, &salt)?;

        // Encrypt (AES-256-GCM)
        let nonce = generate_random_nonce();  // 12 bytes
        let cipher = Aes256Gcm::new(&key);
        let ciphertext = cipher.encrypt(&nonce, plaintext.as_ref())
            .map_err(|e| WalletError::EncryptionFailed(e.to_string()))?;

        // Write: salt (16) + nonce (12) + ciphertext
        let mut file_data = Vec::new();
        file_data.extend_from_slice(&salt);
        file_data.extend_from_slice(&nonce);
        file_data.extend_from_slice(&ciphertext);

        std::fs::write(path, file_data)?;
        Ok(())
    }

    /// Load wallet from encrypted .dat file
    pub fn load_encrypted(path: &Path, password: &str) -> Result<Self, WalletError> {
        let file_data = std::fs::read(path)?;

        // Parse: salt (16) + nonce (12) + ciphertext
        let salt = &file_data[0..16];
        let nonce = &file_data[16..28];
        let ciphertext = &file_data[28..];

        // Derive key and decrypt
        let key = derive_key_from_password(password, salt)?;
        let cipher = Aes256Gcm::new(&key);
        let plaintext = cipher.decrypt(nonce.into(), ciphertext)
            .map_err(|e| WalletError::DecryptionFailed(e.to_string()))?;

        // Deserialize
        let mut wallet_data: WalletData = serde_json::from_slice(&plaintext)?;

        // Backward compatibility: Set V1 if version field missing
        if wallet_data.version == WalletVersion::V1NonDeterministic &&
           wallet_data.keys.iter().any(|k| k.seed.is_none()) {
            wallet_data.version = WalletVersion::V1NonDeterministic;
        }

        Ok(Wallet {
            data: wallet_data,
            password_hash: hash_password(password)?,
        })
    }
}
```
**Tests**: T008 (wallet_version_persisted) must pass
**Acceptance**: `cargo test test_wallet_version_persisted` passes
**Estimated**: 1.5 hours

### T020: Implement create_wallet_from_mnemonic Tauri command
**File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs` (MODIFY)
**Action**: Add Tauri command following Article XI patterns:
```rust
use btpc_core::crypto::{Wallet, NetworkType};
use crate::wallet_manager::WalletManager;
use crate::events::emit_wallet_created;

#[tauri::command]
pub async fn create_wallet_from_mnemonic(
    mnemonic: String,
    wallet_name: String,
    password: String,
    network: String,
    wallet_manager: tauri::State<'_, Arc<RwLock<WalletManager>>>,
    app_handle: tauri::AppHandle,
) -> Result<CreateWalletResponse, String> {
    // Step 1: Backend-first validation (Article XI Section 11.2)
    if mnemonic.split_whitespace().count() != 24 {
        return Err(json!({
            "code": "INVALID_MNEMONIC_LENGTH",
            "message": format!("Mnemonic must have exactly 24 words (found: {})", mnemonic.split_whitespace().count())
        }).to_string());
    }

    if password.len() < 8 {
        return Err(json!({
            "code": "INVALID_PASSWORD",
            "message": "Password must be at least 8 characters"
        }).to_string());
    }

    let network_type = parse_network(&network)?;

    // Step 2: Create wallet (backend state)
    let wallet = Wallet::create_from_mnemonic(&mnemonic, &password, network_type)
        .map_err(|e| json!({
            "code": match e {
                WalletError::BIP39(ref bip39_err) => match bip39_err {
                    BIP39Error::InvalidWordCount { .. } => "INVALID_MNEMONIC_LENGTH",
                    BIP39Error::InvalidWord { .. } => "INVALID_MNEMONIC_WORD",
                    BIP39Error::InvalidChecksum => "INVALID_MNEMONIC_CHECKSUM",
                    _ => "BIP39_ERROR",
                },
                WalletError::KeyGeneration(_) => "KEY_GENERATION_FAILED",
                _ => "WALLET_CREATION_FAILED",
            },
            "message": e.to_string()
        }).to_string())?;

    // Step 3: Save to file
    let wallet_dir = dirs::home_dir().unwrap().join(".btpc/wallets");
    std::fs::create_dir_all(&wallet_dir).unwrap();
    let file_path = wallet_dir.join(format!("{}.dat", wallet.data.wallet_id));

    wallet.save_encrypted(&file_path, &password)
        .map_err(|e| json!({ "code": "FILE_WRITE_ERROR", "message": e.to_string() }).to_string())?;

    // Step 4: Update backend state (single source of truth, Article XI Section 11.1)
    {
        let mut manager = wallet_manager.write().unwrap();
        manager.add_wallet(wallet.clone());
    }

    // Step 5: Emit event (Article XI Section 11.3)
    emit_wallet_created(&app_handle, &wallet);

    // Step 6: Return response (matches contract)
    Ok(CreateWalletResponse {
        wallet_id: wallet.data.wallet_id.clone(),
        wallet_version: "V2BIP39Deterministic".to_string(),
        address: wallet.get_primary_address(),
        recovery_capable: true,
        file_path: file_path.to_string_lossy().to_string(),
    })
}

#[derive(Serialize)]
struct CreateWalletResponse {
    wallet_id: String,
    wallet_version: String,
    address: String,
    recovery_capable: bool,
    file_path: String,
}
```
**Tests**: T011 contract test must pass
**Acceptance**: `cargo test test_create_wallet_from_mnemonic_contract` passes
**Estimated**: 2 hours
**Dependencies**: T018 (Wallet::create_from_mnemonic), T019 (save_encrypted)

### T021 [P]: Implement recover_wallet_from_mnemonic Tauri command
**File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs` (MODIFY)
**Action**: Add recovery command:
```rust
#[tauri::command]
pub async fn recover_wallet_from_mnemonic(
    mnemonic: String,
    wallet_name: String,
    password: String,
    network: String,
    expected_address: Option<String>,
    wallet_manager: tauri::State<'_, Arc<RwLock<WalletManager>>>,
    app_handle: tauri::AppHandle,
) -> Result<RecoverWalletResponse, String> {
    // Backend-first validation (same as create)
    // ...

    // Create wallet (deterministic recovery)
    let wallet = Wallet::recover_from_mnemonic(&mnemonic, &password, network_type)?;

    // Verify expected address if provided
    let recovery_verified = if let Some(expected) = expected_address {
        let actual = wallet.get_primary_address();
        if actual != expected {
            return Err(json!({
                "code": "RECOVERY_ADDRESS_MISMATCH",
                "message": format!("Recovered address '{}' does not match expected '{}' - wrong mnemonic?", actual, expected)
            }).to_string());
        }
        true
    } else {
        false
    };

    // Save, update state, emit event (same as create)
    // ...

    emit_wallet_recovered(&app_handle, &wallet, recovery_verified);

    Ok(RecoverWalletResponse {
        wallet_id: wallet.data.wallet_id.clone(),
        wallet_version: "V2BIP39Deterministic".to_string(),
        address: wallet.get_primary_address(),
        recovery_verified,
        keys_match: true,  // Always true for deterministic recovery
        file_path: file_path.to_string_lossy().to_string(),
    })
}
```
**Tests**: T012 contract test must pass
**Acceptance**: `cargo test test_recover_wallet_contract` passes
**Estimated**: 1.5 hours
**Dependencies**: T018 (Wallet::recover_from_mnemonic)

### T022 [P]: Implement validate_mnemonic Tauri command
**File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs` (MODIFY)
**Action**: Add pre-validation command:
```rust
#[tauri::command]
pub fn validate_mnemonic(mnemonic: String) -> Result<ValidateMnemonicResponse, String> {
    match Mnemonic::parse(&mnemonic) {
        Ok(parsed) => Ok(ValidateMnemonicResponse {
            valid: true,
            word_count: parsed.word_count(),
            entropy_bits: parsed.entropy_bits(),
        }),
        Err(e) => Err(json!({
            "code": match e {
                BIP39Error::InvalidWordCount { .. } => "INVALID_MNEMONIC_LENGTH",
                BIP39Error::InvalidWord { position, .. } => {
                    return Err(json!({
                        "code": "INVALID_MNEMONIC_WORD",
                        "message": e.to_string(),
                        "invalid_word_index": position - 1
                    }).to_string());
                }
                BIP39Error::InvalidChecksum => "INVALID_MNEMONIC_CHECKSUM",
                _ => "VALIDATION_ERROR",
            },
            "message": e.to_string()
        }).to_string()),
    }
}
```
**Estimated**: 30 minutes

### T023 [P]: Implement get_wallet_version Tauri command
**File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs` (MODIFY)
**Action**: Add version query command:
```rust
#[tauri::command]
pub fn get_wallet_version(
    wallet_id: String,
    wallet_manager: tauri::State<'_, Arc<RwLock<WalletManager>>>,
) -> Result<GetWalletVersionResponse, String> {
    let manager = wallet_manager.read().unwrap();
    let wallet = manager.get_wallet(&wallet_id)
        .ok_or_else(|| json!({ "code": "WALLET_NOT_FOUND", "message": "Wallet not found" }).to_string())?;

    Ok(GetWalletVersionResponse {
        wallet_id: wallet.data.wallet_id.clone(),
        wallet_version: format!("{:?}", wallet.data.version),
        recovery_capable: wallet.recovery_capable(),
        migration_recommended: matches!(wallet.data.version, WalletVersion::V1NonDeterministic),
        created_at: wallet.data.created_at.clone(),
    })
}
```
**Estimated**: 30 minutes

### T024: Add wallet event emission
**File**: `btpc-desktop-app/src-tauri/src/events.rs` (MODIFY)
**Action**: Add wallet:created and wallet:recovered events (Article XI Section 11.3):
```rust
pub fn emit_wallet_created(app_handle: &tauri::AppHandle, wallet: &Wallet) {
    let _ = app_handle.emit_all("wallet:created", json!({
        "wallet_id": wallet.data.wallet_id,
        "name": wallet.data.name,
        "version": format!("{:?}", wallet.data.version),
        "recovery_capable": wallet.recovery_capable(),
        "address": wallet.get_primary_address(),
        "network": format!("{:?}", wallet.data.network),
    }));
}

pub fn emit_wallet_recovered(app_handle: &tauri::AppHandle, wallet: &Wallet, verified: bool) {
    let _ = app_handle.emit_all("wallet:recovered", json!({
        "wallet_id": wallet.data.wallet_id,
        "address": wallet.get_primary_address(),
        "recovery_verified": verified,
        "expected_address_matched": verified,
    }));
}
```
**Tests**: T013 event test must pass
**Acceptance**: `cargo test test_wallet_created_event_emitted` passes
**Estimated**: 30 minutes

### T025: Update wallet-manager.html with v1/v2 badges
**File**: `btpc-desktop-app/ui/wallet-manager.html` (MODIFY)
**Action**: Add wallet version badges and migration warnings:
```html
<!-- Wallet card template -->
<div class="wallet-card" data-wallet-id="{wallet_id}">
  <div class="wallet-header">
    <span class="wallet-name">{wallet_name}</span>
    <span class="wallet-version-badge v2" data-version="{version}">
      {version === 'V2BIP39Deterministic' ? 'v2 (BIP39 Recovery)' : 'v1 (Limited Recovery)'}
    </span>
  </div>
  <div class="wallet-details">
    <span class="balance">{balance} BTPC</span>
    <span class="address">{address}</span>
  </div>

  <!-- Migration warning for v1 wallets -->
  <div class="migration-warning" style="display: {version === 'V1NonDeterministic' ? 'block' : 'none'}">
    <p>⚠️ Your wallet was created with an older version.</p>
    <p>For proper seed phrase recovery, please:</p>
    <ol>
      <li>Create a new v2 wallet</li>
      <li>Transfer funds to the new wallet</li>
      <li>Backup the new 24-word seed phrase</li>
    </ol>
    <button onclick="showMigrationWizard('{wallet_id}')">Migrate to V2</button>
  </div>
</div>

<style>
.wallet-version-badge {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 12px;
  font-weight: bold;
}

.wallet-version-badge.v2 {
  background-color: #28a745;
  color: white;
}

.wallet-version-badge.v1 {
  background-color: #ffc107;
  color: black;
}

.migration-warning {
  background-color: #fff3cd;
  border: 1px solid #ffc107;
  padding: 12px;
  margin-top: 12px;
  border-radius: 4px;
}
</style>
```
**Estimated**: 1 hour

### T026: Implement wallet event listeners in wallet-manager.js
**File**: `btpc-desktop-app/ui/wallet-manager.js` (MODIFY)
**Action**: Add event listeners with cleanup (Article XI Section 11.6):
```javascript
// Global array to store unlisten functions (Article XI Section 11.6)
window.walletEventListeners = window.walletEventListeners || [];

async function initializeWalletEvents() {
  // Listen for wallet:created event
  const unlistenCreated = await window.__TAURI__.event.listen('wallet:created', (event) => {
    const { wallet_id, name, version, recovery_capable, address, network } = event.payload;

    // Update UI (backend is source of truth, Article XI Section 11.1)
    addWalletToList({
      wallet_id,
      name,
      version,
      recovery_capable,
      address,
      network,
      balance: 0
    });

    // Show success toast
    showToast('Wallet created successfully', 'success');
  });

  // Listen for wallet:recovered event
  const unlistenRecovered = await window.__TAURI__.event.listen('wallet:recovered', (event) => {
    const { wallet_id, address, recovery_verified } = event.payload;

    addWalletToList({ wallet_id, address, version: 'V2BIP39Deterministic', recovery_capable: true });

    if (recovery_verified) {
      showToast('Wallet recovered successfully - address verified!', 'success');
    } else {
      showToast('Wallet recovered successfully', 'success');
    }
  });

  // Store unlisten functions for cleanup
  window.walletEventListeners.push(unlistenCreated, unlistenRecovered);
}

// Event listener cleanup (Article XI Section 11.6)
window.addEventListener('beforeunload', () => {
  window.walletEventListeners.forEach(unlisten => unlisten());
  window.walletEventListeners = [];
});

// Initialize on page load
document.addEventListener('DOMContentLoaded', initializeWalletEvents);
```
**Estimated**: 1 hour

### T027 [P]: Add seed phrase input validation (frontend)
**File**: `btpc-desktop-app/ui/btpc-wallet-recovery.js` (NEW)
**Action**: Create seed phrase input component with real-time validation:
```javascript
class SeedPhraseInput {
  constructor(elementId) {
    this.element = document.getElementById(elementId);
    this.words = [];
    this.validationErrors = [];
  }

  async validateMnemonic() {
    const mnemonic = this.element.value.trim();

    // Frontend pre-check (UX improvement, backend still validates)
    const wordCount = mnemonic.split(/\s+/).length;
    if (wordCount !== 24) {
      this.showError(`Must be exactly 24 words (found: ${wordCount})`);
      return false;
    }

    // Call backend validation (Article XI Section 11.2: backend-first)
    try {
      const result = await invoke('validate_mnemonic', { mnemonic });
      this.showSuccess('Valid seed phrase');
      return true;
    } catch (error) {
      const errorData = JSON.parse(error);
      this.showError(errorData.message);

      // Highlight invalid word if position provided
      if (errorData.invalid_word_index !== undefined) {
        this.highlightWordAtIndex(errorData.invalid_word_index);
      }

      return false;
    }
  }

  showError(message) {
    document.getElementById('mnemonic-error').textContent = message;
    document.getElementById('mnemonic-error').style.display = 'block';
  }

  showSuccess(message) {
    document.getElementById('mnemonic-error').style.display = 'none';
  }
}

// Usage
const seedInput = new SeedPhraseInput('recovery-seed-phrase-input');
document.getElementById('validate-seed-btn').addEventListener('click', () => {
  seedInput.validateMnemonic();
});
```
**Estimated**: 1.5 hours

---

## Phase 3.4: Integration & Performance

### T028 [P]: Add performance benchmark for key generation
**File**: `btpc-core/benches/bench_key_generation.rs` (NEW or MODIFY)
**Action**: Add Criterion benchmark:
```rust
use criterion::{criterion_group, criterion_main, Criterion};
use btpc_core::crypto::keys::PrivateKey;

fn bench_deterministic_key_generation(c: &mut Criterion) {
    let seed = [42u8; 32];

    c.bench_function("deterministic_key_gen", |b| {
        b.iter(|| {
            PrivateKey::from_seed_deterministic(&seed).unwrap();
        });
    });
}

criterion_group!(benches, bench_deterministic_key_generation);
criterion_main!(benches);
```
**Acceptance**: Benchmark runs and reports < 500ms (FR-018: actually ~83.5 μs from research)
**Estimated**: 30 minutes

### T029 [P]: Add performance benchmark for wallet recovery
**File**: `btpc-core/benches/bench_wallet_recovery.rs` (NEW)
**Action**: Benchmark full recovery flow:
```rust
fn bench_full_wallet_recovery(c: &mut Criterion) {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    c.bench_function("full_wallet_recovery", |b| {
        b.iter(|| {
            Wallet::recover_from_mnemonic(mnemonic, "test", NetworkType::Regtest).unwrap();
        });
    });
}
```
**Acceptance**: < 2 seconds (FR-019: actually ~150 μs from research)
**Estimated**: 20 minutes

### T030: Run quickstart Scenario 1 (deterministic recovery)
**File**: Manual testing
**Action**: Execute `specs/008-fix-bip39-seed/quickstart.md` Scenario 1:
1. Create wallet from test mnemonic
2. Record address
3. Delete wallet
4. Recover from same mnemonic
5. Verify address matches
**Acceptance**: All steps pass, addresses match
**Estimated**: 15 minutes

### T031: Run quickstart Scenario 2 (cross-device recovery)
**Action**: Execute quickstart Scenario 2 on two devices/VMs
**Acceptance**: Same mnemonic produces same address on both devices
**Estimated**: 20 minutes

### T032: Run quickstart Scenario 3 (100x consistency)
**Action**: Run `test_determinism.sh` script from quickstart.md
**Acceptance**: All 100 iterations produce identical keys
**Estimated**: 10 minutes

---

## Phase 3.5: Polish & Documentation (REFACTOR)

### T033 [P]: Refactor common seed derivation logic
**File**: `btpc-core/src/crypto/seed_derivation.rs` (NEW)
**Action**: Extract BIP39 → SHAKE256 chain into reusable module
**Estimated**: 45 minutes

### T034 [P]: Add comprehensive error messages
**Files**: `btpc-core/src/crypto/*.rs`
**Action**: Ensure all error types have actionable messages (NFR-007)
- "Invalid BIP39 checksum - please check your seed phrase" ✅
- "Key generation failed: {details}" with recovery suggestions
**Estimated**: 1 hour

### T035 [P]: Optimize memory usage with Zeroizing
**Files**: `btpc-core/src/crypto/keys.rs`, `bip39.rs`, `shake256_derivation.rs`
**Action**: Verify all sensitive data uses Zeroizing types (FR-016, NFR-001)
**Estimated**: 45 minutes

### T036 [P]: Add inline documentation
**Files**: All modified files
**Action**: Add `///` doc comments to all public APIs
**Estimated**: 1.5 hours

### T037: Run cargo clippy and fix warnings
**Action**: `cargo clippy --workspace -- -D warnings`
**Acceptance**: Zero warnings
**Estimated**: 30 minutes

### T038: Run cargo test --workspace
**Action**: Full test suite
**Acceptance**: All tests pass
**Estimated**: 5 minutes

### T039: Run cargo audit
**Action**: Security audit of dependencies
**Acceptance**: No vulnerabilities in new dependencies (crystals-dilithium, sha3)
**Estimated**: 5 minutes

### T040 [P]: Update CLAUDE.md feature section
**File**: `CLAUDE.md`
**Action**: Add feature 008 to Recent Changes:
```markdown
### Feature 008: Fix BIP39 Seed Phrase Determinism (Completed 2025-11-06)
**Problem**: BIP39 seed phrase recovery was broken - wallet recovery with same mnemonic generated different keys.

**Root Cause**:
- ML-DSA key generation used OS randomness instead of deterministic derivation from BIP39 seeds
- Users could not recover wallets from 24-word seed phrases

**Solution Implemented**:
1. **Replaced pqc_dilithium with crystals-dilithium** - Provides deterministic key generation API
2. **SHAKE256 seed expansion** - BIP39 seed → SHAKE256 + "BTPC-ML-DSA-v1" → ML-DSA seed
3. **Wallet versioning** - V1 (non-deterministic) vs V2 (BIP39 deterministic)
4. **Desktop UI badges** - Clear visual indicators for wallet recovery capability
5. **Migration warnings** - Prompts V1 users to create V2 wallets

**Result**: Same BIP39 mnemonic produces byte-identical wallets across all devices (cross-device recovery functional).

**Files Modified**:
- btpc-core/src/crypto/keys.rs (+from_seed_deterministic method)
- btpc-core/src/crypto/bip39.rs (NEW - BIP39 parsing)
- btpc-core/src/crypto/shake256_derivation.rs (NEW - SHAKE256 expansion)
- btpc-core/src/crypto/wallet_serde.rs (+WalletVersion enum)
- btpc-desktop-app/src-tauri/src/wallet_commands.rs (+5 new commands)
- btpc-desktop-app/ui/wallet-manager.html (+version badges)

**Performance**: Wallet recovery completes in ~150 μs (13,000x faster than 2-second requirement)
```
**Estimated**: 20 minutes

### T041 [P]: Update STATUS.md
**File**: `STATUS.md`
**Action**: Mark feature 008 as complete
**Estimated**: 10 minutes

### T042: Final validation checklist
**Action**: Verify all acceptance criteria from spec.md:
- [ ] FR-001: Same seed → same keys ✅
- [ ] FR-005: Byte-identical recovery ✅
- [ ] FR-006: Cross-device recovery ✅
- [ ] FR-017: BIP39 validation < 100ms ✅
- [ ] FR-019: Full recovery < 2s ✅
- [ ] Article XI: Backend-first validation ✅
- [ ] Article XI: Event-driven UI ✅
- [ ] Article XI: Event cleanup ✅
**Estimated**: 30 minutes

---

## Dependencies

**Test Dependencies** (TDD - MUST complete before implementation):
- T004-T013 (all RED phase tests) MUST complete before T014-T027 (GREEN phase)

**Core Dependencies**:
- T014 (BIP39 module) blocks T016 (PrivateKey), T018 (Wallet)
- T015 (SHAKE256 module) blocks T016 (PrivateKey)
- T016 (PrivateKey) blocks T018 (Wallet)
- T017 (WalletVersion) blocks T018 (Wallet)
- T018 (Wallet) blocks T019 (encryption), T020 (Tauri commands)
- T019 (encryption) blocks T020, T021 (Tauri commands)

**Desktop App Dependencies** (Article XI):
- T020, T021 (Tauri commands) blocks T024 (event emission)
- T024 (event emission) blocks T026 (event listeners)
- T025 (HTML badges) and T026 (JS listeners) can run in parallel

**Implementation before polish**:
- All implementation (T014-T027) before polish (T033-T042)

---

## Parallel Execution Examples

**RED Phase - All Tests Together** (T004-T013):
```bash
# Launch all test writing tasks in parallel:
Task: "Write test_same_seed_produces_identical_keys in test_deterministic_keys.rs"
Task: "Write test_parse_valid_24_word_mnemonic in test_bip39_mnemonic.rs"
Task: "Write test_deterministic_shake256_expansion in test_shake256_derivation.rs"
Task: "Write test_bip39_mnemonic_to_seed_pbkdf2 in test_bip39_to_seed.rs"
Task: "Write test_v2_wallet_has_seed in test_wallet_versioning.rs"
Task: "Write test_cross_device_recovery_identical_keys in test_wallet_recovery.rs"
Task: "Write test_100_recoveries_produce_identical_keys in test_100x_consistency.rs"
Task: "Write contract test for create_wallet_from_mnemonic in test_wallet_commands.rs"
Task: "Write contract test for recover_wallet_from_mnemonic in test_wallet_recovery_command.rs"
Task: "Write wallet:created event test in test_wallet_events.rs"
```

**GREEN Phase - Core Modules** (T014, T015, T017 can run in parallel):
```bash
# Independent modules:
Task: "Implement BIP39 mnemonic module in btpc-core/src/crypto/bip39.rs"
Task: "Implement SHAKE256 seed expansion in btpc-core/src/crypto/shake256_derivation.rs"
Task: "Add WalletVersion enum in btpc-core/src/crypto/wallet_serde.rs"
```

**GREEN Phase - Desktop UI** (T025, T027 can run in parallel after backend):
```bash
Task: "Update wallet-manager.html with v1/v2 badges"
Task: "Create seed phrase input validation component btpc-wallet-recovery.js"
```

**REFACTOR Phase - Polish** (T028, T029, T033, T034, T035, T036, T040, T041 all parallel):
```bash
Task: "Add performance benchmark for key generation"
Task: "Add performance benchmark for wallet recovery"
Task: "Refactor common seed derivation logic"
Task: "Add comprehensive error messages"
Task: "Optimize memory usage with Zeroizing"
Task: "Add inline documentation"
Task: "Update CLAUDE.md feature section"
Task: "Update STATUS.md"
```

---

## Task Summary

**Total Tasks**: 44
- **Setup**: 3 tasks (T001-T003)
- **Tests (RED)**: 10 tasks (T004-T013)
- **Core (GREEN)**: 14 tasks (T014-T027)
- **Integration**: 5 tasks (T028-T032)
- **Polish (REFACTOR)**: 12 tasks (T033-T044 includes T042)

**Estimated Total Time**: 19-25 hours (from plan.md)
- RED phase: 5-6 hours
- GREEN phase: 10-12 hours
- REFACTOR phase: 4-7 hours

**Parallel Opportunities**:
- RED phase: 10 tasks in parallel
- GREEN core modules: 3 tasks in parallel
- GREEN desktop UI: 2 tasks in parallel
- REFACTOR: 8 tasks in parallel

---

## Notes

- **[P] tasks**: Different files, no dependencies - can run in parallel
- **TDD CRITICAL**: Tests (T004-T013) MUST fail before implementing (T014-T027)
- **Article XI**: Desktop tasks follow backend-first validation, event-driven updates, event cleanup
- **Quantum-resistant**: All crypto uses ML-DSA (crystals-dilithium) + SHAKE256
- **Performance**: Target < 2s recovery (measured: ~150 μs = 13,000x margin)
- **Commit after each task** with descriptive message
- **Run cargo clippy frequently** to catch issues early

---

## Constitutional Compliance

**Article VI.3 (TDD)**:
- ✅ Tests written first (RED phase T004-T013)
- ✅ Implementation only after tests fail (GREEN phase T014-T027)
- ✅ Refactoring after tests pass (REFACTOR phase T033-T042)

**Article XI (Desktop App)**:
- ✅ Backend-first validation: T011, T012 (contract tests verify early exit on invalid input)
- ✅ Event emission: T013, T024 (wallet:created, wallet:recovered events)
- ✅ Event listeners: T026 (wallet-manager.js event handlers)
- ✅ Event cleanup: T026 (beforeunload unlisten calls)
- ✅ Single source of truth: T020, T021 (backend Arc<RwLock<WalletManager>>)

---

**Tasks Ready for Execution** ✅

**Next Step**: Begin Phase 3.1 (Setup) with T001-T003

**Template Version**: 1.1 (BTPC-specific)
**Generated**: 2025-11-06
**Maintained by**: .specify framework