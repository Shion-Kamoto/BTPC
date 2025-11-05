# Tasks: Fix Transaction Signing and Wallet Backup Failures

**Feature**: 005-fix-transaction-signing
**Input**: Design documents from `/home/bob/BTPC/BTPC/specs/005-fix-transaction-signing/`
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅
**Constitution**: Article VI.3 (TDD), Article VIII (ML-DSA), Article XI (Desktop Patterns)

## Executive Summary

**Critical Bugs**:
1. Transaction signing fails: "Failed to sign input 0: Signature creation failed"
2. Wallet backup fails: "backup_wallet missing required key walletId"

**Root Causes** (from research.md):
1. ML-DSA keypair cannot be reconstructed from stored bytes (pqc_dilithium library limitation)
2. `wallet_id` field missing from `WalletData` struct serialization
3. UTXO race condition during concurrent transactions

**Implementation Strategy**:
- Store ML-DSA key generation seed (32 bytes) → regenerate keypair on wallet load
- Add `wallet_id: String` to `WalletData` struct
- Implement optimistic UTXO reservation with rollback

**Task Count**: 28 tasks (8 parallel groups, 20 sequential)
**Estimated Duration**: 12-16 hours with TDD methodology

---

## Phase 3.1: Setup & Prerequisites

### T001: Verify Development Environment
**File**: N/A (environment check)
**Type**: Setup
**Dependencies**: None
**Parallel**: No

**Description**:
Verify Rust 1.75+, cargo, pqcrypto-dilithium, and Tauri 2.0 are installed and configured.

**Acceptance Criteria**:
- [ ] `rustc --version` shows 1.75 or higher
- [ ] `cargo clippy -- -D warnings` runs without errors
- [ ] pqcrypto-dilithium dependency compiles successfully
- [ ] Tauri dev mode starts without errors

**Commands**:
```bash
rustc --version
cargo check --workspace
cargo clippy --all-targets
cd btpc-desktop-app && npm run tauri:dev
```

---

### T002 [P]: Configure Test Infrastructure
**File**: `btpc-core/Cargo.toml`, `btpc-desktop-app/src-tauri/Cargo.toml`
**Type**: Setup
**Dependencies**: T001
**Parallel**: Yes (different crates)

**Description**:
Add test dependencies: `tokio-test`, `temp-env`, `criterion` for benchmarks.

**Acceptance Criteria**:
- [ ] tokio-test added for async test support
- [ ] temp-env added for environment variable testing
- [ ] criterion added for performance benchmarks
- [ ] All test dependencies compile successfully

**Files to Modify**:
```toml
# btpc-core/Cargo.toml
[dev-dependencies]
tokio-test = "0.4"
criterion = "0.5"

# btpc-desktop-app/src-tauri/Cargo.toml
[dev-dependencies]
tokio-test = "0.4"
temp-env = "0.3"
```

---

## Phase 3.2: Tests First (TDD - RED Phase) ⚠️ MANDATORY

**CRITICAL**: All T003-T010 tests MUST be written and MUST FAIL before proceeding to T011+

### T003 [P]: Write failing test for ML-DSA seed-based key reconstruction
**File**: `btpc-core/src/crypto/keys.rs` (test module)
**Type**: Unit Test (RED)
**Dependencies**: T002
**Parallel**: Yes

**Description**:
Write test that creates ML-DSA keypair from seed, serializes private key, then reconstructs and signs message. Test will FAIL because `from_key_pair_bytes()` creates `keypair: None`.

**Test Code**:
```rust
#[test]
fn test_private_key_from_seed_can_sign() {
    let seed = [42u8; 32]; // Deterministic seed
    let private_key = PrivateKey::from_seed(&seed).unwrap();

    // Serialize and reconstruct
    let key_bytes = private_key.to_bytes();
    let pub_bytes = private_key.public_key().to_bytes();
    let reconstructed = PrivateKey::from_key_pair_bytes(&key_bytes, &pub_bytes).unwrap();

    // MUST be able to sign after reconstruction
    let message = b"test message";
    let signature = reconstructed.sign(message).unwrap(); // ← FAILS HERE (keypair: None)

    assert!(private_key.public_key().verify(message, &signature));
}
```

**Expected Failure**: `SignatureError::SigningFailed` at sign() call

---

### T004 [P]: Write failing test for wallet backup with walletId
**File**: `btpc-core/src/crypto/wallet_serde.rs` (test module)
**Type**: Unit Test (RED)
**Dependencies**: T002
**Parallel**: Yes

**Description**:
Write test that creates `WalletData` with walletId, encrypts/decrypts, and verifies walletId persists. Test will FAIL because `WalletData` struct lacks `wallet_id` field.

**Test Code**:
```rust
#[test]
fn test_wallet_backup_includes_wallet_id() {
    let wallet_id = "550e8400-e29b-41d4-a716-446655440000";
    let wallet_data = WalletData {
        wallet_id: wallet_id.to_string(), // ← COMPILE ERROR (field doesn't exist)
        network: "mainnet".to_string(),
        keys: vec![],
        created_at: 1234567890,
        modified_at: 1234567890,
    };

    let password = SecurePassword::new("test_password").unwrap();
    let encrypted = EncryptedWallet::encrypt(&wallet_data, &password).unwrap();
    let decrypted = encrypted.decrypt(&password).unwrap();

    assert_eq!(decrypted.wallet_id, wallet_id);
}
```

