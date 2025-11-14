# .specify Framework Templates Updated for BTPC

**Date:** 2025-10-11 18:00 UTC
**Status:** ✅ Complete
**Templates Updated:** spec-template.md, tasks-template.md

---

## Summary

Updated both .specify framework templates to be BTPC-specific, incorporating quantum-resistant blockchain requirements, Article XI constitutional patterns, and BTPC project structure. These templates now integrate seamlessly with the `/start` and `/stop` session handoff commands.

---

## Changes Made

### 1. spec-template.md (Feature Specification Template)

**Version:** 1.0 → 1.1 (BTPC-specific)

**Key Updates:**

**Constitutional Integration:**
- Added "Constitution: Article XI Compliance Required for Desktop Features" header
- Added constitutional requirement checking in execution flow
- Added complete "Constitutional Compliance" section with Article XI checklist
- Flags desktop features for Article XI pattern application

**BTPC-Specific Guidelines:**
- Added quantum-resistance considerations
- Added blockchain layer specifications (Mainnet/Testnet/Regtest)
- Added crypto operation specifications (ML-DSA/Dilithium5)
- Added desktop app state management requirements (Article XI)
- Added wallet encryption requirements (AES-256-GCM, Argon2id)
- Added common underspecified areas in BTPC projects

**Requirements Section:**
- Added blockchain-specific functional requirements (FR-001 to FR-004)
- Added wallet & cryptography requirements (FR-005 to FR-008)
- Added desktop application requirements with Article XI references (FR-009 to FR-012)
- Added network & node operation requirements (FR-013 to FR-015)
- Added performance requirements with specific targets (FR-016 to FR-018)
- Added non-functional requirements (security, performance, usability)

**Key Entities Section:**
- Added blockchain entities (Block, Transaction, UTXO, Wallet)
- Added desktop app entities (NetworkConfig, NodeStatus, WalletMetadata)
- Included Article XI single source of truth references

**User Scenarios:**
- Added BTPC-specific user story format (cryptocurrency user/miner/node operator)
- Added quantum-resistant transaction examples
- Added blockchain edge cases (forks, orphaned blocks, network partitions)
- Added desktop app state synchronization scenarios

**Project Context Section:**
- Added complete BTPC tech stack overview
- Added constitutional framework version (1.0.1)
- Added project structure reference
- Added key documentation pointers

**Article XI Compliance Checklist:**
- Section 11.1: Single source of truth verification
- Section 11.2: Backend-first validation patterns
- Section 11.3: Event-driven architecture requirements
- Section 11.6: Event listener cleanup
- Section 11.7: Prohibited patterns checklist

---

### 2. tasks-template.md (Implementation Tasks Template)

**Version:** 1.0 → 1.1 (BTPC-specific)

**Key Updates:**

**Constitutional Integration:**
- Added "Constitution: Article XI patterns for desktop features" header
- Added constitutional requirement checking in execution flow
- Added Article XI compliance tasks in Polish phase (T042-T045)
- Flags desktop features for Article XI pattern tasks

**BTPC Path Conventions:**
- Updated with complete BTPC project structure
- Added btpc-core/ structure (blockchain, crypto, consensus, storage, network, rpc)
- Added bins/ structure (btpc_node, btpc_wallet, btpc_miner)
- Added btpc-desktop-app/ structure (src-tauri, ui)
- Added tests/ structure (integration, contract)

**Task Phases:**

**Phase 3.1 - Setup:**
- Rust-specific dependencies (pqcrypto-dilithium, rocksdb, tokio, tauri)
- BTPC code quality tools (clippy.toml, rustfmt.toml)
- Tauri permissions configuration

**Phase 3.2 - Tests (TDD):**
- Blockchain/core tests (ML-DSA validation, UTXO consistency, block validation)
- Desktop app tests with Article XI references
- Contract tests for Tauri commands
- Backend-first validation tests (Article XI, Section 11.2)
- Event synchronization tests (Article XI, Section 11.3)
- Memory leak tests (Article XI, Section 11.6)

**Phase 3.3 - Core Implementation:**
- Blockchain/crypto implementation (Block, Transaction, UTXO, ML-DSA, SHA-512)
- Desktop app implementation with Article XI patterns
- Backend state management (Arc<RwLock>) - Article XI, Section 11.1
- Event emission - Article XI, Section 11.3
- Event cleanup - Article XI, Section 11.6
- Backend-first validation - Article XI, Section 11.2
- RPC/API implementation

