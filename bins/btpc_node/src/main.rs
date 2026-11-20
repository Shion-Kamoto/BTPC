//! BTPC Full Node
//!
//! A complete Bitcoin-compatible full node implementation with quantum-resistant features.

#![allow(dead_code)]

use std::{path::PathBuf, sync::Arc};

use btpc_core::{
    consensus::{BlockValidator, TransactionValidator},
    crypto::Hash,
    network::{discovery::PeerDiscovery, sync::SyncManager, PeerEvent, SimplePeerManager},
    rpc::{
        handlers::BlockchainRpcHandlers,
        server::{RpcConfig, RpcServer},
    },
    storage::{
        blockchain_db::{BlockchainDatabase, BlockchainDb},
        database::{Database, DatabaseConfig},
        utxo_db::{UTXODatabase, UtxoDb},
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
    /// Listen address for P2P (None = use the network-specific default)
    pub listen_addr: Option<String>,
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
            listen_addr: None,
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
    mempool: Arc<std::sync::RwLock<btpc_core::mempool::Mempool>>,
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

                println!(
                    "Initializing blockchain with genesis block for {:?}...",
                    config.network
                );

                // Try to load genesis block from file first
                let genesis_path = config.datadir.join("genesis.json");
                let genesis_block = if genesis_path.exists() {
                    println!("Loading genesis block from {:?}", genesis_path);
                    use btpc_core::blockchain::Block;

                    let genesis_json = std::fs::read_to_string(&genesis_path)
                        .expect("Failed to read genesis.json");
                    let genesis_value: serde_json::Value =
                        serde_json::from_str(&genesis_json).expect("Failed to parse genesis.json");
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
                    println!(
                        "No genesis.json found, using well-known genesis for {:?}",
                        config.network
                    );
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

        // Create network config based on the BTPC network type. Each network
        // defines its own P2P listen address + port:
        //   mainnet [::]:18341, testnet [::]:18351, regtest [::]:18361.
        let mut network_config = match config.network {
            Network::Mainnet => btpc_core::network::NetworkConfig::mainnet(),
            Network::Testnet => btpc_core::network::NetworkConfig::testnet(),
            Network::Regtest => btpc_core::network::NetworkConfig::regtest(),
        };

        // --listen is optional: only override the network default when the user
        // explicitly provided an address. This fixes the previous bug where the
        // hardcoded default "0.0.0.0:8333" forced every network onto Bitcoin's
        // P2P port (and regtest was mis-mapped to testnet's config).
        if let Some(addr_str) = &config.listen_addr {
            match addr_str.parse::<std::net::SocketAddr>() {
                Ok(addr) => network_config.listen_addr = addr,
                Err(_) => eprintln!(
                    "⚠️  Invalid --listen address '{}', using network default {}",
                    addr_str, network_config.listen_addr
                ),
            }
        }

        let peer_manager = Arc::new(SimplePeerManager::new(
            network_config,
            Arc::clone(&block_height),
        ));

        // BTC-standard peers.dat equivalent: open a persistent, RocksDB-backed
        // addrman so outbound selection and failure eviction survive restarts.
        // Falls back to in-memory if the DB cannot be opened.
        let addrman_path = config.datadir.join("addrman");
        match btpc_core::network::address_manager::AddressManager::open_at(&addrman_path) {
            Ok(am) => peer_manager.install_address_manager(am).await,
            Err(e) => eprintln!(
                "⚠️  Could not open persistent addrman at {} (in-memory fallback): {}",
                addrman_path.display(),
                e
            ),
        }

        // Initialize validators
        let block_validator = Arc::new(BlockValidator::new());
        let tx_validator = Arc::new(TransactionValidator::new());

        // Initialize mempool
        let mempool = Arc::new(std::sync::RwLock::new(btpc_core::mempool::Mempool::new()));

        // Initialize RPC server
        let rpc_server = Arc::new(RpcServer::new_validated(config.rpc.clone())?);

        // Register RPC handlers
        let blockchain_handlers = BlockchainRpcHandlers::new(
            Arc::clone(&blockchain_db),
            Arc::clone(&utxo_db),
            Arc::clone(&mempool),
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

            // Save address book on clean shutdown
            let peers_dat = self.config.datadir.join("peers.dat");
            self.peer_manager.save_peers(&peers_dat).await;
        }

        Ok(())
    }

    /// Start P2P networking
    async fn start_networking(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "Starting P2P networking on {}",
            self.config
                .listen_addr
                .as_deref()
                .unwrap_or("(network default)")
        );

        // ── Bitcoin-style bootstrap ──────────────────────────────────────────

        // 1. Load peers.dat — reconnect to previously-known peers instantly
        //    without waiting for DNS resolution on every restart.
        let peers_dat = self.config.datadir.join("peers.dat");
        self.peer_manager.load_peers(&peers_dat).await;

        // 2. Start accepting inbound connections
        self.peer_manager.start_listening().await?;

        // 3. Query DNS seeds via the peer manager's internal address book
        //    (replaces the old separate peer_discovery field)
        self.peer_manager
            .discovery
            .write()
            .await
            .query_dns_seeds()
            .await
            .ok(); // Best effort

        // 4. Connect to any explicitly-specified peers (--connect flag)
        for peer_addr_str in &self.config.connect_peers {
            if let Ok(peer_addr) = peer_addr_str.parse() {
                println!("Connecting to peer: {}", peer_addr);
                self.peer_manager.connect_to_peer(peer_addr).await.ok();
            } else {
                eprintln!("Invalid peer address: {}", peer_addr_str);
            }
        }

        // 5. Auto-connection manager — maintains 8 outbound peers, re-queries
        //    DNS when isolated.  Mirrors Bitcoin Core's ThreadOpenConnections.
        SimplePeerManager::start_connection_manager(Arc::clone(&self.peer_manager));

        // 6. Auto-save peers.dat every 15 minutes
        SimplePeerManager::start_peer_saver(Arc::clone(&self.peer_manager), peers_dat);

        // ────────────────────────────────────────────────────────────────────

        Ok(())
    }

    /// Start P2P event handler to process blocks from peers
    async fn start_peer_event_handler(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting P2P event handler...");

        // Take the event receiver from peer manager
        let mut event_rx = self
            .peer_manager
            .take_event_receiver()
            .await
            .ok_or("Event receiver already taken")?;

        let blockchain_db = Arc::clone(&self.blockchain_db);
        let utxo_db = Arc::clone(&self.utxo_db);
        let block_validator = Arc::clone(&self.block_validator);
        let block_height = Arc::clone(&self.block_height);
        let mempool = Arc::clone(&self.mempool);

        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match event {
                    PeerEvent::PeerConnected { addr, height, .. } => {
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
                                    eprintln!(
                                        "Failed to store block {}: {}",
                                        block_hash.to_hex(),
                                        e
                                    );
                                } else {
                                    println!(
                                        "✅ Block {} stored successfully",
                                        block_hash.to_hex()
                                    );

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
                                eprintln!(
                                    "Invalid block {} from {}: {}",
                                    block_hash.to_hex(),
                                    from,
                                    e
                                );
                            }
                        }
                    }
                    PeerEvent::TransactionReceived { from, tx } => {
                        let tx_hash = tx.hash();
                        println!("💳 Received transaction {} from {}", tx_hash.to_hex(), from);

                        // Calculate fee (input value - output value), matching the
                        // desktop node's sendrawtransaction path.
                        let fee = {
                            let utxo = utxo_db.read().await;
                            let mut input_value: u64 = 0;
                            for input in &tx.inputs {
                                if let Ok(Some(u)) = utxo.get_utxo(&input.previous_output) {
                                    input_value = input_value.saturating_add(u.output.value);
                                }
                            }
                            let output_value: u64 = tx.outputs.iter().map(|o| o.value).sum();
                            input_value.saturating_sub(output_value)
                        };

                        // Add to fee-tracking mempool (matches desktop node)
                        match mempool.write() {
                            Ok(mut mp) => {
                                if let Err(e) = mp.add_transaction(tx, fee) {
                                    eprintln!(
                                        "Failed to add transaction {} to mempool: {}",
                                        tx_hash.to_hex(),
                                        e
                                    );
                                } else {
                                    println!("✅ Transaction {} added to mempool", tx_hash.to_hex());
                                }
                            }
                            Err(e) => eprintln!("Mempool lock poisoned: {}", e),
                        }
                    }
                    PeerEvent::InventoryReceived { from: _, inv: _ } => {
                        // Inventory is already logged by SimplePeerManager
                    }
                    PeerEvent::HeadersReceived { from, headers } => {
                        println!(
                            "📜 Received {} header(s) from {} (headless node — no catch-up impl)",
                            headers.len(),
                            from
                        );
                    }
                    PeerEvent::GetHeadersRequested { from, locator, .. } => {
                        println!(
                            "📖 Peer {} requested headers (locator len={}) — headless node has no BlockSource, ignoring",
                            from,
                            locator.len()
                        );
                    }
                    PeerEvent::GetDataRequested { from, inv } => {
                        println!(
                            "📥 Peer {} requested {} object(s) — headless node has no BlockSource, ignoring",
                            from,
                            inv.len()
                        );
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
                blockchain::merkle::calculate_merkle_root,
                blockchain::{
                    Block, BlockHeader, OutPoint, Transaction, TransactionInput, TransactionOutput,
                },
                consensus::pow::{MiningTarget, ProofOfWork},
                crypto::{Hash, Script},
                economics::rewards::RewardCalculator,
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
                        format!("Block {} mined by btpc_node", height)
                            .as_bytes()
                            .to_vec(),
                    ),
                    sequence: 0xffffffff,
                };

                let coinbase_output = TransactionOutput {
                    value: reward,
                    script_pubkey: Script::from_bytes(
                        format!("miner_address_{}", height).as_bytes().to_vec(),
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
                        println!("   Height: {}, Reward: {} credits", height, reward);

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
                            for (vout, output) in
                                mined_block.transactions[0].outputs.iter().enumerate()
                            {
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
                        println!(
                            "✅ Block {} added to blockchain (total mined: {})",
                            height, block_count
                        );

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

    /// Add a transaction to the mempool (computing fee from the UTXO set)
    pub fn add_transaction_to_mempool(
        &self,
        tx: btpc_core::blockchain::Transaction,
    ) -> Result<(), String> {
        let fee = match self.utxo_db.try_read() {
            Ok(db) => {
                let mut input_value: u64 = 0;
                for input in &tx.inputs {
                    if let Ok(Some(u)) = db.get_utxo(&input.previous_output) {
                        input_value = input_value.saturating_add(u.output.value);
                    }
                }
                let output_value: u64 = tx.outputs.iter().map(|o| o.value).sum();
                input_value.saturating_sub(output_value)
            }
            Err(_) => 0,
        };
        match self.mempool.write() {
            Ok(mut mp) => mp
                .add_transaction(tx, fee)
                .map(|_| ())
                .map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Get transactions from mempool for mining
    pub fn get_mempool_transactions(
        &self,
        max_count: usize,
    ) -> Vec<btpc_core::blockchain::Transaction> {
        match self.mempool.read() {
            Ok(mp) => mp
                .get_transactions_by_fee(max_count)
                .into_iter()
                .map(|e| e.transaction.clone())
                .collect(),
            Err(_) => Vec::new(),
        }
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

/// Expand a leading `~/` into the user's home directory.
fn expand_tilde(path: &std::path::Path) -> PathBuf {
    if let Some(rest) = path.to_str().and_then(|s| s.strip_prefix("~/")) {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    path.to_path_buf()
}

/// One-command setup: generate a self-signed TLS certificate + key, random RPC
/// credentials, and a complete rig config file (with reward address + rig name).
fn run_setup(
    datadir: &std::path::Path,
    rpc_bind: &str,
    rpc_port: u16,
    setup_hosts: &[String],
    address: Option<&str>,
    rig_name: Option<&str>,
    network: Network,
) -> Result<(), Box<dyn std::error::Error>> {
    use rand::distributions::{Alphanumeric, DistString};

    let datadir = expand_tilde(datadir);
    std::fs::create_dir_all(&datadir)?;

    let cert_path = datadir.join("rpc.crt");
    let key_path = datadir.join("rpc.key");

    // Subject alt names: localhost + loopback + any user-specified hosts/IPs.
    let mut san = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ];
    for host in setup_hosts {
        if !san.contains(host) {
            san.push(host.clone());
        }
    }

    // Generate the self-signed certificate (no openssl required).
    let rcgen::CertifiedKey { cert, key_pair } = rcgen::generate_simple_self_signed(san)?;
    std::fs::write(&cert_path, cert.pem())?;
    std::fs::write(&key_path, key_pair.serialize_pem())?;

    // Random credentials (username >= 8 chars, password >= 16 chars).
    let username = Alphanumeric.sample_string(&mut rand::thread_rng(), 10);
    let password = Alphanumeric.sample_string(&mut rand::thread_rng(), 24);

    let url_host = if rpc_bind == "0.0.0.0" || rpc_bind == "::" {
        "localhost".to_string()
    } else {
        rpc_bind.to_string()
    };

    let network_type = match network {
        Network::Mainnet => "mainnet",
        Network::Testnet => "testnet",
        Network::Regtest => "regtest",
    };

    // Reward address falls back to a placeholder so the file is always valid,
    // but we warn when the address (the one thing only the user knows) wasn't
    // supplied.
    let mining_address = address.unwrap_or("1YourBTPCAddressHere");

    // Rig name: use --name, otherwise generate a unique default so several rigs
    // are distinguishable without any extra input.
    let rig_name = match rig_name {
        Some(n) => n.to_string(),
        None => format!(
            "rig-{}",
            Alphanumeric.sample_string(&mut rand::thread_rng(), 6).to_lowercase()
        ),
    };

    let cert_path_str = cert_path.display().to_string();

    // Write a complete, ready-to-use rig config (matches the remote-node.toml
    // layout in the BTPC_RIG config/ directory).
    let rig_config = format!(
        r#"# BTPC Mining Rig — remote node configuration
# Generated by `btpc_node --setup`. Ready to use.

rig_name = "{rig_name}"

[network]
network_type = "{network_type}"
node_mode = "remote"
remote_rpc_url = "https://{url_host}:{rpc_port}"
rpc_username = "{username}"
rpc_password = "{password}"
tls_ca_cert = "{cert_path_str}"
data_dir = "/var/lib/btpc"

[mining]
mining_address = "{mining_address}"
coinbase_message = "{rig_name}"

[gpu]
enabled = true
device_ids = []
thermal_limit = 85.0

[logging]
level = "info"

[watchdog]
enabled = true
socket_path = "/run/btpc-miner.sock"
reboot_on_gpu_hang = false
"#,
        network_type = network_type,
        url_host = url_host,
        rpc_port = rpc_port,
        username = username,
        password = password,
        cert_path_str = cert_path_str,
        mining_address = mining_address,
        rig_name = rig_name,
    );

    let rig_config_path = datadir.join("rig-config.toml");
    std::fs::write(&rig_config_path, rig_config)?;

    println!();
    println!("BTPC Node setup complete");
    println!("  certificate : {}", cert_path.display());
    println!("  private key : {}", key_path.display());
    println!("  rig config  : {}", rig_config_path.display());
    println!();
    println!("Start the node with:");
    println!(
        "  btpc_node --rpcuser {} --rpcpass {} --tlscert {} --tlskey {}",
        username,
        password,
        cert_path.display(),
        key_path.display()
    );
    println!();
    if address.is_none() {
        println!("NOTE: no --address was given, so the rig config still has a");
        println!("placeholder mining_address. Re-run with --address <wallet> or edit");
        println!("{} to set your reward deposit address.", rig_config_path.display());
        println!();
    }
    println!(
        "Copy {} to the mining rig and point the rig at it.",
        rig_config_path.display()
    );
    println!();

    Ok(())
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
                .help("RPC server port (defaults to the network port: 18340/18350/18360)"),
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
                .help("P2P listen address (defaults to the network port: 18341/18351/18361)"),
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
        .arg(
            Arg::new("rpcuser")
                .long("rpcuser")
                .value_name("USER")
                .help("RPC username (required; min 8 characters)"),
        )
        .arg(
            Arg::new("rpcpass")
                .long("rpcpass")
                .value_name("PASS")
                .help("RPC password (required; min 16 characters)"),
        )
        .arg(
            Arg::new("tlscert")
                .long("tlscert")
                .value_name("PATH")
                .help("TLS certificate file (PEM) — enables HTTPS RPC"),
        )
        .arg(
            Arg::new("tlskey")
                .long("tlskey")
                .value_name("PATH")
                .help("TLS private key file (PEM) — enables HTTPS RPC"),
        )
        .arg(
            Arg::new("setup")
                .long("setup")
                .help("Generate TLS cert + credentials and print ready-to-use config")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("setup-host")
                .long("setup-host")
                .value_name("HOST")
                .help("Extra hostname/IP to include in the generated cert (repeatable)")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("address")
                .long("address")
                .value_name("ADDRESS")
                .help("BTPC wallet address for block reward deposits (goes in rig config)"),
        )
        .arg(
            Arg::new("name")
                .long("name")
                .value_name("NAME")
                .help("Rig name (embedded in the coinbase message)"),
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
    let rpc_port: Option<u16> = matches
        .get_one::<String>("rpcport")
        .map(|s| s.parse::<u16>().map_err(|_| "Invalid RPC port"))
        .transpose()?;
    // Effective RPC port: the network-specific default (mainnet 18340, testnet 18350,
    // regtest 18360) unless overridden via --rpcport.
    let rpc_port = rpc_port.unwrap_or_else(|| RpcConfig::for_network(network).port);
    let rpc_bind = matches.get_one::<String>("rpcbind").unwrap().clone();

    // One-command setup: generate cert + credentials + rig config, then exit.
    if matches.get_flag("setup") {
        let setup_hosts: Vec<String> = matches
            .get_many::<String>("setup-host")
            .unwrap_or_default()
            .cloned()
            .collect();
        let address = matches.get_one::<String>("address").map(|s| s.as_str());
        let name = matches.get_one::<String>("name").map(|s| s.as_str());
        run_setup(&datadir, &rpc_bind, rpc_port, &setup_hosts, address, name, network)?;
        return Ok(());
    }

    let listen_addr: Option<String> = matches.get_one::<String>("listen").cloned();
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

    // RPC authentication is mandatory — the node refuses to start without
    // valid credentials (enforced by RpcServer::new_validated).
    rpc_config.enable_auth = true;
    rpc_config.username = matches.get_one::<String>("rpcuser").cloned();
    rpc_config.password = matches.get_one::<String>("rpcpass").cloned();

    // TLS is optional: enabled when cert/key paths are provided
    // (new_validated requires BOTH once enabled).
    let tls_cert = matches.get_one::<String>("tlscert").cloned();
    let tls_key = matches.get_one::<String>("tlskey").cloned();
    rpc_config.enable_tls = tls_cert.is_some() || tls_key.is_some();
    rpc_config.tls_cert_path = tls_cert;
    rpc_config.tls_key_path = tls_key;

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
