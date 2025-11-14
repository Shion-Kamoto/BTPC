# Stop Command

## Overview
Prepares the project for session handoff by updating all documentation with current status, completed work, active processes, and pending tasks to enable seamless continuation in the next session.

**Efficiency Requirements:**
- Use `grep` to locate code references in documentation
- Create bash scripts for large documentation updates
- Be brief/concise - sacrifice grammar for concision

## Usage
```
/stop [session_summary]
```

## Parameters
- `session_summary` (optional): Brief description of work completed in this session. If not provided, will be auto-generated from recent work.

## What This Command Does

### Step 1: Gather Current System State

Collect real-time information about the project:

```bash
# Check active processes
ps aux | grep btpc_node | grep -v grep
ps aux | grep stress-test | grep -v grep

# Get blockchain status
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[]}' | jq

# Check database sizes
du -sh /home/bob/BTPC/BTPC/testnet-deployment/data/*

# Get system resources
top -bn1 | grep "Cpu(s)"
free -h | grep Mem

# Check for recent errors
tail -100 /tmp/node1_final.log 2>/dev/null | grep -i error || echo "No errors"
```

### Step 2: Review Constitution and Core Documentation

**MANDATORY**: Align with `/home/bob/BTPC/BTPC/MD/CONSTITUTION.md` (authoritative):

```bash
# Check Constitution
ls -lh /home/bob/BTPC/BTPC/MD/CONSTITUTION.md
```

**Read and verify:**
1. **`/MD/CONSTITUTION.md`** - Verify Version 1.0 compliance
   - SHA-512 PoW, ML-DSA signatures (Articles II & VIII)
   - Linear decay economics (Article III)
   - Bitcoin-compatible structure (Articles II & V)
   - No prohibited changes (Article VII.3)

2. **Constitutional Compliance:**
   - SHA-512/ML-DSA unchanged?
   - Linear decay calculation correct?
   - Bitcoin compatibility maintained?
   - No halving, PoS, or smart contracts?

3. **Update documentation directories:**
```bash
# Update all project docs
/home/bob/BTPC/BTPC/MD/                    # Session summaries, status
/home/bob/BTPC/BTPC/ref-tools-mcp/         # MCP integrations
/home/bob/BTPC/BTPC/specs/                 # Feature specifications
/home/bob/BTPC/BTPC/specs/*/spec.md        # Feature specifications (WHAT was requested)
/home/bob/BTPC/BTPC/specs/*/plan.md        # Implementation plans (HOW was built)
/home/bob/BTPC/BTPC/specs/*/tasks.md       # Validation/implementation tasks (WHAT was done)
/home/bob/BTPC/BTPC/supabase/              # Database schemas
/home/bob/BTPC/spec-kit/                   # Spec framework docs
```

**Output:**
```markdown
### Constitutional Compliance (MD/CONSTITUTION.md v1.1)
- ✅/❌ SHA-512/ML-DSA: [Status]
- ✅/❌ Linear Decay Economics: [Status]
- ✅/❌ Bitcoin Compatibility: [Status]
- ✅/❌ No Prohibited Features: [Status]
- ✅/❌ TDD Methodology (Art VI.3): [Status]

### TDD Compliance (Article VI, Section 6.3 - MANDATORY)
- ✅/❌ RED: Tests written first
- ✅/❌ GREEN: Implementation passes tests
- ✅/❌ REFACTOR: Code improved, tests still pass
- 📝 Test files: [List test files with line numbers]
- 📝 Evidence: [cargo test output, test file locations]
```

### Step 3: Review Design and Architecture Files

Check for updates needed in design documentation:

```bash
# Check if design files need updating
ls -lh /home/bob/BTPC/BTPC/btpc-desktop-app/ARCHITECTURE.md
ls -lh /home/bob/BTPC/BTPC/.playwright-mcp/style-guide.md
ls -lh /home/bob/BTPC/BTPC/style-guide/ux-rules.md
```

If any UI/design work was completed, update:
- `/home/bob/BTPC/BTPC/btpc-desktop-app/ARCHITECTURE.md` - Architecture changes
- `/home/bob/BTPC/BTPC/.playwright-mcp/style-guide.md` - Design system updates
- `/home/bob/BTPC/BTPC/style-guide/ux-rules.md` - UX pattern changes

### Step 4: Update STATUS.md

Update `/home/bob/BTPC/BTPC/STATUS.md` with:

