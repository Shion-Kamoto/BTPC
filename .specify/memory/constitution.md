# BTPC Quantum-Resistant Cryptocurrency Constitution
# BTPC (Bitcoin-Time Protocol Chain) Constitution

**Foreword**



**Version 1.0**
**Effective Date: September 24, 2025**
**Status: IMMUTABLE WITHOUT EXPLICIT AMENDMENT**



## Core Principles

### I. Security-First
Every feature MUST prioritize quantum-resistant security; All cryptographic operations MUST use NIST-approved post-quantum algorithms (ML-DSA (Dilithium) signatures, SHA-512 hashing); No hardcoded secrets or credential exposure; All private keys encrypted at rest; Constant-time operations for all security-critical code

### II. Rust-First Development
All core blockchain components MUST be written in Rust; Use cargo workspace for multi-crate organization; Follow Rust memory safety and ownership principles; Zero-cost abstractions preferred; All dependencies audited with cargo-audit

### III. Test-Driven Development (NON-NEGOTIABLE)
TDD mandatory for all code: Tests written → Tests fail → Implementation → Tests pass; Maintain >90% test coverage for new code; Integration tests for all cryptographic functions; Comprehensive benchmarks for performance-critical paths; Miri validation for unsafe code

### IV. Blockchain-Specific Testing
Mandatory integration tests for: Consensus mechanism validation, Block validation and propagation, Transaction pool management, Wallet-node communication, Mining and difficulty adjustment, Post-quantum signature verification across all components

### V. Production Readiness
Structured logging with tracing support; Prometheus metrics for all components; Graceful degradation and error recovery; Configurable via TOML files; Docker-ready with health checks; Database migrations for schema changes; Backward-compatible RPC protocols

## Performance Standards

Signature operations: ML-DSA generation <2ms, verification <1.5ms; Block validation: <10ms per 1MB block; Transaction throughput: Target 1000 TPS; Memory usage: <200MB base + ~1GB per million UTXOs; Storage efficiency: ~2GB per million transactions; Network sync: Full node sync in <24 hours

## Development Workflow

All changes via pull requests with mandatory code review; Security-critical changes require security team approval; Automated CI/CD with cargo check, cargo test, cargo clippy, cargo audit; Pre-commit hooks for formatting and basic validation; Release branches for mainnet deployments; Staging environment for testnet validation

## Governance
<!-- Example: Constitution supersedes all other practices; Amendments require documentation, approval, migration plan -->

All PRs must verify constitutional compliance; Security violations are blocking; Performance regressions require justification; Cryptographic changes require expert review; Breaking changes need migration plans; Use CLAUDE.md for AI development guidance


---

## Article I: Project Identity and Purpose

### Section 1.1 - Project Name and Identity
- **Official Name**: BTPC (Bitcoin-Time Protocol Chain)
- **Classification**: Quantum-Resistant Blockchain with Linear Decay Economics
- **Mission**: To create a quantum-resistant cryptocurrency that maintains Bitcoin's proven economic model while incorporating post-quantum cryptography and sustainable reward mechanics

### Section 1.2 - Core Objectives
1. **Quantum Resistance**: Implement ML-DSA (Dilithium) signatures to protect against quantum computer attacks
2. **Economic Sustainability**: Replace Bitcoin's halving model with linear decay to ensure long-term miner incentives
3. **Bitcoin Compatibility**: Maintain Bitcoin's core blockchain structure and proven consensus mechanisms
4. **Long-term Viability**: Create a cryptocurrency designed to function for decades with predictable economics

---

## Article II: Technical Specifications (IMMUTABLE)

### Section 2.1 - Cryptographic Algorithms
- **Proof of Work**: SHA-512 (double hashing, 64-byte output)
- **Digital Signatures**: ML-DSA (Module-Lattice-Based Digital Signature Algorithm) - standardized Dilithium
- **Block Hashing**: SHA-512 throughout the entire system
- **Merkle Tree**: SHA-512 based merkle tree construction
- **Target Difficulty**: 64-byte arrays for all difficulty calculations

### Section 2.2 - Block Structure
- **Block Time**: 10 minutes (same as Bitcoin)
- **Block Size**: Maximum 1MB (same as Bitcoin)
- **Transaction Structure**: Bitcoin-compatible transaction format
- **Block Header**: Contains prev_hash, merkle_root, timestamp, bits, nonce (Bitcoin format)
- **Genesis Block**: Network-specific genesis blocks with embedded messages

### Section 2.3 - Network Parameters
- **Port**: Standard P2P networking protocols
- **Networks**: Mainnet, Testnet, Regtest
- **Protocol Version**: Bitcoin-compatible versioning
- **Address Format**: BTPC-specific addressing scheme

