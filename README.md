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

RPC authentication is **mandatory** — the node refuses to start without valid credentials
(`--rpcuser` ≥ 8 chars, `--rpcpass` ≥ 16 chars).

```bash
# One-command setup: generate a self-signed TLS cert + random RPC credentials +
# a complete rig config (rig-config.toml) — no openssl needed.
./target/release/btpc_node --setup --network mainnet --address <wallet> --name rig-01

# Start a node (mainnet P2P [::]:18341, RPC on 127.0.0.1:18340)
./target/release/btpc_node --network mainnet \
    --rpcuser <user> --rpcpass <pass> \
    --tlscert <datadir>/rpc.crt --tlskey <datadir>/rpc.key

# Testnet / regtest
./target/release/btpc_node --network testnet --rpcuser <u> --rpcpass <p>
./target/release/btpc_node --network regtest --rpcuser <u> --rpcpass <p>
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

### Network Parameters

| Network | P2P port | RPC port | User agent |
|---|---|---|---|
| mainnet | 18341 | 18340 | `/BTPC:1.0.0/` |
| testnet | 18351 | 18350 | `/BTPC:1.0.0-testnet/` |
| regtest | 18361 | 18360 | `/BTPC:1.0.0-regtest/` |

The node listens on `[::]:<port>` (dual-stack). `--listen` overrides the port only when
passed explicitly; by default each network uses its own port above. RPC is a separate
`--rpcport` (defaults to the network's RPC port above) bound to `127.0.0.1` by default.

### Node Configuration

The node is configured via **CLI flags**, not a config file:

- `--network mainnet|testnet|regtest` (default `mainnet`)
- `--datadir <dir>` (default `~/.btpc`)
- `--listen <addr>` (optional; defaults to the network port above)
- `--connect <peer>` (repeatable; explicit outbound peer)
- `--rpcport <port>` / `--rpcbind <addr>` (defaults to 18340/18350/18360 / 127.0.0.1)
- `--rpcuser <user>` / `--rpcpass <pass>` (**required** — auth is mandatory)
- `--tlscert <path>` / `--tlskey <path>` (optional; enable HTTPS RPC)
- `--daemon` (run without waiting on stdin)
- `--mine` (in-process CPU mining, test path)
- `--setup` (one-command TLS cert + credential + rig-config generator)

`--setup` writes `<datadir>/rpc.crt`, `rpc.key`, and a complete `rig-config.toml`
ready to hand to the mining rig.

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

## Mining Rig & OS Image

The dedicated GPU mining rig lives in a separate tree: **BTPC_RIG**
(`/media/bob/HDD1/BTPC_RIG`). It's a thin JSON-RPC client (not a P2P node) that mines
against a `btpc_node` over HTTPS with a pinned self-signed cert. It ships **mainnet-only**
(`[::]:18341`); testnet/regtest are dev-only on the node. See its own README.

`BTPC_RIG/btpc-os/` builds a BalenaEtcher-flashable OS image (Ubuntu 24.04 + NVIDIA 580 +
AMD ROCm 5.7.2 + the node/rig/dashboard/watchdog binaries) that boots a rig straight into
mining. Build with `sudo ./build.sh`; the image lands in `btpc-os/out/`.

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