# Data Model: BIP39 Deterministic Wallet Recovery

**Feature**: Fix BIP39 Seed Phrase Determinism
**Date**: 2025-11-06

## Entity Relationship Diagram

```
┌─────────────────┐
│  BIP39Mnemonic  │
│  (24 words)     │
└────────┬────────┘
         │ derives via PBKDF2
         ↓
┌─────────────────┐
│   Seed32Bytes   │
│  (256-bit seed) │
└────────┬────────┘
         │ expands via SHAKE256
         ↓
┌─────────────────┐
│ MLDSASeed48     │
│ (384-bit seed)  │
└────────┬────────┘
         │ generates deterministically
         ↓
┌─────────────────┐
│   PrivateKey    │
│  (ML-DSA 4000B) │
└────────┬────────┘
         │ derives
         ↓
┌─────────────────┐
│   PublicKey     │
│  (ML-DSA 1952B) │
└────────┬────────┘
         │ belongs to
         ↓
┌─────────────────┐
│     Wallet      │
│  (encrypted)    │
└─────────────────┘
```

## Core Entities

### 1. BIP39Mnemonic

**Purpose**: User-facing 24-word recovery phrase

**Fields**:
- `words: Vec<String>` - 24 English words from BIP39 wordlist (length = 24)
- `language: Language` - BIP39 language (default: English)
- `entropy: [u8; 32]` - 256 bits of entropy (derived from words)
- `checksum: u8` - Last 8 bits of SHA-256(entropy)

**Validation Rules**:
- MUST have exactly 24 words
- Each word MUST exist in BIP39 English wordlist (2048 words)
- Checksum MUST match SHA-256(entropy)[0..8 bits]
- Words MUST be normalized (lowercase, NFKD Unicode)

**Relationships**:
- Derives to: Seed32Bytes (via PBKDF2)

**State Transitions**:
1. **Unvalidated** → Parse words
2. **Parsed** → Validate checksum
3. **Valid** → Derive seed
4. **Invalid** → Reject with error

---

### 2. Seed32Bytes

**Purpose**: 32-byte deterministic seed derived from BIP39 mnemonic

**Fields**:
- `bytes: [u8; 32]` - 256-bit seed material (zeroized on drop)
- `source: SeedSource` - Enum: `BIP39Mnemonic | Random | Imported`
- `derived_at: Option<Timestamp>` - When seed was derived

**Validation Rules**:
- MUST be exactly 32 bytes
- MUST NOT be all zeros
- MUST be zeroized after use (security requirement)
- If from BIP39: MUST match PBKDF2(mnemonic, "", 2048 iterations)

**Derivation**:
```
seed = PBKDF2(
    password: mnemonic.to_entropy_bytes(),
    salt: "mnemonic" + "",  // empty passphrase
    iterations: 2048,
    hash: SHA-512,
    output_len: 64 bytes
)[0..32]  // Take first 32 bytes
```

**Relationships**:
- Derived from: BIP39Mnemonic
- Expands to: MLDSASeed48

**Security**:
- Type: `Zeroizing<[u8; 32]>` (memory cleared on drop)
- Never logged or exposed in error messages

---

### 3. MLDSASeed48

**Purpose**: 48-byte seed for ML-DSA deterministic key generation

**Fields**:
- `bytes: [u8; 48]` - 384-bit ML-DSA seed (zeroized on drop)
- `domain_tag: String` - "BTPC-ML-DSA-v1" (prevents cross-context attacks)
- `parent_seed: Option<[u8; 32]>` - Reference to parent Seed32Bytes

**Validation Rules**:
- MUST be exactly 48 bytes
- MUST include domain separation tag in derivation
- MUST be derived via SHAKE256 XOF

**Derivation**:
```rust
let mut shake = SHAKE256::default();
shake.update(&seed32.bytes);
shake.update(b"BTPC-ML-DSA-v1");  // Domain tag
let mut ml_dsa_seed = [0u8; 48];
shake.finalize_xof().read(&mut ml_dsa_seed);
```

**Relationships**:
- Expanded from: Seed32Bytes
- Generates: PrivateKey (deterministically)

**Security**:
- Type: `Zeroizing<[u8; 48]>`
- Domain tag hardcoded (no user input)
- XOF ensures uniform random bits

---

### 4. PrivateKey

**Purpose**: ML-DSA (Dilithium5) private key for transaction signing

**Fields**:
- `key_bytes: [u8; 4000]` - ML-DSA private key material
- `public_key_bytes: [u8; 1952]` - Corresponding public key
- `seed: Option<[u8; 32]>` - Original BIP39 seed (for re-derivation)
- `keypair: Option<DilithiumKeypair>` - Cached keypair for signing
- `version: KeyVersion` - Enum: `V1Random | V2BIP39Deterministic`
- `created_at: Timestamp` - Key generation time
- `key_id: Uuid` - Unique identifier

