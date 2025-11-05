---
description: Resume work from previous session by reading project docs and continuing pending tasks
---

# Start Command

## Overview
Resumes work from the previous session by reading all project documentation to understand current status, pending tasks, and context needed to continue development.

**CRITICAL PROJECT SCOPE:**
- **Primary codebase**: `/home/bob/BTPC/BTPC/btpc-desktop-app/ui` (Desktop GUI)
- **Core library**: `/home/bob/BTPC/BTPC/btpc-core` (Blockchain core - shared library)
- **CLI tools**: Reference implementations only (NOT the main project)
- **ALL work applies to desktop app unless explicitly stated otherwise**

To minimise Token use, always uses grep where possible to locate code, as well as create bash script for large code modifications.
Be brief and concise with all responses as well with all summaries. Sacrifice grammar for the sake of concision. 
## Usage
```
/start [task_description]
```

## Parameters
- `task_description` (optional): Specific task to work on. If not provided, continues with the most recent pending work.

## What This Command Does

### Step 1: Read Constitution and All Documentation Directories (MANDATORY)

**FIRST PRIORITY**: Review `/home/bob/BTPC/BTPC/MD/CONSTITUTION.md` (authoritative v1.1):

1. **`/MD/CONSTITUTION.md`** - Verify compliance:
   - SHA-512 PoW, ML-DSA signatures (Articles II & VIII)
   - Linear decay economics, NOT halving (Article III)
   - Bitcoin-compatible structure (Articles II & V)
   - NO prohibited changes (Article VII.3: no halving, PoS, smart contracts)
   - **TDD MANDATORY** (Article VI.3): RED-GREEN-REFACTOR cycle for ALL code

2. **Read all documentation directories:**
```bash
# Project documentation sources
/home/bob/BTPC/BTPC/MD/                    # Session summaries, status, fixes
/home/bob/BTPC/BTPC/ref-tools-mcp/         # MCP server integrations
/home/bob/BTPC/BTPC/specs/                 # Feature specs (001-*, 002-*)
/home/bob/BTPC/BTPC/specs/*/spec.md        # Feature specifications (WHAT users need)
/home/bob/BTPC/BTPC/specs/*/plan.md        # Implementation plans (HOW to build)
/home/bob/BTPC/BTPC/specs/*/tasks.md       # Validation/implementation tasks
/home/bob/BTPC/BTPC/supabase/              # DB schemas, migrations
/home/bob/BTPC/spec-kit/                   # Spec framework documentation
```

3. **Constitutional Requirements:**
- SHA-512/ML-DSA only (no other crypto)
- Linear decay formula (32.375 BTPC → 0.5 tail emission)
- 10-minute blocks, Bitcoin UTXO model
- No halving, PoS, or smart contracts

**Output:**
```markdown
## Constitution Compliance (MD/CONSTITUTION.md v1.1)

**Core Principles**:
- ✅ SHA-512 PoW + ML-DSA signatures
- ✅ Linear decay (NOT halving)
- ✅ Bitcoin-compatible structure
- ✅ Quantum-resistant (Article X)
- ✅ **TDD MANDATORY** (Article VI.3)

**TDD Requirements (ALL code changes)**:
1. RED: Write failing tests FIRST
2. GREEN: Implement minimum code to pass
3. REFACTOR: Improve while keeping tests green

**Documentation Sources Loaded**:
- MD/ (session state)
- ref-tools-mcp/ (MCP configs)
- specs/ (feature plans)
- supabase/ (database)
- spec-kit/ (framework)
```

### Step 2: Read Core Status Documents
Load and analyze the following files to understand project state:

1. **`/home/bob/BTPC/BTPC/STATUS.md`** - Primary project status document
   - Current implementation status
   - Recently completed work
   - Known issues and pending items
   - Active processes and system state

2. **`/home/bob/BTPC/BTPC/CLAUDE.md`** - Project guidelines and conventions
   - Active technologies and dependencies
   - Code style requirements
   - Project structure
   - Available commands

### Step 3: Read Feature Specifications and Implementation Plans
Review specs directory for feature specifications, implementation plans, and tasks:

1. **Feature Specifications** (`/home/bob/BTPC/BTPC/specs/*/spec.md`)
   - Functional and non-functional requirements (FR-XXX)
   - User scenarios and acceptance criteria
   - Key entities and data models
   - Edge cases and validation rules
   - **Primary**: `specs/001-core-blockchain-implementation/spec.md` (core blockchain)

2. **Implementation Plans** (`/home/bob/BTPC/BTPC/specs/*/plan.md`)
   - Technical context and architecture decisions
   - Phase-by-phase implementation breakdown
   - Constitutional compliance verification
   - Dependency and technology choices
   - **Primary**: `specs/001-core-blockchain-implementation/plan.md` (550-line implementation plan)

3. **Task Lists** (`/home/bob/BTPC/BTPC/specs/*/tasks.md`)
   - Validation and implementation tasks (V001-V023 or similar)
   - Parallel execution markers [P]
   - Acceptance criteria and test commands
   - Phase organization and dependencies
   - **Primary**: `specs/001-core-blockchain-implementation/tasks.md` (23 validation tasks)

