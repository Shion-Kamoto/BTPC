# Desktop App Troubleshooting Guide

**Last Updated**: 2025-10-06 14:10

---

## Issue: QR Code Not Displaying

### Symptoms:
- QR code canvas appears empty/white
- No error in console
- Wallet address shows correctly but no QR pattern

### Diagnosis Steps:

1. **Open Browser DevTools**
   - In Tauri app, press `F12` or `Ctrl+Shift+I`
   - Go to Console tab
   - Look for errors:
     - `"QR canvas not found"` → Canvas element missing
     - `"Cannot get 2d context"` → Canvas rendering issue
     - Any JavaScript errors

2. **Verify Wallet Selection**
   - Go to Wallet Manager → Show Address tab
   - Ensure a wallet is selected in dropdown
   - The `updateShowAddress()` function should call `generateAddressQRCode()`

3. **Check Canvas Element**
   - In DevTools Console, run:
     ```javascript
     document.getElementById('address-qr-canvas')
     ```
   - Should return: `<canvas id="address-qr-canvas" width="256" height="256">`
   - If `null`, the canvas element is missing

4. **Test QR Function Directly**
   - In DevTools Console, run:
     ```javascript
     generateAddressQRCode('test-address-123')
     ```
   - Check if pattern appears on canvas

### Common Fixes:

**Fix 1: Reload the Page**
- Press `Ctrl+R` or `F5` to reload
- Sometimes canvas needs fresh initialization

**Fix 2: Check Tab Visibility**
- Make sure "Show Address" tab is active
- Canvas is in `address-display-section` which has `display: none` until wallet selected

**Fix 3: Verify Function Loading**
- In Console, check if function exists:
  ```javascript
  typeof generateAddressQRCode
  ```
- Should return: `"function"`

**Fix 4: Clear Browser Cache**
- If using cached version, clear with `Ctrl+Shift+Delete`
- Or hard reload: `Ctrl+Shift+R`

---

## Issue: Node Won't Start

### Symptoms:
- "Failed to start node" error
- Node status shows "Offline" after clicking Start

### Diagnosis Steps:

1. **Check Error Logs**
   ```bash
   cat ~/.btpc/logs/node.err
   ```

2. **Common Errors:**

   **Error: "Resource temporarily unavailable" (Lock conflict)**
   - Another node is using the same data directory
   - **Fix**: The app now uses `~/.btpc/data/desktop-node` (separate from testnet)
   - Verify in node.log that it shows: `Data directory: "/home/bob/.btpc/data/desktop-node"`

   **Error: "Binary not found"**
   - Node binary missing from `~/.btpc/bin/`
   - **Fix**: Copy binary:
     ```bash
     cp /home/bob/BTPC/BTPC/target/release/btpc_node ~/.btpc/bin/
     ```

   **Error: "unexpected argument '--sync-interval-secs'"**
   - Using old code with invalid arguments
   - **Fix**: Rebuild the app:
     ```bash
     cd /home/bob/BTPC/BTPC/btpc-desktop-app
     cargo build
     ```

3. **Verify Node is Running**
   ```bash
   ps aux | grep btpc_node | grep -v grep
   ```
   - Should show node running on port 18360

4. **Check Port Availability**
   ```bash
   lsof -i :18360
   ```
   - If port in use, another service is blocking it

---

## Issue: Wallet Creation Fails

### Symptoms:
- Error: "null is not an object"
- Wallet form submits but nothing happens

### Diagnosis Steps:

1. **Check Console Errors**
   - Look for: `"wallet-count-text"` error → Fixed in latest version
   - Any `getElementById` errors → DOM element missing

2. **Verify Tauri API Ready**
   - In Console, check:
     ```javascript
     window.invoke
     window.tauriReady
     ```
   - Both should exist

3. **Test Backend Connection**
   - In Console, run:
     ```javascript
     window.invoke('list_wallets').then(console.log)
     ```
   - Should return array of wallets or empty array `[]`

### Fixes:

**Fix 1: Wait for Tauri API**
- Error shows immediately on page load → API not ready
- Invoke guards now prevent this
- If still occurs, reload page

**Fix 2: Rebuild App**
```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app
cargo build
npm run tauri:dev
```

---

## Issue: Mining Won't Start

### Symptoms:
- "Mining binary not found" error
- Start Mining button does nothing

### Diagnosis Steps:

1. **Check Binary Exists**
   ```bash
   ls -lh ~/.btpc/bin/btpc_miner
   ```
   - Should show: `-rwxrwxr-x 1 bob bob 1.1M`

2. **Verify Wallet Selected**
   - Mining requires a wallet address
   - Check that wallet dropdown has selection

### Fixes:

**Fix 1: Install Mining Binary**
```bash
cd /home/bob/BTPC/BTPC
cargo build --release --bin btpc_miner
cp target/release/btpc_miner ~/.btpc/bin/
```

**Fix 2: Select Mining Wallet**
- Go to Mining page
- Select wallet from "Mining Address" dropdown
- Click "Start Mining"

---

## General Debugging Commands

### Check All Binaries:
```bash
ls -lh ~/.btpc/bin/
```
Expected:
```
btpc_miner (1.1M)
btpc_node (11M)
btpc_wallet (127M)
```

### Check Node Logs:
```bash
tail -f ~/.btpc/logs/node.log
tail -f ~/.btpc/logs/node.err
```

### Check Process Status:
```bash
# Check node
ps aux | grep btpc_node | grep -v grep

# Check mining
ps aux | grep btpc_miner | grep -v grep

# Check ports
lsof -i :18360  # RPC port
lsof -i :18361  # P2P port
```

### Restart Desktop App:
```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app

# Kill any running processes
pkill btpc_node
pkill btpc_miner

# Rebuild and run
cargo build
npm run tauri:dev
```

---

## Desktop App Configuration

**Current Setup:**
- Node Data: `~/.btpc/data/desktop-node`
- RPC Port: `18360`
- P2P Port: `18361`
- Network: `Testnet`

**Port Allocation:**
- `18350` → Testnet node (external)
- `18360` → Desktop app RPC
- `18361` → Desktop app P2P

**Binary Locations:**
- Source: `/home/bob/BTPC/BTPC/target/release/`
- Installed: `~/.btpc/bin/`

---

## Quick Fixes Summary

| Issue | Quick Fix |
|-------|-----------|
| QR code blank | Reload page (Ctrl+R), select wallet in dropdown |
| Node won't start | Check `~/.btpc/logs/node.err`, verify binary exists |
| Wallet creation fails | Check console for errors, reload page |
| Mining won't start | Install btpc_miner binary, select wallet |
| Port conflicts | Desktop app uses ports 18360/18361 (not 18350) |
| Lock errors | Uses separate directory: `~/.btpc/data/desktop-node` |

---

## Still Having Issues?

1. **Clean Rebuild:**
   ```bash
   cd /home/bob/BTPC/BTPC/btpc-desktop-app
   cargo clean
   cargo build
   npm run tauri:dev
   ```

2. **Check All Files Updated:**
   - Verify `wallet-manager.html` has QR code functions (lines 441-533)
   - Verify `main.rs` uses correct ports and data directory
   - Check git status: `git status`

3. **Enable Debug Mode:**
   - Open DevTools (F12)
   - Go to Console tab
   - All errors will be logged there

4. **Get Help:**
   - Check STATUS.md for latest updates
   - Review DESKTOP_APP_STATUS.md for fix history
   - See QR_CODE_STATUS.md for QR implementation details
