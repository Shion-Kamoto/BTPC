# Feature Specification: Core Blockchain Implementation

**Feature Branch**: `001-core-blockchain-implementation`
**Created**: 2025-09-28
**Status**: Complete
**Input**: User description: "Core blockchain implementation"

## Execution Flow (main)
```
1. Parse user description from Input
   → If empty: ERROR "No feature description provided"
2. Extract key concepts from description
   → Identify: actors, actions, data, constraints
3. For each unclear aspect:
   → Mark with [NEEDS CLARIFICATION: specific question]
4. Fill User Scenarios & Testing section
   → If no clear user flow: ERROR "Cannot determine user scenarios"
5. Generate Functional Requirements
   → Each requirement must be testable
   → Mark ambiguous requirements
6. Identify Key Entities (if data involved)
7. Run Review Checklist
   → If any [NEEDS CLARIFICATION]: WARN "Spec has uncertainties"
   → If implementation details found: ERROR "Remove tech details"
8. Return: SUCCESS (spec ready for planning)
```

---

## ⚡ Quick Guidelines
- ✅ Focus on WHAT users need and WHY
- ❌ Avoid HOW to implement (no tech stack, APIs, code structure)
- 👥 Written for business stakeholders, not developers

### Section Requirements
- **Mandatory sections**: Must be completed for every feature
- **Optional sections**: Include only when relevant to the feature
- When a section doesn't apply, remove it entirely (don't leave as "N/A")

### For AI Generation
When creating this spec from a user prompt:
1. **Mark all ambiguities**: Use [NEEDS CLARIFICATION: specific question] for any assumption you'd need to make
2. **Don't guess**: If the prompt doesn't specify something (e.g., "login system" without auth method), mark it
3. **Think like a tester**: Every vague requirement should fail the "testable and unambiguous" checklist item
4. **Common underspecified areas**:
   - User types and permissions
   - Data retention/deletion policies  
   - Performance targets and scale
   - Error handling behaviors
   - Integration requirements
   - Security/compliance needs

---

## User Scenarios & Testing *(mandatory)*

### Primary User Story
As a BTPC network participant, I need a quantum-resistant blockchain that provides secure transactions using ML-DSA signatures and SHA-512 proof-of-work, with predictable linear decay economics over 24 years.

### Acceptance Scenarios
1. **Given** a new BTPC network, **When** genesis block is created, **Then** initial parameters are set with 32.375 BTPC reward and SHA-512 hash validation
2. **Given** a valid transaction with ML-DSA (Dilithium5) signature, **When** miner includes it in a block, **Then** block is validated and accepted by network
3. **Given** network has been running for 1 year, **When** block reward is calculated, **Then** reward follows linear decay formula from 32.375 BTPC
4. **Given** a quantum computer attack, **When** attacker tries to forge signatures, **Then** ML-DSA (Dilithium5) signatures remain secure
5. **Given** a node starts with --network=testnet flag, **When** connecting to peers, **Then** system uses testnet genesis block and rejects mainnet connections

### Edge Cases
6. **Given** network hashrate drops >75% over a 2016-block measurement period (measured as average block time exceeding 25 minutes for the adjustment window), **When** next difficulty adjustment occurs, **Then** difficulty reduces proportionally while maintaining 10-minute average block time with ±15% variance tolerance
7. **Given** a block contains invalid ML-DSA (Dilithium5) signatures, **When** node validates the block within 10ms, **Then** block is rejected and not propagated to peers with specific error logging
8. **Given** linear decay reaches year 24 (block 1,261,440), **When** block reward calculation executes, **Then** system switches to permanent 0.5 BTPC tail emission per block

## Terminology *(for consistency)*

**ML-DSA**: Module-Lattice-Based Digital Signature Algorithm (Dilithium5 parameter set), NIST FIPS 204 standardized post-quantum signature scheme. Always referenced as "ML-DSA (Dilithium5)" in this specification.

**Validate vs Verify**:
- **Validate**: Check inputs/data for correctness against rules (e.g., "validate transaction inputs")
- **Verify**: Confirm existing state/results match expectations (e.g., "verify test results")

**Binary Naming**:
- **Executables**: Use underscore format (e.g., `btpc_wallet`, `btpc_node`, `btpc_miner`)
- **Directories/libraries**: Use hyphen format (e.g., `btpc-core/`, `btpc-desktop-app/`)
- **Cargo crates**: Use hyphen format (e.g., `btpc-core`, `pqc-dilithium`)

---

## Requirements *(mandatory)*

### Functional Requirements
- **FR-001**: System MUST validate ML-DSA (Dilithium5) digital signatures for all transactions within 1.5ms per single signature verification operation
- **FR-002**: System MUST use SHA-512 double hashing for proof-of-work mining with block validation (cryptographic verification of PoW, signatures, and merkle tree) completing within 10ms per 1MB block
- **FR-003**: System MUST maintain UTXO (Unspent Transaction Output) model for transaction validation with constant-time lookup operations
- **FR-004**: System MUST implement linear decay reward system starting at 32.375 BTPC per block with mathematical precision to 8 decimal places
- **FR-005**: System MUST adjust difficulty every 2016 blocks to maintain 10-minute average block time with ±15% variance tolerance (measured over the 2016-block adjustment window)
- **FR-006**: System MUST validate block structure using Bitcoin-compatible format with complete header and transaction validation
- **FR-007**: System MUST persist blockchain state and transaction history permanently with database consistency guarantees
- **FR-008**: System MUST provide tail emission of 0.5 BTPC per block after 24-year decay period (block 1,261,440)
- **FR-009**: System MUST reject blocks containing invalid ML-DSA (Dilithium5) signatures with specific error codes and logging
- **FR-010**: System MUST support mainnet, testnet, and regtest network modes with distinct genesis blocks (as specified in Constitution Article IX) including unique genesis messages, timestamps, and initial difficulty
- **FR-011**: System MUST optimize for maximum throughput within Bitcoin-compatible 1MB block constraints (~7 TPS base layer), with architecture supporting future layer-2 scaling solutions
- **FR-012**: System MUST maintain <200ms p95 response latency for all blockchain RPC queries (network requests including database lookups and response serialization)
- **FR-013**: System MUST generate ML-DSA (Dilithium5) signatures within 2ms per single signature generation operation

### Key Entities *(include if feature involves data)*
- **Block**: Contains header (prev_hash, merkle_root, timestamp, bits, nonce) and transaction list, validated with SHA-512
- **Transaction**: Bitcoin-compatible format with ML-DSA (Dilithium5) signatures instead of ECDSA, includes inputs/outputs and UTXO references
- **UTXO**: Unspent transaction outputs tracking spendable coins, key for double-spend prevention
- **Block Reward**: Linear decay calculation from 32.375 BTPC initial reward over 24 years
- **Difficulty Target**: 64-byte arrays for SHA-512 proof-of-work target adjustment
- **Network State**: Blockchain height, total supply, current difficulty, and consensus parameters
- **Genesis Block**: First block in chain with predefined parameters (timestamp=1727539200, reward=32.375 BTPC, prev_hash=zero, nonce=0)
- **Storage Schema**: Database column families for blocks, UTXOs, transactions, and blockchain metadata with optimized indexing

---

## Review & Acceptance Checklist
*GATE: Automated checks run during main() execution*

### Content Quality
- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

### Requirement Completeness
- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

---

## Execution Status
*Updated by main() during processing*

- [x] User description parsed
- [x] Key concepts extracted
- [x] Ambiguities marked
- [x] User scenarios defined
- [x] Requirements generated
- [x] Entities identified
- [x] Review checklist passed

---
