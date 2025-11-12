# Transaction Serialization Format Mismatch Analysis
**Date**: 2025-11-08
**Issue**: RPC error -32602 (Invalid params) when broadcasting transactions
**Root Cause**: Desktop app signing uses DIFFERENT format than blockchain validation

## Critical Discovery

The desktop app has **TWO serialization functions**:
1. `serialize_for_signature()` - Used during **signing** (transaction_commands_core.rs:638-675)
2. `serialize_transaction_to_bytes()` - Used during **broadcast** (transaction_commands_core.rs:385-437)

**CRITICAL**: These two functions use DIFFERENT formats, causing signature verification to fail!

## Format Comparison

| Field | btpc-core (blockchain) | Desktop serialize_for_signature | Desktop serialize_to_bytes | Match? |
|-------|----------------------|--------------------------------|---------------------------|--------|
| **Version** | 4 bytes LE | 4 bytes LE ✓ | 4 bytes LE ✓ | ✓ |
| **Input Count** | **varint** | **4 bytes u32 LE ❌** | varint ✓ | MISMATCH |
| **Txid** | 64 bytes (decoded hex) | **128 bytes (hex string as ASCII) ❌** | 64 bytes ✓ | MISMATCH |
| **Vout** | 4 bytes LE | 4 bytes LE ✓ | 4 bytes LE ✓ | ✓ |
| **Script Length** | varint | **4 bytes u32 LE ❌** | varint ✓ | MISMATCH |
| **Script Data** | Raw bytes | Raw bytes ✓ | Raw bytes ✓ | ✓ |
| **Sequence** | 4 bytes LE | 4 bytes LE ✓ | 4 bytes LE ✓ | ✓ |
| **Output Count** | varint | **4 bytes u32 LE ❌** | varint ✓ | MISMATCH |
| **Value** | 8 bytes LE | 8 bytes LE ✓ | 8 bytes LE ✓ | ✓ |
| **Script Length** | varint | **4 bytes u32 LE ❌** | varint ✓ | MISMATCH |
| **Script Data** | Raw bytes | Raw bytes ✓ | Raw bytes ✓ | ✓ |
| **Lock Time** | 4 bytes LE | 4 bytes LE ✓ | 4 bytes LE ✓ | ✓ |
| **Fork ID** | 1 byte (at end) | **MISSING ❌** | 1 byte ✓ | MISMATCH |

## Impact Analysis

### What Happens During Transaction Signing

1. Desktop app calls `serialize_for_signature(tx)` (line 612)
2. This produces **WRONG serialization**:
   - Input count: 4 bytes instead of varint
   - Txid: 128 bytes (ASCII hex) instead of 64 bytes (decoded)
   - Script lengths: 4 bytes instead of varint
   - Output count: 4 bytes instead of varint
   - **Fork ID: MISSING** (most critical!)
3. Signs this WRONG data with ML-DSA → produces signature for WRONG transaction
4. Desktop app calls `serialize_transaction_to_bytes(tx)` (line 336)
5. This produces **CORRECT serialization** with fork_id
6. Broadcasts correct bytes with WRONG signature

### What Happens During Blockchain Validation

1. btpc-core deserializes transaction (correct format) ✓
2. btpc-core calls `transaction.serialize_for_signature()` to verify signatures
3. btpc-core's version produces:
   - varint input count
   - 64-byte decoded txid
   - varint script lengths
   - **Fork ID at end**
4. Verifies ML-DSA signature against this data
5. **SIGNATURE MISMATCH** - desktop signed DIFFERENT data!

## Code Locations

### btpc-core (CORRECT - Reference Implementation)

**File**: `/home/bob/BTPC/BTPC/btpc-core/src/blockchain/transaction.rs`

**Serialization (lines 199-228)**:
```rust
pub fn serialize(&self) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&self.version.to_le_bytes());
    Self::write_varint(&mut bytes, self.inputs.len() as u64);  // VARINT
    for input in &self.inputs {
        input.serialize_into(&mut bytes);
    }
    Self::write_varint(&mut bytes, self.outputs.len() as u64); // VARINT
    for output in &self.outputs {
        output.serialize_into(&mut bytes);
    }
    bytes.extend_from_slice(&self.lock_time.to_le_bytes());
    bytes.push(self.fork_id);  // FORK_ID AT END
    bytes
}
```

**Signing Serialization (lines 232-268)**:
```rust
pub fn serialize_for_signature(&self) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&self.version.to_le_bytes());
    Self::write_varint(&mut bytes, self.inputs.len() as u64);  // VARINT
    for input in &self.inputs {
        bytes.extend_from_slice(input.previous_output.txid.as_slice()); // 64 BYTES
        bytes.extend_from_slice(&input.previous_output.vout.to_le_bytes());
        Self::write_varint(&mut bytes, 0);  // Empty script VARINT
        bytes.extend_from_slice(&input.sequence.to_le_bytes());
    }
    Self::write_varint(&mut bytes, self.outputs.len() as u64); // VARINT
    for output in &self.outputs {
        output.serialize_into(&mut bytes);
    }
    bytes.extend_from_slice(&self.lock_time.to_le_bytes());
    bytes.push(self.fork_id);  // FORK_ID AT END - CRITICAL!
    bytes
}
```

