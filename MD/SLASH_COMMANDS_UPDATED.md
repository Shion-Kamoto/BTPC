# Slash Commands Updated for .specify Integration

**Date:** 2025-10-11
**Status:** ✅ COMPLETE
**Integration Level:** Full

---

## Summary

Successfully integrated `.specify` framework documentation review into `/stop` and `/start` slash commands. These commands now ensure constitutional compliance is checked at every session handoff, creating a robust governance system for the BTPC project.

---

## Commands Updated

### 1. `/stop` Command - Session Handoff

**Purpose**: Prepares project for session handoff by documenting current state

**New Step 2**: Review .specify Framework Documentation (MANDATORY)

**What It Does:**
1. Reads entire `.specify/memory/constitution.md` file
2. Checks current version (should be 1.0.1 or higher)
3. Reviews Article XI (Desktop Application Development)
4. Identifies any constitutional violations that occurred
5. Notes any amendments needed
6. Updates constitution if new patterns emerged

**Constitutional Compliance Check:**
- Did any code violate Article XI principles?
- Were backend-first validation patterns followed?
- Were event listeners cleaned up properly?
- Was single source of truth maintained?

**Output Format:**
```markdown
### Constitutional Compliance
- ✅/❌ Article XI Compliance: [Status]
- ✅/❌ Backend-First Validation: [Status]
- ✅/❌ Event-Driven Architecture: [Status]
- 📝 Constitution Version: [X.X.X]
- 📝 Amendments: [None / List of changes]
```

**Session Handoff Summary Includes:**
```markdown
### Constitutional Compliance
- ✅ Article XI Compliance: [All patterns followed / Violations noted]
- 📝 Constitution Version: [X.X.X]
- 📝 Amendments: [None / List of amendments]

### .specify Framework State
- Constitution Version: [X.X.X]
- Pending Spec Reviews: [None / List]
- Compliance Issues: [None / List]
```

**Final Report Includes:**
```
**Constitutional Compliance**:
- ✅ Article XI (Desktop App): Compliant
- 📝 Version: X.X.X
- 📝 Amendments: [None / Listed]

**.specify Framework State**:
- Constitution reviewed and up-to-date
- No pending compliance issues
- Ready for `/start` to review
```

---

### 2. `/start` Command - Session Resume

**Purpose**: Resumes work from previous session with full context

**New Step 1**: Read .specify Framework Documentation (MANDATORY - FIRST PRIORITY)

**What It Does:**
1. Reads entire `.specify/memory/constitution.md` BEFORE any other files
2. Checks current version
3. Reviews Article XI completely
4. Understands mandatory patterns:
   - Backend-first validation
   - Event-driven architecture
   - Single source of truth
   - Memory management (event listener cleanup)
5. Notes any pending amendments or compliance issues

**Constitutional Pattern Review:**
- ✅ Backend is single source of truth
- ✅ Always validate backend BEFORE localStorage
- ✅ Use Tauri events for state synchronization
- ✅ Clean up event listeners on page unload
- ✅ No duplicate notifications
- ✅ Cross-page state consistency

**Output Format:**
```markdown
## .specify Framework Status

**Constitution Version**: X.X.X
**Last Amendment**: 2025-XX-XX
**Compliance Status**: ✅ Reviewed

**Key Patterns to Follow**:
- Backend-first validation (Article XI, Section 11.2)
- Event-driven state management (Article XI, Section 11.3)
- Memory-safe event cleanup (Article XI, Section 11.6)
```

**Session Resume Summary Includes:**
```markdown
**.specify Framework**:
- Constitution Version: X.X.X
- Article XI Compliance: ✅ Reviewed
- Patterns to follow: Backend-first validation, Event cleanup, Single source of truth

**Constitutional Compliance Check**:
- All new code will follow Article XI patterns
- Backend validation before localStorage
- Event listeners will be cleaned up
- Cross-page state consistency maintained
```

---

## Integration Flow

### Session End (`/stop`)

```
1. Gather System State
     ↓
2. Review .specify Framework (MANDATORY)
   - Read constitution
   - Check compliance
   - Update if needed
   - Document violations
     ↓
3. Review Design Files
     ↓
4. Update STATUS.md
     ↓
5. Update Session Docs
     ↓
6. Update CLAUDE.md
     ↓
7. Check Git Status (.specify changes)
     ↓
8. Create Handoff Summary (includes constitutional compliance)
     ↓
9. Validate Docs (check .specify version)
     ↓
10. Final Report (includes .specify framework state)
```

### Session Start (`/start`)

