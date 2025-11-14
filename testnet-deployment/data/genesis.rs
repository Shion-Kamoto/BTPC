//! Generated genesis block for Testnet network

use btpc_core::{
    blockchain::{Block, BlockHeader, Transaction, TransactionInput, TransactionOutput, OutPoint},
    crypto::{Hash, Script},
    Network,
};

/// Get the genesis block for Testnet
pub fn get_genesis_block() -> Block {
    let header = BlockHeader {
        version: 1,
        prev_hash: Hash::from_hex("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap(),
        merkle_root: Hash::from_hex("de9cc322120388aff278e62e335840ad6398b5d0b6f66f0c3f4f2d6af97eca9c70648715ef6a665031592c50cc6402560e10a60ece7d883c1081be57fb980708").unwrap(),
        timestamp: 1727913600,
        bits: 0x207fffff,
        nonce: 3,
    };

    let coinbase_tx = Transaction {
        version: 1,
        inputs: vec![
            TransactionInput {
                prev_out: OutPoint {
                    txid: Hash::from_hex("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap(),
                    vout: 0xffffffff,
                },
                script_sig: Script::new(hex::decode("").unwrap()),
                sequence: 0xffffffff,
            }
        ],
        outputs: vec![
            TransactionOutput {
                value: 5000000000000,
                script_pubkey: Script::new(hex::decode("").unwrap()),
            },
            TransactionOutput {
                value: 5000000000000,
                script_pubkey: Script::new(hex::decode("").unwrap()),
            },
        ],
        lock_time: 0,
    };

    Block {
        header,
        transactions: vec![coinbase_tx],
    }
}

/// Genesis block hash
pub const GENESIS_HASH: &str = "292b1e19b70988b0ea1f38415905768abac0698b516396ac72b56e3a03471c2dc7e3a86b38246edf053bff84b6ecca6d471677ef3b879c20be7696dd83b79ae1";

/// Genesis block timestamp
pub const GENESIS_TIMESTAMP: u32 = 1727913600;
