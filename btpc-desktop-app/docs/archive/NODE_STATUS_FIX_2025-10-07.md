# Node Status Fix - 2025-10-07

## Issue
The node status indicator was incorrectly switching to "Offline" after starting the node, even though the node was actually running.

## Root Cause
**API Mismatch**: The frontend JavaScript was checking for the wrong property name in the node status response.

- **Backend returns**: `{ "running": true, "status": "running", "pid": 12345 }`
- **Frontend was checking**: `nodeStatus.is_running` ❌
- **Should be checking**: `nodeStatus.running` ✅

## Files Modified

### `/btpc-desktop-app/ui/node.html`

**Line 354** - Fixed in `refreshNodeStatus()` function:
```javascript
// Before:
if (nodeStatus && nodeStatus.is_running) {

// After:
if (nodeStatus && nodeStatus.running) {
```

**Line 437** - Fixed in update manager subscription:
```javascript
// Before:
if (data.is_running) {

// After:
if (data.running) {
```

## Testing
1. Start the node via the UI
2. Observe the status indicator shows "🟢 Running"
3. Stop the node via the UI
4. Observe the status indicator shows "🔴 Offline"
5. Verify button states toggle correctly

## Impact
- ✅ Node status now correctly reflects actual node state
- ✅ Start/Stop buttons toggle properly
- ✅ Update manager subscription works correctly
- ✅ No backend changes required

## Related Files
- Backend: `/src-tauri/src/main.rs:597` (get_node_status function)
- Frontend: `/ui/node.html` (node management page)