```
1. Read .specify Framework (MANDATORY - FIRST)
   - Read constitution
   - Review Article XI
   - Understand patterns
   - Check compliance
     ↓
2. Read Core Status (STATUS.md, CLAUDE.md)
     ↓
3. Read Design Files
     ↓
4. Read Session Docs
     ↓
5. Check Active Processes
     ↓
6. Identify Next Tasks
     ↓
7. Present Summary (includes .specify framework state)
   - Show constitution version
   - List patterns to follow
   - Confirm compliance check
     ↓
Continue Work (with constitutional patterns in mind)
```

---

## Key Features

### 1. Constitutional Compliance Tracking

**Every session documents:**
- Which Article XI principles were followed
- Any violations that occurred
- Patterns that emerged
- Amendments needed

**Example:**
```markdown
### Constitutional Compliance
- ✅ Article XI Compliance: All patterns followed
- ✅ Backend-First Validation: Implemented in settings.html
- ✅ Event-Driven Architecture: Unified state management active
- ✅ Event Listener Cleanup: Memory leaks fixed
- 📝 Constitution Version: 1.0.1
- 📝 Amendments: None
```

### 2. Automatic Version Tracking

**Commands check:**
- Current constitution version
- Last amendment date
- Pending amendments

**Example:**
```bash
cat /home/bob/BTPC/BTPC/.specify/memory/constitution.md | grep "Version:"
```

### 3. Git Integration

**Tracks .specify changes:**
```bash
git diff .specify/memory/constitution.md
```

**Documents in session summary:**
- Constitution version changes
- New articles or amendments
- New constitutional patterns

### 4. Handoff Continuity

**/stop creates:**
- Constitutional compliance status
- .specify framework state
- Pending compliance issues

**/start reads:**
- Constitution (complete)
- Article XI patterns
- Compliance requirements

---

## Usage Examples

### Example 1: Clean Session with Full Compliance

**`/stop`:**
```
→ Reviews constitution
→ Checks: All Article XI patterns followed ✅
→ Documents: No violations, no amendments needed
→ Output: "✅ Article XI (Desktop App): Compliant"
```

**`/start`:**
```
→ Reads constitution version 1.0.1
→ Reviews Article XI patterns
→ Output: "Constitution Version: 1.0.1, Article XI Compliance: ✅ Reviewed"
→ Continues work with patterns in mind
```

### Example 2: Session with Constitutional Violation

**`/stop`:**
```
→ Reviews constitution
→ Detects: Settings saved to localStorage BEFORE backend validation ❌
→ Documents: Violation of Article XI, Section 11.2
→ Fixes: Updated settings.html to validate backend first
→ Amends: No amendment needed (fix applied)
→ Output: "❌ Article XI Compliance: Violation fixed in settings.html"
```

**`/start`:**
```
→ Reads constitution
→ Sees: Previous session fixed validation bug
→ Reviews: Article XI, Section 11.2 pattern
→ Output: "Backend-first validation pattern restored"
→ Continues with extra attention to validation order
```

### Example 3: Session with New Pattern Discovery

**`/stop`:**
```
→ Reviews constitution
→ Discovers: New pattern for process lifecycle management
→ Documents: "Verify process stopped before UI update"
→ Amends: Adds to Article XI, Section 11.5
→ Increments: Version 1.0.1 → 1.0.2
→ Output: "Constitution Updated: v1.0.2 (Added process verification pattern)"
```

**`/start`:**
```
→ Reads constitution version 1.0.2
→ Sees: New process verification requirement
→ Reviews: Article XI, Section 11.5 (updated)
→ Output: "New pattern: Verify process state before UI update"
→ Applies pattern to all process management code
```

---

## Benefits

### 1. Governance Enforcement

**Constitutional compliance is now:**
- Checked at every session handoff
- Documented in session summaries
- Tracked in git history
- Part of development workflow

**Result:** No pattern violations go undocumented

### 2. Knowledge Continuity

**Next session developer knows:**
- Which patterns to follow
- Which violations occurred
- Which amendments were made
- Current constitutional state

**Result:** Seamless handoff between sessions

### 3. Pattern Evolution

**Constitution evolves through:**
- Violation discovery
- Pattern emergence
- Amendment proposals
- Version incrementing

**Result:** Living document that improves over time

### 4. Automated Compliance

**Commands automatically:**
- Read constitution
- Check compliance
- Document status
- Suggest fixes

**Result:** No manual compliance checks needed

---

## Files Modified

### 1. `/home/bob/BTPC/BTPC/.claude/commands/stop.md`

**Added:**
- Step 2: Review .specify Framework Documentation (MANDATORY)
- Constitutional Compliance Check section
- .specify changes in git diff
- Constitutional compliance in handoff summary
- .specify framework state in final report