**Input Serialization (lines 438-451)**:
```rust
pub fn serialize_into(&self, bytes: &mut Vec<u8>) {
    bytes.extend_from_slice(self.previous_output.txid.as_slice()); // 64 bytes
    bytes.extend_from_slice(&self.previous_output.vout.to_le_bytes());
    let script_bytes = self.script_sig.serialize();
    Transaction::write_varint(bytes, script_bytes.len() as u64);  // VARINT
    bytes.extend_from_slice(&script_bytes);
    bytes.extend_from_slice(&self.sequence.to_le_bytes());
}
```

### Desktop App (INCORRECT - Needs Fixing)

**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/transaction_commands_core.rs`

**serialize_for_signature() (lines 638-675)** - BROKEN:
```rust
fn serialize_for_signature(tx: &Transaction) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tx.version.to_le_bytes());
    bytes.extend_from_slice(&(tx.inputs.len() as u32).to_le_bytes()); // ❌ 4 BYTES NOT VARINT
    for input in &tx.inputs {
        bytes.extend_from_slice(input.prev_txid.as_bytes());  // ❌ 128 BYTES ASCII HEX
        bytes.extend_from_slice(&input.prev_vout.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());  // ❌ 4 BYTES NOT VARINT
        bytes.extend_from_slice(&input.sequence.to_le_bytes());
    }
    bytes.extend_from_slice(&(tx.outputs.len() as u32).to_le_bytes()); // ❌ 4 BYTES NOT VARINT
    for output in &tx.outputs {
        bytes.extend_from_slice(&output.value.to_le_bytes());
        bytes.extend_from_slice(&(output.script_pubkey.len() as u32).to_le_bytes()); // ❌ 4 BYTES
        bytes.extend_from_slice(&output.script_pubkey);
    }
    bytes.extend_from_slice(&tx.lock_time.to_le_bytes());
    // ❌ MISSING: bytes.push(tx.fork_id);
    bytes
}
```

**serialize_transaction_to_bytes() (lines 385-437)** - CORRECT (after 2025-11-07 fix):
```rust
fn serialize_transaction_to_bytes(tx: &Transaction) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tx.version.to_le_bytes());
    write_varint(&mut bytes, tx.inputs.len() as u64);  // ✓ VARINT
    for input in &tx.inputs {
        let txid_bytes = hex::decode(&input.prev_txid).expect("..."); // ✓ 64 BYTES
        bytes.extend_from_slice(&txid_bytes);
        bytes.extend_from_slice(&input.prev_vout.to_le_bytes());
        write_varint(&mut bytes, input.signature_script.len() as u64); // ✓ VARINT
        bytes.extend_from_slice(&input.signature_script);
        bytes.extend_from_slice(&input.sequence.to_le_bytes());
    }
    write_varint(&mut bytes, tx.outputs.len() as u64);  // ✓ VARINT
    for output in &tx.outputs {
        bytes.extend_from_slice(&output.value.to_le_bytes());
        write_varint(&mut bytes, output.script_pubkey.len() as u64); // ✓ VARINT
        bytes.extend_from_slice(&output.script_pubkey);
    }
    bytes.extend_from_slice(&tx.lock_time.to_le_bytes());
    bytes.push(tx.fork_id);  // ✓ FORK_ID AT END
    bytes
}
```

## Fix Required

**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/transaction_commands_core.rs`
**Function**: `serialize_for_signature()` (lines 638-675)

### Changes Needed

1. **Line 645**: Replace `(tx.inputs.len() as u32).to_le_bytes()` with `write_varint()`
2. **Line 650**: Replace `input.prev_txid.as_bytes()` with `hex::decode(&input.prev_txid)`
3. **Line 654**: Replace `0u32.to_le_bytes()` with `write_varint(&mut bytes, 0)`
4. **Line 660**: Replace `(tx.outputs.len() as u32).to_le_bytes()` with `write_varint()`
5. **Line 665**: Replace `(output.script_pubkey.len() as u32).to_le_bytes()` with `write_varint()`
6. **Line 673**: **ADD MISSING**: `bytes.push(tx.fork_id);` before return

## Verification Steps

After applying the fix:

1. Sign a transaction with desktop app
2. Serialize with `serialize_for_signature()` → produces hash H1
3. Serialize with btpc-core's `serialize_for_signature()` → produces hash H2
4. **H1 MUST EQUAL H2** (currently they differ!)
5. Broadcast transaction
6. btpc-core verifies signature against H2
7. **SUCCESS** (currently fails with "Invalid params")

## Historical Context

- **2025-11-05**: Fixed `serialize_transaction_to_bytes()` to add fork_id
- **2025-11-07**: Fixed `serialize_transaction_to_bytes()` to use varint and hex::decode
- **2025-11-08**: **DISCOVERED**: `serialize_for_signature()` still uses old broken format!

This explains why transactions still fail even after the broadcast serialization was fixed.
The signature was created for the WRONG transaction format!
