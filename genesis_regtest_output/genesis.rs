//! Generated genesis block for Regtest network

use btpc_core::{
    blockchain::{Block, BlockHeader, Transaction, TransactionInput, TransactionOutput, OutPoint},
    crypto::{Hash, Script},
    Network,
};

/// Get the genesis block for Regtest
pub fn get_genesis_block() -> Block {
    let header = BlockHeader {
        version: 1,
        prev_hash: Hash::from_hex("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap(),
        merkle_root: Hash::from_hex("7681a8382612a2734142b1a1472b6ff6a1b5bdaa22869d6e6ce216339bcaaaa5be1c744ef4a732660a11c2d792638b3655e47caf6159b8731aae5961f68280f4").unwrap(),
        timestamp: 1760842137,
        bits: 0x207fffff,
        nonce: 0,
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
                value: 100000000000000,
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
pub const GENESIS_HASH: &str = "4691e9fe9de2c8c21028482edececb5b97ed9eb5d3d9dcb87188f9a129b53b89861cc09d777cd7869086a9edd7aacfe8d3e123436249f9ea7ade04f4a66b1e60";

/// Genesis block timestamp
pub const GENESIS_TIMESTAMP: u32 = 1760842137;
