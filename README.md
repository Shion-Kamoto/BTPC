# BTPC - Bitcoin Post-Quantum Cryptocurrency

A quantum-resistant cryptocurrency implementation using post-quantum cryptography (ML-DSA/Dilithium5) with a Proof-of-Work consensus mechanism.

## Features

- **Post-Quantum Cryptography**: ML-DSA (Dilithium5) signatures for quantum resistance
- **Proof-of-Work**: SHA-512 based mining with CPU and GPU support
- **UTXO Model**: Bitcoin-style transaction model with RocksDB storage
- **P2P Network**: Asynchronous networking with peer discovery and sync
- **Desktop Wallet**: Tauri-based GUI with authentication and multi-wallet support
- **RPC Server**: JSON-RPC API with authentication and rate limiting

## Prerequisites

### Required
- **Rust** 1.75 or later
- **Node.js** 16+ and npm (for desktop app)
- **RocksDB** development libraries

### Install Dependencies

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install build-essential pkg-config libssl-dev librocksdb-dev clang
```

**macOS:**
```bash
brew install rocksdb
```

**Arch Linux:**
```bash
sudo pacman -S rocksdb
```

## Building

### 1. Clone the Repository
```bash
git clone https://github.com/Shion-Kamoto/BTPC.git
cd BTPC
```

### 2. Build Core Blockchain & Binaries
```bash
cargo build --release
```

This builds:
- `target/release/btpc_node` - Full blockchain node
- `target/release/btpc_miner` - Mining software
- `target/release/btpc_wallet` - CLI wallet
- `target/release/genesis_tool` - Genesis block generator

### 3. Build Desktop Application (Optional)
```bash
cd btpc-desktop-app
npm install
npm run tauri:build
```

The desktop app binary will be in `btpc-desktop-app/src-tauri/target/release/`

## Quick Start

### Running a Node
```bash
# Start a node on regtest network (for testing)
./target/release/btpc_node --network regtest

# Start a node on mainnet
./target/release/btpc_node --network mainnet
```

### Creating a Wallet
```bash
# Create a new wallet
./target/release/btpc_wallet create --network regtest --wallet-name my_wallet

# Check balance
./target/release/btpc_wallet balance --wallet-name my_wallet
```

### Mining
```bash
# Start mining with 4 threads
./target/release/btpc_miner --threads 4 --network regtest --wallet my_wallet
```

### Using the Desktop App
```bash
# Run in development mode
cd btpc-desktop-app
npm run tauri:dev

# Or run the built binary
./btpc-desktop-app/src-tauri/target/release/btpc-desktop-app
```

## Project Structure

```
BTPC/
├── btpc-core/           # Core blockchain library
│   ├── src/
│   │   ├── blockchain/  # Block and transaction logic
│   │   ├── consensus/   # PoW validation and difficulty
│   │   ├── crypto/      # ML-DSA signatures and keys
│   │   ├── network/     # P2P networking
│   │   ├── rpc/         # JSON-RPC server
│   │   └── storage/     # RocksDB integration
│   └── tests/           # Integration tests
├── bins/                # Binary executables
│   ├── btpc_node/       # Full node implementation
│   ├── btpc_miner/      # Mining software
│   ├── btpc_wallet/     # CLI wallet
│   └── genesis_tool/    # Genesis block generator
└── btpc-desktop-app/    # Tauri desktop application
    ├── src-tauri/       # Rust backend
    └── ui/              # Web frontend
```

## Testing

### Run All Tests
```bash
cargo test --workspace
```

### Run Specific Test Suites
```bash
# Core blockchain tests
cargo test -p btpc-core

# Desktop app backend tests
cargo test -p btpc-desktop-app

# Integration tests
cargo test --test multi_node_network
```

## Development

### Code Style
- Format code: `cargo fmt`
- Lint code: `cargo clippy -- -D warnings`
- Security audit: `cargo audit`

### Performance Benchmarks
```bash
cargo bench
```

## Configuration

### Node Configuration
Create `config.toml`:
```toml
[network]
network_type = "mainnet"  # or "testnet", "regtest"
listen_addr = "0.0.0.0:8333"
max_peers = 125

[rpc]
enabled = true
bind_address = "127.0.0.1:8332"
username = "your_rpc_user"
password = "your_rpc_password"

[storage]
data_dir = "~/.btpc/mainnet"
```

### Mining Configuration
Set mining address in wallet:
```bash
./target/release/btpc_wallet get-address --wallet-name mining_wallet
```

## Architecture

- **Consensus**: SHA-512 Proof-of-Work with dynamic difficulty adjustment
- **Signatures**: ML-DSA (Dilithium5) - NIST post-quantum standard
- **Block Time**: ~10 minutes target
- **Supply**: Emission schedule with halving (see economics module)
- **Storage**: RocksDB with column families for blocks, transactions, UTXO set

## Security

- Post-quantum signatures (ML-DSA) for long-term security
- Constant-time cryptographic operations
- Input validation at all API boundaries
- Rate limiting on RPC and P2P connections
- Encrypted wallet storage with Argon2id key derivation

## License

See [LICENSE](LICENSE) file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test --workspace`
5. Submit a pull request

## Support

For issues and questions:
- GitHub Issues: https://github.com/Shion-Kamoto/BTPC/issues

## Acknowledgments

- NIST Post-Quantum Cryptography standardization project
- Bitcoin Core for design inspiration
- RocksDB team for the storage engine