### Step 4: Read Design and Architecture Documentation
Review all design-related files to understand UI state:

1. **`/home/bob/BTPC/BTPC/btpc-desktop-app/ARCHITECTURE.md`** (if exists)
   - Complete desktop app architecture
   - Technology stack and design system
   - All 68 Tauri commands mapped
   - Data flows and security model

2. **`/home/bob/BTPC/BTPC/.playwright-mcp/style-guide.md`** (if exists)
   - UI design principles
   - Current design status
   - Visual guidelines

3. **`/home/bob/BTPC/BTPC/style-guide/ux-rules.md`** (if exists)
   - Monero-inspired UX patterns
   - Component standards
   - Interaction patterns
   - Accessibility requirements

4. **`/home/bob/BTPC/BTPC/.playwright-mcp/BTPC-GUI-guide.md`** (if exists)
   - Complete GUI user guide
   - Feature documentation

### Step 5: Read Session-Specific Documentation
Check for active deployment/test documentation:

1. **`/home/bob/BTPC/BTPC/testnet-deployment/STRESS_TEST_STATUS.md`** (if exists)
   - Active stress test status
   - Current metrics and progress

2. **`/home/bob/BTPC/BTPC/testnet-deployment/SESSION_COMPLETION_SUMMARY.md`** (if exists)
   - Last session's work summary
   - Completed objectives
   - Technical details

3. **`/home/bob/BTPC/BTPC/testnet-deployment/QUICK_REFERENCE.md`** (if exists)
   - Quick commands for monitoring
   - Current system state

### Step 6: Check Active Processes
Verify what's currently running:

```bash
# Check for running nodes
ps aux | grep btpc_node | grep -v grep

# Check for stress tests
ps aux | grep stress-test | grep -v grep

# Check RPC status
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[]}' | jq '.result.blocks'
```

### Step 7: Identify Next Tasks
Based on STATUS.md, determine what needs to be done:

- Check "Next Steps" sections
- Look for pending items marked with ⏳
- Review "Known Issues" section
- Check if any active tests/processes need monitoring

### Step 8: Present Summary and Continue
Provide concise summary:

```
## Session Resumed

**Constitution (MD/CONSTITUTION.md v1.1)**:
- ✅ SHA-512 PoW + ML-DSA signatures
- ✅ Linear decay (NOT halving)
- ✅ Bitcoin-compatible UTXO
- ⚠️ **TDD REQUIRED** (Art VI.3): RED-GREEN-REFACTOR for ALL code

**Documentation Loaded**:
- MD/ (session state)
- specs/*/spec.md (feature specifications + requirements)
- specs/*/plan.md (implementation plans + architecture)
- specs/*/tasks.md (validation/implementation tasks)
- ref-tools-mcp/ (MCP servers)
- supabase/ (DB schemas)
- spec-kit/ (framework)

**Project Status**: [From STATUS.md]

**Active**:
- Node: PID XXXXX (height: XXXX)
- [Other processes]

**Last Session**: [Brief summary]

**Pending**:
1. [Task 1]
2. [Task 2]

**Continuing**: [Task or urgent item]
```

Then proceed without waiting.

## Task Priority Logic

When no specific task is provided, prioritize in this order:

1. **Critical Issues** - Any failures or errors in active processes
2. **Active Tests** - Monitor/complete running stress tests
3. **Short Term Items** - Tasks marked for immediate completion
4. **Medium Term Items** - Next development tasks
5. **Long Term Items** - Future roadmap items

## Example Workflows

### Example 1: Resuming After Stress Test
```
/start

→ Reads STATUS.md: Stress test at 50/288 checks
→ Checks processes: Node and test still running
→ Reports: "Stress test 17% complete, all systems healthy"
→ Continues monitoring or moves to next pending task
```

### Example 2: Resuming with Specific Task
```
/start implement wallet persistence

→ Reads STATUS.md: Wallet persistence marked as pending
→ Checks CLAUDE.md for project structure
→ Reports current status
→ Begins implementing wallet persistence feature
```

### Example 3: Resuming After Completion
```
/start

→ Reads STATUS.md: All core tasks complete
→ Reads SESSION_COMPLETION_SUMMARY.md: Last session completed RPC fix
→ Identifies next item: "Complete wallet persistence"
→ Begins work on next priority task
```

## Integration with Other Commands

This command is designed to work with:
- `/stop` - Updates documentation before ending session
- `/tasks` - May generate tasks.md for complex features
- `/implement` - May use for feature implementation

## Output Format

Always start with a concise summary (3-5 sentences) covering:
1. Current project status
2. What was last completed
3. What will be worked on next

Then proceed directly with the work.

## Notes

- **Non-interactive**: After presenting summary, continue work without prompting user
- **Context-aware**: Adapts to project state (development vs testing vs deployment)
- **Process-safe**: Checks for active processes before starting potentially conflicting work
- **Documentation-driven**: Relies on accurate STATUS.md maintained by /stop command