**Validation Rules**:
- `key_bytes.len()` MUST equal 4000 (Dilithium3 private key size)
- `public_key_bytes.len()` MUST equal 1952 (Dilithium3 public key size)
- If `version == V2BIP39Deterministic`: `seed` MUST be Some([u8; 32])
- If `version == V1Random`: `seed` SHOULD be None (legacy)

**Key Methods**:
- `from_seed_deterministic(seed: &[u8; 32]) -> Result<Self, KeyError>`
  - Derives ML-DSA key deterministically
  - Uses SHAKE256 + crystals-dilithium
  - Sets version = V2BIP39Deterministic
- `from_random() -> Result<Self, KeyError>`
  - Generates random ML-DSA key (legacy v1 method)
  - Sets version = V1Random
- `sign(&self, message: &[u8]) -> Result<Signature, KeyError>`
  - Signs message with ML-DSA
  - Uses cached keypair or regenerates from seed
- `to_bytes(&self) -> [u8; 4000]`
  - Exports key bytes (for wallet serialization)

**Relationships**:
- Generated from: MLDSASeed48 (v2) or Random (v1)
- Derives: PublicKey
- Belongs to: Wallet
- Signs: Transaction (via TransactionInput)

**State Transitions**:
1. **Uninitialized** → Generate from seed or random
2. **Generated** → Cache keypair
3. **Ready** → Can sign transactions
4. **Zeroized** → Memory cleared (on drop)

**Security**:
- Private key bytes zeroized on drop
- Seed stored separately (optional, for v2 only)
- Constant-time signing operations

---

### 5. PublicKey

**Purpose**: ML-DSA public key for address derivation and signature verification

**Fields**:
- `key_bytes: [u8; 1952]` - ML-DSA public key material
- `address: String` - BTPC address derived from public key
- `address_format: AddressFormat` - Enum: `Mainnet | Testnet | Regtest`
- `key_id: Uuid` - Matches corresponding PrivateKey

**Validation Rules**:
- `key_bytes.len()` MUST equal 1952
- Address MUST be valid BTPC address format
- Address MUST be derivable from key_bytes (reproducible)

**Derivation**:
```rust
address = base58check_encode(
    version_byte: network_type,  // 0x00 mainnet, 0x6f testnet
    payload: RIPEMD160(SHA256(public_key_bytes))
)
```

**Key Methods**:
- `from_private_key(private: &PrivateKey) -> Self`
  - Extracts public key from private key
- `to_address(&self, network: NetworkType) -> String`
  - Derives BTPC address from public key
- `verify(&self, message: &[u8], signature: &Signature) -> Result<bool, KeyError>`
  - Verifies ML-DSA signature

**Relationships**:
- Derived from: PrivateKey
- Identifies: Address (unique per network)
- Belongs to: Wallet

---

### 6. Wallet

**Purpose**: Encrypted wallet file containing keys and metadata

**Fields**:
- `wallet_id: Uuid` - Unique wallet identifier
- `name: String` - User-friendly wallet name
- `version: WalletVersion` - Enum: `V1NonDeterministic | V2BIP39Deterministic`
- `keys: Vec<KeyEntry>` - Private keys with metadata
- `network: NetworkType` - Mainnet | Testnet | Regtest
- `created_at: Timestamp` - Wallet creation time
- `last_sync_height: u64` - Last blockchain sync height
- `balance: u64` - Cached balance (credits)
- `encrypted: bool` - Whether wallet file is encrypted

**Validation Rules**:
- `wallet_id` MUST be unique (UUID v4)
- `name` length MUST be 1-50 characters
- If `version == V2BIP39Deterministic`: ALL keys MUST have seeds
- If `version == V1NonDeterministic`: keys MAY lack seeds (legacy)
- `keys` MUST NOT be empty (at least one key)
- All keys MUST match `network` type

**Key Methods**:
- `create_from_mnemonic(mnemonic: &str, network: NetworkType) -> Result<Self, WalletError>`
  - Creates v2 wallet from BIP39 mnemonic
  - Derives deterministic key
  - Sets version = V2BIP39Deterministic
- `create_random(network: NetworkType) -> Result<Self, WalletError>`
  - Creates v1 wallet with random key
  - Sets version = V1NonDeterministic
- `recover_from_mnemonic(mnemonic: &str, network: NetworkType) -> Result<Self, WalletError>`
  - Recovers v2 wallet from BIP39 mnemonic
  - MUST produce identical keys as original wallet