**Expected Failure**: Compile error - `WalletData` has no field `wallet_id`

---

### T005 [P]: Write failing integration test for single-input transaction signing
**File**: `btpc-desktop-app/src-tauri/tests/integration/transaction_signing.rs`
**Type**: Integration Test (RED)
**Dependencies**: T002
**Parallel**: Yes

**Description**:
Write end-to-end test: load wallet from file, create transaction with 1 input, sign, verify signature. Test will FAIL at signing step.

**Test Code**:
```rust
#[tokio::test]
async fn test_send_transaction_single_input() {
    // Setup: Create test wallet with 100 BTPC UTXO
    let wallet_manager = create_test_wallet_manager().await;
    let wallet_id = create_wallet_with_utxo(&wallet_manager, 100_0000_0000).await;

    // Act: Send 50 BTPC transaction
    let result = wallet_manager.send_transaction(
        &wallet_id,
        "BTPC1qRecipient",
        50_0000_0000,
        1000 // fee
    ).await;

    // Assert: Transaction signs successfully
    assert!(result.is_ok(), "Expected successful signing, got: {:?}", result.err());
    let tx = result.unwrap();
    assert_eq!(tx.inputs.len(), 1);
    assert!(tx.inputs[0].signature.is_some(), "Input 0 should be signed");
}
```

**Expected Failure**: `SignatureError::SigningFailed` when signing input 0

---

### T006 [P]: Write failing integration test for multi-input transaction signing
**File**: `btpc-desktop-app/src-tauri/tests/integration/transaction_signing.rs`
**Type**: Integration Test (RED)
**Dependencies**: T002
**Parallel**: Yes

**Description**:
Write test for transaction with 3 UTXOs requiring multi-input signing. Test will FAIL at first input.

**Test Code**:
```rust
#[tokio::test]
async fn test_send_transaction_multi_input() {
    let wallet_manager = create_test_wallet_manager().await;
    let wallet_id = create_wallet_with_utxos(
        &wallet_manager,
        vec![40_0000_0000, 35_0000_0000, 25_0000_0000]
    ).await;

    // Send 80 BTPC (requires 2-3 inputs)
    let result = wallet_manager.send_transaction(
        &wallet_id,
        "BTPC1qRecipient",
        80_0000_0000,
        1000
    ).await;

    assert!(result.is_ok());
    let tx = result.unwrap();
    assert!(tx.inputs.len() >= 2, "Should use multiple inputs");

    // All inputs must be signed
    for (i, input) in tx.inputs.iter().enumerate() {
        assert!(input.signature.is_some(), "Input {} should be signed", i);
    }
}
```

**Expected Failure**: `SignatureError::SigningFailed` at input 0

---

### T007 [P]: Write failing test for wallet backup restoration with walletId
**File**: `btpc-desktop-app/src-tauri/tests/integration/wallet_backup.rs`
**Type**: Integration Test (RED)
**Dependencies**: T002
**Parallel**: Yes

**Description**:
Write test: backup wallet, restore from backup, verify walletId matches. Test will FAIL because walletId not serialized.

**Test Code**:
```rust
#[tokio::test]
async fn test_wallet_backup_restoration() {
    let wallet_manager = create_test_wallet_manager().await;
    let original_wallet_id = "550e8400-e29b-41d4-a716-446655440000";
    let wallet = create_test_wallet(&wallet_manager, original_wallet_id).await;

    // Backup
    let backup_path = wallet_manager.backup_wallet(original_wallet_id).await.unwrap();

    // Restore
    let restored_wallet = wallet_manager.restore_from_backup(
        &backup_path,
        "test_password"
    ).await.unwrap();

    // Verify walletId persisted
    assert_eq!(restored_wallet.id, original_wallet_id,
        "Restored wallet should have same walletId");
}
```

**Expected Failure**: Assertion fails or deserialization error (walletId missing)

---

### T008 [P]: Write failing test for concurrent transaction UTXO reservation
**File**: `btpc-desktop-app/src-tauri/tests/integration/concurrent_transactions.rs`
**Type**: Integration Test (RED)
**Dependencies**: T002
**Parallel**: Yes

**Description**:
Write test: two concurrent send_transaction calls, verify they don't select same UTXO. Test may PASS initially, then FAIL after implementing reservation system.

**Test Code**:
```rust
#[tokio::test]
async fn test_concurrent_transactions_no_utxo_conflict() {
    let wallet_manager = Arc::new(create_test_wallet_manager().await);
    let wallet_id = create_wallet_with_utxos(
        &wallet_manager,
        vec![50_0000_0000, 50_0000_0000] // 2 separate UTXOs
    ).await;

    // Spawn two concurrent transactions
    let wm1 = wallet_manager.clone();
    let wm2 = wallet_manager.clone();
    let wid1 = wallet_id.clone();
    let wid2 = wallet_id.clone();

    let tx1 = tokio::spawn(async move {
        wm1.send_transaction(&wid1, "BTPC1qAddr1", 30_0000_0000, 1000).await
    });

    let tx2 = tokio::spawn(async move {
        wm2.send_transaction(&wid2, "BTPC1qAddr2", 30_0000_0000, 1000).await
    });

    let (result1, result2) = tokio::join!(tx1, tx2);

    // Both should succeed using different UTXOs
    assert!(result1.unwrap().is_ok());
    assert!(result2.unwrap().is_ok());

    // Verify no UTXO used twice (check transaction inputs don't overlap)
    // ...implementation of overlap check...
}
```

