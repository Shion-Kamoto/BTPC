//! BTPC Full Node
//!
//! A complete Bitcoin-compatible full node implementation with quantum-resistant features.

#![allow(dead_code)]

use std::{path::PathBuf, sync::Arc};

use btpc_core::{
    consensus::{BlockValidator, TransactionValidator},
    crypto::Hash,
    network::{
        discovery::PeerDiscovery,
        PeerEvent,
        SimplePeerManager,
        sync::SyncManager,
    },
    rpc::{
        handlers::BlockchainRpcHandlers,
        server::{RpcConfig, RpcServer},
    },
    storage::{
        blockchain_db::{BlockchainDb, BlockchainDatabase},
        database::{Database, DatabaseConfig},
        utxo_db::{UtxoDb, UTXODatabase},
        StorageConfig,
    },
    Network,
};
use clap::{Arg, Command};
use tokio::{
    sync::RwLock,
    time::{interval, Duration},
};

/// Node configuration
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Network (mainnet, testnet, regtest)
    pub network: Network,
    /// Data directory
    pub datadir: PathBuf,
    /// RPC configuration
    pub rpc: RpcConfig,
    /// Maximum peer connections
    pub max_peers: usize,
    /// Enable mining
    pub enable_mining: bool,
    /// Listen address for P2P
    pub listen_addr: String,
    /// Connect to specific peers
    pub connect_peers: Vec<String>,
    /// Enable daemon mode
    pub daemon: bool,
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            network: Network::Mainnet,
            datadir: PathBuf::from("~/.btpc"),
            rpc: RpcConfig::default(),
            max_peers: 8,
            enable_mining: false,
            listen_addr: "0.0.0.0:8333".to_string(),
            connect_peers: Vec::new(),
            daemon: false,
        }
    }
}

/// Full node implementation
pub struct Node {
    config: NodeConfig,
    blockchain_db: Arc<RwLock<BlockchainDb>>,
    utxo_db: Arc<RwLock<UtxoDb>>,
    sync_manager: Arc<RwLock<SyncManager>>,
    peer_discovery: Arc<RwLock<PeerDiscovery>>,
    peer_manager: Arc<SimplePeerManager>,
    rpc_server: Arc<RpcServer>,
    block_validator: Arc<BlockValidator>,
    tx_validator: Arc<TransactionValidator>,
    mempool: Arc<btpc_core::storage::mempool::Mempool>,
    block_height: Arc<RwLock<u32>>,
}