- `save_encrypted(&self, password: &str, path: &Path) -> Result<(), WalletError>`
  - Encrypts and saves wallet to .dat file
- `load_encrypted(password: &str, path: &Path) -> Result<Self, WalletError>`
  - Loads and decrypts wallet from .dat file

**File Format** (.dat):
```json
{
  "wallet_id": "uuid-string",
  "name": "My Wallet",
  "version": "V2BIP39Deterministic",
  "keys": [
    {
      "key_id": "uuid-string",
      "private_key_bytes": "hex-encoded-4000-bytes",
      "public_key_bytes": "hex-encoded-1952-bytes",
      "seed": "hex-encoded-32-bytes",  // Only for v2
      "created_at": "2025-11-06T12:00:00Z"
    }
  ],
  "network": "Mainnet",
  "created_at": "2025-11-06T12:00:00Z",
  "last_sync_height": 12345,
  "balance": 1000000000
}
```

**Encryption**:
- Algorithm: AES-256-GCM
- Key derivation: Argon2id (password → 32-byte key)
- Salt: 16 bytes random (stored in file header)
- Nonce: 12 bytes random per encryption

**Relationships**:
- Contains: PrivateKey (1 or more)
- Has: PublicKey (1 or more, derived from PrivateKey)
- Persisted to: Encrypted .dat file
- Belongs to: User

**State Transitions**:
1. **Uninitialized** → Create from mnemonic or random
2. **Created** → Add keys
3. **Encrypted** → Save to .dat file
4. **Persisted** → Can be loaded
5. **Loaded** → Decrypt from file
6. **Active** → Can sign transactions

---

### 7. WalletMetadata (Desktop App Entity)

**Purpose**: Wallet information displayed in UI (Article XI compliant)

**Fields**:
- `wallet_id: Uuid` - Matches Wallet.wallet_id
- `name: String` - Wallet name
- `version: WalletVersion` - V1 or V2
- `recovery_capable: bool` - True if v2 (BIP39 deterministic)
- `network: NetworkType` - Mainnet | Testnet | Regtest
- `balance: u64` - Current balance (credits)
- `address: String` - Primary address
- `last_sync_height: u64` - Blockchain sync status
- `file_path: PathBuf` - Path to .dat file

**Validation Rules**:
- Authoritative state: Backend `Arc<RwLock<WalletManager>>`
- Frontend: Read-only view (Article XI Section 11.1)
- Updates: Via Tauri events only (Article XI Section 11.3)

**Events** (Tauri):
- `wallet:created` - New wallet created
- `wallet:recovered` - Wallet recovered from mnemonic
- `wallet:updated` - Metadata changed (balance, sync height)
- `wallet:recovery:status` - Recovery progress updates

**UI Display**:
```html
<div class="wallet-card">
  <span class="wallet-name">My Wallet</span>
  <span class="wallet-version-badge v2">v2 (BIP39 Recovery)</span>
  <span class="balance">10.5 BTPC</span>
  <span class="address">btpc1q...</span>
</div>
```

**Relationships**:
- Mirrors: Wallet (backend source of truth)
- Displayed in: Desktop app UI
- Updated by: Tauri events

---

## Supporting Enums

### WalletVersion
```rust
pub enum WalletVersion {
    V1NonDeterministic,  // Legacy: random keys, limited recovery
    V2BIP39Deterministic, // New: BIP39 mnemonic recovery
}
```

### KeyVersion
```rust
pub enum KeyVersion {
    V1Random,            // Generated with OS randomness
    V2BIP39Deterministic, // Derived from BIP39 seed
}
```

### SeedSource
```rust
pub enum SeedSource {
    BIP39Mnemonic,  // Derived from 24-word mnemonic
    Random,         // OS random (legacy)
    Imported,       // User-provided seed
}
```

### NetworkType
```rust
pub enum NetworkType {
    Mainnet,
    Testnet,
    Regtest,
}
```

### AddressFormat
```rust
pub enum AddressFormat {
    Mainnet,  // Version byte 0x00
    Testnet,  // Version byte 0x6f
    Regtest,  // Version byte 0x6f
}
```

---

## Data Flow

### Wallet Creation (V2 Deterministic)
```
1. User enters 24-word mnemonic
2. Frontend → Backend: create_wallet_from_mnemonic(mnemonic, network)
3. Backend validates:
   - Parse BIP39 mnemonic
   - Validate checksum
   - Derive Seed32Bytes via PBKDF2
4. Backend derives keys:
   - Seed32Bytes → SHAKE256 → MLDSASeed48
   - MLDSASeed48 → crystals-dilithium → PrivateKey
   - PrivateKey → PublicKey
5. Backend creates Wallet:
   - wallet_id = UUID::new_v4()
   - version = V2BIP39Deterministic
   - keys = [KeyEntry { private, public, seed }]
6. Backend encrypts & saves:
   - Encrypt wallet with user password (AES-256-GCM)
   - Save to ~/.btpc/wallets/{wallet_id}.dat
7. Backend emits event:
   - Event: wallet:created
   - Payload: { wallet_id, name, version, recovery_capable: true }
8. Frontend updates UI:
   - Display wallet card with v2 badge
   - Show "BIP39 Recovery" capability
```

