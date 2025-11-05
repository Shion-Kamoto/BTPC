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
        merkle_root: Hash::from_hex("a21552cf353981c1418145531ce7f1ca380350788f34851df14c17e8ba60788a0201eb71f642ffea67998cfe525085c641675186a912309ea8d2db86d4d0e623").unwrap(),
        timestamp: 1759614480,
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
pub const GENESIS_HASH: &str = "66f93816446e9aae8eebd6a26c4bc9b74f161c54871a59d4722d39baf194df3ec91605384f048c76c6524089ce7f2029e89557a0c482db56aa44aaf58028ad6c";

/// Genesis block timestamp
pub const GENESIS_TIMESTAMP: u32 = 1759614480;