impl Node {
    /// Create a new node instance
    pub async fn new(config: NodeConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize storage
        let storage_config = StorageConfig {
            data_dir: config.datadir.to_string_lossy().to_string(),
            cache_size: 100 * 1024 * 1024,       // 100MB
            write_buffer_size: 16 * 1024 * 1024, // 16MB
            max_open_files: 1000,
            enable_compression: true,
            enable_statistics: true,
            bloom_filter_bits: 10,
            mempool_config: btpc_core::storage::MempoolConfig::default(),
        };

        let db_config = DatabaseConfig::from_storage_config(&storage_config);
        let database = Arc::new(Database::open(
            config.datadir.join("blockchain"),
            db_config,
        )?);

        let blockchain_db = Arc::new(RwLock::new(BlockchainDb::new(Arc::clone(&database))));
        let utxo_db = Arc::new(RwLock::new(UtxoDb::new(Arc::clone(&database))));

        // Load genesis block if database is empty
        {
            let blockchain = blockchain_db.read().await;
            if blockchain.get_chain_tip().ok().flatten().is_none() {
                drop(blockchain); // Release read lock

                println!("Initializing blockchain with genesis block for {:?}...", config.network);

                // Try to load genesis block from file first
                let genesis_path = config.datadir.join("genesis.json");
                let genesis_block = if genesis_path.exists() {
                    println!("Loading genesis block from {:?}", genesis_path);
                    use btpc_core::blockchain::Block;

                    let genesis_json = std::fs::read_to_string(&genesis_path)
                        .expect("Failed to read genesis.json");
                    let genesis_value: serde_json::Value = serde_json::from_str(&genesis_json)
                        .expect("Failed to parse genesis.json");
                    let block: Block = serde_json::from_value(genesis_value)
                        .expect("Failed to deserialize genesis block");

                    // Validate genesis hash matches expected network hash
                    use btpc_core::blockchain::genesis::WellKnownGenesis;
                    let expected_hash = match config.network {
                        Network::Mainnet => WellKnownGenesis::mainnet_hash(),
                        Network::Testnet => WellKnownGenesis::testnet_hash(),
                        Network::Regtest => {
                            // Regtest uses custom genesis, no validation needed
                            println!("Using custom regtest genesis block");
                            block.hash()
                        }
                    };

                    let block_hash = block.hash();
                    if config.network != Network::Regtest && block_hash != expected_hash {
                        panic!(
                            "Genesis block hash mismatch for {:?}!\nExpected: {}\nGot: {}",
                            config.network,
                            expected_hash.to_hex(),
                            block_hash.to_hex()
                        );
                    }

                    println!("✅ Genesis hash validated for {:?}", config.network);
                    block
                } else {
                    // Fallback to well-known genesis blocks
                    println!("No genesis.json found, using well-known genesis for {:?}", config.network);
                    use btpc_core::blockchain::genesis::WellKnownGenesis;

                    match config.network {
                        Network::Mainnet => WellKnownGenesis::mainnet_block(),
                        Network::Testnet => WellKnownGenesis::testnet_block(),
                        Network::Regtest => {
                            // Auto-generate regtest genesis
                            println!("Generating new regtest genesis block...");
                            use btpc_core::blockchain::genesis::{GenesisConfig, GenesisCreator};
                            let genesis_config = GenesisConfig::regtest();
                            let creator = GenesisCreator::new(genesis_config);
                            let block = creator.create_genesis_block();

                            // Save regtest genesis for future use
                            if let Ok(json) = serde_json::to_string_pretty(&block) {
                                let _ = std::fs::write(&genesis_path, json);
                                println!("Regtest genesis saved to {:?}", genesis_path);
                            }

                            block
                        }
                    }
                };

                let genesis_hash = genesis_block.hash();
                println!("Genesis hash: {}", genesis_hash.to_hex());

                // Store genesis block
                let mut blockchain_mut = blockchain_db.write().await;
                if let Err(e) = blockchain_mut.store_block(&genesis_block) {
                    eprintln!("Failed to store genesis block: {}", e);
                } else {
                    println!("✅ Genesis block stored successfully");

                    // Store genesis block UTXOs
                    use btpc_core::blockchain::{utxo::UTXO, OutPoint};
                    let mut utxo_mut = utxo_db.write().await;
                    for (vout, output) in genesis_block.transactions[0].outputs.iter().enumerate() {
                        let outpoint = OutPoint {
                            txid: genesis_block.transactions[0].hash(),
                            vout: vout as u32,
                        };
                        let utxo = UTXO::new(outpoint, output.clone(), 0, true);
                        if let Err(e) = utxo_mut.store_utxo(&utxo) {
                            eprintln!("Failed to store genesis UTXO: {}", e);
                        }
                    }
                }
            }
        }

        // Get initial block height
        let block_height = Arc::new(RwLock::new(0u32));

        // Initialize sync and networking
        let sync_manager = Arc::new(RwLock::new(SyncManager::new(config.max_peers)));
        let peer_discovery = Arc::new(RwLock::new(PeerDiscovery::new(1000)));

        // Initialize peer manager
        let listen_addr: std::net::SocketAddr = config.listen_addr.parse()
            .unwrap_or_else(|_| "0.0.0.0:8333".parse().unwrap());

        // Create network config based on BTPC network type
        let mut network_config = match config.network {
            Network::Mainnet => btpc_core::network::NetworkConfig::mainnet(),
            Network::Testnet => btpc_core::network::NetworkConfig::testnet(),
            Network::Regtest => btpc_core::network::NetworkConfig::testnet(), // Use testnet config for regtest
        };
        network_config.listen_addr = listen_addr;

        let peer_manager = Arc::new(SimplePeerManager::new(
            network_config,
            Arc::clone(&block_height),
        ));

        // Initialize validators
        let block_validator = Arc::new(BlockValidator::new());
        let tx_validator = Arc::new(TransactionValidator::new());

        // Initialize mempool
        let mempool = Arc::new(btpc_core::storage::mempool::Mempool::new()?);

        // Initialize RPC server
        let rpc_server = Arc::new(RpcServer::new(config.rpc.clone()));

        // Register RPC handlers
        let blockchain_handlers = BlockchainRpcHandlers::new(
            Arc::clone(&blockchain_db),
            Arc::clone(&utxo_db),
            config.network,
        );
        blockchain_handlers.register_methods(&rpc_server).await;

        Ok(Node {
            config,
            blockchain_db,
            utxo_db,
            sync_manager,
            peer_discovery,
            peer_manager,
            rpc_server,
            block_validator,
            tx_validator,
            mempool,
            block_height,
        })
    }

