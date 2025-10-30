//! BTPC Genesis Block Generation Tool
//!
//! Creates genesis blocks for different BTPC networks with proper configuration.

use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use btpc_core::{
    blockchain::{Block, BlockHeader, OutPoint, Transaction, TransactionInput, TransactionOutput},
    consensus::pow::MiningTarget,
    crypto::{Hash, Script},
    Network,
};
use clap::{Arg, Command};
use serde::{Deserialize, Serialize};

/// Genesis block configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    /// Network type
    pub network: Network,
    /// Genesis timestamp (Unix timestamp)
    pub timestamp: u32,
    /// Genesis message in coinbase
    pub message: String,
    /// Initial reward allocation
    pub rewards: Vec<GenesisReward>,
    /// Mining difficulty target
    pub difficulty_target: u32,
    /// Maximum nonce to try during mining
    pub max_nonce: u32,
}

/// Genesis reward allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisReward {
    /// Recipient address/pubkey
    pub address: String,
    /// Amount in satoshis
    pub amount: u64,
    /// Description/label
    pub label: String,
}

impl Default for GenesisConfig {
    fn default() -> Self {
        GenesisConfig {
            network: Network::Mainnet,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as u32,
            message: "BTPC Genesis Block - Quantum-resistant Bitcoin".to_string(),
            rewards: vec![GenesisReward {
                address: "genesis_dev_fund".to_string(),
                amount: 1_000_000 * 100_000_000, // 1M BTPC for development
                label: "Development Fund".to_string(),
            }],
            difficulty_target: 0x207fffff, // Easy target for genesis
            max_nonce: u32::MAX,
        }
    }
}

/// Genesis block generator
pub struct GenesisGenerator {
    config: GenesisConfig,
}

impl GenesisGenerator {
    /// Create a new genesis generator
    pub fn new(config: GenesisConfig) -> Self {
        GenesisGenerator { config }
    }

    /// Generate the genesis block
    pub fn generate(&self) -> Result<Block, Box<dyn std::error::Error>> {
        println!(
            "Generating genesis block for {:?} network...",
            self.config.network
        );
        println!("Message: {}", self.config.message);
        println!("Timestamp: {}", self.config.timestamp);
        println!("Target: 0x{:08x}", self.config.difficulty_target);

        // Create coinbase transaction
        let coinbase_tx = self.create_coinbase_transaction()?;
        let transactions = vec![coinbase_tx];

        // Calculate merkle root
        let merkle_root = self.calculate_merkle_root(&transactions);

        // Create block header
        let mut header = BlockHeader {
            version: 1,
            prev_hash: Hash::zero(),
            merkle_root,
            timestamp: self.config.timestamp as u64,
            bits: self.config.difficulty_target,
            nonce: 0,
        };

        // Mine the block
        println!("Mining genesis block...");
        let start_time = std::time::Instant::now();
        let mut hashes_computed = 0u64;

        let target = MiningTarget::easy_target();

        for nonce in 0..self.config.max_nonce {
            header.nonce = nonce;
            let block_hash = header.hash();
            hashes_computed += 1;

            // Progress reporting
            if nonce % 100_000 == 0 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let hashrate = hashes_computed as f64 / elapsed;
                println!(
                    "Nonce: {} | Hash: {} | Rate: {:.0} H/s",
                    nonce,
                    block_hash.to_hex(),
                    hashrate
                );
            }

            if block_hash.meets_target(&target.as_hash()) {
                let elapsed = start_time.elapsed();
                println!("✅ Genesis block mined!");
                println!("Nonce: {}", nonce);
                println!("Hash: {}", block_hash.to_hex());
                println!("Time: {:.2}s", elapsed.as_secs_f64());
                println!("Hashes: {}", hashes_computed);
                println!(
                    "Rate: {:.0} H/s",
                    hashes_computed as f64 / elapsed.as_secs_f64()
                );

                return Ok(Block {
                    header,
                    transactions,
                });
            }
        }

