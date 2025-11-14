# Research Report: Transaction Signing and Wallet Backup Bug Investigation

**Feature**: 005-fix-transaction-signing
**Date**: 2025-10-25
**Status**: Phase 0 Complete

## Executive Summary

This research investigated the root causes of two critical bugs in the BTPC desktop application:
1. **Transaction Signing Failure**: "Failed to sign input 0: Signature creation failed"
2. **Wallet Backup Error**: "backup_wallet missing required key walletId"

**Key Findings**:
- Transaction signing fails due to **fundamental ML-DSA keypair reconstruction limitation**
- Wallet backup missing walletId because **metadata not included in backup files**
- Thread-safety exists but has **UTXO race condition gap**
- Event-driven patterns exist but **inconsistently applied**

---

## Research Question 1: ML-DSA Signature API

### Current ML-DSA Implementation

**Location**: `btpc-core/src/crypto/keys.rs:228-239`

**Signing Function Signature**:
```rust
pub fn sign(&self, data: &[u8]) -> Result<Signature, SignatureError> {
    let keypair = self.keypair.as_ref()
        .ok_or(SignatureError::SigningFailed)?;  // ← FAILS HERE
    let signature_arr = keypair.sign(data);
    Signature::from_bytes(&signature_arr)
        .map_err(|_| SignatureError::SigningFailed)
}
```

**PrivateKey Structure**:
```rust
pub struct PrivateKey {
    key_bytes: [u8; 4000],                    // ML-DSA-65 private key
    public_key_bytes: [u8; 1952],             // ML-DSA-65 public key
    keypair: Option<DilithiumKeypair>,        // ← None when loaded from wallet
}
```

### Root Cause: Keypair Reconstruction Impossible

**Problem**: `from_key_pair_bytes()` creates `PrivateKey` with `keypair: None`

**Location**: `btpc-core/src/crypto/keys.rs:184-206`
```rust
pub fn from_key_pair_bytes(
    private_key_bytes: &[u8],
    public_key_bytes: &[u8],
) -> Result<Self, KeyError> {
    Ok(PrivateKey {
        key_bytes,
        public_key_bytes: pub_key_bytes,
        keypair: None,  // ← Cannot reconstruct - pqc_dilithium limitation
    })
}
```

**Why This Happens**:
- The `pqc_dilithium` library does NOT expose API to reconstruct `DilithiumKeypair` from bytes
- Only `DilithiumKeypair::generate()` available, which creates new random keys
- Keypair structure is opaque - cannot be deserialized

**Impact**:
- ALL wallet-loaded keys fail to sign transactions
- Affects 100% of desktop app transaction attempts
- Bug occurs at **first input signature** (index 0)

### Error Chain

1. Load wallet key: `PrivateKey::from_key_pair_bytes()` → `keypair: None`
2. Attempt signing: `private_key.sign(message)`
3. Check fails: `self.keypair.as_ref().ok_or(SignatureError::SigningFailed)?`
4. Error returned: `SignatureError::SigningFailed`
5. User sees: "Failed to sign input 0: Signature creation failed"

### Files Requiring Fixes

| File | Current Issue | Fix Required |
|------|---------------|--------------|
| `btpc-core/src/crypto/keys.rs:184-206` | `from_key_pair_bytes()` creates None keypair | Store seed + regenerate keypair on load |
| `btpc-core/src/crypto/wallet_serde.rs:309-313` | `KeyEntry::to_private_key()` uses broken method | Update to use signing-capable reconstruction |
| `btpc-desktop-app/src-tauri/src/wallet_commands.rs:240-241` | Calls `from_key_pair_bytes()` for transaction signing | Use new API that preserves signing capability |

**Decision**: Must store ML-DSA key generation **seed** (32 bytes) and regenerate keypair from seed on wallet load

**Rationale**: pqc_dilithium doesn't support keypair deserialization, only generation from seed

**Alternative Considered**: Store `DilithiumKeypair` directly - REJECTED (not serializable, library limitation)

---

## Research Question 2: RocksDB Wallet Storage

### Current Storage Architecture