**Phase 3.4 - Integration:**
- RocksDB column families setup
- UTXO and block persistence
- ML-DSA library integration (constant-time operations)
- AES-256-GCM wallet encryption with Argon2id
- P2P protocol integration

**Phase 3.5 - Polish:**
- Code quality tasks (clippy, unit tests, performance tests)
- Article XI compliance verification tasks (T042-T045)
- Documentation updates (CLAUDE.md, STATUS.md)
- Security validation (cargo audit, constant-time crypto, no key exposure)

**Task Generation Rules:**
- Added blockchain-specific rules (validation tests, consensus tests, crypto tests)
- Added desktop-specific rules with Article XI patterns
- Added BTPC-specific ordering (blockchain → crypto → desktop → storage → network)

**Dependencies Section:**
- Added blockchain dependencies (Block → UTXO → BlockchainService)
- Added desktop app dependencies with Article XI references
- Added storage dependencies (RocksDB → persistence)

**Validation Checklist:**
- Added blockchain-specific validation (crypto constant-time, entity validation)
- Added desktop-specific validation (Article XI compliance)
- Added constitutional compliance checks

**Project Context Section:**
- Added BTPC tech stack (Rust 1.75+, btpc-core, RocksDB, Tauri 2.0)
- Added code quality standards (clippy -D warnings, rustfmt, no unsafe)
- Added performance targets (< 100ms validation, < 10ms signatures, < 200ms UI updates)
- Added constitutional framework reference
- Added key documentation pointers

---

## Integration with Session Handoff Commands

### /start Command Integration

**Current Integration (Already Active):**
The `/start` command already reads the constitution FIRST (Step 1) before any other documentation.

**Template Usage:**
When generating specs or tasks during a session, these templates will now:
- Automatically flag blockchain features for quantum-resistance requirements
- Automatically flag desktop features for Article XI compliance
- Include BTPC-specific requirements, entities, and test scenarios
- Use BTPC project structure paths
- Reference constitutional patterns where applicable

**Example:**
```bash
/start implement wallet synchronization feature
```
1. Reads constitution v1.0.1 (Article XI patterns)
2. Identifies "wallet" + "synchronization" → likely desktop feature
3. If using `/specify` to create spec → uses updated spec-template.md
4. Spec will include Article XI compliance checklist
5. If using `/plan` → generates plan with Article XI patterns
6. If using `/implement` → uses updated tasks-template.md with Article XI tasks

### /stop Command Integration

**Current Integration (Already Active):**
The `/stop` command already reviews .specify framework documentation (Step 2) and checks Article XI compliance.

**Template Usage:**
When ending a session, the `/stop` command will:
- Use updated templates if generating any new specs/tasks
- Verify constitutional compliance against Article XI checklist
- Include template version in session summary
- Document any template deviations in handoff

**Example:**
```bash
/stop
```
1. Reviews constitution v1.0.1 (Article XI)
2. Checks session work against Article XI compliance
3. If new features were spec'd → verifies used spec-template.md v1.1
4. If new tasks were created → verifies used tasks-template.md v1.1
5. Documents compliance status in session summary
6. Updates STATUS.md with constitutional compliance notes

---

## How to Use Updated Templates

### For Feature Specifications (`/specify`)

**When to use:**
Creating specs for new BTPC features (blockchain, wallet, desktop UI)

**What to expect:**
1. Template will prompt for quantum-resistance requirements
2. Desktop features will include Article XI compliance checklist
3. Blockchain features will include network type specifications
4. All specs will reference BTPC project structure
5. Performance targets will be specific (< 100ms, < 10ms, etc.)

**Example usage:**
```bash
/specify Add hardware wallet support
```
Generated spec will include:
- Blockchain entities (UTXO, Transaction)
- Wallet entities (encrypted keys)
- Desktop app requirements (Article XI patterns)
- Quantum-resistance requirements (ML-DSA)
- BTPC-specific acceptance scenarios

### For Implementation Tasks (`/implement`)

**When to use:**
Generating implementation tasks from a feature plan

