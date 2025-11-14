//! Generated genesis block for Mainnet network

use btpc_core::{
    blockchain::{Block, BlockHeader, Transaction, TransactionInput, TransactionOutput, OutPoint},
    crypto::{Hash, Script},
    Network,
};

/// Get the genesis block for Mainnet
pub fn get_genesis_block() -> Block {
    let header = BlockHeader {
        version: 1,
        prev_hash: Hash::from_hex("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap(),
        merkle_root: Hash::from_hex("611d3d35aeae8c0f0a380fc12795957b0e0d85dfc2122727810c7c5665a89a626ef42b109f644f3f5df270cf9756eaec5229587394a9326a9770ff9d82bc88d3").unwrap(),
        timestamp: 1735689600,
        bits: 0x207fffff,
        nonce: 1,
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
                value: 5000000000,
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
pub const GENESIS_HASH: &str = "060fc7adbfa428aa9e222798cf26fdd83f4b30f2cb6c95a331b69d7d93c11f58ce69e7840395dcd52b4420ac4c07994817d3d14da8e81f1ecf3296ac2723d2d7";

/// Genesis block timestamp
pub const GENESIS_TIMESTAMP: u32 = 1735689600;
