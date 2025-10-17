//! BTPC Wallet CLI
//!
//! A command-line wallet for managing BTPC addresses, keys, and transactions.

#![allow(unused_variables)]

use std::{collections::HashMap, fs, path::PathBuf};

use btpc_core::{
    blockchain::{OutPoint, Transaction, TransactionInput, TransactionOutput},
    crypto::{
        EncryptedWallet, Hash, KeyEntry, PrivateKey, PublicKey, Script,
        SecurePassword, WalletData,
    },
    rpc::{RpcRequest, RpcResponse},
    Network,
};
use clap::{Arg, Command};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;

/// Wallet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    /// Network (mainnet, testnet, regtest)
    pub network: Network,
    /// Wallet data directory
    pub datadir: PathBuf,
    /// RPC server URL for node communication
    pub rpc_url: String,
    /// Default fee rate (satoshis per byte)
    pub fee_rate: u64,
}

impl Default for WalletConfig {
    fn default() -> Self {
        WalletConfig {
            network: Network::Mainnet,
            datadir: PathBuf::from("~/.btpc/wallet"),
            rpc_url: "http://127.0.0.1:8332".to_string(),
            fee_rate: 1000, // 0.00001 BTPC per byte
        }
    }
}

/// Wallet address entry
#[derive(Debug, Clone)]
pub struct WalletAddress {
    /// Address label
    pub label: String,
    /// Private key
    pub private_key: PrivateKey,
    /// Public key
    pub public_key: PublicKey,
    /// Address string
    pub address: String,
    /// Creation timestamp
    pub created_at: u64,
}

/// Wallet transaction history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    /// Transaction ID
    pub txid: Hash,
    /// Amount (positive for received, negative for sent)
    pub amount: i64,
    /// Fee paid (if sending)
    pub fee: u64,
    /// Block height (None if unconfirmed)
    pub block_height: Option<u32>,
    /// Timestamp
    pub timestamp: u64,
    /// Transaction type
    pub tx_type: TransactionType,
    /// Associated addresses
    pub addresses: Vec<String>,
}

/// Transaction type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Received,
    Sent,
    SelfTransfer,
}

/// BTPC Wallet implementation
pub struct Wallet {
    config: WalletConfig,
    addresses: HashMap<String, WalletAddress>,
    transactions: Vec<WalletTransaction>,
    rpc_client: reqwest::Client,
}

impl Wallet {
    /// Create a new wallet instance
    pub fn new(config: WalletConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let rpc_client = reqwest::Client::new();

        let mut wallet = Wallet {
            config,
            addresses: HashMap::new(),
            transactions: Vec::new(),
            rpc_client,
        };

        // Load existing wallet data
        wallet.load_wallet_data()?;

        Ok(wallet)
    }