**Expected Behavior**: Test may fail due to race condition (both transactions select same UTXO)

---

### T009 [P]: Write failing test for transaction-broadcast event emission (Article XI.3)
**File**: `btpc-desktop-app/src-tauri/tests/integration/event_emission.rs`
**Type**: Integration Test (RED)
**Dependencies**: T002
**Parallel**: Yes

**Description**:
Write test: send transaction, verify `transaction-broadcast` event emitted. Test will FAIL because event not implemented.

**Test Code**:
```rust
#[tokio::test]
async fn test_transaction_broadcast_event_emission() {
    let (app, event_receiver) = create_test_app_with_events().await;
    let wallet_id = create_test_wallet(&app).await;

    // Send transaction
    send_transaction_command(&app, wallet_id, "BTPC1qAddr", 50_0000_0000).await.unwrap();

    // Wait for event
    let event = timeout(Duration::from_secs(2), event_receiver.recv()).await
        .expect("Event should be emitted within 2 seconds");

    assert_eq!(event.name, "transaction-broadcast");
    assert!(event.payload.contains_key("txid"));
}
```

**Expected Failure**: Timeout waiting for event (event not emitted)

---

### T010 [P]: Write failing test for backup-completed event emission (Article XI.3)
**File**: `btpc-desktop-app/src-tauri/tests/integration/event_emission.rs`
**Type**: Integration Test (RED)
**Dependencies**: T002
**Parallel**: Yes

**Description**:
Write test: backup wallet, verify `wallet-backup-completed` event emitted. Test will FAIL because event not implemented.

**Test Code**:
```rust
#[tokio::test]
async fn test_backup_completed_event_emission() {
    let (app, event_receiver) = create_test_app_with_events().await;
    let wallet_id = create_test_wallet(&app).await;

    // Backup wallet
    backup_wallet_command(&app, wallet_id).await.unwrap();

    // Wait for event
    let event = timeout(Duration::from_secs(2), event_receiver.recv()).await
        .expect("Backup event should be emitted");

    assert_eq!(event.name, "wallet-backup-completed");
    assert_eq!(event.payload["wallet_id"], wallet_id);
}
```

**Expected Failure**: Timeout (event not emitted)

---

## Phase 3.3: Core Implementation (GREEN Phase)

**GATE CHECK**: Verify T003-T010 tests exist and FAIL before proceeding

### T011: Add ML-DSA seed storage to PrivateKey struct
**File**: `btpc-core/src/crypto/keys.rs`
**Type**: Core Implementation
**Dependencies**: T003 (test must fail first)
**Parallel**: No

**Description**:
Add `seed: Option<[u8; 32]>` field to `PrivateKey` struct. Implement `from_seed()` method that stores seed and generates keypair. Update `from_key_pair_bytes()` to check for seed and regenerate keypair if available.

**Implementation**:
```rust
pub struct PrivateKey {
    key_bytes: [u8; ML_DSA_PRIVATE_KEY_SIZE],
    public_key_bytes: [u8; ML_DSA_PUBLIC_KEY_SIZE],
    seed: Option<[u8; 32]>,  // ← ADD THIS
    #[zeroize(skip)]
    keypair: Option<DilithiumKeypair>,
}

impl PrivateKey {
    /// Create PrivateKey from seed (preserves signing capability)
    pub fn from_seed(seed: &[u8; 32]) -> Result<Self, KeyError> {
        // Generate keypair from seed
        let keypair = DilithiumKeypair::generate_from_seed(seed)?;

        Ok(PrivateKey {
            key_bytes: keypair.secret_key_bytes(),
            public_key_bytes: keypair.public_key_bytes(),
            seed: Some(*seed),  // ← Store seed for reconstruction
            keypair: Some(keypair),
        })
    }

    /// Reconstruct from key bytes - regenerates keypair if seed available
    pub fn from_key_pair_bytes_with_seed(
        private_key_bytes: &[u8],
        public_key_bytes: &[u8],
        seed: Option<[u8; 32]>,
    ) -> Result<Self, KeyError> {
        let keypair = if let Some(seed_bytes) = seed {
            Some(DilithiumKeypair::generate_from_seed(&seed_bytes)?)
        } else {
            None  // No seed = can't sign (backward compat)
        };

        Ok(PrivateKey {
            key_bytes: private_key_bytes.try_into()?,
            public_key_bytes: public_key_bytes.try_into()?,
            seed,
            keypair,
        })
    }
}
```