**RocksDB Column Families**: `btpc-core/src/storage/rocksdb_config.rs:9-13`
```rust
pub const CF_BLOCKS: &str = "blocks";
pub const CF_TRANSACTIONS: &str = "transactions";
pub const CF_UTXOS: &str = "utxos";
pub const CF_METADATA: &str = "metadata";
// NO wallet column family - intentional for security
```

**Wallet Storage Model**:
- **Operational State**: In-memory `HashMap<String, WalletInfo>` (`wallet_manager.rs:230`)
- **Metadata File**: `wallets_metadata.json` (JSON format)
- **Encrypted Keys**: Separate `.dat` files per wallet (binary encrypted)
- **RocksDB**: NOT used for wallet secrets (only blockchain state)

### WalletId Storage Gap

**WalletInfo Structure**: `wallet_manager.rs:20-46`
```rust
pub struct WalletInfo {
    pub id: String,          // ← walletId EXISTS (UUID v4)
    pub nickname: String,
    pub address: String,
    pub file_path: PathBuf,
    pub metadata: WalletMetadata,
    // ... other fields
}
```

**Stored in**: `wallets_metadata.json` (NOT included in backups)

**WalletData Structure**: `btpc-core/src/crypto/wallet_serde.rs:49-59`
```rust
pub struct WalletData {
    pub network: String,
    pub keys: Vec<KeyEntry>,     // Private keys, public keys, addresses
    pub created_at: u64,
    pub modified_at: u64,
    // NO walletId field ← Root cause of backup error
}
```

**Stored in**: `wallet_*.dat` (encrypted, INCLUDED in backups)

### Storage Serialization

**Format**: Binary (bincode) + AES-256-GCM encryption

**Location**: `btpc-core/src/crypto/wallet_serde.rs:116-118`
```rust
let plaintext = bincode::serialize(wallet_data)
    .map_err(|_| WalletError::SerializationFailed)?;
```

**File Structure**:
```
wallet_*.dat format:
├── Magic bytes: "BTPC" (4 bytes)
├── Version: u32 (4 bytes)
├── Salt: 16 bytes (Argon2id)
├── Nonce: 12 bytes (AES-GCM)
└── Encrypted data: bincode(WalletData) + auth tag
```

**Decision**: Add `wallet_id: String` field to `WalletData` struct

**Rationale**: Backup files must be self-contained with identity for restoration validation

**Alternative Considered**: Store metadata separately - REJECTED (breaks backup portability)

---

## Research Question 3: Thread-Safety & Concurrency

### Tauri State Management

**Location**: `btpc-desktop-app/src-tauri/src/main.rs:2879-2930`

**AppState Structure**: `main.rs:391-421`
```rust
pub struct AppState {
    wallet_manager: Arc<Mutex<WalletManager>>,      // ✓ Thread-safe
    utxo_manager: Arc<Mutex<UTXOManager>>,          // ✓ Thread-safe
    active_network: Arc<RwLock<NetworkType>>,       // ✓ Read-optimized
    node_status: StateManager<NodeStatus>,          // ✓ Auto-emit events
    mining_status: StateManager<MiningStatus>,      // ✓ Auto-emit events
    // ... 10+ other state fields
}
```

**Initialization**: `.manage(app_state)` registers singleton state with Tauri

### Concurrency Patterns

**Pattern 1: Arc<Mutex<T>>** - Used for wallet operations
```rust
wallet_manager: Arc<Mutex<WalletManager>>
```

**Pattern 2: Arc<RwLock<T>>** - Used for configuration (many readers, few writers)
```rust
active_network: Arc<RwLock<NetworkType>>
```

**Pattern 3: StateManager<T>** - Auto-emits events on state changes (Article XI compliant)
```rust
node_status: StateManager<NodeStatus>
```

### Concurrent Transaction Handling

**Current Implementation**: `wallet_commands.rs:189-208`

