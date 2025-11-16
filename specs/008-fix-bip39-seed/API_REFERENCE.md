# BIP39 Mnemonic API Reference

**Feature 008: Deterministic Wallet Recovery**
**Version**: 1.0.0
**Last Updated**: 2025-11-06
**Status**: Production Ready

---

## Table of Contents

1. [Rust Core API](#rust-core-api)
2. [Tauri Commands API](#tauri-commands-api)
3. [Frontend Events API](#frontend-events-api)
4. [Error Types](#error-types)
5. [Type Definitions](#type-definitions)
6. [Code Examples](#code-examples)

---

## Rust Core API

### Module: `btpc_core::crypto::bip39`

#### `struct Mnemonic`

Represents a validated BIP39 mnemonic phrase.

**Fields** (private):
- `words: Vec<String>` - The 24 mnemonic words
- `entropy: Vec<u8>` - The 256-bit entropy

**Methods**:

##### `pub fn parse(phrase: &str) -> Result<Self, BIP39Error>`

Parses and validates a BIP39 mnemonic phrase.

**Parameters**:
- `phrase: &str` - The mnemonic phrase (24 words, whitespace-separated)

**Returns**:
- `Ok(Mnemonic)` - Successfully parsed and validated mnemonic
- `Err(BIP39Error)` - Validation failed

**Validation**:
1. Normalizes whitespace (spaces, tabs, newlines)
2. Converts to lowercase (case-insensitive)
3. Checks word count == 24
4. Verifies all words exist in BIP39 English wordlist
5. Validates checksum (8-bit checksum in last word)

**Example**:
```rust
let phrase = "abandon abandon abandon ... art"; // 24 words
let mnemonic = Mnemonic::parse(phrase)?;
```

**Errors**:
- `InvalidWordCount { expected: 24, found: usize }` - Wrong number of words
- `InvalidWord { word: String, position: usize }` - Word not in wordlist
- `InvalidChecksum` - Checksum verification failed

---

##### `pub fn to_seed(&self, passphrase: &str) -> Result<[u8; 32], BIP39Error>`

Derives a 256-bit seed from the mnemonic and optional passphrase.

**Parameters**:
- `passphrase: &str` - Optional passphrase (use `""` for none)

**Returns**:
- `Ok([u8; 32])` - 256-bit seed for key derivation
- `Err(BIP39Error)` - Seed derivation failed

**Algorithm**:
1. Concatenates words with spaces: `"word1 word2 ... word24"`
2. Constructs salt: `"mnemonic" + passphrase`
3. Applies PBKDF2-HMAC-SHA512 with 2048 iterations
4. Truncates 512-bit output to 256 bits (first 32 bytes)

**Example**:
```rust
let seed_no_passphrase = mnemonic.to_seed("")?;
let seed_with_passphrase = mnemonic.to_seed("my_secret")?;

// Different passphrases = different seeds
assert_ne!(seed_no_passphrase, seed_with_passphrase);
```

**Performance**: ~100-200ms (PBKDF2 intentionally slow)

---

##### `pub fn words(&self) -> &[String]`

Returns a slice of the 24 mnemonic words.

**Returns**:
- `&[String]` - Reference to word array

**Example**:
```rust
let words = mnemonic.words();
println!("Word 1: {}", words[0]);
println!("Word 24: {}", words[23]);
```

---

##### `pub fn generate() -> Result<Self, BIP39Error>`

Generates a new random 24-word mnemonic using cryptographically secure RNG.

**Returns**:
- `Ok(Mnemonic)` - Newly generated mnemonic
- `Err(BIP39Error)` - RNG failure (very rare)

**Example**:
```rust
let new_mnemonic = Mnemonic::generate()?;
let phrase = new_mnemonic.words().join(" ");
println!("New mnemonic: {}", phrase);
```

**Entropy Source**: OS-provided CSPRNG (`getrandom` crate)

---

#### `enum BIP39Error`

Error types for BIP39 operations.

**Variants**:

```rust
pub enum BIP39Error {
    InvalidWordCount {
        expected: usize,
        found: usize,
    },
    InvalidWord {
        word: String,
        position: usize,
    },
    InvalidChecksum,
    EntropyDerivationError,
    Pbkdf2Error(String),
}
```

**Display**:
- Implements `std::fmt::Display` for user-friendly error messages
- Implements `std::error::Error` for error propagation

---

### Module: `btpc_core::crypto::shake256_derivation`

#### `pub fn derive_ml_dsa_seed_from_bip39(bip39_seed: &[u8; 32]) -> Result<[u8; 32], String>`

Expands a 32-byte BIP39 seed to a 32-byte ML-DSA seed using SHAKE256 XOF.

**Parameters**:
- `bip39_seed: &[u8; 32]` - Input seed from BIP39 derivation

**Returns**:
- `Ok([u8; 32])` - Expanded seed for ML-DSA key generation
- `Err(String)` - SHAKE256 operation failed

**Algorithm**:
```rust
SHAKE256(bip39_seed) → 32 bytes output
```

**Purpose**: Cryptographic domain separation between BIP39 and ML-DSA

**Example**:
```rust
let bip39_seed = mnemonic.to_seed("")?;
let ml_dsa_seed = derive_ml_dsa_seed_from_bip39(&bip39_seed)?;
```

**Performance**: < 1ms

---

### Module: `btpc_core::crypto::keys`

#### `struct PrivateKey`

Represents an ML-DSA (Dilithium5) private key.

**Fields** (private):
- `key_pair: SigningKeyPair` - ML-DSA key pair (4000 bytes private, 1952 bytes public)
- `seed: Option<[u8; 32]>` - Optional seed for deterministic signing (V2 wallets)

**Methods**:

##### `pub fn from_seed_deterministic(seed: &[u8; 32]) -> Result<Self, String>`

Creates an ML-DSA key pair deterministically from a 32-byte seed.

**Parameters**:
- `seed: &[u8; 32]` - Input seed (typically from BIP39)

**Returns**:
- `Ok(PrivateKey)` - Key pair with seed stored
- `Err(String)` - Key generation failed

**Algorithm**:
1. Validates seed (cannot be all zeros)
2. Expands seed using SHAKE256
3. Generates ML-DSA key pair from expanded seed
4. Stores original seed for signing operations

**Example**:
```rust
let mnemonic = Mnemonic::parse(phrase)?;
let seed = mnemonic.to_seed("")?;
let private_key = PrivateKey::from_seed_deterministic(&seed)?;
```

**Determinism**: Same seed always produces same key pair

**Performance**: 2-3ms per key generation

---

##### `pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, String>`

Signs a message using ML-DSA signature algorithm.

**Parameters**:
- `message: &[u8]` - Message bytes to sign

**Returns**:
- `Ok(Vec<u8>)` - Signature bytes (~2420 bytes for Dilithium5)
- `Err(String)` - Signature creation failed

**Behavior**:
- If `seed` is available (V2 wallet): Regenerates key pair from seed for signing
- If `seed` is None (V1 wallet): Uses stored key pair directly

**Example**:
```rust
let message = b"Transaction data";
let signature = private_key.sign(message)?;
```

**Performance**: ~5-10ms per signature

---

##### `pub fn public_key(&self) -> PublicKey`

Extracts the public key from the key pair.

**Returns**:
- `PublicKey` - ML-DSA public key (1952 bytes)

**Example**:
```rust
let public_key = private_key.public_key();
let address = Address::from_public_key(&public_key, Network::Mainnet);
```

---

##### `pub fn to_bytes(&self) -> Vec<u8>`

Serializes the private key to bytes.

**Returns**:
- `Vec<u8>` - Private key bytes (4000 bytes for Dilithium5)

**Example**:
```rust
let key_bytes = private_key.to_bytes();
assert_eq!(key_bytes.len(), 4000);
```

---

### Module: `btpc_core::crypto::wallet_serde`

#### `struct WalletData`

Represents a complete wallet with keys and metadata.

**Fields**:
```rust
pub struct WalletData {
    pub wallet_id: String,              // UUID v4
    pub version: WalletVersion,         // V1Original or V2BIP39Deterministic
    pub network: String,                // "mainnet", "testnet", "regtest"
    pub keys: Vec<KeyEntry>,            // Wallet keys
    pub created_at: u64,                // Unix timestamp
    pub modified_at: u64,               // Unix timestamp
}
```

**Methods**:

##### `pub fn save(&self, path: &Path, password: &str) -> Result<(), String>`

Encrypts and saves the wallet to a .dat file.

**Parameters**:
- `path: &Path` - File path (typically `~/.btpc/wallets/{wallet_id}.dat`)
- `password: &str` - Encryption password

**Returns**:
- `Ok(())` - Successfully saved
- `Err(String)` - IO or encryption error

**Encryption**: AES-256-GCM with Argon2 key derivation

**Example**:
```rust
let path = PathBuf::from("~/.btpc/wallets/my_wallet.dat");
wallet.save(&path, "strong_password")?;
```

---

##### `pub fn load(path: &Path, password: &str) -> Result<Self, String>`

Decrypts and loads a wallet from a .dat file.

**Parameters**:
- `path: &Path` - File path
- `password: &str` - Decryption password

**Returns**:
- `Ok(WalletData)` - Successfully loaded wallet
- `Err(String)` - IO, decryption, or deserialization error

**Example**:
```rust
let wallet = WalletData::load(&path, "strong_password")?;
println!("Loaded wallet version: {:?}", wallet.version);
```

---

#### `struct KeyEntry`

Represents a single key within a wallet.

**Fields**:
```rust
pub struct KeyEntry {
    pub label: String,                    // User-friendly label (e.g., "Primary")
    pub address: String,                  // BTPC address (bech32)
    pub private_key_bytes: Vec<u8>,       // ML-DSA private key (4000 bytes)
    pub public_key_bytes: Vec<u8>,        // ML-DSA public key (1952 bytes)
    pub seed: Option<Vec<u8>>,            // 32-byte seed (V2 only)
    pub created_at: u64,                  // Unix timestamp
}
```

**Methods**:

##### `pub fn to_private_key(&self) -> Result<PrivateKey, String>`

Reconstructs a PrivateKey from the stored key data.

**Returns**:
- `Ok(PrivateKey)` - Reconstructed private key
- `Err(String)` - Reconstruction failed

**Behavior**:
- V2 wallets (with seed): Uses `from_key_pair_bytes_with_seed()`
- V1 wallets (no seed): Uses `from_bytes()` (legacy)

**Example**:
```rust
let private_key = key_entry.to_private_key()?;
let signature = private_key.sign(message)?;
```

---

#### `enum WalletVersion`

Identifies wallet format version.

**Variants**:
```rust
pub enum WalletVersion {
    V1Original,              // Legacy wallets (pre-BIP39)
    V2BIP39Deterministic,    // BIP39 mnemonic wallets
}
```

**Serialization**: JSON-compatible (via serde)

**Example**:
```rust
match wallet.version {
    WalletVersion::V1Original => println!("Legacy wallet"),
    WalletVersion::V2BIP39Deterministic => println!("BIP39 wallet"),
}
```

---

## Tauri Commands API

### `create_wallet_from_mnemonic`

Creates a new wallet from a BIP39 mnemonic phrase.

**Signature**:
```rust
#[tauri::command]
pub async fn create_wallet_from_mnemonic(
    wallet_name: String,
    mnemonic_phrase: String,
    passphrase: Option<String>,
    network: String,
    state: State<'_, AppState>,
) -> Result<WalletCreationResponse, String>
```

**Parameters**:
- `wallet_name: String` - User-friendly wallet name
- `mnemonic_phrase: String` - 24-word BIP39 phrase
- `passphrase: Option<String>` - Optional passphrase (None or Some(""))
- `network: String` - "mainnet", "testnet", or "regtest"
- `state: State<'_, AppState>` - Tauri app state

**Returns**:
```typescript
interface WalletCreationResponse {
  wallet_id: string;        // UUID v4
  address: string;          // Bech32 address
  version: string;          // "V2BIP39Deterministic"
}
```

**Frontend Usage**:
```javascript
const response = await invoke('create_wallet_from_mnemonic', {
  walletName: 'My Wallet',
  mnemonicPhrase: 'abandon abandon abandon ... art',
  passphrase: '',
  network: 'mainnet'
});

console.log('Wallet ID:', response.wallet_id);
console.log('Address:', response.address);
```

**Emits Events**:
- `wallet:created` on success
- `wallet:error` on failure

---

### `recover_wallet_from_mnemonic`

Recovers an existing wallet from a BIP39 mnemonic phrase.

**Signature**:
```rust
#[tauri::command]
pub async fn recover_wallet_from_mnemonic(
    wallet_name: String,
    mnemonic_phrase: String,
    passphrase: Option<String>,
    network: String,
    state: State<'_, AppState>,
) -> Result<WalletRecoveryResponse, String>
```

**Parameters**: Same as `create_wallet_from_mnemonic`

**Returns**:
```typescript
interface WalletRecoveryResponse {
  wallet_id: string;
  address: string;
  version: string;
  recovered: boolean;       // Always true
}
```

**Frontend Usage**:
```javascript
const response = await invoke('recover_wallet_from_mnemonic', {
  walletName: 'Recovered Wallet',
  mnemonicPhrase: 'legal winner thank ... title',
  passphrase: 'my_secret',
  network: 'mainnet'
});

console.log('Recovered address:', response.address);
```

**Emits Events**:
- `wallet:recovered` on success
- `wallet:error` on failure

---

### `generate_mnemonic`

Generates a new random 24-word BIP39 mnemonic.

**Signature**:
```rust
#[tauri::command]
pub async fn generate_mnemonic() -> Result<String, String>
```

**Parameters**: None

**Returns**:
- `Ok(String)` - 24-word mnemonic phrase (space-separated)
- `Err(String)` - RNG failure

**Frontend Usage**:
```javascript
const mnemonic = await invoke('generate_mnemonic');
console.log('New mnemonic:', mnemonic);
// Output: "word1 word2 word3 ... word24"
```

---

## Frontend Events API

### Event: `wallet:created`

Emitted when a new wallet is successfully created.

**Payload**:
```typescript
interface WalletCreatedEvent {
  wallet_id: string;
  address: string;
  version: string;        // "V2BIP39Deterministic"
  network: string;
  timestamp: number;      // Unix timestamp
}
```

**Listener**:
```javascript
import { listen } from '@tauri-apps/api/event';

listen('wallet:created', (event) => {
  console.log('New wallet created:', event.payload.wallet_id);
  updateUI(event.payload);
});
```

---

### Event: `wallet:recovered`

Emitted when a wallet is successfully recovered from mnemonic.

**Payload**:
```typescript
interface WalletRecoveredEvent {
  wallet_id: string;
  address: string;
  version: string;
  network: string;
  recovered: boolean;
  timestamp: number;
}
```

**Listener**:
```javascript
listen('wallet:recovered', (event) => {
  console.log('Wallet recovered:', event.payload.address);
  showRecoverySuccess(event.payload);
});
```

---

### Event: `wallet:error`

Emitted when wallet operations fail.

**Payload**:
```typescript
interface WalletErrorEvent {
  operation: string;      // "create", "recover", "sign", etc.
  error: string;          // Error message
  timestamp: number;
}
```

**Listener**:
```javascript
listen('wallet:error', (event) => {
  console.error('Wallet error:', event.payload.error);
  showErrorNotification(event.payload);
});
```

---

## Error Types

### `BIP39Error`

```rust
pub enum BIP39Error {
    InvalidWordCount { expected: usize, found: usize },
    InvalidWord { word: String, position: usize },
    InvalidChecksum,
    EntropyDerivationError,
    Pbkdf2Error(String),
}
```

**User-Facing Messages**:
- `InvalidWordCount`: "Expected 24 words, found {found}"
- `InvalidWord`: "Invalid word '{word}' at position {position}"
- `InvalidChecksum`: "Checksum verification failed - check your mnemonic"
- `Pbkdf2Error`: "Key derivation failed: {message}"

---

## Type Definitions

### TypeScript Definitions

```typescript
// Wallet creation request
interface CreateWalletRequest {
  wallet_name: string;
  mnemonic_phrase: string;
  passphrase?: string;
  network: 'mainnet' | 'testnet' | 'regtest';
}

// Wallet creation response
interface WalletCreationResponse {
  wallet_id: string;
  address: string;
  version: 'V1Original' | 'V2BIP39Deterministic';
}

// Wallet recovery response
interface WalletRecoveryResponse extends WalletCreationResponse {
  recovered: boolean;
}

// Wallet version badge
type WalletVersionBadge = {
  version: 'V1 Legacy' | 'V2 BIP39';
  color: 'gray' | 'green';
  tooltip: string;
};
```

---

## Code Examples

### Complete Wallet Creation Flow

```rust
use btpc_core::crypto::{
    bip39::Mnemonic,
    keys::PrivateKey,
    wallet_serde::{WalletData, WalletVersion, KeyEntry},
};
use btpc_core::Address;

// 1. Generate mnemonic
let mnemonic = Mnemonic::generate()?;
let phrase = mnemonic.words().join(" ");

// 2. Derive seed (with optional passphrase)
let seed = mnemonic.to_seed("my_passphrase")?;

// 3. Generate keys
let private_key = PrivateKey::from_seed_deterministic(&seed)?;
let public_key = private_key.public_key();

// 4. Derive address
let address = Address::from_public_key(&public_key, Network::Mainnet);

// 5. Create wallet
let key_entry = KeyEntry {
    label: "Primary".to_string(),
    address: address.to_string(),
    private_key_bytes: private_key.to_bytes().to_vec(),
    public_key_bytes: public_key.to_bytes().to_vec(),
    seed: Some(seed.to_vec()),
    created_at: current_timestamp(),
};

let wallet = WalletData {
    wallet_id: Uuid::new_v4().to_string(),
    version: WalletVersion::V2BIP39Deterministic,
    network: "mainnet".to_string(),
    keys: vec![key_entry],
    created_at: current_timestamp(),
    modified_at: current_timestamp(),
};

// 6. Save wallet
let path = PathBuf::from("~/.btpc/wallets/my_wallet.dat");
wallet.save(&path, "encryption_password")?;

println!("Wallet created successfully!");
println!("Mnemonic: {}", phrase);
println!("Address: {}", address.to_string());
```

### Complete Wallet Recovery Flow

```rust
// 1. Parse mnemonic (user provides)
let mnemonic = Mnemonic::parse("abandon abandon abandon ... art")?;

// 2. Derive seed (with same passphrase as original)
let seed = mnemonic.to_seed("my_passphrase")?;

// 3. Regenerate keys (deterministic!)
let private_key = PrivateKey::from_seed_deterministic(&seed)?;
let public_key = private_key.public_key();

// 4. Verify address matches original
let recovered_address = Address::from_public_key(&public_key, Network::Mainnet);
assert_eq!(recovered_address.to_string(), original_address);

println!("Wallet recovered successfully!");
```

### Frontend Integration

```html
<!DOCTYPE html>
<html>
<head>
  <title>BTPC Wallet</title>
</head>
<body>
  <div id="create-wallet">
    <h2>Create New Wallet</h2>
    <button id="generate-btn">Generate Mnemonic</button>
    <textarea id="mnemonic-display" readonly></textarea>
    <input type="password" id="passphrase" placeholder="Optional passphrase">
    <button id="create-btn">Create Wallet</button>
  </div>

  <div id="recover-wallet">
    <h2>Recover Wallet</h2>
    <textarea id="mnemonic-input" placeholder="Enter 24-word mnemonic"></textarea>
    <input type="password" id="recover-passphrase" placeholder="Passphrase (if used)">
    <button id="recover-btn">Recover Wallet</button>
  </div>

  <script type="module">
    import { invoke } from '@tauri-apps/api/core';
    import { listen } from '@tauri-apps/api/event';

    // Generate mnemonic
    document.getElementById('generate-btn').addEventListener('click', async () => {
      const mnemonic = await invoke('generate_mnemonic');
      document.getElementById('mnemonic-display').value = mnemonic;
    });

    // Create wallet
    document.getElementById('create-btn').addEventListener('click', async () => {
      const mnemonic = document.getElementById('mnemonic-display').value;
      const passphrase = document.getElementById('passphrase').value;

      try {
        const result = await invoke('create_wallet_from_mnemonic', {
          walletName: 'My Wallet',
          mnemonicPhrase: mnemonic,
          passphrase: passphrase || '',
          network: 'mainnet'
        });

        alert(`Wallet created!\nAddress: ${result.address}`);
      } catch (error) {
        alert(`Error: ${error}`);
      }
    });

    // Recover wallet
    document.getElementById('recover-btn').addEventListener('click', async () => {
      const mnemonic = document.getElementById('mnemonic-input').value;
      const passphrase = document.getElementById('recover-passphrase').value;

      try {
        const result = await invoke('recover_wallet_from_mnemonic', {
          walletName: 'Recovered',
          mnemonicPhrase: mnemonic,
          passphrase: passphrase || '',
          network: 'mainnet'
        });

        alert(`Wallet recovered!\nAddress: ${result.address}`);
      } catch (error) {
        alert(`Recovery failed: ${error}`);
      }
    });

    // Listen for events
    listen('wallet:created', (event) => {
      console.log('Wallet created:', event.payload);
    });

    listen('wallet:error', (event) => {
      console.error('Wallet error:', event.payload);
    });
  </script>
</body>
</html>
```

---

## Performance Characteristics

| Operation | Time (avg) | Notes |
|-----------|-----------|-------|
| `Mnemonic::parse()` | 0.1-0.5 ms | Checksum validation |
| `to_seed()` | 100-200 ms | PBKDF2 (intentionally slow) |
| `derive_ml_dsa_seed_from_bip39()` | < 1 ms | SHAKE256 expansion |
| `from_seed_deterministic()` | 2-3 ms | ML-DSA key generation |
| `sign()` | 5-10 ms | ML-DSA signature |
| `WalletData::save()` | 50-100 ms | AES encryption + file I/O |
| `WalletData::load()` | 50-100 ms | File I/O + decryption |

**Total wallet creation**: ~110-210 ms (dominated by PBKDF2)

---

## Security Notes

1. **Never log mnemonics**: Treat as highly sensitive data
2. **Wipe from memory**: Consider zeroization after use
3. **Validate inputs**: Always use `Mnemonic::parse()` for validation
4. **Test passphrases**: Empty string vs None are equivalent
5. **Backup verification**: Always test recovery before sending funds
6. **Network awareness**: Same mnemonic generates different addresses on different networks

---

*This API reference is part of Feature 008: BIP39 Deterministic Wallet Recovery*
*For implementation details, see DEVELOPER_GUIDE.md*
*For user instructions, see USER_GUIDE.md*