**Acceptance Criteria**:
- [ ] T003 test now PASSES
- [ ] PrivateKey created from seed can sign messages
- [ ] Seed is zeroized on drop (security)
- [ ] Backward compatibility: old keys without seed still load (can't sign)

---

### T012: Add wallet_id field to WalletData struct
**File**: `btpc-core/src/crypto/wallet_serde.rs`
**Type**: Core Implementation
**Dependencies**: T004 (test must fail first)
**Parallel**: No

**Description**:
Add `wallet_id: String` field to `WalletData` struct. Update serialization/deserialization. Ensure backward compatibility for old wallet files.

**Implementation**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletData {
    #[serde(default = "default_wallet_id")]  // Backward compat
    pub wallet_id: String,  // ← ADD THIS
    pub network: String,
    pub keys: Vec<KeyEntry>,
    pub created_at: u64,
    pub modified_at: u64,
}

fn default_wallet_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
```

**Acceptance Criteria**:
- [ ] T004 test now PASSES
- [ ] New wallet backups include walletId
- [ ] Old wallet files load successfully (generate UUID if missing)
- [ ] Serialization format remains bincode (no breaking changes)

---

### T013: Update KeyEntry to store ML-DSA seeds
**File**: `btpc-core/src/crypto/wallet_serde.rs`
**Type**: Core Implementation
**Dependencies**: T011, T012
**Parallel**: No (same file as T012)

**Description**:
Add `seed: Option<Vec<u8>>` to `KeyEntry` struct. Update `to_private_key()` to use seed if available.

**Implementation**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEntry {
    pub label: String,
    pub private_key_bytes: Vec<u8>,
    pub public_key_bytes: Vec<u8>,
    #[serde(default)]
    pub seed: Option<Vec<u8>>,  // ← ADD THIS (32 bytes for ML-DSA)
    pub address: String,
    pub created_at: u64,
}

impl KeyEntry {
    pub fn to_private_key(&self) -> Result<PrivateKey, WalletError> {
        let seed = self.seed.as_ref().and_then(|s| {
            if s.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(s);
                Some(arr)
            } else {
                None
            }
        });

        PrivateKey::from_key_pair_bytes_with_seed(
            &self.private_key_bytes,
            &self.public_key_bytes,
            seed  // ← Use seed if available
        ).map_err(|_| WalletError::KeyReconstructionFailed)
    }
}
```

**Acceptance Criteria**:
- [ ] Keys created with seed can sign after wallet reload
- [ ] Old keys without seed still load (backward compat)
- [ ] Seed is encrypted with AES-256-GCM (part of WalletData)

---

### T014: Fix wallet_commands.rs to use seed-based key creation
**File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs`
**Type**: Desktop Implementation
**Dependencies**: T011, T013
**Parallel**: No

**Description**:
Update wallet creation code to generate seed, create PrivateKey from seed, and store seed in KeyEntry.

**Implementation**:
```rust
pub async fn create_wallet(/* ... */) -> Result<WalletInfo, String> {
    // Generate secure random seed
    let mut seed = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut seed);

    // Create PrivateKey from seed (preserves signing capability)
    let private_key = btpc_core::crypto::PrivateKey::from_seed(&seed)
        .map_err(|e| format!("Failed to generate key: {}", e))?;

    let key_entry = KeyEntry {
        label: "Default".to_string(),
        private_key_bytes: private_key.to_bytes().to_vec(),
        public_key_bytes: private_key.public_key().to_bytes().to_vec(),
        seed: Some(seed.to_vec()),  // ← Store seed for future signing
        address: private_key.to_address(&network),
        created_at: Utc::now().timestamp() as u64,
    };

    // ... rest of wallet creation
}
```

**Acceptance Criteria**:
- [ ] T005 test now PASSES (single-input transaction signing works)
- [ ] New wallets create keys with seeds
- [ ] Transaction signing no longer fails with `SigningFailed` error

---

### T015: Update WalletManager backup_wallet to include walletId
**File**: `btpc-desktop-app/src-tauri/src/wallet_manager.rs:581-599`
**Type**: Desktop Implementation
**Dependencies**: T012
**Parallel**: Yes (different file than T014)

**Description**:
Update `backup_wallet()` to serialize entire `WalletInfo` (including walletId) into backup file, not just copy encrypted .dat file.

**Implementation**:
```rust
pub fn backup_wallet(&self, wallet_id: &str) -> BtpcResult<PathBuf> {
    let wallet = self.get_wallet(wallet_id)?;

    // Load encrypted wallet data
    let encrypted_wallet = EncryptedWallet::load_from_file(&wallet.file_path)?;

    // Create backup with metadata
    let backup_data = WalletBackupData {
        wallet_id: wallet.id.clone(),  // ← Include walletId
        nickname: wallet.nickname.clone(),
        encrypted_wallet,
        backup_version: 1,
        created_at: Utc::now().timestamp() as u64,
    };

    let backup_filename = format!("backup_{}_{}.btpc", wallet.nickname, timestamp);
    let backup_path = self.config.backups_dir.join(&backup_filename);

    // Serialize to bincode
    let serialized = bincode::serialize(&backup_data)
        .map_err(|_| BtpcError::SerializationFailed)?;

    std::fs::write(&backup_path, serialized)?;
    Ok(backup_path)
}
```

**Acceptance Criteria**:
- [ ] T007 test now PASSES (backup restoration includes walletId)
- [ ] Backup files contain walletId field
- [ ] Restoration verifies walletId matches

---

### T016: Implement optimistic UTXO reservation system
**File**: `btpc-desktop-app/src-tauri/src/utxo_manager.rs`
**Type**: Core Implementation
**Dependencies**: T008 (test must fail/show race condition first)
**Parallel**: Yes (different file)

**Description**:
Add UTXO reservation mechanism: mark UTXOs as "reserved" during transaction creation, release on success/failure. Prevents concurrent transactions from selecting same UTXO.

**Implementation**:
```rust
pub struct UTXOManager {
    storage: Arc<RwLock<UTXOStorage>>,
    reserved_utxos: Arc<RwLock<HashSet<(Txid, u32)>>>,  // ← ADD THIS
}