1. **Last Updated timestamp** - Current date/time
2. **Project Status** - Overall status (e.g., "CORE COMPLETE - TESTNET OPERATIONAL")
3. **Implementation Status** - Percentage completion of all components
4. **Recent Changes** - What was completed in this session
5. **Current Testnet Status** (if applicable):
   - Node PID and status
   - Current blockchain height
   - Database size
   - Active test progress
   - Performance metrics
6. **Pending Items** - Updated list of what's not done
7. **Known Issues** - Any new issues discovered
8. **Next Steps** - Prioritized tasks for next session

### Step 5: Update Multi-Directory Documentation

**Use grep for code location, bash scripts for bulk updates:**

```bash
# Find modified files with grep
grep -r "function_name" btpc-core/src --include="*.rs" -l

# Create bash script for bulk doc updates (if >5 files)
cat > /tmp/update_docs.sh <<'EOF'
#!/bin/bash
for file in MD/*.md; do
  sed -i "s/old_pattern/new_pattern/g" "$file"
done
EOF
chmod +x /tmp/update_docs.sh && /tmp/update_docs.sh
```

**Update directories (concise, grep-assisted):**

**Primary (`/MD/`):**
1. **SESSION_HANDOFF_2025-MM-DD.md** - Session summary
2. **STATUS.md** - Current state

**Feature Specs (`/specs/`):**
- **spec.md**: Update requirement completion status (FR-XXX) - mark ✅ when implemented
- **plan.md**: Update phase completion status - mark phases complete with metrics
- **tasks.md**: Mark validation/implementation tasks complete - update task status (V001, etc.)
- Use `grep -l "status:" specs/*/*.md` to find spec files needing updates

**MCP (`/ref-tools-mcp/`):**
- Doc new MCP servers

**Database (`/supabase/`):**
- Migration logs if schema changed

**Spec Kit (`/spec-kit/`):**
- Framework doc updates if changed

### Step 6: Update CLAUDE.md (if needed)

Only update `/home/bob/BTPC/BTPC/CLAUDE.md` if:
- New technologies/dependencies were added
- Project structure changed
- New commands were created
- Implementation status percentages changed significantly

Update sections:
- **Active Technologies** - Add new dependencies
- **Project Structure** - Reflect structural changes
- **Implementation Status** - Update completion percentages
- **Recent Changes** - Add entry for this session

### Step 7: Check Git Status and Document Changes

```bash
cd /home/bob/BTPC/BTPC
git status --short
git diff --stat

# Specifically check .specify changes
git diff .specify/memory/constitution.md
```

Document modified files in STATUS.md "Recent Changes" section.

**Include .specify changes:**
- Note constitution version changes
- Document any new articles or amendments
- List any new constitutional patterns

### Step 8: Create Session Handoff Summary

Generate a concise summary for the next session:

```markdown
## Session Handoff Summary

**Date**: 2025-10-05 XX:XX:XX
**Duration**: ~X hours
**Status**: ✅ SESSION COMPLETE

### Completed This Session
1. [Major item 1]
2. [Major item 2]
...

### Constitutional Compliance
- ✅ Article XI Compliance: [All patterns followed / Violations noted]
- 📝 Constitution Version: [X.X.X]
- 📝 Amendments: [None / List of amendments]

### Active Processes
- Node (PID: XXXXX) - Height: XXXX blocks
- Stress Test (PID: XXXXX) - Progress: XX/288 checks

### Pending for Next Session
1. [Priority 1 task]
2. [Priority 2 task]
...

### .specify Framework State
- Constitution Version: [X.X.X]
- Pending Spec Reviews: [None / List]
- Compliance Issues: [None / List]

### Important Notes
- [Any critical information for next session]
- [Process monitoring instructions]
- [Known issues to be aware of]
- [Constitutional patterns to follow]
```

Add this to the top of SESSION_COMPLETION_SUMMARY.md.

### Step 9: Validate Documentation Completeness

Verify all required files exist and are up-to-date:

```bash
# Check core docs
ls -lh /home/bob/BTPC/BTPC/STATUS.md
ls -lh /home/bob/BTPC/BTPC/CLAUDE.md

# Check .specify framework docs
ls -lh /home/bob/BTPC/BTPC/.specify/memory/constitution.md
cat /home/bob/BTPC/BTPC/.specify/memory/constitution.md | grep "Version:"

# Check testnet docs (if applicable)
ls -lh /home/bob/BTPC/BTPC/testnet-deployment/*.md

# Check for any unsaved changes
git status
git diff .specify/
```

