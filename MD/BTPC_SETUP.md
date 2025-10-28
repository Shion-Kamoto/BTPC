# BTPC Setup and File Structure

This document explains the complete file structure needed to run BTPC node, miner, and wallet with full functionality.

## Quick Setup

### Option 1: Automated Setup (Recommended)
```bash
# Clone repository
git clone <your-repo-url>
cd BTPC

# Run setup script
chmod +x scripts/setup-btpc.sh
./scripts/setup-btpc.sh
```

### Option 2: Manual Setup
```bash
# Build binaries
cargo build --release --bin btpc_node --bin btpc_miner --bin btpc_wallet

# Create directory structure
mkdir -p ~/.btpc/{bin,config/security,data/{wallet,blockchain,tx_storage,address_book,node},logs}

# Copy binaries
cp target/release/btpc_{node,miner,wallet} ~/.btpc/bin/
chmod +x ~/.btpc/bin/*

# Create template files
echo "[]" > ~/.btpc/data/wallet/wallet_utxos.json
echo "[]" > ~/.btpc/data/wallet/wallet_transactions.json
echo "[]" > ~/.btpc/data/address_book/address_book.json
```

## Required Files

### 1. Binaries (in `~/.btpc/bin/`)
These are built from source and should NOT be committed to git:

| Binary | Purpose | Source |
|--------|---------|--------|
| `btpc_node` | Blockchain node management | `bins/btpc_node/` |
| `btpc_miner` | Block mining with SHA-512 PoW | `bins/btpc_miner/` |
| `btpc_wallet` | Wallet and key management | `bins/btpc_wallet/` |

### 2. Data Files (in `~/.btpc/data/`)

#### Wallet Files (`~/.btpc/data/wallet/`)
```
wallet.json                  # Main wallet (created by app)
wallet_utxos.json           # UTXO tracking (MUST be valid JSON array)
wallet_transactions.json    # Transaction history (MUST be valid JSON array)
```

**Critical**: These JSON files MUST contain valid empty arrays `[]` if no data exists. Invalid JSON will cause app crashes.

#### Blockchain Storage (`~/.btpc/data/blockchain/`)
RocksDB database (auto-created by btpc_node):
- `.sst` files (SSTable files)
- `MANIFEST-*` (database manifest)
- `CURRENT` (current manifest pointer)
- `LOCK` (database lock file)

#### Transaction Storage (`~/.btpc/data/tx_storage/`)
RocksDB database for transaction indexing (auto-created by desktop app).

#### Address Book (`~/.btpc/data/address_book/`)
```
address_book.json           # Saved addresses (empty array by default)
```

### 3. Configuration Files (in `~/.btpc/config/`)
```
security/SK.json            # Security keys (auto-created)
```

### 4. Logs (in `~/.btpc/logs/`)
Runtime log files (auto-created).

## File Format Examples

### `wallet_utxos.json` (Empty)
```json
[]
```

### `wallet_transactions.json` (Empty)
```json
[]
```

### `wallet.json` (Example - Created by App)
```json
{
  "version": "3.0",
  "address": "mkJcKzZPdsWhCsLc7WbYVMmhUmbkYVAQ7u",
  "public_key": "hex_encoded_public_key...",
  "encrypted_private_key": "base64_encrypted_data...",
  "seed_phrase_hash": "hex_hash...",
  "created_at": "2025-10-17T12:00:00Z",
  "crypto_type": "ML-DSA-65",
  "address_type": "P2PKH-Base58",
  "encryption": "AES-256-GCM"
}
```

### `mining_stats.json` (Template)
```json
{
  "total_blocks_found": 0,
  "total_hashes_computed": 0,
  "average_hashrate": 0.0,
  "sessions": []
}
```

## Directory Structure