impl UTXOManager {
    /// Reserve UTXOs for transaction (optimistic locking)
    pub async fn reserve_utxos(&self, utxos: &[(Txid, u32)]) -> Result<ReservationToken, Error> {
        let mut reserved = self.reserved_utxos.write().unwrap();

        // Check if any UTXO already reserved
        for utxo in utxos {
            if reserved.contains(utxo) {
                return Err(Error::UtxoAlreadyReserved);
            }
        }

        // Reserve all
        for utxo in utxos {
            reserved.insert(*utxo);
        }

        Ok(ReservationToken { utxos: utxos.to_vec(), manager: self.clone() })
    }

    /// Release UTXO reservation (on drop or explicit call)
    pub async fn release_utxos(&self, utxos: &[(Txid, u32)]) {
        let mut reserved = self.reserved_utxos.write().unwrap();
        for utxo in utxos {
            reserved.remove(utxo);
        }
    }
}

/// RAII token that releases UTXOs on drop
pub struct ReservationToken {
    utxos: Vec<(Txid, u32)>,
    manager: UTXOManager,
}

impl Drop for ReservationToken {
    fn drop(&mut self) {
        // Release on drop (transaction success or failure)
        tokio::spawn(self.manager.release_utxos(&self.utxos));
    }
}
```

**Acceptance Criteria**:
- [ ] T008 test now PASSES (concurrent transactions use different UTXOs)
- [ ] Race condition eliminated
- [ ] UTXOs auto-released on transaction failure (RAII pattern)

---

### T017: Update send_transaction to use UTXO reservation
**File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs:189-208`
**Type**: Desktop Implementation
**Dependencies**: T016
**Parallel**: No (same file as T014)

**Description**:
Integrate UTXO reservation into transaction creation flow.

**Implementation**:
```rust
pub async fn send_btpc_from_wallet(/* ... */) -> Result<String, String> {
    // ... UTXO selection ...

    // Reserve UTXOs before signing (prevents race condition)
    let reservation = utxo_manager.reserve_utxos(&selected_utxos).await
        .map_err(|e| format!("UTXO reservation failed: {}", e))?;

    // Sign transaction (reservation auto-released on drop if this fails)
    let signed_tx = sign_transaction(&transaction, &private_key)
        .map_err(|e| format!("Signing failed: {}", e))?;

    // Broadcast
    let txid = rpc_client.sendrawtransaction(&signed_tx).await
        .map_err(|e| {
            // Reservation auto-released here (drop)
            format!("Broadcast failed: {}", e)
        })?;

    // Mark as spent AFTER successful broadcast
    utxo_manager.mark_utxos_spent(&selected_utxos).await?;

    // Reservation released here (success - drop)
    Ok(txid)
}
```

**Acceptance Criteria**:
- [ ] Concurrent transactions don't conflict
- [ ] Failed transactions release UTXO reservations
- [ ] Successful transactions mark UTXOs as spent

---

### T018: Implement transaction-broadcast event emission (Article XI.3)
**File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs`
**Type**: Desktop Implementation (Article XI)
**Dependencies**: T014, T017
**Parallel**: No (same file)

**Description**:
Emit `transaction-broadcast` event after successful transaction broadcast.

**Implementation**:
```rust
pub async fn send_btpc_from_wallet(
    app: AppHandle,  // ← Need AppHandle for event emission
    state: State<'_, AppState>,
    /* ... */
) -> Result<String, String> {
    // ... transaction creation and broadcast ...

    let txid = rpc_client.sendrawtransaction(&signed_tx).await?;

    // Emit event (Article XI.3)
    app.emit("transaction-broadcast", json!({
        "txid": txid,
        "wallet_id": wallet_id,
        "amount": amount,
        "recipient": to_address,
        "inputs_signed": transaction.inputs.len(),
        "timestamp": Utc::now().timestamp(),
    })).map_err(|e| format!("Event emission failed: {}", e))?;

    Ok(txid)
}
```

**Acceptance Criteria**:
- [ ] T009 test now PASSES (event emitted on broadcast)
- [ ] Event payload includes txid, wallet_id, amount
- [ ] Frontend can listen for transaction-broadcast events

---

### T019 [P]: Implement wallet-backup-completed event emission (Article XI.3)
**File**: `btpc-desktop-app/src-tauri/src/wallet_manager.rs`
**Type**: Desktop Implementation (Article XI)
**Dependencies**: T015
**Parallel**: Yes (different file than T018)

**Description**:
Emit `wallet-backup-completed` event after successful backup creation.

**Implementation**:
```rust
pub fn backup_wallet(
    &self,
    wallet_id: &str,
    app: &AppHandle,  // ← Need for event
) -> BtpcResult<PathBuf> {
    // ... backup creation ...

    std::fs::write(&backup_path, serialized)?;

    // Emit event (Article XI.3)
    app.emit("wallet-backup-completed", json!({
        "wallet_id": wallet_id,
        "backup_path": backup_path.to_string_lossy(),
        "backup_size_bytes": serialized.len(),
        "timestamp": Utc::now().timestamp(),
    })).map_err(|_| BtpcError::EventEmissionFailed)?;

    Ok(backup_path)
}
```

**Acceptance Criteria**:
- [ ] T010 test now PASSES (event emitted on backup)
- [ ] Event payload includes backup_path, size
- [ ] Frontend can listen for wallet-backup-completed events

---

### T020 [P]: Add frontend event listener for transaction-broadcast
**File**: `btpc-desktop-app/ui/transactions.html`
**Type**: Frontend Implementation (Article XI)
**Dependencies**: T018
**Parallel**: Yes (different file)

**Description**:
Add event listener in transactions page to update UI when transaction broadcasts.

**Implementation**:
```javascript
// transactions.html <script> section
import { listen } from '@tauri-apps/api/event';
import { EventListenerManager } from './btpc-event-manager.js';

