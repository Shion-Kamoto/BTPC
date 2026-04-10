# BTPC Desktop App - Quick Fix for UI Issues

**Date:** 2025-10-06
**Status:** Immediate fixes for glitchy behavior

---

## Issues

1. **Node appears to stop/start** - Actually running but UI not refreshing
2. **Analytics not auto-updating** - Need manual refresh
3. **Conflicting intervals** - Multiple setInterval causing race conditions

---

## Immediate Fixes (No Backend Changes Needed)

### Fix 1: Consolidate Update Logic

**Problem:** Each page has its own polling intervals
**Solution:** Single coordinated update manager

### Fix 2: Better Error Handling

**Problem:** Failed backend calls break UI
**Solution:** Graceful degradation with retry logic

### Fix 3: Debounce Updates

**Problem:** Multiple simultaneous calls
**Solution:** Queue updates and debounce

---

## Implementation

Create `btpc-update-manager.js` to coordinate all updates.

This will:
- Manage all polling intervals
- Prevent duplicate calls
- Handle errors gracefully
- Provide consistent state across pages