    /// Generate a new address
    pub fn generate_address(
        &mut self,
        label: Option<String>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let private_key = PrivateKey::generate_ml_dsa()?;
        let public_key = private_key.public_key();

        // Create address from public key hash
        let address = self.create_address_from_pubkey(&public_key)?;

        let wallet_address = WalletAddress {
            label: label.unwrap_or_else(|| format!("Address {}", self.addresses.len() + 1)),
            private_key,
            public_key,
            address: address.clone(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };

        self.addresses.insert(address.clone(), wallet_address);
        self.save_wallet_data()?;

        println!("Generated new address: {}", address);
        Ok(address)
    }

    /// Get wallet balance
    pub async fn get_balance(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let mut total_balance = 0u64;

        for address in self.addresses.keys() {
            let balance = self.get_address_balance(address).await?;
            total_balance += balance;
        }

        Ok(total_balance)
    }

    /// List all addresses
    pub fn list_addresses(&self) -> Vec<&WalletAddress> {
        self.addresses.values().collect()
    }

    /// Send transaction
    pub async fn send_transaction(
        &mut self,
        to_address: &str,
        amount: u64,
        fee_rate: Option<u64>,
    ) -> Result<Hash, Box<dyn std::error::Error>> {
        let fee_rate = fee_rate.unwrap_or(self.config.fee_rate);

        // Find UTXOs to spend
        let utxos = self.select_utxos(amount + fee_rate * 250).await?; // Estimate 250 bytes

        if utxos.is_empty() {
            return Err("Insufficient funds".into());
        }

        // Calculate total input value
        let total_input = utxos.iter().map(|(_, _, value)| *value).sum::<u64>();

        // Create transaction inputs
        let mut inputs = Vec::new();
        for (outpoint, address, _) in &utxos {
            let wallet_addr = self
                .addresses
                .get(address)
                .ok_or("Address not found in wallet")?;

            let input = TransactionInput {
                previous_output: outpoint.clone(),
                script_sig: Script::new(), // Will be signed later
                sequence: 0xffffffff,
            };
            inputs.push(input);
        }

        // Create transaction outputs
        let mut outputs = Vec::new();

        // Main output to recipient
        outputs.push(TransactionOutput {
            value: amount,
            script_pubkey: Script::from_bytes(to_address.as_bytes().to_vec()),
        });

        // Change output if needed
        let estimated_fee = fee_rate * 250; // Rough estimate
        if total_input > amount + estimated_fee {
            let change_amount = total_input - amount - estimated_fee;
            let change_address = self
                .addresses
                .keys()
                .next()
                .ok_or("No change address available")?;

            outputs.push(TransactionOutput {
                value: change_amount,
                script_pubkey: Script::from_bytes(change_address.as_bytes().to_vec()),
            });
        }

        // Create transaction
        let mut transaction = Transaction {
            version: 1,
            inputs,
            outputs,
            lock_time: 0,
            fork_id: 0, // Testnet fork_id is 0
        };

        // Sign transaction
        self.sign_transaction(&mut transaction, &utxos)?;

        // Broadcast transaction
        let txid = self.broadcast_transaction(&transaction).await?;

        // Add to transaction history
        self.transactions.push(WalletTransaction {
            txid: txid.clone(),
            amount: -(amount as i64),
            fee: estimated_fee,
            block_height: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            tx_type: TransactionType::Sent,
            addresses: vec![to_address.to_string()],
        });

        self.save_wallet_data()?;

        println!("Transaction sent: {}", txid.to_hex());
        Ok(txid)
    }

    /// Get transaction history
    pub fn get_transaction_history(&self) -> &[WalletTransaction] {
        &self.transactions
    }

    /// Sync wallet with blockchain
    pub async fn sync(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Syncing wallet with blockchain...");

        // Update transaction confirmations
        let pending_txids: Vec<Hash> = self
            .transactions
            .iter()
            .filter(|tx| tx.block_height.is_none())
            .map(|tx| tx.txid.clone())
            .collect();

        for txid in pending_txids {
            if let Ok(info) = self.get_transaction_info(&txid).await {
                if let Some(height) = info.get("blockheight").and_then(|v| v.as_u64()) {
                    if let Some(tx) = self.transactions.iter_mut().find(|t| t.txid == txid) {
                        tx.block_height = Some(height as u32);
                    }
                }
            }
        }

        // Check for new incoming transactions
        let addresses: Vec<String> = self.addresses.keys().cloned().collect();
        for address in &addresses {
            self.check_address_transactions(address).await?;
        }

        self.save_wallet_data()?;
        println!("Wallet sync completed");
        Ok(())
    }

    /// Create address from public key
    fn create_address_from_pubkey(
        &self,
        pubkey: &PublicKey,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Simple address format: base58-encoded public key hash
        let pubkey_hash = Hash::hash(&pubkey.to_bytes());
        Ok(format!(
            "btpc_{}",
            hex::encode(&pubkey_hash.as_bytes()[..20])
        ))
    }

    /// Get balance for a specific address
    async fn get_address_balance(&self, address: &str) -> Result<u64, Box<dyn std::error::Error>> {
        // This would query the node for UTXO set for this address
        // For now, return 0 as placeholder
        Ok(0)
    }

    /// Select UTXOs for spending
    async fn select_utxos(
        &self,
        amount: u64,
    ) -> Result<Vec<(OutPoint, String, u64)>, Box<dyn std::error::Error>> {
        // This would implement coin selection algorithm
        // For now, return empty vec as placeholder
        Ok(Vec::new())
    }

    /// Sign transaction
    fn sign_transaction(
        &self,
        transaction: &mut Transaction,
        utxos: &[(OutPoint, String, u64)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        for (i, (_, address, _)) in utxos.iter().enumerate() {
            let wallet_addr = self
                .addresses
                .get(address)
                .ok_or("Address not found in wallet")?;

            // Create signature hash
            let sig_hash = transaction.hash(); // Simplified

            // Sign with private key
            let signature = wallet_addr.private_key.sign(sig_hash.as_slice())?;

            // Create script signature
            let mut script_data = Vec::new();
            script_data.extend_from_slice(&signature.to_bytes());
            script_data.extend_from_slice(&wallet_addr.public_key.to_bytes());

            transaction.inputs[i].script_sig = Script::from_bytes(script_data);
        }

        Ok(())
    }

    /// Broadcast transaction to network
    async fn broadcast_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<Hash, Box<dyn std::error::Error>> {
        // Serialize transaction to hex
        let tx_bytes = transaction.serialize();
        let tx_hex = hex::encode(&tx_bytes);

        // Send via RPC sendrawtransaction
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "sendrawtransaction".to_string(),
            params: Some(serde_json::json!([tx_hex])),
            id: Some(serde_json::Value::Number(1.into())),
        };

        let response = self
            .rpc_client
            .post(&self.config.rpc_url)
            .json(&request)
            .send()
            .await?;

        let rpc_response: RpcResponse = response.json().await?;

        if let Some(error) = rpc_response.error {
            return Err(format!("RPC error: {:?}", error).into());
        }

        if let Some(result) = rpc_response.result {
            let txid_hex = result
                .as_str()
                .ok_or("Invalid txid format from RPC")?;
            let txid = Hash::from_hex(txid_hex)?;
            Ok(txid)
        } else {
            Err("No result from RPC".into())
        }
    }

    /// Get transaction information from node
    async fn get_transaction_info(
        &self,
        txid: &Hash,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "gettransaction".to_string(),
            params: Some(serde_json::json!([txid.to_hex()])),
            id: Some(serde_json::Value::Number(1.into())),
        };

        let response = self
            .rpc_client
            .post(&self.config.rpc_url)
            .json(&request)
            .send()
            .await?;

        let rpc_response: RpcResponse = response.json().await?;

        if let Some(result) = rpc_response.result {
            Ok(result)
        } else {
            Err("Transaction not found".into())
        }
    }