async function initTransactionListeners() {
    const unlisten = await listen('transaction-broadcast', (event) => {
        const { txid, amount, recipient, timestamp } = event.payload;

        // Update transaction history table
        addTransactionToHistory({
            txid,
            amount,
            recipient,
            status: 'broadcast',
            timestamp
        });

        // Show success toast
        showToast('Transaction broadcast successfully', 'success');

        // Refresh balance
        refreshWalletBalance();
    });

    // Register for cleanup (Article XI.6)
    EventListenerManager.register('transaction-broadcast', unlisten);
}

// Initialize on page load
document.addEventListener('DOMContentLoaded', initTransactionListeners);

// Cleanup on unload (Article XI.6)
window.addEventListener('beforeunload', () => {
    EventListenerManager.cleanup();
});
```

**Acceptance Criteria**:
- [ ] UI updates when transaction broadcasts
- [ ] Event listener cleaned up on page unload (Article XI.6)
- [ ] No memory leaks from forgotten listeners

---

### T021 [P]: Add frontend event listener for wallet-backup-completed
**File**: `btpc-desktop-app/ui/wallet-manager.html`
**Type**: Frontend Implementation (Article XI)
**Dependencies**: T019
**Parallel**: Yes (different file than T020)

**Description**:
Add event listener in wallet manager page to show backup confirmation.

**Implementation**:
```javascript
// wallet-manager.html <script> section
async function initBackupListeners() {
    const unlisten = await listen('wallet-backup-completed', (event) => {
        const { wallet_id, backup_path, backup_size_bytes } = event.payload;

        // Show success notification
        showToast(`Wallet backed up to ${backup_path}`, 'success');

        // Update backup history list
        addBackupToHistory({
            wallet_id,
            path: backup_path,
            size: formatBytes(backup_size_bytes),
            date: new Date()
        });
    });

    EventListenerManager.register('wallet-backup-completed', unlisten);
}

document.addEventListener('DOMContentLoaded', initBackupListeners);
window.addEventListener('beforeunload', () => EventListenerManager.cleanup());
```

**Acceptance Criteria**:
- [ ] UI shows backup confirmation
- [ ] Event listener cleaned up on unload
- [ ] Follows Article XI.6 patterns

---

## Phase 3.4: Integration & Verification

### T022: Verify all tests pass (GREEN Phase validation)
**File**: N/A (test execution)
**Type**: Verification
**Dependencies**: T011-T021
**Parallel**: No

**Description**:
Run full test suite to verify all RED phase tests now PASS with implementation complete.

**Commands**:
```bash
# Unit tests
cargo test --package btpc-core --lib crypto::keys::tests
cargo test --package btpc-core --lib crypto::wallet_serde::tests

# Integration tests
cargo test --package btpc-desktop-app --test transaction_signing
cargo test --package btpc-desktop-app --test wallet_backup
cargo test --package btpc-desktop-app --test concurrent_transactions
cargo test --package btpc-desktop-app --test event_emission

# Full suite
cargo test --workspace
```

**Acceptance Criteria**:
- [ ] All T003-T010 tests PASS (were failing before implementation)
- [ ] No new test failures introduced
- [ ] Test coverage >90% for modified files

---

### T023: Run manual quickstart scenarios
**File**: `specs/005-fix-transaction-signing/quickstart.md` (if exists, otherwise manual)
**Type**: Manual Testing
**Dependencies**: T022
**Parallel**: No

**Description**:
Execute manual test scenarios from plan.md:

1. **Single-Input Transaction**: Create wallet with 100 BTPC, send 50 BTPC → should succeed
2. **Multi-Input Transaction**: Create wallet with 3 UTXOs (40+35+25), send 80 BTPC → should succeed
3. **Wallet Backup**: Backup wallet → file should contain walletId
4. **Wallet Restoration**: Restore from backup → walletId should match
5. **Concurrent Transactions**: Two simultaneous sends → no UTXO conflict

**Acceptance Criteria**:
- [ ] All 5 scenarios pass without errors
- [ ] No "Signature creation failed" errors
- [ ] No "missing required key walletId" errors

---

### T024: Performance benchmarking
**File**: `btpc-core/benches/signing_benchmark.rs`, `btpc-desktop-app/benches/transaction_benchmark.rs`
**Type**: Performance Test
**Dependencies**: T022
**Parallel**: Yes

**Description**:
Create criterion benchmarks to verify performance targets from spec.

**Benchmark Code**:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_single_input_signing(c: &mut Criterion) {
    let seed = [42u8; 32];
    let private_key = PrivateKey::from_seed(&seed).unwrap();
    let message = b"transaction data";

    c.bench_function("ML-DSA single-input sign", |b| {
        b.iter(|| {
            private_key.sign(black_box(message)).unwrap()
        });
    });
}

fn bench_multi_input_signing(c: &mut Criterion) {
    let seed = [42u8; 32];
    let private_key = PrivateKey::from_seed(&seed).unwrap();
    let messages: Vec<_> = (0..10).map(|i| format!("tx:{}", i)).collect();

    c.bench_function("ML-DSA 10-input transaction sign", |b| {
        b.iter(|| {
            for msg in &messages {
                private_key.sign(black_box(msg.as_bytes())).unwrap();
            }
        });
    });
}

criterion_group!(benches, bench_single_input_signing, bench_multi_input_signing);
criterion_main!(benches);
```