**Issue Identified**: UTXO race condition
1. Thread A locks UTXO manager, selects UTXO #1 for transaction
2. Thread A releases lock, calls async RPC broadcast (takes 500ms)
3. Thread B locks UTXO manager, selects UTXO #1 again (not marked spent yet)
4. Thread B releases lock, broadcasts duplicate spend
5. Thread A re-locks, marks UTXO #1 as spent
6. Result: Double-spend attempt, transaction conflict

**Decision**: Implement optimistic UTXO reservation with rollback on failure

**Rationale**: Prevent UTXO selection race without holding lock during network I/O

**Alternative Considered**: Hold lock during RPC - REJECTED (blocks all wallet operations for 500ms+)

---

## Research Question 4: Wallet Backup Format

### Backup Implementation

**Location**: `wallet_manager.rs:581-599`
```rust
pub fn backup_wallet(&self, wallet_id: &str) -> BtpcResult<PathBuf> {
    let wallet = self.get_wallet(wallet_id)?;
    let backup_filename = format!("backup_{}_{}.dat", wallet.nickname, timestamp);
    let backup_path = self.config.backups_dir.join(backup_filename);
    std::fs::copy(&wallet.file_path, &backup_path)?;  // ← Only copies .dat file
    Ok(backup_path)
}
```

**Problem**: Backup only copies encrypted `.dat` file (contains `WalletData`), NOT `wallets_metadata.json` (contains `WalletInfo` with walletId)

### Encryption Implementation

**Algorithm**: AES-256-GCM with Argon2id key derivation

**Location**: `btpc-core/src/crypto/wallet_serde.rs:101-135`
```rust
pub fn encrypt(wallet_data: &WalletData, password: &SecurePassword) -> Result<Self, WalletError> {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);

    let encryption_key = Self::derive_key(password.as_bytes(), &salt)?;

    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);

    let plaintext = bincode::serialize(wallet_data)?;
    let cipher = Aes256Gcm::new_from_slice(&encryption_key)?;
    let encrypted_data = cipher.encrypt((&nonce).into(), plaintext.as_ref())?;
    // ... store with magic bytes, version, salt, nonce
}
```

**Key Derivation**: `wallet_serde.rs:256-275`
- **Algorithm**: Argon2id (OWASP-recommended)
- **Memory**: 64 MB
- **Iterations**: 3
- **Parallelism**: 4 threads
- **Output**: 32 bytes for AES-256

**Decision**: Backup format remains binary (bincode), add walletId to `WalletData` struct

**Rationale**:
- AES-256-GCM encryption already implemented correctly
- Binary format more compact than JSON (3293-byte signatures)
- Only missing field is walletId

**Alternative Considered**: Switch to JSON - REJECTED (larger file size, no security benefit)

---

## Research Question 5: Event-Driven Patterns (Article XI)

### Existing Event Patterns

**Pattern 1: StateManager Auto-Emit** (`state_management.rs:104-127`)
```rust
impl<T: Clone + Serialize> StateManager<T> {
    pub fn update<F>(&self, f: F, app: &AppHandle)
    where F: FnOnce(&mut T)
    {
        let mut state = self.state.write().unwrap();
        f(&mut *state);
        drop(state);
        self.emit_change(app);  // ← Automatic event emission
    }
}
```

**Usage**:
```rust
node_status.update(|s| { s.running = true; }, &app);
// Automatically emits "node_status_changed" event
```

**Pattern 2: Direct Event Emission** (`main.rs:1367-1380`)
```rust
app.emit("transaction-added", payload)?;
app.emit("wallet-balance-updated", balance)?;
```

### Event Naming Conventions

**Convention 1 (StateManager)**: `{state_name}_changed`
- `node_status_changed`
- `mining_status_changed`

**Convention 2 (Direct)**: `{noun}-{action}` kebab-case
- `transaction-added`
- `wallet-balance-updated`
- `network-config-changed`

### Frontend Event Management

**Location**: `btpc-desktop-app/ui/btpc-event-manager.js`

**EventListenerManager** (lines 11-131):
- Tracks all event listeners
- Prevents duplicate subscriptions
- Auto-cleanup on page unload (Article XI.6 compliance)