        Err("Failed to mine genesis block within nonce range".into())
    }

    /// Create the coinbase transaction for genesis block
    fn create_coinbase_transaction(&self) -> Result<Transaction, Box<dyn std::error::Error>> {
        println!(
            "Creating coinbase transaction with {} outputs...",
            self.config.rewards.len()
        );

        // Create coinbase input
        let coinbase_input = TransactionInput {
            previous_output: OutPoint {
                txid: Hash::zero(),
                vout: 0xffffffff,
            },
            script_sig: Script::new(),
            sequence: 0xffffffff,
        };

        // Create outputs for each reward
        let mut outputs = Vec::new();
        let mut total_value = 0u64;

        for reward in &self.config.rewards {
            outputs.push(TransactionOutput {
                value: reward.amount,
                script_pubkey: Script::new(),
            });
            total_value += reward.amount;
            println!(
                "  {} BTPC -> {} ({})",
                reward.amount as f64 / 100_000_000.0,
                reward.address,
                reward.label
            );
        }

        println!(
            "Total genesis allocation: {} BTPC",
            total_value as f64 / 100_000_000.0
        );

        Ok(Transaction {
            version: 1,
            inputs: vec![coinbase_input],
            outputs,
            lock_time: 0,
            fork_id: 0, // Testnet fork_id is 0
        })
    }

    /// Calculate merkle root of transactions
    fn calculate_merkle_root(&self, transactions: &[Transaction]) -> Hash {
        if transactions.is_empty() {
            return Hash::zero();
        }

        let mut hashes: Vec<Hash> = transactions.iter().map(|tx| tx.hash()).collect();

        while hashes.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in hashes.chunks(2) {
                let combined = if chunk.len() == 2 {
                    // Combine two hashes
                    let mut data = Vec::new();
                    data.extend_from_slice(chunk[0].as_bytes());
                    data.extend_from_slice(chunk[1].as_bytes());
                    Hash::hash(&data)
                } else {
                    // Odd number, duplicate the last hash
                    let mut data = Vec::new();
                    data.extend_from_slice(chunk[0].as_bytes());
                    data.extend_from_slice(chunk[0].as_bytes());
                    Hash::hash(&data)
                };
                next_level.push(combined);
            }

            hashes = next_level;
        }

        hashes[0]
    }

    /// Validate the generated genesis block
    pub fn validate_genesis(&self, block: &Block) -> Result<(), Box<dyn std::error::Error>> {
        println!("Validating genesis block...");

        // Check block hash meets target
        let target = MiningTarget::easy_target();
        let block_hash = block.header.hash();

        if !block_hash.meets_target(&target.as_hash()) {
            return Err("Genesis block hash does not meet difficulty target".into());
        }

        // Check merkle root
        let calculated_merkle = self.calculate_merkle_root(&block.transactions);
        if calculated_merkle != block.header.merkle_root {
            return Err("Invalid merkle root in genesis block".into());
        }

        // Check coinbase transaction
        if block.transactions.len() != 1 {
            return Err("Genesis block must have exactly one transaction".into());
        }

        let coinbase = &block.transactions[0];
        if coinbase.inputs.len() != 1 {
            return Err("Coinbase transaction must have exactly one input".into());
        }

        let coinbase_input = &coinbase.inputs[0];
        if coinbase_input.previous_output.txid != Hash::zero()
            || coinbase_input.previous_output.vout != 0xffffffff
        {
            return Err("Invalid coinbase input".into());
        }

        println!("✅ Genesis block validation passed");
        Ok(())
    }

    /// Export genesis block to various formats
    pub fn export_genesis(
        &self,
        block: &Block,
        output_dir: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(output_dir)?;

        // Export as JSON
        self.export_json(block, &format!("{}/genesis.json", output_dir))?;

        // Export as Rust code
        self.export_rust_code(block, &format!("{}/genesis.rs", output_dir))?;

        // Export configuration
        self.export_config(&format!("{}/genesis_config.json", output_dir))?;

        // Export block info
        self.export_block_info(block, &format!("{}/genesis_info.txt", output_dir))?;

        println!("Genesis block exported to: {}", output_dir);
        Ok(())
    }

    /// Export genesis block as JSON
    fn export_json(&self, block: &Block, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Use Block's native Serialize implementation for correct format
        let json_str = serde_json::to_string_pretty(block)?;
        fs::write(path, json_str)?;
        println!("Exported JSON: {}", path);
        Ok(())
    }

    /// Export genesis block as Rust code
    fn export_rust_code(
        &self,
        block: &Block,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let rust_code = format!(
            r#"//! Generated genesis block for {:?} network

use btpc_core::{{
    blockchain::{{Block, BlockHeader, Transaction, TransactionInput, TransactionOutput, OutPoint}},
    crypto::{{Hash, Script}},
    Network,
}};

/// Get the genesis block for {:?}
pub fn get_genesis_block() -> Block {{
    let header = BlockHeader {{
        version: {},
        prev_hash: Hash::from_hex("{}").unwrap(),
        merkle_root: Hash::from_hex("{}").unwrap(),
        timestamp: {},
        bits: 0x{:08x},
        nonce: {},
    }};

    let coinbase_tx = Transaction {{
        version: {},
        inputs: vec![
            TransactionInput {{
                prev_out: OutPoint {{
                    txid: Hash::from_hex("{}").unwrap(),
                    vout: 0xffffffff,
                }},
                script_sig: Script::new(hex::decode("{}").unwrap()),
                sequence: 0xffffffff,
            }}
        ],
        outputs: vec![{}
        ],
        lock_time: {},
    }};

    Block {{
        header,
        transactions: vec![coinbase_tx],
    }}
}}

/// Genesis block hash
pub const GENESIS_HASH: &str = "{}";

/// Genesis block timestamp
pub const GENESIS_TIMESTAMP: u32 = {};
"#,
            self.config.network,
            self.config.network,
            block.header.version,
            block.header.prev_hash.to_hex(),
            block.header.merkle_root.to_hex(),
            block.header.timestamp,
            block.header.bits,
            block.header.nonce,
            block.transactions[0].version,
            block.transactions[0].inputs[0].previous_output.txid.to_hex(),
            hex::encode(&block.transactions[0].inputs[0].script_sig),
            block.transactions[0].outputs.iter()
                .map(|output| format!(
                    "\n            TransactionOutput {{\n                value: {},\n                script_pubkey: Script::new(hex::decode(\"{}\").unwrap()),\n            }},",
                    output.value,
                    hex::encode(&output.script_pubkey)
                ))
                .collect::<String>(),
            block.transactions[0].lock_time,
            block.header.hash().to_hex(),
            block.header.timestamp
        );

        fs::write(path, rust_code)?;
        println!("Exported Rust code: {}", path);
        Ok(())
    }

    /// Export configuration
    fn export_config(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(path, serde_json::to_string_pretty(&self.config)?)?;
        println!("Exported config: {}", path);
        Ok(())
    }

    /// Export block information
    fn export_block_info(
        &self,
        block: &Block,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let info = format!(
            r#"BTPC Genesis Block Information
=============================

Network: {:?}
Hash: {}
Merkle Root: {}
Timestamp: {} ({})
Difficulty Target: 0x{:08x}
Nonce: {}

Coinbase Transaction:
- TXID: {}
- Message: {}
- Outputs: {}
- Total Value: {} BTPC

Reward Allocations:
{}
"#,
            self.config.network,
            block.header.hash().to_hex(),
            block.header.merkle_root.to_hex(),
            block.header.timestamp,
            chrono::DateTime::from_timestamp(block.header.timestamp as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "Invalid timestamp".to_string()),
            block.header.bits,
            block.header.nonce,
            block.transactions[0].hash().to_hex(),
            self.config.message,
            block.transactions[0].outputs.len(),
            block.transactions[0]
                .outputs
                .iter()
                .map(|o| o.value)
                .sum::<u64>() as f64
                / 100_000_000.0,
            self.config
                .rewards
                .iter()
                .map(|r| format!(
                    "- {} BTPC -> {} ({})",
                    r.amount as f64 / 100_000_000.0,
                    r.address,
                    r.label
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );

        fs::write(path, info)?;
        println!("Exported info: {}", path);
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("btpc-genesis")
        .version("0.1.0")
        .about("BTPC Genesis Block Generator")
        .arg(
            Arg::new("network")
                .long("network")
                .value_name("NETWORK")
                .help("Network type (mainnet, testnet, regtest)")
                .default_value("mainnet"),
        )
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .value_name("FILE")
                .help("Configuration file path"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .value_name("DIR")
                .help("Output directory")
                .default_value("./genesis_output"),
        )
        .arg(
            Arg::new("message")
                .long("message")
                .value_name("TEXT")
                .help("Genesis block message"),
        )
        .arg(
            Arg::new("timestamp")
                .long("timestamp")
                .value_name("UNIX_TIME")
                .help("Genesis timestamp (Unix timestamp)"),
        )
        .arg(
            Arg::new("difficulty")
                .long("difficulty")
                .value_name("TARGET")
                .help("Difficulty target (hex, e.g., 0x207fffff)"),
        )
        .get_matches();

    // Parse network
    let network = match matches.get_one::<String>("network").unwrap().as_str() {
        "mainnet" => Network::Mainnet,
        "testnet" => Network::Testnet,
        "regtest" => Network::Regtest,
        _ => {
            eprintln!("Invalid network. Use mainnet, testnet, or regtest");
            std::process::exit(1);
        }
    };

    // Load or create configuration
    let mut config = if let Some(config_path) = matches.get_one::<String>("config") {
        let config_data = fs::read_to_string(config_path)?;
        serde_json::from_str(&config_data)?
    } else {
        GenesisConfig::default()
    };

    // Override with command line arguments
    config.network = network;

    if let Some(message) = matches.get_one::<String>("message") {
        config.message = message.clone();
    }

    if let Some(timestamp_str) = matches.get_one::<String>("timestamp") {
        config.timestamp = timestamp_str.parse()?;
    }

    if let Some(difficulty_str) = matches.get_one::<String>("difficulty") {
        config.difficulty_target = if let Some(hex_str) = difficulty_str.strip_prefix("0x") {
            u32::from_str_radix(hex_str, 16)?
        } else {
            difficulty_str.parse()?
        };
    }

    let output_dir = matches.get_one::<String>("output").unwrap();

    // Generate genesis block
    let generator = GenesisGenerator::new(config);
    let genesis_block = generator.generate()?;

    // Validate genesis block
    generator.validate_genesis(&genesis_block)?;

    // Export genesis block
    generator.export_genesis(&genesis_block, output_dir)?;

    println!("\n🎉 Genesis block generation completed successfully!");
    println!("Genesis hash: {}", genesis_block.header.hash().to_hex());

    Ok(())
}
