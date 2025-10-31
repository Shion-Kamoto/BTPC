# BTPC Quantum-Resistant Cryptocurrency Development Guidelines

Auto-generated from all feature plans. Last updated: [DATE]

## Active Technologies
- **Language**: Rust 1.75+ (core blockchain), TypeScript/React (desktop wallet frontend)
- **Crypto**: Dilithium5 (NIST post-quantum signatures), SHA-512 hashing
- **Storage**: RocksDB (blockchain state), Encrypted files (wallet data)
- **UI**: Tauri (desktop app), CLI (command-line wallet)
- **Networking**: Tokio async runtime, Custom P2P protocol
- **Testing**: cargo test, criterion benchmarks, Miri unsafe validation

## Project Structure
```
BTPC/
├── core/                    # Blockchain core implementation
│   ├── src/consensus/       # Proof-of-work consensus
│   ├── src/crypto/          # Post-quantum cryptography
│   ├── src/blockchain/      # Block and transaction logic
│   └── src/network/         # P2P networking
├── wallet/                  # CLI wallet implementation
│   ├── src/key_management/  # Dilithium5 key operations
│   └── src/transaction/     # UTXO transaction building
├── btpc-desktop-app/        # Tauri desktop application
│   ├── src-tauri/src/       # Rust backend
│   └── src/                # Web frontend
└── deployment/              # Docker and CI/CD
```

## Commands
**Rust Development**:
- `cargo build --release` - Build optimized binaries
- `cargo test --workspace` - Run all tests
- `cargo bench` - Performance benchmarks
- `cargo audit` - Security audit dependencies
- `cargo clippy -- -D warnings` - Linting

**Blockchain Operations**:
- `./target/release/btpc-node --config mainnet.toml` - Start mainnet node
- `./target/release/btpc-wallet create --network mainnet` - Create wallet
- `./target/release/btpc-wallet balance` - Check balance

**Desktop App**:
- `npm run tauri:dev` - Development mode
- `npm run tauri:build` - Production build

## Code Style
**Rust**:
- Follow `cargo fmt` standard formatting
- Use `#![deny(unsafe_code)]` unless cryptographically required
- Prefer owned types over lifetimes for API boundaries
- Use `anyhow::Result` for error handling
- Document all public APIs with `///` comments

**Security**:
- All cryptographic operations must use constant-time functions
- No hardcoded secrets or credentials
- Validate all inputs at API boundaries
- Use `SecureString` or `Zeroizing` for sensitive data

## Recent Changes
1. **Desktop App Integration** - Added Tauri-based GUI with wallet management
2. **Mining Integration** - Unified launcher for mining and wallet operations
3. **Security Hardening** - Enhanced key storage and signature validation

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->