**What to expect:**
1. Tasks will use BTPC project structure paths
2. Desktop features will include Article XI compliance tasks
3. Blockchain features will include crypto validation tests
4. All tasks will follow TDD (tests before implementation)
5. Performance tests will have specific targets

**Example usage:**
```bash
/implement specs/042-hardware-wallet-support/plan.md
```
Generated tasks will include:
- Blockchain tests (ML-DSA signature validation)
- Desktop tests (Article XI patterns)
- Implementation tasks with exact file paths
- Article XI compliance verification tasks (T042-T045)
- Performance tests with BTPC targets

---

## Template Versioning

### spec-template.md
- **Version**: 1.1 (BTPC-specific)
- **Last Updated**: 2025-10-11
- **Previous Version**: 1.0 (generic)
- **Breaking Changes**: None (backward compatible)
- **New Sections**: Constitutional Compliance, BTPC Project Context
- **Modified Sections**: User Scenarios, Requirements, Key Entities

### tasks-template.md
- **Version**: 1.1 (BTPC-specific)
- **Last Updated**: 2025-10-11
- **Previous Version**: 1.0 (generic)
- **Breaking Changes**: None (backward compatible)
- **New Sections**: BTPC Path Conventions, Article XI Compliance tasks, BTPC Project Context
- **Modified Sections**: All task phases, Dependencies, Validation Checklist

---

## Validation

### Template Quality Checks

**spec-template.md:**
- ✅ All BTPC technologies referenced (Rust, Tauri, ML-DSA, RocksDB)
- ✅ Article XI compliance integrated throughout
- ✅ Quantum-resistance requirements mandatory
- ✅ BTPC project structure documented
- ✅ Performance targets specific (not "fast" or "slow")
- ✅ Blockchain entities defined (Block, UTXO, Transaction, Wallet)
- ✅ Desktop entities reference Article XI
- ✅ Example scenarios use BTPC terminology
- ✅ Constitutional compliance checklist complete

**tasks-template.md:**
- ✅ BTPC path conventions match actual project structure
- ✅ Task phases include blockchain-specific tasks
- ✅ Task phases include desktop-specific tasks with Article XI
- ✅ TDD mandatory (tests before implementation)
- ✅ Article XI compliance tasks in Polish phase
- ✅ Dependencies reference Article XI patterns
- ✅ Validation checklist includes constitutional compliance
- ✅ Performance targets match BTPC standards
- ✅ Code quality standards reference clippy, rustfmt

### Integration Validation

**With /start command:**
- ✅ Constitution read FIRST (already implemented)
- ✅ Article XI patterns understood before spec/task generation
- ✅ Templates reference constitutional patterns
- ✅ BTPC project structure known

**With /stop command:**
- ✅ Constitution reviewed (already implemented)
- ✅ Article XI compliance checked (already implemented)
- ✅ Template versions can be documented in session summary
- ✅ Constitutional compliance status tracked

---

## Benefits

### For Development

**Consistency:**
- All specs follow same BTPC-specific format
- All tasks use same BTPC project structure
- All desktop features include Article XI patterns
- All blockchain features include quantum-resistance requirements

**Quality:**
- Performance targets are specific and measurable
- Security requirements are explicit (constant-time, no key exposure)
- Constitutional patterns are mandatory for desktop features
- TDD is enforced (tests before implementation)

**Documentation:**
- Specs reference BTPC tech stack
- Tasks reference BTPC documentation
- All features link to constitutional framework
- Project context always available

### For Session Handoff

**Continuity:**
- Next session starts with constitution review (Article XI)
- Templates ensure consistent pattern application
- No knowledge loss between sessions
- Constitutional compliance tracked across sessions

**Governance:**
- Article XI patterns enforced in spec generation
- Constitutional compliance verified in task execution
- Template versions documented in handoffs
- Pattern violations flagged early

### For Future Development

**Scalability:**
- Templates can be extended with new articles
- Project context section can be updated
- Tech stack can evolve
- Performance targets can be adjusted

**Maintainability:**
- Single source of truth for BTPC patterns
- Constitutional patterns centralized
- Project structure documented in templates
- Easy to update as project evolves

---

## Examples

### Example 1: Blockchain Feature Spec

**Feature:** "Add support for mining pools"

