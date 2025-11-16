# Feature Specification: Fix BIP39 Seed Phrase Determinism

**Feature Branch**: `008-fix-bip39-seed`
**Created**: 2025-11-06
**Status**: Draft
**Input**: CRITICAL: BIP39 seed phrase recovery is broken - wallet recovery with same mnemonic generates different keys
**Constitution**: Article VI.3 TDD Required

## Execution Flow (main)
```
1. ✅ Parse user description from Input
   → CRITICAL security issue: non-deterministic key generation from BIP39 seeds
2. ✅ Extract key concepts from description
   → Actors: wallet users, cryptocurrency holders
   → Actions: wallet creation, recovery from mnemonic, key generation
   → Blockchain data: ML-DSA private keys, public keys, wallet metadata
   → Quantum-resistant requirements: maintain ML-DSA (Dilithium5) signatures
3. ✅ For each unclear aspect: NONE - issue is well-defined
4. ✅ Check constitutional requirements:
   → Article VI.3 TDD methodology applies (cryptographic correctness required)
   → Desktop app feature: Article XI patterns apply for wallet manager UI
5. ✅ Fill User Scenarios & Testing section: COMPLETE
6. ✅ Generate Functional Requirements: COMPLETE
7. ✅ Identify Key Entities: PrivateKey, Mnemonic, Seed, Wallet
8. ✅ Run Review Checklist: COMPLETE
9. ✅ Return: SUCCESS (spec ready for planning)
```

---

## ⚡ Quick Guidelines
- ✅ Focus on WHAT users need and WHY
- ❌ Avoid HOW to implement (no Rust code, Tauri commands, RocksDB schemas)
- 👥 Written for cryptocurrency users and stakeholders, not developers
- 🔒 Always consider quantum-resistance and security implications

### Section Requirements
- **Mandatory sections**: Must be completed for every feature
- **Optional sections**: Include only when relevant to the feature
- When a section doesn't apply, remove it entirely (don't leave as "N/A")

---

## User Scenarios & Testing *(mandatory)*

### Primary User Story
**As a** cryptocurrency wallet owner,
**I want to** recover my wallet using my 24-word BIP39 seed phrase,
**So that** I can access my funds if I lose my device or need to restore on a different device

### Acceptance Scenarios
1. **Given** user creates a new wallet with BIP39 seed phrase, **When** user writes down the 24-word mnemonic and later imports it on the same device, **Then** the recovered wallet MUST have identical ML-DSA keys and addresses
2. **Given** user creates wallet on device A with seed phrase, **When** user imports the same seed phrase on device B, **Then** both devices MUST generate identical wallets with the same keys and addresses
3. **Given** user has existing non-deterministic wallet (v1), **When** user checks wallet version, **Then** system MUST display migration warning with instructions to create v2 deterministic wallet
4. **Given** user creates v2 deterministic wallet, **When** user exports and re-imports the seed phrase 100 times, **Then** all 100 recovery attempts MUST produce byte-identical keys

### Edge Cases
- What happens when user tries to recover v1 (non-deterministic) wallet using seed phrase? System MUST warn that recovery is not guaranteed for v1 wallets
- How does system handle malformed BIP39 mnemonics (invalid word count, invalid words, bad checksum)? System MUST reject with clear error message before any key generation
- What if user imports seed phrase for existing wallet ID? System MUST detect duplicate and offer to restore existing wallet or create new one with different ID
- How does wallet manager UI update when switching between v1 and v2 wallets? UI MUST display wallet version badge and recovery capability status (Article XI event-driven updates)

---

## Requirements *(mandatory)*