**Key Additions:**
- Lines 40-77: Mandatory constitutional review
- Lines 157-166: Git tracking of .specify changes
- Lines 184-187: Constitutional compliance in summary
- Lines 198-201: .specify framework state
- Lines 245-250: Constitutional compliance in final report
- Lines 265-268: .specify framework state in final report

### 2. `/home/bob/BTPC/BTPC/.claude/commands/start.md`

**Added:**
- Step 1: Read .specify Framework Documentation (MANDATORY - FIRST PRIORITY)
- Constitutional Pattern Review section
- .specify framework status output
- Constitutional compliance check in resume summary

**Key Additions:**
- Lines 20-48: Mandatory constitutional reading (FIRST)
- Renumbered all subsequent steps
- Added .specify framework status to session resume
- Added constitutional compliance check to output

---

## Testing the Integration

### Test 1: Run /stop After This Session

**Expected Output:**
```
**Constitutional Compliance**:
- ✅ Article XI (Desktop App): Compliant
- 📝 Version: 1.0.1
- 📝 Amendments: Added Article XI on 2025-10-11

**.specify Framework State**:
- Constitution reviewed and up-to-date
- No pending compliance issues
- Ready for `/start` to review
```

### Test 2: Run /start in Next Session

**Expected Output:**
```
**.specify Framework**:
- Constitution Version: 1.0.1
- Article XI Compliance: ✅ Reviewed
- Patterns to follow: Backend-first validation, Event cleanup, Single source of truth

**Constitutional Compliance Check**:
- All new code will follow Article XI patterns
- Backend validation before localStorage
- Event listeners will be cleaned up
- Cross-page state consistency maintained
```

### Test 3: Verify Git Tracking

**Command:**
```bash
git diff .specify/memory/constitution.md
```

**Expected:** Shows Article XI addition and version update

---

## Command Reference

### `/stop` Constitutional Review Checklist

When running `/stop`, verify:

- [ ] Constitution read completely
- [ ] Article XI reviewed
- [ ] Violations identified (if any)
- [ ] Patterns documented
- [ ] Version checked
- [ ] Amendments proposed (if needed)
- [ ] Git diff shows .specify changes
- [ ] Session summary includes compliance
- [ ] Final report includes framework state

### `/start` Constitutional Review Checklist

When running `/start`, verify:

- [ ] Constitution read FIRST (before other docs)
- [ ] Current version identified
- [ ] Article XI patterns reviewed
- [ ] Mandatory patterns understood:
  - Backend-first validation
  - Event-driven architecture
  - Single source of truth
  - Event listener cleanup
  - Cross-page consistency
- [ ] Session resume includes .specify status
- [ ] Compliance check confirmed

---

## Integration with Other Systems

### Git Integration

**Commands track:**
- `.specify/memory/constitution.md` changes
- Version increments
- Amendment dates
- Pattern additions

**In session summaries:**
```markdown
### Files Modified
- ✅ .specify/memory/constitution.md - v1.0.0 → v1.0.1 (Added Article XI)
```

### STATUS.md Integration

**Updates include:**
- Constitutional compliance status
- .specify framework version
- Pending amendments

### CLAUDE.md Integration

**Only updated if:**
- New constitutional requirements added
- Project structure changed
- Major patterns codified

---

## Future Enhancements

### Potential Additions

1. **Automated Compliance Scanning**
   - Scan code for pattern violations
   - Compare against Article XI requirements
   - Flag violations automatically

2. **Constitutional Amendment Workflow**
   - Formal amendment proposal process
   - Vote/approval tracking
   - Amendment history log

3. **Pattern Library**
   - Code examples for each Article XI pattern
   - Anti-patterns to avoid
   - Quick reference guide

4. **Compliance Metrics**
   - Track compliance over time
   - Violation frequency by pattern
   - Amendment velocity

---

## Conclusion

The `/stop` and `/start` commands are now fully integrated with the `.specify` framework, creating a robust governance system that:

✅ **Ensures constitutional compliance** at every session handoff
✅ **Tracks pattern evolution** through version control
✅ **Documents violations and fixes** in session summaries
✅ **Provides knowledge continuity** between sessions
✅ **Automates compliance checks** without manual intervention

**Every session now:**
1. Ends with constitutional review (`/stop`)
2. Starts with constitutional awareness (`/start`)
3. Documents compliance status
4. Tracks .specify framework state

**Result:** Living, enforced governance for BTPC development.

---

**Updated By:** Claude Code (AI Assistant)
**Date:** 2025-10-11
**Commands Modified:** `/stop`, `/start`
**Framework Version:** .specify v1.0 with Constitution v1.0.1
**Status:** ✅ READY FOR USE