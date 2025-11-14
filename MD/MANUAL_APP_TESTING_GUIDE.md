# BTPC Desktop App Manual Testing Guide

**Date**: 2025-10-19
**Purpose**: Guide for manually testing all P0 and P1 bug fixes

---

## Important: Browser vs Tauri App

### ❌ WRONG: Opening in Browser
```
DO NOT OPEN: file:///home/bob/BTPC/BTPC/btpc-desktop-app/ui/index.html
```
- This opens the HTML files in a web browser (Firefox, Chrome, Vivaldi)
- Browser has NO Tauri runtime
- Will show error: "Tauri API not ready. Please wait a moment and try again"
- This error is EXPECTED and CORRECT (it's the P0-1 bug fix working!)

### ✅ CORRECT: Using Tauri Desktop App
The Tauri app opens a SEPARATE DESKTOP WINDOW with these characteristics:
- Window title: **"BTPC Blockchain Manager"**
- Standalone application window (not a browser tab)
- Has window controls (minimize, maximize, close buttons)
- Desktop app icon in taskbar/window list

---

## Step-by-Step Startup Guide

### Option 1: Clean Start (Recommended)

```bash
# 1. Open a new terminal
cd /home/bob/BTPC/BTPC/btpc-desktop-app

# 2. Clean any orphaned processes
npm run cleanup

# 3. Start the app
npm run tauri:dev

# 4. Wait for compilation (30-60 seconds)
# Look for: "Finished dev [optimized + debuginfo] target(s)"

# 5. Tauri window should appear automatically
# Window title: "BTPC Blockchain Manager"
```

### Option 2: Manual Process Control

```bash
# Kill all BTPC processes first
pkill -f "btpc-desktop-app"
pkill -f "tauri dev"
pkill -f "btpc_node"
pkill -f "btpc_miner"

# Start fresh
cd /home/bob/BTPC/BTPC/btpc-desktop-app
npm run tauri:dev
```

---

## Troubleshooting Startup Issues

### Issue 1: "Tauri API not ready" Error

**Symptom**: Error message in browser
**Cause**: You opened `index.html` in a web browser instead of the Tauri app
**Solution**:
1. Close the browser tab
2. Look for the actual Tauri desktop window
3. If no window appears, check terminal for errors

### Issue 2: No Window Appears

**Check 1**: Is the app running?
```bash
ps aux | grep btpc-desktop-app
# Should see: target/debug/btpc-desktop-app
```

**Check 2**: Check terminal output
```bash
# Look for these messages:
# ✅ "Finished dev [optimized + debuginfo]"
# ❌ Any "error:" messages (compilation failed)
```

**Check 3**: Display issues
```bash
# Ensure DISPLAY is set
echo $DISPLAY  # Should show ":0" or similar

# Restart with explicit display
DISPLAY=:0 npm run tauri:dev
```

### Issue 3: Compilation Errors

```bash
# Check for syntax errors
cd src-tauri
cargo check

# If errors, read error messages and fix code
# If only warnings, app should still run
```

### Issue 4: Port Already in Use

```bash
# Check if port 1420 (Tauri dev server) is in use
lsof -i :1420

# Kill process using port
kill -9 <PID>

# Restart app
npm run tauri:dev
```

---

## Manual Testing Checklist

Once the Tauri desktop app window appears:

### ✅ P0-1: Tauri Context Detection

**Test**:
1. Open browser console (F12) in Tauri window
2. Check for: "Event management initialized for page"
3. Should see NO errors about `window.invoke`

**Expected**: Tauri API available, no errors

---

### ✅ P0-2: Process Cleanup

**Test**:
```bash
# Terminal 1: Check processes BEFORE starting
ps aux | grep -E "(btpc-desktop-app|btpc_node)" | grep -v grep

# Terminal 2: Start app
npm run dev:clean

# In Tauri app: Close window (X button)

# Terminal 1: Check processes AFTER closing (wait 5 seconds)
ps aux | grep -E "(btpc-desktop-app|btpc_node)" | grep -v grep
```

**Expected**:
- Before: Clean (or cleanup script ran)
- During: App processes running
- After: No zombie `btpc_node` processes

---

### ✅ P0-3: Blockchain Info Panel

**Test**:
1. Navigate to **Node** page (sidebar)
2. Scroll to "Blockchain Info" section
3. Verify all 7 fields show data (not "-"):
   - Chain (mainnet/regtest)
   - Blocks (number)
   - Headers (number)
   - Difficulty (number)
   - Network Nodes (number)
   - Network Status (icon + text)
   - Best Block Hash (64-character hex)

**Expected**: All 7 fields populated

---

### ✅ P1-4: Event Listener Cleanup

**Test**:
1. Open browser console (F12)
2. Navigate: Dashboard → Mining → Node → Settings → Transactions → Dashboard
3. On each navigation, check console for:
   - "Cleaning up X event listeners..."
   - "Event management initialized for page"
4. Repeat 5+ times

**Expected**:
- Cleanup message on each page change
- Listener count stays low (3-5 per page)
- NO warning: "High listener count: X active listeners"

---

### ✅ P1-5: Backend-First Validation

**Test**:
1. Go to **Settings** page
2. Open browser console (F12) → Application tab → Local Storage
3. Change a setting (e.g., node URL)
4. Watch localStorage updates

**Expected**:
- localStorage writes AFTER backend confirms save
- No localStorage writes before backend validation

---

### ✅ P1-6: Process Health Monitoring

**Test**:
```bash
# In Tauri app:
# 1. Go to Node page
# 2. Click "Start Node"
# 3. Note the btpc_node PID

# In terminal:
ps aux | grep btpc_node  # Get PID

# Kill the node process manually:
kill -9 <btpc_node_PID>

# In Tauri app:
# Wait 30 seconds
# Node status should update to "Stopped" or "Crashed"
```

**Expected**: Status updates within 30 seconds

---

### ✅ Bonus: Recent Activity Panel

**Test**:
1. Go to **Dashboard**
2. Scroll to "Recent Activity" panel
3. If mining is active, should see recent mining events
4. Panel updates every 5 seconds

**Expected**: Mining events displayed with color-coded badges

---

## Expected App Behavior

### On Startup
1. Terminal shows compilation output (30-60 seconds)
2. Tauri desktop window appears
3. Dashboard page loads
4. Console shows: "Event management initialized for page"

### During Use
1. Navigation between pages is smooth
2. Event cleanup messages in console on each navigation
3. Blockchain info updates every 5 seconds (if node running)
4. Recent Activity updates every 5 seconds (if mining active)

### On Close
1. Click X button on Tauri window
2. App closes gracefully
3. All processes cleaned up within 5 seconds
4. Terminal shows npm process exits

---

## Quick Verification Commands

### Check Process Cleanup
```bash
# Should be clean before starting
npm run cleanup

# Start app
npm run tauri:dev

# After closing app, verify cleanup
ps aux | grep -E "(btpc_node|btpc_miner)" | grep -v grep
# Should show: 0 processes
```

### Check Script Integrations
```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app/ui

# Verify all 6 pages have Tauri context
grep -l "btpc-tauri-context.js" *.html | wc -l  # Should be: 6

# Verify all 6 pages have event manager
grep -l "btpc-event-manager.js" *.html | wc -l   # Should be: 6

# Verify 3 settings pages have backend-first
grep -l "btpc-backend-first.js" *.html | wc -l   # Should be: 3
```

### Check Compilation
```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri
cargo check --quiet
# Should have: 0 errors (warnings OK)
```

---

## Common Mistakes to Avoid

1. ❌ Opening `index.html` in browser → Use Tauri desktop window
2. ❌ Not waiting for compilation → Wait for "Finished dev" message
3. ❌ Expecting instant startup → App takes 30-60s to compile first time
4. ❌ Using Ctrl+C to close → Use window X button to test cleanup
5. ❌ Not checking console → Open F12 to see event management logs

---

## Success Criteria

All bug fixes are working if:

- ✅ Tauri window opens successfully
- ✅ No "window.invoke" errors in console
- ✅ All 7 blockchain info fields populate
- ✅ Event cleanup messages appear on navigation
- ✅ No zombie processes after app close
- ✅ Process status updates within 30 seconds
- ✅ Recent Activity panel shows mining events

---

## Getting Help

### If App Won't Start
1. Check terminal for error messages
2. Run: `cargo check` in `src-tauri/` directory
3. Check: `ps aux | grep btpc` for orphaned processes
4. Try: `npm run cleanup` then `npm run tauri:dev`

### If Window Doesn't Appear
1. Verify app is running: `ps aux | grep btpc-desktop-app`
2. Check DISPLAY: `echo $DISPLAY`
3. Try: `DISPLAY=:0 npm run tauri:dev`
4. Look for window in taskbar/window list

### If Getting Errors
1. Check browser console (F12) for error messages
2. Check terminal for Rust compilation errors
3. Verify all script tags present in HTML files
4. Run: `npm run cleanup` to clear state

---

**Bottom Line**:
- Use `npm run dev:clean` to start
- Look for **"BTPC Blockchain Manager"** desktop window
- Don't open HTML files in browser directly
- App takes 30-60 seconds to compile on first start