**Performance Targets** (from spec.md NFR-005, NFR-006):
- [ ] Single-input signing: <50ms
- [ ] Multi-input (10 inputs) signing: <500ms
- [ ] Wallet backup: <2 seconds

**Run Command**:
```bash
cargo bench --package btpc-core signing_benchmark
cargo bench --package btpc-desktop-app transaction_benchmark
```

---

## Phase 3.5: Polish & Documentation

### T025 [P]: Run cargo clippy and fix warnings
**File**: Workspace-wide
**Type**: Code Quality
**Dependencies**: T022
**Parallel**: Yes

**Description**:
Run clippy with strict warnings, fix all issues.

**Commands**:
```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --fix --allow-dirty --allow-staged
```

**Common Issues to Fix**:
- [ ] Unused imports
- [ ] Unnecessary `.clone()` calls
- [ ] Missing documentation on public items
- [ ] Potential panic points (unwrap → proper error handling)

---

### T026 [P]: Add inline documentation to modified functions
**File**: All files modified in T011-T021
**Type**: Documentation
**Dependencies**: T025
**Parallel**: Yes (different concern than clippy)

**Description**:
Add `///` doc comments to all new public functions following Rust conventions.

**Example**:
```rust
/// Creates a new PrivateKey from a 32-byte seed.
///
/// This method generates a deterministic ML-DSA keypair from the provided seed,
/// allowing the keypair to be reconstructed for transaction signing after
/// deserialization from wallet storage.
///
/// # Arguments
/// * `seed` - A 32-byte array used for deterministic key generation
///
/// # Returns
/// * `Ok(PrivateKey)` - A new PrivateKey with signing capability
/// * `Err(KeyError)` - If keypair generation from seed fails
///
/// # Example
/// ```
/// let seed = [42u8; 32];
/// let key = PrivateKey::from_seed(&seed)?;
/// let signature = key.sign(b"message")?;
/// ```
pub fn from_seed(seed: &[u8; 32]) -> Result<Self, KeyError> {
    // ...
}
```

**Acceptance Criteria**:
- [ ] All public functions have `///` doc comments
- [ ] Examples included for complex functions
- [ ] Safety notes for crypto functions
- [ ] `cargo doc --no-deps --open` generates clean docs

---

### T027: Update CLAUDE.md with feature details
**File**: `/home/bob/BTPC/BTPC/CLAUDE.md`
**Type**: Documentation
**Dependencies**: T024
**Parallel**: No (single file, sequential update)

**Description**:
Execute `.specify/scripts/bash/update-agent-context.sh claude` to incrementally update CLAUDE.md.

**Command**:
```bash
.specify/scripts/bash/update-agent-context.sh claude
```

**Manual Additions** (between `<!-- MANUAL ADDITIONS START -->` and `<!-- MANUAL ADDITIONS END -->`):
```markdown
### Feature 005: Transaction Signing & Wallet Backup Fix (Completed 2025-10-25)
**Problem**: ML-DSA keypair reconstruction limitation caused all transaction signing to fail.

**Solution Implemented**:
- Store ML-DSA key generation seed (32 bytes) in wallet
- Regenerate keypair from seed on wallet load
- Add `wallet_id` field to `WalletData` for backup integrity
- Implement UTXO reservation system for concurrent transaction safety
- Add event-driven updates (Article XI compliance)

**Files Modified**:
- `btpc-core/src/crypto/keys.rs` - Seed-based PrivateKey creation
- `btpc-core/src/crypto/wallet_serde.rs` - WalletData.wallet_id field
- `btpc-desktop-app/src-tauri/src/wallet_commands.rs` - Transaction signing + events
- `btpc-desktop-app/src-tauri/src/wallet_manager.rs` - Backup with walletId
- `btpc-desktop-app/src-tauri/src/utxo_manager.rs` - UTXO reservation
- `btpc-desktop-app/ui/transactions.html` - Event listeners
- `btpc-desktop-app/ui/wallet-manager.html` - Event listeners
```

**Acceptance Criteria**:
- [ ] CLAUDE.md updated with new patterns
- [ ] Manual additions preserved
- [ ] File size remains <150 lines (token efficiency)

---

### T028: Verify Article XI compliance
**File**: N/A (compliance audit)
**Type**: Verification (Article XI)
**Dependencies**: T020, T021, T027
**Parallel**: No

**Description**:
Audit implementation against Article XI checklist from spec.md.

**Article XI Compliance Checklist**:
- [ ] **Section 11.1 - Single Source of Truth**: Wallet state in backend (Rust), not frontend
- [ ] **Section 11.2 - Backend-First Validation**: send_transaction and backup_wallet validate before UI updates
- [ ] **Section 11.3 - Event-Driven Architecture**: transaction-broadcast and wallet-backup-completed events emitted
- [ ] **Section 11.6 - Event Listener Cleanup**: EventListenerManager.cleanup() called on beforeunload
- [ ] **Section 11.7 - Prohibited Patterns**: No localStorage before backend validation, no polling