    /// Start the node
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting BTPC Node...");
        println!("Network: {:?}", self.config.network);
        println!("Data directory: {:?}", self.config.datadir);
        println!(
            "RPC server: {}:{}",
            self.config.rpc.bind_address, self.config.rpc.port
        );

        // Start RPC server
        let rpc_server = Arc::clone(&self.rpc_server);
        tokio::spawn(async move {
            if let Err(e) = rpc_server.start().await {
                eprintln!("RPC server error: {}", e);
            }
        });

        // Start P2P networking
        self.start_networking().await?;

        // Start P2P event handler
        self.start_peer_event_handler().await?;

        // Start sync process
        self.start_sync().await?;

        // Start mining if enabled
        if self.config.enable_mining {
            self.start_mining().await?;
        }

        // Keep the node running
        println!("BTPC Node started successfully");

        if self.config.daemon {
            // In daemon mode, run indefinitely
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        } else {
            // In interactive mode, wait for user input
            println!("Press Ctrl+C to stop the node");
            tokio::signal::ctrl_c().await?;
            println!("Shutting down BTPC Node...");
        }

        Ok(())
    }

    /// Start P2P networking
    async fn start_networking(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting P2P networking on {}", self.config.listen_addr);

        // Start listening for incoming connections
        self.peer_manager.start_listening().await?;

        // Bootstrap peer discovery
        let mut discovery = self.peer_discovery.write().await;
        discovery.query_dns_seeds().await.ok(); // Best effort
        drop(discovery);

        // Connect to specific peers if configured
        for peer_addr_str in &self.config.connect_peers {
            if let Ok(peer_addr) = peer_addr_str.parse() {
                println!("Connecting to peer: {}", peer_addr);
                self.peer_manager.connect_to_peer(peer_addr).await.ok(); // Best effort
            } else {
                eprintln!("Invalid peer address: {}", peer_addr_str);
            }
        }

        Ok(())
    }

    /// Start P2P event handler to process blocks from peers
    async fn start_peer_event_handler(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting P2P event handler...");

        // Take the event receiver from peer manager
        let mut event_rx = self.peer_manager.take_event_receiver().await
            .ok_or("Event receiver already taken")?;

        let blockchain_db = Arc::clone(&self.blockchain_db);
        let utxo_db = Arc::clone(&self.utxo_db);
        let block_validator = Arc::clone(&self.block_validator);
        let block_height = Arc::clone(&self.block_height);
        let mempool = Arc::clone(&self.mempool);

        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match event {
                    PeerEvent::PeerConnected { addr, height } => {
                        println!("🔗 Peer {} connected (height: {})", addr, height);
                    }
                    PeerEvent::PeerDisconnected { addr, reason } => {
                        println!("❌ Peer {} disconnected (reason: {:?})", addr, reason);
                    }
                    PeerEvent::BlockReceived { from, block } => {
                        let block_hash = block.hash();
                        println!("📦 Processing block {} from {}", block_hash.to_hex(), from);

                        // Validate block
                        let blockchain = blockchain_db.read().await;
                        match block_validator.validate_block(&block) {
                            Ok(_) => {
                                drop(blockchain);

                                // Store block
                                let mut blockchain_mut = blockchain_db.write().await;
                                if let Err(e) = blockchain_mut.store_block(&block) {
                                    eprintln!("Failed to store block {}: {}", block_hash.to_hex(), e);
                                } else {
                                    println!("✅ Block {} stored successfully", block_hash.to_hex());

                                    // Update UTXO set
                                    let mut utxo_mut = utxo_db.write().await;
                                    for (tx_idx, tx) in block.transactions.iter().enumerate() {
                                        use btpc_core::blockchain::utxo::UTXO;
                                        use btpc_core::blockchain::OutPoint;

                                        for (vout, output) in tx.outputs.iter().enumerate() {
                                            let outpoint = OutPoint {
                                                txid: tx.hash(),
                                                vout: vout as u32,
                                            };
                                            let utxo = UTXO::new(
                                                outpoint,
                                                output.clone(),
                                                block.header.timestamp as u32, // Using timestamp as height placeholder
                                                tx_idx == 0, // First transaction is coinbase
                                            );
                                            if let Err(e) = utxo_mut.store_utxo(&utxo) {
                                                eprintln!("Failed to store UTXO: {}", e);
                                            }
                                        }
                                    }

                                    // Update block height
                                    *block_height.write().await += 1;
                                }
                            }
                            Err(e) => {
                                eprintln!("Invalid block {} from {}: {}", block_hash.to_hex(), from, e);
                            }
                        }
                    }
                    PeerEvent::TransactionReceived { from, tx } => {
                        let tx_hash = tx.hash();
                        println!("💳 Received transaction {} from {}", tx_hash.to_hex(), from);

                        // Add to mempool
                        if let Err(e) = mempool.add_transaction(tx) {
                            eprintln!("Failed to add transaction {} to mempool: {}", tx_hash.to_hex(), e);
                        } else {
                            println!("✅ Transaction {} added to mempool", tx_hash.to_hex());
                        }
                    }
                    PeerEvent::InventoryReceived { from: _, inv: _ } => {
                        // Inventory is already logged by SimplePeerManager
                    }
                }
            }
        });

        Ok(())
    }

    /// Start blockchain synchronization
    async fn start_sync(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting blockchain synchronization...");

        let mut sync_manager = self.sync_manager.write().await;
        sync_manager.start_sync()?;

        // Start periodic sync status reporting
        let sync_manager_clone = Arc::clone(&self.sync_manager);
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));
            let mut was_synced = false;
            loop {
                interval.tick().await;
                let sync = sync_manager_clone.read().await;
                if !sync.is_synced() {
                    println!("Sync progress: {:.1}%", sync.progress());
                    was_synced = false;
                } else if !was_synced {
                    println!("✅ Blockchain synchronized (100.0%)");
                    was_synced = true;
                }
            }
        });

        Ok(())
    }

    /// Start mining
    async fn start_mining(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting mining...");

        let blockchain_db = Arc::clone(&self.blockchain_db);
        let utxo_db = Arc::clone(&self.utxo_db);
        let block_validator = Arc::clone(&self.block_validator);
        let peer_manager = Arc::clone(&self.peer_manager);
        let block_height = Arc::clone(&self.block_height);
        let _network = self.config.network;

        // Spawn mining thread
        tokio::spawn(async move {
            use btpc_core::{
                blockchain::{Block, BlockHeader, Transaction, TransactionInput, TransactionOutput, OutPoint},
                consensus::pow::{ProofOfWork, MiningTarget},
                crypto::{Hash, Script},
                economics::rewards::RewardCalculator,
                blockchain::merkle::calculate_merkle_root,
            };
            use std::time::{SystemTime, UNIX_EPOCH};

            println!("Mining thread started");
            let mut block_count = 0u64;

            loop {
                // Get current chain tip (best block)
                let best_block = {
                    let blockchain = blockchain_db.read().await;
                    match blockchain.get_chain_tip() {
                        Ok(Some(block)) => block,
                        Ok(None) => {
                            eprintln!("No genesis block found, cannot mine");
                            tokio::time::sleep(Duration::from_secs(10)).await;
                            continue;
                        }
                        Err(e) => {
                            eprintln!("Error getting chain tip: {}", e);
                            tokio::time::sleep(Duration::from_secs(10)).await;
                            continue;
                        }
                    }
                };

                let prev_hash = best_block.hash();
                // Estimate height from blocks mined (genesis = 0, next = 1, etc.)
                let height = block_count as u32 + 1;

                // Create coinbase transaction
                let reward = RewardCalculator::calculate_block_reward(height);
                let coinbase_input = TransactionInput {
                    previous_output: OutPoint {
                        txid: Hash::zero(),
                        vout: 0xffffffff,
                    },
                    script_sig: Script::from_bytes(
                        format!("Block {} mined by btpc_node", height).as_bytes().to_vec()
                    ),
                    sequence: 0xffffffff,
                };

                let coinbase_output = TransactionOutput {
                    value: reward,
                    script_pubkey: Script::from_bytes(
                        format!("miner_address_{}", height).as_bytes().to_vec()
                    ),
                };

                let coinbase_tx = Transaction {
                    version: 1,
                    inputs: vec![coinbase_input],
                    outputs: vec![coinbase_output],
                    lock_time: 0,
                    fork_id: _network.fork_id(),
                };

                // TODO: Add transactions from mempool here
                let transactions = vec![coinbase_tx];

                // Calculate merkle root
                let merkle_root = match calculate_merkle_root(&transactions) {
                    Ok(root) => root,
                    Err(e) => {
                        eprintln!("Error calculating merkle root: {}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                };

                // Create block header
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                // Use network-appropriate difficulty
                use btpc_core::consensus::DifficultyTarget;
                let network_difficulty = DifficultyTarget::minimum_for_network(_network);

                let header = BlockHeader {
                    version: 1,
                    prev_hash,
                    merkle_root,
                    timestamp,
                    bits: network_difficulty.bits,
                    nonce: 0,
                };

                // Mine the block with network difficulty
                let target = MiningTarget::from_bytes(*network_difficulty.as_bytes());
                println!("Mining block {} at height {}...", block_count, height);

                match ProofOfWork::mine(&header, &target) {
                    Ok(proof) => {
                        // Update header with found nonce
                        let mut mined_header = header.clone();
                        mined_header.nonce = proof.nonce() as u32;

                        let mined_block = Block {
                            header: mined_header,
                            transactions: transactions.clone(),
                        };

                        let block_hash = mined_block.hash();
                        println!("🎉 Block mined! Hash: {}", block_hash.to_hex());
                        println!("   Height: {}, Reward: {} satoshis", height, reward);

                        // Validate block
                        if let Err(e) = block_validator.validate_block(&mined_block) {
                            eprintln!("Mined block failed validation: {}", e);
                            continue;
                        }

                        // Store block
                        {
                            let mut blockchain = blockchain_db.write().await;
                            if let Err(e) = blockchain.store_block(&mined_block) {
                                eprintln!("Error storing mined block: {}", e);
                                continue;
                            }
                        }

                        // Update UTXO set
                        {
                            use btpc_core::blockchain::utxo::UTXO;
                            let mut utxo_db_mut = utxo_db.write().await;
                            for (vout, output) in mined_block.transactions[0].outputs.iter().enumerate() {
                                let outpoint = OutPoint {
                                    txid: mined_block.transactions[0].hash(),
                                    vout: vout as u32,
                                };
                                let utxo = UTXO::new(outpoint, output.clone(), height, true); // coinbase = true
                                if let Err(e) = utxo_db_mut.store_utxo(&utxo) {
                                    eprintln!("Error storing UTXO: {}", e);
                                }
                            }
                        }

                        // Update block height
                        *block_height.write().await = height;

                        // Broadcast block to peers
                        peer_manager.broadcast_block(&mined_block).await;

                        block_count += 1;
                        println!("✅ Block {} added to blockchain (total mined: {})", height, block_count);

                        // Small delay between blocks
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    Err(e) => {
                        eprintln!("Mining failed: {}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// Broadcast a new block to all peers
    pub async fn broadcast_block(&self, block: &btpc_core::blockchain::Block) {
        self.peer_manager.broadcast_block(block).await;
    }

    /// Broadcast a transaction to all peers
    pub async fn broadcast_transaction(&self, tx: &btpc_core::blockchain::Transaction) {
        self.peer_manager.broadcast_transaction(tx).await;
    }

    /// Add a transaction to the mempool
    pub fn add_transaction_to_mempool(&self, tx: btpc_core::blockchain::Transaction) -> Result<(), String> {
        self.mempool.add_transaction(tx)
            .map_err(|e| e.to_string())
    }

    /// Get transactions from mempool for mining
    pub fn get_mempool_transactions(&self, max_count: usize) -> Vec<btpc_core::blockchain::Transaction> {
        let all_txs = self.mempool.get_all_transactions();
        all_txs.into_iter().take(max_count).collect()
    }

    /// Get node status
    pub async fn get_status(&self) -> NodeStatus {
        let sync = self.sync_manager.read().await;
        let discovery = self.peer_discovery.read().await;

        NodeStatus {
            network: self.config.network,
            sync_progress: sync.progress(),
            is_synced: sync.is_synced(),
            peer_count: discovery.address_count(),
            best_block_hash: Hash::zero(), // TODO: Get actual best block
            block_height: 0,               // TODO: Get actual height
        }
    }
}

/// Node status information
#[derive(Debug, Clone)]
pub struct NodeStatus {
    pub network: Network,
    pub sync_progress: f32,
    pub is_synced: bool,
    pub peer_count: usize,
    pub best_block_hash: Hash,
    pub block_height: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let matches = Command::new("btpc-node")
        .version("0.1.0")
        .about("BTPC Full Node - Quantum-resistant Bitcoin implementation")
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
                .help("Data directory")
                .default_value("~/.btpc"),
        )
        .arg(
            Arg::new("rpcport")
                .long("rpcport")
                .value_name("PORT")
                .help("RPC server port")
                .default_value("8332"),
        )
        .arg(
            Arg::new("rpcbind")
                .long("rpcbind")
                .value_name("ADDR")
                .help("RPC server bind address")
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::new("listen")
                .long("listen")
                .value_name("ADDR")
                .help("P2P listen address")
                .default_value("0.0.0.0:8333"),
        )
        .arg(
            Arg::new("connect")
                .long("connect")
                .value_name("PEER")
                .help("Connect to specific peer")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("daemon")
                .long("daemon")
                .help("Run as daemon")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("mine")
                .long("mine")
                .help("Enable mining")
                .action(clap::ArgAction::SetTrue),
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

    // Parse other configuration
    let datadir = PathBuf::from(matches.get_one::<String>("datadir").unwrap());
    let rpc_port: u16 = matches
        .get_one::<String>("rpcport")
        .unwrap()
        .parse()
        .map_err(|_| "Invalid RPC port")?;
    let rpc_bind = matches.get_one::<String>("rpcbind").unwrap().clone();
    let listen_addr = matches.get_one::<String>("listen").unwrap().clone();
    let connect_peers: Vec<String> = matches
        .get_many::<String>("connect")
        .unwrap_or_default()
        .cloned()
        .collect();
    let daemon = matches.get_flag("daemon");
    let enable_mining = matches.get_flag("mine");

    // Create node configuration with network-specific RPC rate limits
    let mut rpc_config = RpcConfig::for_network(network);
    rpc_config.bind_address = rpc_bind;
    rpc_config.port = rpc_port;
    rpc_config.enable_auth = false; // Disable auth for local testing

    let config = NodeConfig {
        network,
        datadir,
        rpc: rpc_config,
        max_peers: 8,
        enable_mining,
        listen_addr,
        connect_peers,
        daemon,
    };

    // Create and start node
    let node = Node::new(config).await?;
    node.start().await?;

    Ok(())
}
