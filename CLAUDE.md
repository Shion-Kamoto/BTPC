# BTPC Quantum-Resistant Cryptocurrency Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-10-25

## Active Technologies
- **Language**: Rust 1.75+ (core blockchain), TypeScript/React (desktop wallet frontend)
- **Crypto**: Dilithium5 (NIST post-quantum signatures), SHA-512 hashing
- **Storage**: RocksDB (blockchain state), Encrypted files (wallet data)
- **UI**: Tauri (desktop app), CLI (command-line wallet)
- **Networking**: Tokio async runtime, Custom P2P protocol
- **Testing**: cargo test, criterion benchmarks, Miri unsafe validation
- JavaScript ES6+ (vanilla JS, no framework required) + btpc-common.js (existing utility module), btpc-storage.js (localStorage wrapper), browser localStorage API (004-fix-non-functional)
- Browser localStorage for tab state persistence (keys: btpc_active_tab_settings, btpc_active_tab_transactions, btpc_active_tab_mining) (004-fix-non-functional)

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

### Feature 004: Fix Non-Functional Sub-Tabs (Completed 2025-10-25)
**Problem**: Sub-tabs on Settings, Transactions, and Mining pages were non-responsive to clicks.

**Root Cause**:
- Broken `switchTab()` function with undefined `event` variable
- Duplicate JavaScript initialization conflicts (btpc-common.js vs btpc-tauri-context.js)
- Orphaned closing braces in transactions.html and mining.html causing parser errors

**Solution Implemented**:
1. **Created btpc-tab-manager.js** - Professional tab management module with:
   - Event delegation pattern for efficient event handling
   - ARIA accessibility (roles, aria-selected, aria-controls, keyboard navigation)
   - localStorage persistence per page (keys: btpc_active_tab_{page})
   - Graceful degradation when localStorage unavailable

2. **Fixed JavaScript Conflicts**:
   - Removed duplicate `invoke` variable in btpc-common.js
   - Fixed password-modal.js Tauri API initialization timing
   - Added API readiness checks in btpc-backend-first.js
   - Removed orphaned closing braces causing parser errors

3. **Updated HTML Files**:
   - settings.html, mining.html, transactions.html - Added ARIA attributes and TabManager integration
   - Added :focus and :focus-visible CSS for keyboard navigation accessibility

**Result**: All tabs now functional with click response, visual feedback, keyboard navigation, and state persistence. WCAG 2.1 AA compliant.

**Files Modified**:
- btpc-desktop-app/ui/btpc-tab-manager.js (created)
- btpc-desktop-app/ui/{settings,mining,transactions}.html (updated)
- btpc-desktop-app/ui/btpc-common.js (fixed)
- btpc-desktop-app/ui/password-modal.js (fixed)
- btpc-desktop-app/ui/btpc-backend-first.js (fixed)

---

### Feature 005: Fix Transaction Signing & Wallet Backup (Completed 2025-10-28)
**Problem**: Transaction signing failed with "Failed to sign input 0: Signature creation failed" and wallet backup missing walletId field.

**Root Cause**:
- ML-DSA keypair reconstruction limitation in pqc_dilithium library
- Keys loaded from wallet couldn't sign (keypair field was None)
- WalletData struct missing wallet_id field for backup identification

**Solution Implemented**:
1. **Seed Storage System** (btpc-core/src/crypto/keys.rs):
   - Added `seed: Option<[u8; 32]>` field to PrivateKey struct
   - Created `from_key_pair_bytes_with_seed()` method for signing-capable key reconstruction
   - Implemented `sign_with_seed_regeneration()` helper for on-demand keypair generation

2. **Wallet Metadata** (btpc-core/src/crypto/wallet_serde.rs):
   - Added `wallet_id: String` field to WalletData struct (UUID v4)
   - Added `seed: Option<Vec<u8>>` field to KeyEntry struct
   - Updated `to_private_key()` to use seed-based reconstruction when available

3. **Desktop App Integration** (btpc-desktop-app/src-tauri/src/wallet_commands.rs):
   - Transaction signing now uses `key_entry.to_private_key()` with seed support
   - Wallet creation stores seeds via `PrivateKey::from_seed()`
   - Backup operations preserve wallet_id in encrypted .dat files

**Test Fixes**:
- btpc-core/tests/wallet_persistence_test.rs: Added wallet_id to 5 test initializations
- bins/btpc_wallet/Cargo.toml: Added uuid dependency for wallet_id generation

**Result**:
- ✅ Transaction signing works after wallet load (349/350 tests passing)
- ✅ Wallet backup includes wallet_id for restoration tracking
- ✅ Backward compatibility: Legacy keys without seeds still load (can't sign)

**Files Modified**:
- btpc-core/src/crypto/keys.rs (seed storage + reconstruction)
- btpc-core/src/crypto/wallet_serde.rs (wallet_id + seed fields)
- btpc-core/tests/wallet_persistence_test.rs (test fixes)
- bins/btpc_wallet/Cargo.toml (uuid dependency)

---

1. **Desktop App Integration** - Added Tauri-based GUI with wallet management
2. **Mining Integration** - Unified launcher for mining and wallet operations
3. **Security Hardening** - Enhanced key storage and signature validation

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