**Verification Method**:
1. Code review of modified files
2. Test event emission (T009, T010)
3. Verify cleanup (inspect browser DevTools → Memory)

---

## Dependencies Summary

```
Setup Phase:
T001 (env check)
  └─> T002 [P] (test deps)

RED Phase (TDD - Tests First):
T002
  ├─> T003 [P] (ML-DSA seed test)
  ├─> T004 [P] (walletId test)
  ├─> T005 [P] (single-input signing test)
  ├─> T006 [P] (multi-input signing test)
  ├─> T007 [P] (backup restoration test)
  ├─> T008 [P] (concurrent UTXO test)
  ├─> T009 [P] (transaction event test)
  └─> T010 [P] (backup event test)

GREEN Phase (Implementation):
T003 → T011 (add seed to PrivateKey)
T004 → T012 (add wallet_id to WalletData)
T011 → T013 (update KeyEntry with seed)
T011, T013 → T014 (wallet_commands seed usage)
T012 → T015 [P] (backup_wallet with walletId)
T008 → T016 [P] (UTXO reservation)
T016 → T017 (use UTXO reservation in send_transaction)
T014, T017 → T018 (transaction-broadcast event)
T015 → T019 [P] (wallet-backup-completed event)
T018 → T020 [P] (frontend transaction listener)
T019 → T021 [P] (frontend backup listener)

Verification Phase:
T011-T021 → T022 (test suite)
T022 → T023 (manual testing)
T022 → T024 [P] (benchmarks)

Polish Phase:
T022 → T025 [P] (clippy)
T025 → T026 [P] (documentation)
T024 → T027 (update CLAUDE.md)
T020, T021, T027 → T028 (Article XI audit)
```

---

## Parallel Execution Examples

**RED Phase (all tests in parallel)**:
```bash
# Launch T003-T010 concurrently (8 independent test files):
cargo test --lib crypto::keys::test_private_key_from_seed_can_sign &
cargo test --lib crypto::wallet_serde::test_wallet_backup_includes_wallet_id &
cargo test --test transaction_signing::test_send_transaction_single_input &
cargo test --test transaction_signing::test_send_transaction_multi_input &
cargo test --test wallet_backup::test_wallet_backup_restoration &
cargo test --test concurrent_transactions::test_concurrent_transactions_no_utxo_conflict &
cargo test --test event_emission::test_transaction_broadcast_event_emission &
cargo test --test event_emission::test_backup_completed_event_emission &
wait
```

**GREEN Phase (parallel implementations)**:
```bash
# T015, T016, T019 can run in parallel (different files):
# Terminal 1: T015
vim btpc-desktop-app/src-tauri/src/wallet_manager.rs  # Implement backup with walletId

# Terminal 2: T016
vim btpc-desktop-app/src-tauri/src/utxo_manager.rs  # Implement UTXO reservation

# Terminal 3: T019
vim btpc-desktop-app/src-tauri/src/wallet_manager.rs  # Add event emission (same file as T015, coordinate)
```

**Frontend Phase (parallel UI updates)**:
```bash
# T020, T021 can run in parallel (different HTML files):
# Terminal 1:
vim btpc-desktop-app/ui/transactions.html  # Add transaction-broadcast listener

# Terminal 2:
vim btpc-desktop-app/ui/wallet-manager.html  # Add wallet-backup-completed listener
```

---

## Task Execution Checklist

**Before Starting Implementation**:
- [ ] All T003-T010 tests written
- [ ] All T003-T010 tests FAIL (RED phase verified)
- [ ] No implementation code written yet

**During Implementation**:
- [ ] Complete tasks in dependency order
- [ ] Run `cargo test` after each task
- [ ] Commit after each task with message: "Txxx: <description>"
- [ ] Never skip tests (TDD mandatory per Article VI.3)

**After Implementation**:
- [ ] All tests PASS (GREEN phase)
- [ ] `cargo clippy -- -D warnings` clean
- [ ] Performance benchmarks meet targets
- [ ] Article XI compliance verified
- [ ] CLAUDE.md updated

---

## Notes

**TDD Enforcement** (Article VI.3):
- Tests (T003-T010) MUST be written before implementation (T011-T021)
- Tests MUST FAIL initially (RED phase)
- Implementation makes tests PASS (GREEN phase)
- Violating TDD order is constitutional violation

**Article XI Compliance** (Desktop Features):
- T018, T019: Event emission on state changes (Section 11.3)
- T020, T021: Frontend event listeners (Section 11.3)
- T028: Compliance audit (all sections)

**Parallel Execution**:
- [P] tasks = different files, can run simultaneously
- Non-[P] tasks = sequential dependencies or same file

**Security**:
- Seeds must be zeroized on drop
- No seeds in logs or error messages
- Constant-time crypto operations

**Performance Targets**:
- Single-input signing: <50ms (NFR-005)
- 10-input signing: <500ms (NFR-006)
- Wallet backup: <2s (NFR-007)

---

**Generated**: 2025-10-25
**Template Version**: 1.1 (BTPC-specific)
**Constitution**: Article VI.3 (TDD), Article VIII (ML-DSA), Article XI (Desktop Patterns)
**Total Tasks**: 28
**Estimated Duration**: 12-16 hours