    /// Check for new transactions for an address
    async fn check_address_transactions(
        &mut self,
        address: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // This would query the blockchain for transactions involving this address
        // For now, this is a placeholder
        Ok(())
    }

    /// Get wallet file path
    fn wallet_file_path(&self) -> PathBuf {
        let mut path = self.config.datadir.clone();
        path.push(format!("wallet-{}.dat", self.config.network.as_str()));
        path
    }

    /// Get password from user (securely)
    fn get_password(&self, prompt: &str) -> Result<SecurePassword, Box<dyn std::error::Error>> {
        use std::io::{self, Write};

        print!("{}: ", prompt);
        io::stdout().flush()?;

        // In production, use a secure password input method (no echo)
        // For now, use simple input
        let mut password = String::new();
        io::stdin().read_line(&mut password)?;

        Ok(SecurePassword::new(password.trim().to_string()))
    }

    /// Load wallet data from disk
    fn load_wallet_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let wallet_path = self.wallet_file_path();

        // Check if wallet file exists
        if !wallet_path.exists() {
            println!("No existing wallet found. Starting with empty wallet.");
            return Ok(());
        }

        // Load encrypted wallet
        let encrypted_wallet = EncryptedWallet::load_from_file(&wallet_path)?;

        // Get password from user
        let password = self.get_password("Enter wallet password")?;

        // Decrypt wallet
        let wallet_data = encrypted_wallet.decrypt(&password)?;

        // Reconstruct addresses from wallet data
        for key_entry in wallet_data.keys {
            let private_key = key_entry.to_private_key()?;
            let public_key = key_entry.to_public_key()?;

            let wallet_address = WalletAddress {
                label: key_entry.label.clone(),
                private_key,
                public_key,
                address: key_entry.address.clone(),
                created_at: key_entry.created_at,
            };

            self.addresses.insert(key_entry.address.clone(), wallet_address);
        }