---

## Article III: Economic Model (LINEAR DECAY)

### Section 3.1 - Reward Structure
- **Initial Block Reward**: 32.375 BTPC per block
- **Economic Model**: Linear decay over 24 years, NOT halving
- **Blocks Per Year**: 52,560 (10-minute blocks)
- **Base Unit**: 1 BTPC = 100,000,000 base units (same as Bitcoin satoshi's)

### Section 3.2 - Linear Decay Formula
```
Total Supply Formula:
- Year 1: 32.375 BTPC per block
- Year 24: Approaches 0 BTPC per block
- Tail Emission: 0.5 BTPC per block after year 24
```

### Section 3.3 - Economic Principles
1. **Predictability**: Linear decay provides predictable inflation schedule
2. **Sustainability**: Gradual reduction ensures long-term mining viability
3. **Fair Distribution**: 24-year linear distribution period
4. **Tail Emission**: Permanent 0.5 BTPC block reward ensures network security

---

## Article IV: Consensus Mechanism

### Section 4.1 - Proof of Work
- **Algorithm**: SHA-512 based proof of work
- **Difficulty Adjustment**: Every 2016 blocks (same as Bitcoin)
- **Target Block Time**: 10 minutes
- **Mining**: CPU and GPU friendly (no ASICs initially due to SHA-512)

### Section 4.2 - Validation Rules
- **Block Validation**: Bitcoin-compatible block structure validation
- **Transaction Validation**: ML-DSA signature verification
- **UTXO Model**: Unspent Transaction Output model (same as Bitcoin)
- **Chain Selection**: Longest valid chain rule

---

## Article V: Software Architecture

### Section 5.1 - Core Components
- **Blockchain Engine**: Rust-based quantum-resistant blockchain core
- **Consensus Layer**: Difficulty adjustment and proof-of-work validation
- **Network Layer**: P2P networking and block propagation
- **Database Layer**: UTXO storage and blockchain state management
- **RPC Interface**: JSON-RPC API for wallet and application integration

### Section 5.2 - Module Structure
```
btpc-core/
├── blockchain/     # Chain logic and block validation
├── consensus/      # PoW and difficulty management
├── crypto/         # ML-DSA signatures and SHA-512 hashing
├── database/       # UTXO and block storage
├── network/        # P2P protocol and sync
└── rpc/           # API interface
```

### Section 5.3 - Binary Applications
- **btpc_miner**: Mining application with SHA-512 PoW
- **btpc_wallet**: Quantum-resistant wallet with ML-DSA
- **genesis_tool**: Network genesis block generation
- **integrated_mining_demo**: Development and testing tool

---

## Article VI: Development Principles

### Section 6.1 - Code Quality Standards
- **Language**: Rust for memory safety and performance
- **Testing**: Comprehensive unit and integration tests
- **Documentation**: Inline documentation for all public APIs
- **Security**: Quantum-resistant cryptography throughout

### Section 6.2 - Compatibility Requirements
- **Bitcoin Compatibility**: Maintain Bitcoin's proven blockchain structure
- **Network Compatibility**: Standard P2P protocols where possible
- **API Compatibility**: Bitcoin-compatible RPC interface
- **Wallet Compatibility**: Support standard wallet derivation paths

---

## Article VII: Governance and Amendment Process

### Section 7.1 - Constitutional Authority
- This Constitution is the supreme authority for BTPC development
- NO changes to core specifications without explicit constitutional amendment
- All development must comply with constitutional requirements
- Deviations from this Constitution are PROHIBITED

### Section 7.2 - Amendment Process
1. **Proposal**: Constitutional amendments must be explicitly proposed
2. **Documentation**: All changes must be documented with justification
3. **Review**: Technical review of constitutional compliance required
4. **Approval**: Formal approval process for constitutional amendments
5. **Version Control**: All amendments must increment the Constitution version

### Section 7.3 - Prohibited Changes
The following changes are PROHIBITED without constitutional amendment:
- Changing from SHA-512 to any other hashing algorithm
- Changing from ML-DSA to any other signature scheme
- Modifying the linear decay economic model
- Altering the 10-minute block time
- Changing the 32.375 BTPC initial reward
- Modifying the 24-year linear decay period
- Removing quantum-resistance features

---

## Article VIII: Implementation Standards

### Section 8.1 - Required Features
All BTPC implementations MUST include:
- SHA-512 proof of work
- ML-DSA quantum-resistant signatures
- Linear decay reward calculation
- Bitcoin-compatible transaction structure
- UTXO model implementation
- 64-byte hash arrays throughout

### Section 8.2 - Forbidden Features
All BTPC implementations MUST NOT include:
- Non-quantum-resistant signature schemes
- Halving-based reward systems
- Proof-of-stake or other consensus mechanisms
- Smart contract capabilities (Bitcoin model only)
- Privacy features that compromise transparency

---

## Article IX: Network Launch Parameters

### Section 9.1 - Genesis Block
- **Mainnet**: Specific genesis with mainnet message
- **Testnet**: Test network genesis for development
- **Regtest**: Local testing environment

### Section 9.2 - Initial Difficulty
- **Starting Difficulty**: Appropriate for initial network hashrate
- **Adjustment Period**: First 2016 blocks for network stabilization

---

## Article X: Long-term Vision

### Section 10.1 - Quantum Readiness
BTPC is designed to remain secure in a post-quantum world through:
- ML-DSA signatures resistant to quantum attacks
- Conservative cryptographic choices
- Future-proof architecture design

### Section 10.2 - Economic Sustainability
The linear decay model ensures:
- Predictable monetary policy for 24 years
- Sustainable miner incentives
- Long-term network security through tail emission

---

## CONSTITUTIONAL ENFORCEMENT

**This Constitution is BINDING and IMMUTABLE without explicit amendment.**

All contributors, developers, and users of BTPC acknowledge that:
1. This Constitution defines the authoritative BTPC specification
2. No implementation may deviate from constitutional requirements
3. All changes must comply with the amendment process
4. Violation of constitutional principles invalidates any proposed changes

**Any code, feature, or modification that violates this Constitution is REJECTED by definition.**

---

*End of Constitution*

**Established**: September 24, 2025
**Next Review**: Upon explicit amendment proposal
**Hash**: [To be calculated upon finalization]
**Version**: 1.0.0 | **Ratified**: 2025-09-27 | **Last Amended**: 2025-09-27
---

## Article XI: Desktop Application Development (ADDED 2025-10-11)

### Section 11.1 - Single Source of Truth Principle
- **Backend State Authority**: Backend (Rust/Tauri) is the ONLY source of truth for all critical state
- **Frontend as View**: Frontend (JavaScript/HTML) displays backend state, never maintains authoritative state
- **Validation Order**: ALWAYS validate with backend FIRST, then save to localStorage/browser storage
- **State Synchronization**: Use Tauri event system for cross-page state synchronization

### Section 11.2 - State Management Patterns (MANDATORY)
```
CORRECT Flow (Settings Example):
1. User changes setting → Frontend sends to backend
2. Backend validates (e.g., "node must be stopped")
3. If INVALID: Return error, frontend shows error, NO localStorage save
4. If VALID: Backend saves, emits event, frontend updates UI, saves to localStorage

INCORRECT Flow (FORBIDDEN):
1. User changes setting → Frontend saves to localStorage immediately
2. Frontend sends to backend → Backend validation fails
3. localStorage has invalid state, backend has different state
4. Result: Desynchronized state, confusing user experience
```

### Section 11.3 - Event-Driven Architecture
- **Backend Emission**: Backend MUST emit events on all state changes
- **Frontend Listening**: Frontend MUST listen for events and update UI
- **Memory Management**: Event listeners MUST be cleaned up on page unload
- **No Polling**: Use events instead of polling for state updates where possible

### Section 11.4 - Desktop App Error Handling
- **Clear Error Messages**: All errors MUST explain what happened and what the user should do
- **User Guidance**: Errors MUST cite specific requirements (e.g., "node must be stopped")
- **No Silent Failures**: All validation failures MUST be shown to the user
- **Constitutional References**: Critical errors SHOULD reference constitution when applicable

### Section 11.5 - Process Lifecycle Management
- **Detached Processes**: Use ProcessManager for node/miner processes that survive page navigation
- **Cleanup on Close**: MUST stop all managed processes when application closes
- **State Verification**: Verify process actually stopped before updating UI state
- **No Orphaned Processes**: Zero tolerance for orphaned processes after app close

### Section 11.6 - Frontend Development Standards
- **Event Listener Cleanup**: Store unlisten functions, call on page unload
- **Single Toast Notifications**: Prevent duplicate toasts with action flags
- **Backend-First Validation**: Never save to localStorage before backend validation
- **Cross-Page Consistency**: All pages must show identical state at all times

### Section 11.7 - Prohibited Desktop App Patterns
The following patterns are PROHIBITED in desktop app development:
- Saving to localStorage before backend validation
- Maintaining authoritative state in frontend JavaScript
- Polling for state updates when events are available
- Not cleaning up event listeners on page unload
- Silent backend validation failures
- Duplicate notifications for user actions
- Inconsistent state between pages

---

**Constitution Version Updated**: 1.0.1 | **Last Amended**: 2025-10-11 (Added Article XI - Desktop Application Development)
