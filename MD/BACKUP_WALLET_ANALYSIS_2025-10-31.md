# Wallet Backup Analysis - 2025-10-31

## Investigation Summary

Investigated whether wallet backup failure is due to file dialog/permissions issue.

**Finding**: Parameters are correct. Issue is NOT a parameter mismatch, but likely **user expectation mismatch** or **unclear UX**.

## Current Implementation

### Backend Flow (`wallet_manager.rs:581-619`)

1. **Get wallet by ID** (line 582-586)
2. **Ensure backups directory exists** (line 589-595)
   - Creates `~/.btpc/wallet-backups/` if missing
   - Uses `std::fs::create_dir_all()` with error handling
3. **Generate timestamped filename** (line 598-600)
   - Format: `backup_{nickname}_{YYYYMMDD_HHMMSS}.dat`
   - Example: `backup_MyWallet_20251031_143522.dat`
4. **Verify source wallet file exists** (line 603-608)
   - Checks if `~/.btpc/wallets/wallet_{uuid}.dat` exists
   - Returns error if missing
5. **Copy wallet file to backup location** (line 611-615)
   - Uses `std::fs::copy()` to create backup
   - Full encryption preserved (Argon2id encrypted .dat file)
6. **Return success message** (line 618)
   - Returns path: `"Wallet backed up to: /home/user/.btpc/wallet-backups/backup_...dat"`

### Frontend Flow (`wallet-manager.html:937-960`)

1. **Confirmation dialog** (line 949)
   - Shows: `Backup wallet "{nickname}"? This will create an encrypted backup file in your wallet directory.`
2. **Invoke Tauri command** (line 951-953)
   - Correctly passes `{wallet_id: currentWalletId}`
   - ✅ Parameters match backend expectation
3. **Success alert** (line 955)
   - Shows backend message + "Your wallet has been safely backed up!"
4. **Error alert** (line 957)
   - Shows error message from backend

## Parameter Validation ✅

**Backend Signature**:
```rust
pub async fn backup_wallet(
    state: State<'_, AppState>,
    wallet_id: String  // Direct parameter, NOT wrapped in struct
) -> Result<String, String>
```

**Frontend Call**:
```javascript
await window.invoke('backup_wallet', {
    wallet_id: currentWalletId  // Correct - matches backend
});
```

**Status**: ✅ Parameters are CORRECT. This is NOT a parameter mismatch issue.

## Possible Issues

### 1. User Expectation Mismatch (MOST LIKELY)

**Problem**: User expects a "Save As" file picker dialog to choose backup location, but backup saves to fixed directory.

**Current Behavior**:
- Saves to `~/.btpc/wallet-backups/` automatically
- User has NO control over location
- Alert shows path but user might miss it

**User Experience Issues**:
- No visual confirmation (just text alert)
- No file explorer shown
- User doesn't know where to find backup
- Can't choose USB drive, cloud folder, etc.

**Evidence**: User said "cannot export the backup wallet to file" - the word "export" suggests they want to save it somewhere specific.

### 2. Success Message Not Visible

**Problem**: Alert might be dismissed too quickly or hidden behind other windows.

**Current Message**:
```
Wallet backed up to: /home/bob/.btpc/wallet-backups/backup_MyWallet_20251031_143522.dat

Your wallet has been safely backed up!
```

**Issue**: Long path in alert - user might not read it fully.

### 3. Permissions Issue (UNLIKELY)

**Problem**: Cannot create `~/.btpc/wallet-backups/` directory or write backup file.

**Mitigation Already Present**:
- Backend creates directory with `create_dir_all()` (line 590)
- Error handling for directory creation (line 591-594)
- Error handling for file copy (line 612-615)

**Likelihood**: Low - user would see error alert if this failed.

### 4. Source Wallet File Missing (UNLIKELY)

**Problem**: Wallet file `~/.btpc/wallets/wallet_{uuid}.dat` doesn't exist.

**Check**: Line 603-608 verifies file exists before backup.

**Likelihood**: Low - would return explicit error "Wallet file not found".

## Testing Results

To verify which issue it is, check:

1. **Does the backup file exist?**
   ```bash
   ls -la ~/.btpc/wallet-backups/
   ```
   - If files exist → Issue #1 or #2 (UX problem)
   - If no files → Issue #3 or #4 (backend failure)

2. **Check permissions**:
   ```bash
   ls -ld ~/.btpc/wallet-backups/
   stat ~/.btpc/wallet-backups/
   ```
   - Should show `drwxr-xr-x` with user ownership

3. **Check wallet source files**:
   ```bash
   ls -la ~/.btpc/wallets/
   ```
   - Should show `wallet_*.dat` files for each created wallet

## Recommended Fixes

### Fix #1: Add File Save Dialog (Recommended)

**Implementation**: Use Tauri's file dialog API to let user choose backup location.

**Changes Needed**:
1. Add `tauri::api::dialog` dependency
2. Modify `backup_wallet` command to accept optional custom path
3. Show file picker in frontend before invoking command
4. Pass selected path to backend

**Code Sketch**:
```rust
// Backend
pub async fn backup_wallet(
    state: State<'_, AppState>,
    wallet_id: String,
    custom_path: Option<String>  // NEW: Allow custom backup location
) -> Result<String, String>
```

```javascript
// Frontend
async function backupWallet() {
    // Show file save dialog
    const savePath = await window.__TAURI__.dialog.save({
        defaultPath: `backup_${wallet.nickname}_${Date.now()}.dat`,
        filters: [{ name: 'BTPC Wallet Backup', extensions: ['dat'] }]
    });

    if (savePath) {
        const result = await window.invoke('backup_wallet', {
            wallet_id: currentWalletId,
            custom_path: savePath
        });
    }
}
```

### Fix #2: Improve Success Message (Quick Fix)

**Implementation**: Show more prominent success notification.

**Changes Needed**:
```javascript
// Replace alert() with better UI
toast.success(`Backup created!\n\nLocation: ${result}`, 10000);

// Or open file location in file manager
const backupPath = result.match(/Wallet backed up to: (.+)/)[1];
toast.success(`Backup created! Click to open folder.`, {
    duration: 10000,
    onClick: () => window.invoke('open_file_location', { path: backupPath })
});
```

### Fix #3: Add "Open Backup Folder" Button (Quick Fix)

**Implementation**: Add button to open backup directory in file manager.

**Changes Needed**:
```javascript
// After successful backup
const openFolder = confirm(
    `${result}\n\nYour wallet has been safely backed up!\n\nOpen backup folder?`
);

if (openFolder) {
    await window.invoke('open_backup_folder');
}
```

```rust
// New Tauri command
#[tauri::command]
pub async fn open_backup_folder(state: State<'_, AppState>) -> Result<(), String> {
    let wallet_manager = state.wallet_manager.lock().unwrap();
    let backup_dir = &wallet_manager.config.backups_dir;

    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open")
        .arg(backup_dir)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}
```

## Conclusion

**Root Cause**: NOT a parameter mismatch. Parameters are correct.

**Likely Issue**: User expectation mismatch - user expects file save dialog but backup saves to fixed location automatically.

**Evidence**:
1. Parameters match backend signature ✅
2. Backend implementation is sound ✅
3. Error handling is present ✅
4. User complaint says "cannot export" (implies wanting control over save location)

**Recommended Solution**: Implement Fix #1 (file save dialog) for best UX, or Fix #2/#3 as quick improvements.

## Next Steps

1. Test if backup files are being created in `~/.btpc/wallet-backups/`
2. If yes → Implement UX improvement (Fix #1, #2, or #3)
3. If no → Check console errors and backend logs