### Functional Requirements
**Core Cryptographic Operations:**
- **FR-001**: System MUST generate ML-DSA (Dilithium5) keypairs deterministically from BIP39 seed phrases (same seed → same keys, always)
- **FR-002**: System MUST use SHAKE256 (ML-DSA's native PRF) or HKDF-SHA512 for expanding BIP39 seeds to ML-DSA key material
- **FR-003**: System MUST derive 32-byte seed from BIP39 mnemonic using standard BIP39 PBKDF2 process (2048 iterations, empty passphrase)
- **FR-004**: System MUST validate BIP39 mnemonic checksum BEFORE attempting key generation

**Wallet Recovery:**
- **FR-005**: Users MUST be able to import 24-word BIP39 mnemonic and recover wallet with byte-identical keys
- **FR-006**: System MUST support cross-device wallet recovery (same mnemonic on different devices produces same wallet)
- **FR-007**: System MUST persist wallet version metadata (v1=non-deterministic, v2=BIP39-deterministic)
- **FR-008**: System MUST maintain backward compatibility with existing v1 wallets (can still open and use, but warn about limited recovery)

**Desktop Application:**
- **FR-009**: Desktop app MUST implement backend-first validation for seed phrase import (Article XI, Section 11.2)
- **FR-010**: Desktop app MUST use Tauri events for wallet recovery status updates (Article XI, Section 11.3)
- **FR-011**: Desktop app MUST display wallet version badge (v1 vs v2) with recovery capability indicator
- **FR-012**: System MUST show migration warning for v1 wallets: "Your wallet was created with an older version. For proper seed phrase recovery, please create a new v2 wallet and transfer funds."

**Security Requirements:**
- **FR-013**: System MUST never log or expose seed phrases in error messages or debug output
- **FR-014**: System MUST use constant-time operations for all seed-to-key derivation steps
- **FR-015**: System MUST validate that generated ML-DSA keys meet minimum entropy requirements (key size: 4000 bytes for private, 1952 bytes for public)
- **FR-016**: System MUST clear sensitive seed material from memory after key generation (use Zeroizing types)

**Performance Requirements:**
- **FR-017**: BIP39 mnemonic validation MUST complete in < 100ms
- **FR-018**: Deterministic key generation from seed MUST complete in < 500ms
- **FR-019**: Wallet recovery (full process) MUST complete in < 2 seconds

### Non-Functional Requirements
**Security:**
- **NFR-001**: All key derivation operations MUST use cryptographically secure randomness extraction (SHAKE256 or HKDF)
- **NFR-002**: System MUST include domain separation tag in key derivation ("BTPC-ML-DSA-v1") to prevent key reuse across contexts
- **NFR-003**: Seed phrase import MUST validate mnemonic checksum to prevent typos causing silent wallet creation with wrong keys

**Compatibility:**
- **NFR-004**: System MUST follow NIST FIPS 204 (ML-DSA) guidelines for deterministic key generation
- **NFR-005**: System MUST maintain BIP39 standard compliance for mnemonic generation and parsing
- **NFR-006**: Existing v1 wallets MUST continue to function without data migration (opt-in upgrade)

**Usability:**
- **NFR-007**: Error messages MUST distinguish between invalid mnemonic (user error) and key generation failure (system error)
- **NFR-008**: Desktop app MUST show clear visual indicator when wallet has deterministic recovery capability
- **NFR-009**: Migration warning MUST include step-by-step instructions with fund transfer checklist

### Key Entities
**Blockchain/Wallet Entities:**
- **Mnemonic**: 24-word BIP39 seed phrase displayed to user for wallet recovery. Contains: 24 English words from BIP39 wordlist, embedded checksum (last 8 bits), entropy (256 bits). Relationships: derives to Seed via PBKDF2
- **Seed**: 32-byte deterministic seed derived from BIP39 mnemonic. Contains: 256-bit entropy from PBKDF2, used as input for ML-DSA key generation. Relationships: derived from Mnemonic, expands to PrivateKey via SHAKE256/HKDF
- **PrivateKey**: ML-DSA (Dilithium5) private key for signing transactions. Contains: 4000-byte key material, optional seed reference (32 bytes), keypair for signing. Relationships: derived from Seed, generates PublicKey, signs Transactions
- **Wallet**: Encrypted wallet file with deterministic recovery capability. Contains: wallet ID (UUID), wallet version (v1/v2), ML-DSA keys, metadata. Relationships: created from PrivateKey, persisted to .dat file, belongs to User

**Desktop App Entities:**
- **WalletMetadata**: Wallet information displayed in UI. Backend (Rust Arc<RwLock>) is single source of truth (Article XI). Contains: wallet ID, name, version (v1/v2), recovery status, balance, last sync height. Persisted in encrypted wallet file, validated before UI updates.

---

## Constitutional Compliance *(mandatory for desktop features)*

### Article XI Applicability
- [ ] **Not a desktop feature** - Skip Article XI patterns
- [x] **Desktop feature** - Article XI patterns apply (complete checklist below)

### Article XI Compliance Checklist

**Section 11.1 - Single Source of Truth:**
- [x] Identify authoritative state location: Backend Arc<RwLock<WalletManager>> stores wallet metadata including version
- [x] Frontend displays wallet version and recovery status only, never maintains authoritative wallet state
- [x] Specified: Wallet metadata stored in Rust backend, queried via Tauri commands (get_wallet_info)

**Section 11.2 - Backend-First Validation:**
- [x] All seed phrase imports validate with backend FIRST (BIP39 checksum, word validation, key generation test)
- [x] Failure exits early, NO wallet creation on validation failure
- [x] Specified: Validation errors return specific messages ("Invalid BIP39 checksum", "Key generation failed", "Invalid word: xyz")

**Section 11.3 - Event-Driven Architecture:**
- [x] Backend emits events on wallet recovery status changes (wallet:recovery:started, wallet:recovery:progress, wallet:recovery:completed, wallet:recovery:failed)
- [x] Frontend listens for events and updates UI (progress bar, status messages, wallet list refresh)
- [x] Specified: Event names, payloads {wallet_id, status, progress_percent, error_message}, update behavior

**Section 11.6 - Event Listener Cleanup:**
- [x] Event listeners cleaned up on page unload (wallet-manager.html beforeunload handler)
- [x] No memory leaks from forgotten listeners
- [x] Specified: Cleanup mechanism (unlisten functions stored in global array, called on beforeunload)

**Section 11.7 - Prohibited Patterns:**
- [x] Confirmed: NO localStorage before backend validation (seed phrase never stored client-side)
- [x] Confirmed: NO authoritative state in frontend JavaScript (wallet version comes from backend)
- [x] Confirmed: NO polling when events available (recovery status via events, not polling)
- [x] Confirmed: NO duplicate notifications for recovery actions (events fire once per state change)

---

## Dependencies & Assumptions

### Dependencies
- BIP39 mnemonic parsing library (bip39 crate v2.0+) - already in use
- ML-DSA (Dilithium5) cryptographic library - requires deterministic generation API
- SHA-3 library for SHAKE256 (sha3 crate) - for expanding seeds
- btpc-core wallet serialization module - must support version field
- Tauri events system (Article XI) - for wallet recovery status updates

### Assumptions
- Users have secure storage for 24-word seed phrases (paper backup, metal cards)
- Existing v1 wallet users understand migration is recommended but optional
- Desktop app has write access to ~/.btpc directory for wallet files
- BIP39 mnemonic uses empty passphrase (standard BTPC configuration)
- Network connectivity not required for wallet recovery (keys generated locally)

---

## Review & Acceptance Checklist

### Content Quality
- [x] No implementation details (no Rust code, Tauri command names, RocksDB column families)
- [x] Focused on user value and cryptocurrency operations
- [x] Written for non-technical cryptocurrency stakeholders
- [x] All mandatory sections completed
- [x] BTPC-specific considerations addressed (quantum-resistance, ML-DSA, wallet versioning)

### Requirement Completeness
- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable (< 100ms validation, < 500ms key generation, byte-identical keys)
- [x] Scope is clearly bounded (IN: BIP39 determinism, wallet versioning, v1/v2 compatibility; OUT: multi-account wallets, custom derivation paths, hardware wallet integration)
- [x] Dependencies and assumptions identified
- [x] Security implications considered (constant-time ops, memory clearing, domain separation)
- [x] Performance targets specified (validation < 100ms, generation < 500ms, recovery < 2s)

### Constitutional Compliance (Desktop Features Only)
- [x] Article XI applicability determined (desktop feature)
- [x] All Article XI patterns addressed in requirements (FR-009 through FR-012)
- [x] Constitutional compliance checklist completed
- [x] Article VI.3 TDD methodology flagged (cryptographic correctness requires test-driven approach)

---

## Execution Status

- [x] User description parsed
- [x] Key concepts extracted (BIP39 determinism, ML-DSA key generation, wallet recovery, versioning)
- [x] Constitutional requirements flagged (Article VI.3 TDD, Article XI desktop patterns)
- [x] No ambiguities - issue is well-defined
- [x] User scenarios defined (wallet recovery, cross-device restore, v1/v2 migration)
- [x] Functional requirements generated (19 requirements covering crypto, recovery, UI, security, performance)
- [x] Entities identified (Mnemonic, Seed, PrivateKey, Wallet, WalletMetadata)
- [x] Constitutional compliance evaluated (Article XI checklist complete)
- [x] Review checklist passed

---

## BTPC Project Context

**Core Technologies:**
- Blockchain: Rust, btpc-core library, RocksDB, SHA-512 PoW
- Cryptography: ML-DSA (Dilithium5), AES-256-GCM, Argon2id
- Desktop: Tauri 2.0, vanilla JS frontend, 68 Tauri commands
- Wallet: BIP39 mnemonics (24 words), encrypted .dat files

**Constitutional Framework:**
- Constitution version: 1.0.1
- Article VI.3: TDD Methodology - REQUIRED for cryptographic correctness
- Article XI: Desktop Application Development Principles - REQUIRED for wallet manager UI
- See `.specify/memory/constitution.md` for complete governance rules

**Project Structure:**
- `btpc-core/src/crypto/keys.rs` - PrivateKey implementation (current non-deterministic from_seed method)
- `btpc-desktop-app/src-tauri/src/wallet_commands.rs` - Wallet creation and recovery commands
- `btpc-desktop-app/ui/wallet-manager.html` - Wallet manager UI (displays seed phrases)

**Key Documentation:**
- `CLAUDE.md` - Project overview and guidelines
- `MD/CRITICAL_BIP39_DETERMINISM_ISSUE.md` - Detailed technical analysis of this bug
- `style-guide/ux-rules.md` - UI/UX patterns (Monero-inspired)
- `.specify/memory/constitution.md` - Governance rules

**Security Context:**
- CRITICAL ISSUE: Current implementation breaks fundamental promise of wallet recovery
- Users cannot recover funds with seed phrase backup
- False sense of security - users think backup works but it doesn't
- Must maintain ML-DSA (Dilithium5) quantum-resistance while fixing determinism

---

**Template Version**: 1.1 (BTPC-specific)
**Last Updated**: 2025-11-06
**Maintained by**: .specify framework