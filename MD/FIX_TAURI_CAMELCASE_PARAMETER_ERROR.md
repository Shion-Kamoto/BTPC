# Fix: Tauri CamelCase Parameter Error for Transaction Commands

## Problem
The BTPC desktop application was experiencing an error when attempting to send transactions:
```
Transaction failed: invalid args `walletId` for command `create_transaction`: command create_transaction missing required key walletId
```

## Root Cause
Tauri automatically converts snake_case Rust function parameters to camelCase when exposed to JavaScript. The frontend was using snake_case names (e.g., `wallet_id`, `transaction_id`) when calling Tauri commands, but Tauri expected camelCase (e.g., `walletId`, `transactionId`).

## Solution
Updated the JavaScript frontend (`transactions.html`) to use camelCase parameter names when calling Tauri commands.

## Files Modified
- `/home/bob/BTPC/BTPC/btpc-desktop-app/ui/transactions.html`

## Changes Made

### 1. `create_transaction` Command (Line 675-681)
**Before:**
```javascript
const createResult = await window.invoke('create_transaction', {
    wallet_id: txData.walletId,
    from_address: txData.fromAddress,
    to_address: txData.toAddress,
    amount: amountcredits,
    fee_rate: txData.feeRate
});
```

**After:**
```javascript
const createResult = await window.invoke('create_transaction', {
    walletId: txData.walletId,  // Tauri expects camelCase from JavaScript
    fromAddress: txData.fromAddress,
    toAddress: txData.toAddress,
    amount: amountcredits,
    feeRate: txData.feeRate
});
```

### 2. `sign_transaction` Command (Line 696-700)
**Before:**
```javascript
const signResult = await window.invoke('sign_transaction', {
    transaction_id: transactionId,
    wallet_id: txData.walletId,
    password: txData.password
});
```

**After:**
```javascript
const signResult = await window.invoke('sign_transaction', {
    transactionId: transactionId,  // Tauri expects camelCase
    walletId: txData.walletId,
    password: txData.password
});
```

### 3. `broadcast_transaction` Command (Line 708-710)
**Before:**
```javascript
const broadcastResult = await window.invoke('broadcast_transaction', {
    transaction_id: transactionId
});
```

**After:**
```javascript
const broadcastResult = await window.invoke('broadcast_transaction', {
    transactionId: transactionId  // Tauri expects camelCase
});
```

### 4. `get_transaction_status` Command (Line 724-726)
**Before:**
```javascript
const status = await window.invoke('get_transaction_status', {
    transaction_id: transactionId
});
```

**After:**
```javascript
const status = await window.invoke('get_transaction_status', {
    transactionId: transactionId  // Tauri expects camelCase
});
```

### 5. `cancel_transaction` Command (Line 933-935)
**Before:**
```javascript
await window.invoke('cancel_transaction', {
    transaction_id: currentTransactionId
});
```

**After:**
```javascript
await window.invoke('cancel_transaction', {
    transactionId: currentTransactionId  // Tauri expects camelCase
});
```

## Technical Details

### Tauri's Automatic Case Conversion
Tauri 2.0 automatically converts between Rust's snake_case and JavaScript's camelCase conventions:
- Rust function parameters: `wallet_id`, `transaction_id`, `from_address`
- JavaScript expectations: `walletId`, `transactionId`, `fromAddress`

### Backend Function Signature
The Rust backend function signature remains unchanged:
```rust
#[tauri::command]
pub async fn create_transaction(
    wallet_id: String,
    from_address: String,
    to_address: String,
    amount: u64,
    fee_rate: Option<u64>,
    state: State<'_, AppState>,
) -> Result<CreateTransactionResponse, String>
```

## Verification Steps
1. Open the BTPC desktop application
2. Navigate to the Transactions page
3. Select a wallet with funds
4. Enter a recipient address and amount
5. Click "Send BTPC"
6. Enter wallet password when prompted
7. Verify the transaction is created, signed, and broadcast successfully

## Result
The transaction sending functionality now works correctly with proper parameter naming conventions between the JavaScript frontend and Rust backend.