```
~/.btpc/
├── bin/                          # Compiled binaries
│   ├── btpc_node                 # Node binary
│   ├── btpc_miner                # Miner binary
│   └── btpc_wallet               # Wallet binary
├── config/
│   └── security/
│       └── SK.json               # Security keys
├── data/
│   ├── address_book/
│   │   └── address_book.json     # Saved addresses
│   ├── blockchain/               # RocksDB (btpc_node)
│   │   ├── *.sst
│   │   ├── MANIFEST-*
│   │   ├── CURRENT
│   │   └── LOCK
│   ├── tx_storage/               # RocksDB (desktop app)
│   │   └── (similar structure)
│   ├── wallet/
│   │   ├── wallet.json           # Main wallet
│   │   ├── wallet_utxos.json     # UTXOs
│   │   └── wallet_transactions.json  # Transactions
│   ├── node/                     # Node-specific data
│   ├── desktop-node/             # Desktop app node data
│   └── mining_stats.json         # Mining statistics
└── logs/                         # Runtime logs
```

## Usage

### Command Line

#### Start Node
```bash
# Mainnet (default port 8332)
~/.btpc/bin/btpc_node --network mainnet

# Testnet (port 18360)
~/.btpc/bin/btpc_node --network testnet --rpc-port 18360

# Regtest (local testing)
~/.btpc/bin/btpc_node --network regtest --rpc-port 18443
```

#### Create Wallet
```bash
~/.btpc/bin/btpc_wallet generate --output ~/.btpc/data/wallet/wallet.json
```

#### Start Mining
```bash
# Mine to your address
~/.btpc/bin/btpc_miner \
  --network mainnet \
  --rpc-url http://127.0.0.1:8332 \
  --address YOUR_WALLET_ADDRESS \
  --threads 4
```

### Desktop App

```bash
cd btpc-desktop-app
npm run tauri:dev
```

The desktop app will:
1. Auto-detect binaries in `~/.btpc/bin/`
2. Create missing directories and template files
3. Initialize RocksDB databases
4. Provide GUI for all operations

## Troubleshooting

### App Crashes with JSON Parse Error
**Cause**: Corrupted or invalid JSON in UTXO/transaction files

**Fix**:
```bash
# Reset to empty arrays
echo "[]" > ~/.btpc/data/wallet/wallet_utxos.json
echo "[]" > ~/.btpc/data/wallet/wallet_transactions.json
```

### Binaries Not Found
**Cause**: Binaries not in `~/.btpc/bin/` or not executable

**Fix**:
```bash
# Rebuild and reinstall
./scripts/setup-btpc.sh
```

### RocksDB Corruption
**Cause**: Unclean shutdown or disk issues

**Fix**:
```bash
# Backup data (if needed)
mv ~/.btpc/data/blockchain ~/.btpc/data/blockchain.backup
mv ~/.btpc/data/tx_storage ~/.btpc/data/tx_storage.backup

# Databases will be recreated on next startup
# Note: Blockchain will need to resync
```

### Node Won't Start
**Check**:
1. Port not in use: `lsof -i :8332`
2. Correct network flag
3. Check logs in `~/.btpc/logs/`

## Recent Fixes

### Mining Timing Bug (Fixed)
- **Issue**: Miner used hardcoded easy difficulty, producing instant blocks
- **Fix**: Implemented RPC `getblocktemplate` to fetch real network difficulty
- **Files**: `bins/btpc_miner/src/main.rs`, `bins/btpc_miner/Cargo.toml`
- **Result**: Blocks now mine at proper 10-minute intervals

### UTXO Corruption (Fixed)
- **Issue**: Corrupted UTXO/transaction JSON files causing app crashes
- **Fix**: Proper JSON validation, empty array defaults
- **Impact**: Clean initialization, no more parsing errors

## Development

### Building from Source
```bash
# Build all binaries
cargo build --release

# Build specific binary
cargo build --release --bin btpc_node
cargo build --release --bin btpc_miner
cargo build --release --bin btpc_wallet

# Build desktop app
cd btpc-desktop-app
npm install
npm run tauri:build
```

### Running Tests
```bash
# Core tests
cargo test

# Desktop app tests
cd btpc-desktop-app
npm test
```

## Contributing

When contributing:
1. **DO NOT** commit binaries or build artifacts
2. **DO** commit source code and build scripts
3. **DO** update this documentation for new features
4. **DO** test with the setup script before submitting PR

## License

[Your License Here]
