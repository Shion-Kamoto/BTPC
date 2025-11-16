//! Test coinbase transaction serialization to debug block submission bug

use btpc_core::blockchain::Transaction;
use btpc_core::crypto::Hash;

#[test]
fn test_coinbase_transaction_serialization() {
    let reward = 5_000_000_000u64;
    let recipient_hash = Hash::zero();

    let coinbase_tx = Transaction::coinbase(reward, recipient_hash);

    // Check structure
    println!("\n=== Coinbase Transaction Structure ===");
    println!("  Inputs: {}", coinbase_tx.inputs.len());
    println!("  Outputs: {}", coinbase_tx.outputs.len());
    println!("  Input vout: 0x{:08x}", coinbase_tx.inputs[0].previous_output.vout);
    println!("  Input sequence: 0x{:08x}", coinbase_tx.inputs[0].sequence);
    println!("  Output value: {} sats", coinbase_tx.outputs[0].value);
    println!("  Fork ID: {}", coinbase_tx.fork_id);

    // Verify correct structure
    assert_eq!(coinbase_tx.inputs.len(), 1);
    assert_eq!(coinbase_tx.outputs.len(), 1);
    assert_eq!(coinbase_tx.inputs[0].previous_output.vout, 0xffffffff);
    assert_eq!(coinbase_tx.inputs[0].sequence, 0xffffffff);
    assert_eq!(coinbase_tx.outputs[0].value, reward);

    // Serialize
    let serialized = coinbase_tx.serialize();
    println!("\n=== Serialized Transaction ({} bytes) ===", serialized.len());
    println!("{}", hex::encode(&serialized));

    // Manually decode to verify correctness
    println!("\n=== Manual Decode ===");
    let mut cursor = 0;

    // Version (4 bytes)
    let version = u32::from_le_bytes([serialized[0], serialized[1], serialized[2], serialized[3]]);
    println!("  Version: 0x{:08x} ({})", version, version);
    cursor += 4;

    // Input count (1 byte varint for value 1)
    let input_count = serialized[cursor];
    println!("  Input count: {}", input_count);
    cursor += 1;

    // Previous TXID (64 bytes for SHA-512)
    let txid_start = cursor;
    let txid_end = cursor + 64;
    println!("  Prev TXID: {}...", hex::encode(&serialized[txid_start..txid_start+8]));
    cursor = txid_end;

    // Previous vout (4 bytes)
    let vout = u32::from_le_bytes([
        serialized[cursor],
        serialized[cursor+1],
        serialized[cursor+2],
        serialized[cursor+3]
    ]);
    println!("  Prev VOUT: 0x{:08x} ({})", vout, vout);
    assert_eq!(vout, 0xffffffff, "VOUT should be 0xffffffff for coinbase");
    cursor += 4;

    // ScriptSig length
    let script_len = serialized[cursor];
    println!("  ScriptSig length: {}", script_len);
    cursor += 1;
    cursor += script_len as usize;

    // Sequence (4 bytes)
    let sequence = u32::from_le_bytes([
        serialized[cursor],
        serialized[cursor+1],
        serialized[cursor+2],
        serialized[cursor+3]
    ]);
    println!("  Sequence: 0x{:08x}", sequence);
    assert_eq!(sequence, 0xffffffff, "Sequence should be 0xffffffff for coinbase");
    cursor += 4;

    // Output count
    let output_count = serialized[cursor];
    println!("  Output count: {}", output_count);
    assert_eq!(output_count, 1, "Should have exactly 1 output");
    cursor += 1;

    // Output value (8 bytes)
    let value = u64::from_le_bytes([
        serialized[cursor],
        serialized[cursor+1],
        serialized[cursor+2],
        serialized[cursor+3],
        serialized[cursor+4],
        serialized[cursor+5],
        serialized[cursor+6],
        serialized[cursor+7],
    ]);
    println!("  Output value: {} satoshis", value);
    assert_eq!(value, reward, "Output value should match reward");
    cursor += 8;

    // ScriptPubKey length
    let script_pk_len = serialized[cursor];
    println!("  ScriptPubKey length: {}", script_pk_len);
    cursor += 1;
    cursor += script_pk_len as usize;

    // Locktime (4 bytes)
    let locktime = u32::from_le_bytes([
        serialized[cursor],
        serialized[cursor+1],
        serialized[cursor+2],
        serialized[cursor+3],
    ]);
    println!("  Locktime: {}", locktime);
    cursor += 4;

    // Fork ID (1 byte)
    let fork_id = serialized[cursor];
    println!("  Fork ID: {}", fork_id);
    cursor += 1;

    println!("\n=== Total parsed: {} bytes ===", cursor);
    assert_eq!(cursor, serialized.len(), "Should parse entire transaction");

    println!("\n✅ Coinbase transaction serialization is CORRECT");
}