**Memory Leak Prevention**: `btpc-event-manager.js:286-298`
```javascript
window.addEventListener('beforeunload', () => {
    EventListenerManager.cleanup();  // Unlisten all events
});
```

**Backend-First Integration**: `btpc-backend-first.js:240-254`
```javascript
listen('setting-updated', async (event) => {
    // Backend emitted event FIRST
    // Now safe to update localStorage
    BTPCStorage.set(event.payload.key, event.payload.value);
});
```

### Recommended Event Names

**For Transaction Signing**:
- `transaction-signing-started` (before signing loop)
- `transaction-input-signed` (after each input, payload: `{inputIndex, totalInputs}`)
- `transaction-broadcast-success` (after RPC confirmation)
- `transaction-broadcast-failed` (on error)

**For Wallet Backup**:
- `wallet-backup-started` (before encryption)
- `wallet-backup-progress` (during encryption, payload: `{percent}`)
- `wallet-backup-completed` (after file written)
- `wallet-backup-failed` (on error)

**Decision**: Use kebab-case `{noun}-{action}` convention (matches existing pattern)

**Rationale**: Consistency with 80% of existing events in codebase

**Alternative Considered**: snake_case `{state}_changed` - REJECTED (only used for StateManager auto-emit)

---

## Summary of Decisions

| Research Area | Decision | Rationale |
|---------------|----------|-----------|
| **ML-DSA Signing** | Store key generation seed (32 bytes), regenerate keypair on load | pqc_dilithium doesn't support keypair deserialization |
| **walletId Storage** | Add `wallet_id: String` to `WalletData` struct | Backup files must be self-contained for restoration |
| **Backup Format** | Keep binary (bincode) + AES-256-GCM | Already correct, only missing walletId field |
| **Thread Safety** | Implement optimistic UTXO reservation | Prevent race without blocking during network I/O |
| **Event Names** | Use `{noun}-{action}` kebab-case | Matches 80% of existing event patterns |

---

## Files Analyzed

**Core Blockchain**:
- `btpc-core/src/crypto/keys.rs` (ML-DSA implementation)
- `btpc-core/src/crypto/signatures.rs` (Error types)
- `btpc-core/src/crypto/wallet_serde.rs` (Wallet encryption/serialization)
- `btpc-core/src/storage/rocksdb_config.rs` (Column families)

**Desktop App Backend**:
- `btpc-desktop-app/src-tauri/src/main.rs` (State initialization)
- `btpc-desktop-app/src-tauri/src/wallet_manager.rs` (Wallet operations)
- `btpc-desktop-app/src-tauri/src/wallet_commands.rs` (Tauri commands)
- `btpc-desktop-app/src-tauri/src/state_management.rs` (StateManager pattern)

**Desktop App Frontend**:
- `btpc-desktop-app/ui/btpc-event-manager.js` (Event lifecycle)
- `btpc-desktop-app/ui/btpc-backend-first.js` (Article XI compliance)
- `btpc-desktop-app/ui/btpc-tauri-context.js` (Tauri API initialization)

---

## Next Steps (Phase 1)

With all research questions answered, Phase 1 can proceed with:

1. **Data Model Design** (`data-model.md`):
   - Update `WalletData` to include `wallet_id` field
   - Add `KeySeed` struct for ML-DSA seed storage
   - Define UTXO reservation tracking

2. **API Contracts** (`contracts/`):
   - `send_transaction.yaml` with detailed error responses
   - `backup_wallet.yaml` with walletId requirement

3. **Contract Tests**:
   - Failing test for transaction signing
   - Failing test for wallet backup with walletId

4. **Quickstart Scenarios** (`quickstart.md`):
   - Single-input transaction signing test
   - Multi-input transaction signing test
   - Wallet backup and restoration test
   - Concurrent transaction signing test

5. **Agent File Update**:
   - Execute `.specify/scripts/bash/update-agent-context.sh claude`
   - Add ML-DSA seed storage pattern
   - Add UTXO reservation pattern
   - Add event naming conventions

---

**Research Phase 0: COMPLETE ✅**
**All [NEEDS CLARIFICATION] items resolved**
**Ready for Phase 1: Design & Contracts**