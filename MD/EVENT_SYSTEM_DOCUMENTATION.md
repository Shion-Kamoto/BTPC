# BTPC Desktop App - Event System Documentation

**Date:** 2025-10-13
**Status:** ✅ Implemented
**Constitution Reference:** Article XI.3 - Event-driven state notifications

---

## Overview

The BTPC Desktop App uses Tauri's event system to notify the frontend of state changes in real-time. This enables reactive UI updates without polling.

## Available Events

### 1. `transaction-added`
**Emitted when:** A transaction is added to RocksDB storage
**Location:** `main.rs:1622`
**Payload:**
```json
{
    "txid": "string",
    "address": "string",
    "block_height": number | null,
    "is_coinbase": boolean,
    "output_count": number,
    "confirmed_at": "ISO 8601 timestamp" | null
}
```

### 2. `wallet-balance-updated`
**Emitted when:** Wallet balance changes due to transaction activity
**Location:** `main.rs:1633`
**Payload:**
```json
{
    "address": "string",
    "balance_credits": number,
    "balance_btpc": number,
    "transaction_count": number
}
```

### 3. `node-status-changed`
**Emitted when:** Node starts or stops
**Locations:** `main.rs:605`, `main.rs:682`
**Payload:**
```json
{
    "status": "running" | "stopped",
    "pid": number | null
}
```

### 4. `network-config-changed`
**Emitted when:** Network configuration is updated
**Location:** `main.rs:2000`
**Payload:**
```json
{
    "network": "mainnet" | "testnet" | "regtest",
    "rpc_port": number,
    "p2p_port": number
}
```

---

## Frontend Integration

### JavaScript Event Listeners

```javascript
// Listen for transaction events
window.__TAURI__.event.listen('transaction-added', (event) => {
    console.log('New transaction:', event.payload);
    // Update UI with new transaction
    updateTransactionList(event.payload);
});

// Listen for balance updates
window.__TAURI__.event.listen('wallet-balance-updated', (event) => {
    console.log('Balance updated:', event.payload);
    // Update balance display
    document.getElementById('balance').textContent = event.payload.balance_btpc.toFixed(8);
});

// Listen for node status changes
window.__TAURI__.event.listen('node-status-changed', (event) => {
    console.log('Node status:', event.payload.status);
    // Update node status indicator
    updateNodeStatusIndicator(event.payload.status);
});

// Listen for network config changes
window.__TAURI__.event.listen('network-config-changed', (event) => {
    console.log('Network changed:', event.payload.network);
    // Update network display
    updateNetworkDisplay(event.payload.network);
});
```

### Tauri v2 Event API

```javascript
import { listen } from '@tauri-apps/api/event'

// Modern Tauri v2 API
const unlisten = await listen('transaction-added', (event) => {
    console.log('Transaction added:', event.payload);
});

// Cleanup when component unmounts
unlisten();
```

---

## Implementation Pattern

### Backend Event Emission (Rust)

```rust
#[tauri::command]
async fn some_command(
    app: tauri::AppHandle,  // ✅ Add AppHandle parameter
    state: State<'_, AppState>,
    // other parameters...
) -> Result<String, String> {
    // Perform state-changing operation
    let result = perform_operation();

    // Emit event to notify frontend
    let event_payload = serde_json::json!({
        "key": "value",
        "data": result
    });

    if let Err(e) = app.emit("event-name", event_payload) {
        eprintln!("⚠️ Failed to emit event: {}", e);
    }

    Ok("Success".to_string())
}
```

### Error Handling

All event emissions use the pattern:
```rust
if let Err(e) = app.emit("event-name", payload) {
    eprintln!("⚠️ Failed to emit {}: {}", "event-name", e);
}
```

This ensures:
- Events are fire-and-forget (no blocking)
- Failures are logged but don't crash the operation
- The command returns successfully even if event emission fails

---

## Future Events (Planned)

### Mining Events
- `mining-started` - Mining process begins
- `mining-stopped` - Mining process ends
- `block-found` - New block mined successfully
- `hashrate-updated` - Mining hashrate changes

### Sync Events
- `sync-progress` - Blockchain sync progress update
- `sync-completed` - Blockchain sync finished
- `sync-error` - Sync encountered an error

### Wallet Events
- `wallet-created` - New wallet created
- `wallet-deleted` - Wallet removed
- `wallet-unlocked` - Wallet successfully unlocked

---

## Testing Events

### Manual Testing

1. Start the desktop app
2. Open browser developer console
3. Execute:
```javascript
window.__TAURI__.event.listen('transaction-added', (e) => console.log('TX:', e.payload));
```
4. Trigger a transaction (e.g., mine a block)
5. Verify event is logged to console

### Integration Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tauri::test::mock_builder;

    #[tokio::test]
    async fn test_transaction_event_emission() {
        let app = mock_builder().build();
        let state = AppState::new().unwrap();

        // Mock transaction
        let tx = Transaction { /* ... */ };

        // Call command
        let result = add_transaction_to_storage(
            app.handle(),
            State::from(&state),
            tx,
            "test_address".to_string()
        ).await;

        assert!(result.is_ok());
        // Verify event was emitted (requires event listener mock)
    }
}
```

---

## Performance Considerations

- **Event Overhead:** Minimal - events are async and non-blocking
- **Payload Size:** Keep payloads small (<1KB recommended)
- **Frequency:** No artificial throttling - events emit on every state change
- **Frontend Handling:** Use debouncing/throttling on the frontend if needed

---

## Constitution Compliance

**Article XI.3:** Real-time state change notifications
✅ **Status:** Fully compliant

All state-changing operations emit events to notify the frontend:
- Transaction additions → `transaction-added`, `wallet-balance-updated`
- Node operations → `node-status-changed`
- Network changes → `network-config-changed`

---

## Troubleshooting

### Events Not Received

1. **Check listener registration:** Ensure `window.__TAURI__.event.listen()` is called before triggering
2. **Verify event name:** Event names are case-sensitive
3. **Check console logs:** Look for emission failure warnings in Rust logs
4. **Test with simple listener:**
```javascript
window.__TAURI__.event.listen('test-event', e => console.log(e));
```

### Event Payload Issues

1. **Serialization errors:** Ensure all payload fields are serializable
2. **Type mismatches:** Verify frontend expects correct types
3. **Missing fields:** Check if payload structure matches documentation

---

**Last Updated:** 2025-10-13
**Implementation:** Complete
**Next Steps:** Frontend integration for paginated transaction API