        println!("Wallet loaded successfully. {} addresses found.", self.addresses.len());
        Ok(())
    }

    /// Save wallet data to disk
    fn save_wallet_data(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Ensure data directory exists
        fs::create_dir_all(&self.config.datadir)?;

        // Convert addresses to key entries
        let keys: Vec<KeyEntry> = self.addresses.values()
            .map(|addr| KeyEntry::from_private_key(
                &addr.private_key,
                addr.label.clone(),
                addr.address.clone(),
            ))
            .collect();

        // Create wallet data
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let wallet_data = WalletData {
            network: self.config.network.as_str().to_string(),
            keys,
            created_at: now, // TODO: Track real creation time
            modified_at: now,
        };

        // Get password from user (or use existing)
        let password = self.get_password("Enter wallet password (to encrypt)")?;

        // Encrypt wallet
        let encrypted_wallet = EncryptedWallet::encrypt(&wallet_data, &password)?;

        // Save to file
        let wallet_path = self.wallet_file_path();
        encrypted_wallet.save_to_file(&wallet_path)?;

        println!("Wallet saved to: {}", wallet_path.display());
        Ok(())
    }

    /// Backup wallet
    pub fn backup_wallet(&self, backup_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let wallet_path = self.wallet_file_path();

        if !wallet_path.exists() {
            return Err("No wallet file to backup".into());
        }

        // Simple file copy for backup
        fs::copy(&wallet_path, backup_path)?;

        println!("Wallet backed up to: {}", backup_path);
        Ok(())
    }

    /// Restore wallet from backup
    pub fn restore_wallet(&mut self, backup_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Ensure backup file exists
        if !PathBuf::from(backup_path).exists() {
            return Err("Backup file not found".into());
        }

        // Load and decrypt the backup
        let encrypted_wallet = EncryptedWallet::load_from_file(backup_path)?;
        let password = self.get_password("Enter backup password")?;
        let wallet_data = encrypted_wallet.decrypt(&password)?;

        // Clear existing addresses
        self.addresses.clear();

        // Reconstruct addresses from backup
        for key_entry in wallet_data.keys {
            let private_key = key_entry.to_private_key()?;
            let public_key = key_entry.to_public_key()?;

            let wallet_address = WalletAddress {
                label: key_entry.label.clone(),
                private_key,
                public_key,
                address: key_entry.address.clone(),
                created_at: key_entry.created_at,
            };

            self.addresses.insert(key_entry.address.clone(), wallet_address);
        }

        // Save to current wallet file
        self.save_wallet_data()?;

        println!("Wallet restored successfully. {} addresses recovered.", self.addresses.len());
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("btpc-wallet")
        .version("0.1.0")
        .about("BTPC Wallet - Quantum-resistant Bitcoin wallet")
        .arg(
            Arg::new("network")
                .long("network")
                .value_name("NETWORK")
                .help("Network to use (mainnet, testnet, regtest)")
                .default_value("mainnet"),
        )
        .arg(
            Arg::new("datadir")
                .long("datadir")
                .value_name("DIR")
                .help("Wallet data directory")
                .default_value("~/.btpc/wallet"),
        )
        .arg(
            Arg::new("rpc-url")
                .long("rpc-url")
                .value_name("URL")
                .help("RPC server URL")
                .default_value("http://127.0.0.1:8332"),
        )
        .subcommand(
            Command::new("generate")
                .about("Generate a new address")
                .arg(
                    Arg::new("label")
                        .long("label")
                        .value_name("LABEL")
                        .help("Address label"),
                ),
        )
        .subcommand(Command::new("list").about("List all addresses"))
        .subcommand(Command::new("balance").about("Show wallet balance"))
        .subcommand(
            Command::new("send")
                .about("Send BTPC")
                .arg(
                    Arg::new("address")
                        .value_name("ADDRESS")
                        .help("Recipient address")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("amount")
                        .value_name("AMOUNT")
                        .help("Amount to send (in BTPC)")
                        .required(true)
                        .index(2),
                )
                .arg(
                    Arg::new("fee-rate")
                        .long("fee-rate")
                        .value_name("RATE")
                        .help("Fee rate (satoshis per byte)"),
                ),
        )
        .subcommand(Command::new("history").about("Show transaction history"))
        .subcommand(Command::new("sync").about("Sync wallet with blockchain"))
        .subcommand(
            Command::new("backup").about("Backup wallet").arg(
                Arg::new("path")
                    .value_name("PATH")
                    .help("Backup file path")
                    .required(true)
                    .index(1),
            ),
        )
        .subcommand(
            Command::new("restore")
                .about("Restore wallet from backup")
                .arg(
                    Arg::new("path")
                        .value_name("PATH")
                        .help("Backup file path")
                        .required(true)
                        .index(1),
                ),
        )
        .get_matches();

    // Parse configuration
    let network = match matches.get_one::<String>("network").unwrap().as_str() {
        "mainnet" => Network::Mainnet,
        "testnet" => Network::Testnet,
        "regtest" => Network::Regtest,
        _ => {
            eprintln!("Invalid network. Use mainnet, testnet, or regtest");
            std::process::exit(1);
        }
    };

    let config = WalletConfig {
        network,
        datadir: PathBuf::from(matches.get_one::<String>("datadir").unwrap()),
        rpc_url: matches.get_one::<String>("rpc-url").unwrap().clone(),
        fee_rate: 1000,
    };

    let mut wallet = Wallet::new(config)?;

    // Handle subcommands
    match matches.subcommand() {
        Some(("generate", sub_matches)) => {
            let label = sub_matches.get_one::<String>("label").cloned();
            wallet.generate_address(label)?;
        }
        Some(("list", _)) => {
            let addresses = wallet.list_addresses();
            if addresses.is_empty() {
                println!("No addresses in wallet. Use 'generate' to create one.");
            } else {
                println!("Wallet addresses:");
                for addr in addresses {
                    println!(
                        "  {} - {} (created: {})",
                        addr.address, addr.label, addr.created_at
                    );
                }
            }
        }
        Some(("balance", _)) => {
            let balance = wallet.get_balance().await?;
            println!("Total balance: {:.8} BTPC", balance as f64 / 100_000_000.0);
        }
        Some(("send", sub_matches)) => {
            let to_address = sub_matches.get_one::<String>("address").unwrap();
            let amount_str = sub_matches.get_one::<String>("amount").unwrap();
            let amount = (amount_str.parse::<f64>()? * 100_000_000.0) as u64;

            let fee_rate = sub_matches
                .get_one::<String>("fee-rate")
                .map(|s| s.parse::<u64>())
                .transpose()?;

            wallet
                .send_transaction(to_address, amount, fee_rate)
                .await?;
        }
        Some(("history", _)) => {
            let history = wallet.get_transaction_history();
            if history.is_empty() {
                println!("No transactions in wallet history.");
            } else {
                println!("Transaction history:");
                for tx in history {
                    let confirmations = if let Some(height) = tx.block_height {
                        format!("{} confirmations", height)
                    } else {
                        "Unconfirmed".to_string()
                    };

                    println!(
                        "  {} - {:.8} BTPC ({}) - {}",
                        tx.txid.to_hex(),
                        tx.amount as f64 / 100_000_000.0,
                        match tx.tx_type {
                            TransactionType::Received => "Received",
                            TransactionType::Sent => "Sent",
                            TransactionType::SelfTransfer => "Self",
                        },
                        confirmations
                    );
                }
            }
        }
        Some(("sync", _)) => {
            wallet.sync().await?;
        }
        Some(("backup", sub_matches)) => {
            let backup_path = sub_matches.get_one::<String>("path").unwrap();
            wallet.backup_wallet(backup_path)?;
        }
        Some(("restore", sub_matches)) => {
            let backup_path = sub_matches.get_one::<String>("path").unwrap();
            wallet.restore_wallet(backup_path)?;
        }
        _ => {
            println!("No subcommand provided. Use --help to see available commands.");
        }
    }

    Ok(())
}