**Generated spec will include:**
- Quantum-resistance requirements (ML-DSA signatures)
- Network type considerations (Mainnet/Testnet/Regtest)
- Blockchain entities (Block, Transaction, mining pool metadata)
- Performance requirements (< 100ms block validation)
- P2P protocol requirements
- Consensus rule changes

**Constitutional compliance:** Not applicable (not a desktop feature)

### Example 2: Desktop Feature Spec

**Feature:** "Add wallet backup and restore functionality"

**Generated spec will include:**
- Desktop app requirements with Article XI patterns
- Backend-first validation (Article XI, Section 11.2)
- Event-driven state updates (Article XI, Section 11.3)
- Event listener cleanup (Article XI, Section 11.6)
- Encrypted wallet persistence (AES-256-GCM, Argon2id)
- User scenarios with Tauri event references

**Constitutional compliance:** Mandatory (Article XI checklist included)

### Example 3: Implementation Tasks

**Feature:** "Wallet backup functionality" (from spec above)

**Generated tasks will include:**

**Setup (T001-T004):**
- Configure Tauri permissions for file system access

**Tests (T005-T012):**
- Contract test for Tauri command `backup_wallet`
- Backend-first validation test (Article XI)
- Event synchronization test (Article XI)

**Implementation (T013-T027):**
- Tauri command in `src-tauri/src/wallet_commands.rs`
- Backend state management (Arc<RwLock>) - Article XI
- Event emission on backup completion - Article XI
- React component with event listeners
- Event cleanup on page unload - Article XI

**Polish (T037-T053):**
- Verify backend-first validation (Article XI, Section 11.2)
- Verify event cleanup (Article XI, Section 11.6)
- Cross-page state synchronization test (Article XI, Section 11.3)

---

## Migration from Old Templates

**No migration required** - templates are backward compatible.

**For existing specs:**
- Old specs remain valid
- Can optionally update to new format
- Add Article XI checklist if desktop feature
- Add BTPC project context section

**For existing tasks:**
- Old tasks remain valid
- Can optionally update to new format
- Add Article XI compliance tasks if desktop feature
- Update paths to BTPC structure if needed

---

## Next Steps

### Immediate
- ✅ Templates updated (complete)
- ✅ Integration documented (this file)
- ⏳ Test templates with next feature development
- ⏳ Update any existing specs/tasks if needed

### Short Term
- Monitor template usage in next 2-3 sessions
- Gather feedback on template effectiveness
- Refine based on actual usage patterns
- Consider adding more blockchain-specific examples

### Long Term
- Extend templates for additional articles (if constitution grows)
- Add templates for other document types (architecture, API contracts)
- Create template validation tool
- Automate constitutional compliance checking

---

## Documentation References

**Constitution:**
- `.specify/memory/constitution.md` - v1.0.1, Article XI

**Templates:**
- `.specify/templates/spec-template.md` - v1.1 (this update)
- `.specify/templates/tasks-template.md` - v1.1 (this update)

**Session Handoff:**
- `.claude/commands/start.md` - Resume work with constitution review
- `.claude/commands/stop.md` - End session with compliance check

**Project Documentation:**
- `CLAUDE.md` - Project guidelines
- `STATUS.md` - Current status
- `style-guide/ux-rules.md` - UI/UX patterns
- `MANUAL_TESTING_GUIDE.md` - Testing procedures

---

## Questions & Support

**Q: Do I need to manually apply these templates?**
A: No, the `/specify` and `/implement` commands will automatically use these templates.

**Q: What if I'm working on a non-desktop feature?**
A: The templates will skip Article XI sections. The constitutional compliance checklist will show "Not applicable."

**Q: Can I customize the templates?**
A: Yes, templates are markdown files. Edit them to add project-specific sections, but maintain the core structure.

**Q: How do I know which template version was used?**
A: Each template includes a "Template Version" footer. Session summaries should document template versions used.

**Q: What if I find a template issue?**
A: Update the template, increment the version number, and document the change in a session summary.

---

**Status:** ✅ Complete and Ready for Use
**Version:** Templates updated to v1.1 (BTPC-specific)
**Integration:** Seamless with /start and /stop commands
**Validation:** All checks passed
**Next Session:** Templates will be used automatically

---

**Last Updated:** 2025-10-11 18:00 UTC
**Updated By:** Claude (session: unified state management testing)
**Template Versions:** spec-template.md v1.1, tasks-template.md v1.1