### Step 10: Final Report

Present concise summary:

```
## Session Documented

**Updated Directories**:
- ✅ MD/ - Session handoff, status
- ✅ specs/*/spec.md - Requirement completion marks (FR-XXX)
- ✅ specs/*/plan.md - Phase completion status
- ✅ specs/*/tasks.md - Task completion marks (V001-V023, etc.)
- ✅ ref-tools-mcp/ - MCP configs
- ✅ supabase/ - DB migrations (if changed)
- ✅ spec-kit/ - Framework docs (if changed)

**Constitutional Compliance (MD/CONSTITUTION.md v1.1)**:
- ✅ SHA-512/ML-DSA: Unchanged
- ✅ Linear Decay: Correct
- ✅ Bitcoin Compat: Maintained
- ✅ No Prohibited: Verified
- ✅ TDD (Art VI.3): RED-GREEN-REFACTOR followed

**Active Processes**:
- Node: PID XXXXX (height: XXXX)
- Test: XX/288 (if running)

**Next Priority**:
1. [Task 1]
2. [Task 2]

**Ready for `/start` to resume.**
```

## Documentation Update Rules

### Efficiency Requirements (MANDATORY)
- **Use grep** to locate code/patterns before editing
- **Create bash scripts** for updates affecting >5 files
- **Be concise** - short sentences, no fluff
- **Skip grammar** for speed - clarity over correctness

### STATUS.md Updates
- Update timestamp, metrics, changes
- Mark completed: ✅

### SESSION_COMPLETION_SUMMARY.md
- List objectives, files (line numbers), tests
- Root causes, evidence

### Conditional Updates
- Testnet docs only if running
- CLAUDE.md only if significant
- SESSION summary only if substantial

## Integration with /start Command

The `/stop` command updates all directories that `/start` reads:

**Documentation Directories:**
- **MD/** → Session handoffs, status, fix summaries
- **specs/*/spec.md** → Feature specifications with requirement status (FR-XXX)
- **specs/*/plan.md** → Implementation plans with phase completion status
- **specs/*/tasks.md** → Validation/implementation tasks with completion marks
- **ref-tools-mcp/** → MCP integration configs
- **supabase/** → Database migration logs
- **spec-kit/** → Framework documentation

**Key Files:**
- **MD/CONSTITUTION.md** → Authoritative v1.0 (immutable)
- **STATUS.md** → Current project state
- **CLAUDE.md** → Development guidelines

## Example Workflows

### Example 1: Stopping After Feature Implementation
```
/stop Implemented wallet persistence with file encryption

→ Gathers system state
→ Updates STATUS.md: Marks wallet persistence as complete
→ Creates SESSION_COMPLETION_SUMMARY.md with implementation details
→ Updates CLAUDE.md: Changes implementation status to 95% → 98%
→ Reports: "Session documented. Wallet persistence complete."
```

### Example 2: Stopping During Active Test
```
/stop

→ Checks stress test: 120/288 checks completed
→ Updates STRESS_TEST_STATUS.md with current metrics
→ Updates STATUS.md with test progress
→ Does NOT create SESSION_COMPLETION_SUMMARY.md (no major work)
→ Reports: "Stress test 42% complete. Safe to disconnect."
```

### Example 3: Stopping After Bug Fix
```
/stop Fixed RPC height reporting bug

→ Gathers blockchain metrics
→ Creates detailed SESSION_COMPLETION_SUMMARY.md with root cause analysis
→ Updates STATUS.md: Removes RPC bug from "Known Issues"
→ Creates RPC_FIX_SUMMARY.md with technical details
→ Reports: "Bug fix documented. RPC now functional."
```

## Notes

- **Always safe to run**: Can be called multiple times
- **Idempotent**: Running twice with same state won't create duplicates
- **Timestamp-based**: Uses timestamps to avoid overwriting recent updates
- **Process-aware**: Captures state of running processes without stopping them
- **Git-friendly**: Updates .md files that can be committed

## Error Handling

If documentation updates fail:
1. Report which files couldn't be updated
2. Provide manual update instructions
3. Still gather and display system state
4. Don't exit with error (best-effort approach)

## Output Format

Always end with:
- ✅ List of updated files
- 📊 Current system metrics
- 🎯 Next session priorities
- 📝 Any important notes

Keep output concise (< 20 lines) unless major work was completed.