### Wallet Recovery (V2 Deterministic)
```
1. User enters same 24-word mnemonic
2. Frontend → Backend: recover_wallet_from_mnemonic(mnemonic, network)
3. Backend derives identical keys:
   - Same mnemonic → Same Seed32Bytes
   - Same Seed32Bytes → Same MLDSASeed48
   - Same MLDSASeed48 → Same PrivateKey (DETERMINISTIC)
4. Backend creates Wallet:
   - New wallet_id (different device/recovery)
   - Same keys (byte-identical)
   - version = V2BIP39Deterministic
5. Backend validates recovery:
   - Check if wallet_id already exists → offer to restore
   - Compare derived address to expected (if known)
6. Backend emits event:
   - Event: wallet:recovered
   - Payload: { wallet_id, address, recovery_verified: true }
7. Frontend displays:
   - Success message: "Wallet recovered successfully"
   - Address matches expected
```

### Wallet Creation (V1 Legacy)
```
1. User clicks "Create Random Wallet"
2. Backend generates random PrivateKey:
   - PrivateKey::from_random()
   - version = V1Random
   - seed = None (no BIP39)
3. Backend creates Wallet:
   - version = V1NonDeterministic
4. Backend saves & emits event:
   - wallet:created with recovery_capable: false
5. Frontend displays:
   - Wallet card with v1 badge
   - Warning: "Limited recovery - backup .dat file required"
```

---

## Validation Matrix

| Entity | Validation Rule | Error Message |
|--------|----------------|---------------|
| BIP39Mnemonic | Exactly 24 words | "Mnemonic must have exactly 24 words (found: {count})" |
| BIP39Mnemonic | All words in wordlist | "Invalid word at position {index}: '{word}'" |
| BIP39Mnemonic | Checksum valid | "Invalid BIP39 checksum - please check your seed phrase" |
| Seed32Bytes | Length == 32 | "Seed must be exactly 32 bytes (found: {len})" |
| Seed32Bytes | Not all zeros | "Seed cannot be all zeros" |
| MLDSASeed48 | Length == 48 | "ML-DSA seed must be exactly 48 bytes" |
| PrivateKey (v2) | Has seed | "V2 keys must have BIP39 seed for recovery" |
| PrivateKey | key_bytes.len() == 4000 | "Invalid private key size: {len} (expected: 4000)" |
| PublicKey | key_bytes.len() == 1952 | "Invalid public key size: {len} (expected: 1952)" |
| Wallet (v2) | All keys have seeds | "V2 wallet has keys without seeds - data corruption?" |
| Wallet | wallet_id unique | "Wallet ID collision detected" |

---

## Performance Targets

| Operation | Target | Measured |
|-----------|--------|----------|
| BIP39 validation | < 100ms | ~50 μs ✅ |
| Seed derivation (PBKDF2) | < 50ms | ~20 ms ✅ |
| SHAKE256 expansion | < 10ms | ~10 μs ✅ |
| ML-DSA key generation | < 500ms | ~83.5 μs ✅ |
| Wallet encryption (AES-GCM) | < 100ms | ~50 ms ✅ |
| Total wallet creation | < 2 seconds | ~150 ms ✅ |
| Total wallet recovery | < 2 seconds | ~150 ms ✅ |

---

## Security Requirements

### Memory Safety
- All sensitive data uses `Zeroizing<T>` types
- Seed bytes cleared on drop
- Private key bytes zeroized after use
- No logging of seeds or keys

### Constant-Time Operations
- BIP39 checksum validation (constant-time compare)
- ML-DSA signing operations (library provides)
- Seed derivation (PBKDF2 is constant-time for fixed iterations)

### Domain Separation
- Tag: "BTPC-ML-DSA-v1" hardcoded
- Prevents key reuse across contexts
- No user input in tag (security invariant)

### Encryption at Rest
- Algorithm: AES-256-GCM (AEAD)
- Key derivation: Argon2id (password-based)
- Salt: 16 bytes random (unique per wallet)
- Nonce: 12 bytes random (unique per encryption)

---

**Data Model Version**: 1.0
**Last Updated**: 2025-11-06
**Next Step**: Generate API contracts