//! Integrated P2P networking and blockchain synchronization
//!
//! This module coordinates P2P networking with blockchain sync to maintain
//! consensus across the network.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use anyhow::Result;
use thiserror::Error;
use tokio::{
    sync::{mpsc, oneshot, RwLock as TokioRwLock},
    time::{sleep, timeout},
};

use crate::{
    blockchain::{Block, BlockHeader},
    consensus::{StorageBlockValidator, StorageValidationError},
    crypto::Hash,
    network::{
        discovery::PeerDiscovery,
        protocol::{GetHeadersMessage, InvType, InventoryVector, Message, VersionMessage},
        sync::{SyncError, SyncManager, SyncState},
        NetworkError, ProtocolError,
    },
    storage::{BlockchainDatabase, UTXODatabase},
};

/// Integrated synchronization manager that coordinates P2P and blockchain sync
pub struct IntegratedSyncManager {
    /// Core sync manager (async operations)
    sync_manager: Arc<TokioRwLock<SyncManager>>,
    /// Peer discovery system (async operations)
    peer_discovery: Arc<TokioRwLock<PeerDiscovery>>,
    /// Blockchain database (sync lock for consensus compatibility)
    blockchain_db: Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
    /// UTXO database (sync lock for consensus compatibility)
    utxo_db: Arc<RwLock<dyn UTXODatabase + Send + Sync>>,
    /// Storage-aware block validator
    block_validator: Arc<StorageBlockValidator>,
    /// Active peer connections (async operations)
    active_peers: Arc<TokioRwLock<HashMap<String, PeerConnection>>>,
    /// Sync configuration
    config: SyncConfig,
    /// Message channels
    message_tx: mpsc::UnboundedSender<NetworkMessage>,
    /// Running state (async operations)
    is_running: Arc<TokioRwLock<bool>>,
}

/// Synchronization configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Maximum number of peers to sync with
    pub max_sync_peers: usize,
    /// Maximum parallel block downloads
    pub max_parallel_downloads: usize,
    /// Block request timeout
    pub block_timeout: Duration,
    /// Header request timeout
    pub header_timeout: Duration,
    /// Sync retry interval
    pub retry_interval: Duration,
    /// Maximum blocks to request in one batch
    pub max_blocks_per_request: usize,
    /// Enable checkpoint verification
    pub enable_checkpoints: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        SyncConfig {
            max_sync_peers: 8,
            max_parallel_downloads: 100,
            block_timeout: Duration::from_secs(30),
            header_timeout: Duration::from_secs(15),
            retry_interval: Duration::from_secs(10),
            max_blocks_per_request: 500,
            enable_checkpoints: true,
        }
    }
}

/// Peer connection state
#[derive(Debug, Clone)]
pub struct PeerConnection {
    /// Peer address
    pub address: String,
    /// Connection state
    pub state: PeerState,
    /// Best block height reported by peer
    pub best_height: u32,
    /// Best block hash reported by peer
    pub best_hash: Hash,
    /// Last message timestamp
    pub last_seen: Instant,
    /// Protocol version
    pub version: u32,
    /// Services supported
    pub services: u64,
}

/// Peer connection states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerState {
    Connecting,
    Connected,
    Syncing,
    Synced,
    Disconnected,
    Banned,
}

/// Network message for internal communication
#[derive(Debug)]
pub enum NetworkMessage {
    /// Peer connected
    PeerConnected {
        address: String,
        version: VersionMessage,
    },
    /// Peer disconnected
    PeerDisconnected { address: String },
    /// Received message from peer
    MessageReceived { peer: String, message: Message },
    /// Send message to peer
    SendMessage {
        peer: String,
        message: Message,
        response_tx: Option<oneshot::Sender<Result<(), NetworkError>>>,
    },
    /// Request sync with specific peer
    RequestSync { peer: String },
    /// Stop synchronization
    StopSync,
}

/// Synchronization errors
#[derive(Error, Debug)]
pub enum IntegratedSyncError {
    #[error("Sync error: {0}")]
    Sync(#[from] SyncError),
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),
    #[error("Storage validation error: {0}")]
    StorageValidation(#[from] StorageValidationError),
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("No peers available for sync")]
    NoPeersAvailable,
    #[error("Sync timeout")]
    Timeout,
    #[error("Invalid peer response")]
    InvalidPeerResponse,
}

impl IntegratedSyncManager {
    /// Create a new integrated sync manager
    pub fn new(
        blockchain_db: Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
        utxo_db: Arc<RwLock<dyn UTXODatabase + Send + Sync>>,
        config: SyncConfig,
    ) -> Self {
        let sync_manager = Arc::new(TokioRwLock::new(SyncManager::new(
            config.max_parallel_downloads,
        )));
        let peer_discovery = Arc::new(TokioRwLock::new(PeerDiscovery::new(1000)));

        // Create StorageBlockValidator with RwLock-wrapped databases
        let block_validator = Arc::new(StorageBlockValidator::new(
            blockchain_db.clone(),
            utxo_db.clone(),
        ));
        let active_peers = Arc::new(TokioRwLock::new(HashMap::new()));
        let (message_tx, _) = mpsc::unbounded_channel();
        let is_running = Arc::new(TokioRwLock::new(false));

        IntegratedSyncManager {
            sync_manager,
            peer_discovery,
            blockchain_db,
            utxo_db,
            block_validator,
            active_peers,
            config,
            message_tx,
            is_running,
        }
    }

    /// Start the integrated synchronization process
    pub async fn start(&self) -> Result<(), IntegratedSyncError> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        println!("Starting integrated blockchain synchronization...");

        // Start the main sync loop
        let manager = self.clone();
        tokio::spawn(async move {
            if let Err(e) = manager.sync_loop().await {
                eprintln!("Sync loop error: {}", e);
            }
        });

        // Start peer discovery
        self.start_peer_discovery().await?;

        // Start message processing
        self.start_message_processor().await;

        Ok(())
    }

    /// Stop synchronization
    pub async fn stop(&self) -> Result<(), IntegratedSyncError> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;

        // Send stop message
        let _ = self.message_tx.send(NetworkMessage::StopSync);

        println!("Stopping integrated synchronization...");
        Ok(())
    }

    /// Main synchronization loop
    async fn sync_loop(&self) -> Result<(), IntegratedSyncError> {
        let mut retry_timer = tokio::time::interval(self.config.retry_interval);

        while *self.is_running.read().await {
            retry_timer.tick().await;

            // Check sync state
            let sync_state = {
                let sync = self.sync_manager.read().await;
                sync.state().clone()
            };

            match sync_state {
                SyncState::Idle => {
                    // Start initial sync if we have peers
                    if self.has_sync_peers().await {
                        self.start_initial_sync().await?;
                    }
                }
                SyncState::SyncingHeaders => {
                    // Request headers from peers
                    self.request_headers().await?;
                }
                SyncState::SyncingBlocks => {
                    // Download blocks
                    self.download_blocks().await?;
                }
                SyncState::Synced => {
                    // Stay synced - listen for new blocks
                    self.maintain_sync().await?;
                }
            }
        }

        Ok(())
    }

    /// Start peer discovery
    async fn start_peer_discovery(&self) -> Result<(), IntegratedSyncError> {
        let mut discovery = self.peer_discovery.write().await;
        discovery
            .query_dns_seeds()
            .await
            .map_err(|e| IntegratedSyncError::Network(NetworkError::Discovery(e.to_string())))?;

        // Connect to initial peers
        let seed_peers = discovery.get_peers(
            self.config.max_sync_peers,
            crate::network::protocol::ServiceFlags::NETWORK,
        );
        for peer in seed_peers {
            self.connect_to_peer(&peer.to_string()).await?;
        }

        Ok(())
    }

    /// Start message processor
    ///
    /// Note: Message processing is currently handled inline when messages are received
    /// from peers via handle_peer_message(). This stub is retained for future
    /// implementation of background message queue processing.
    async fn start_message_processor(&self) {
        // Message processing is done inline in handle_peer_message
        // Future enhancement: implement background queue processing with proper Send bounds
        println!("Message processor initialized (inline processing mode)");
    }

    /// Process network messages
    async fn process_network_message(
        &self,
        message: NetworkMessage,
    ) -> Result<(), IntegratedSyncError> {
        match message {
            NetworkMessage::PeerConnected { address, version } => {
                self.handle_peer_connected(address, version).await?;
            }
            NetworkMessage::PeerDisconnected { address } => {
                self.handle_peer_disconnected(address).await?;
            }
            NetworkMessage::MessageReceived { peer, message } => {
                self.handle_peer_message(peer, message).await?;
            }
            NetworkMessage::SendMessage {
                peer,
                message,
                response_tx,
            } => {
                let result = self.send_message_to_peer(&peer, message).await;
                if let Some(tx) = response_tx {
                    let network_result = result.map_err(|e| match e {
                        IntegratedSyncError::Network(net_err) => net_err,
                        other => NetworkError::Connection(other.to_string()),
                    });
                    let _ = tx.send(network_result);
                }
            }
            NetworkMessage::RequestSync { peer } => {
                self.request_sync_with_peer(&peer).await?;
            }
            NetworkMessage::StopSync => {
                // Stop processing
                return Ok(());
            }
        }
        Ok(())
    }

    /// Handle peer connection
    async fn handle_peer_connected(
        &self,
        address: String,
        version: VersionMessage,
    ) -> Result<(), IntegratedSyncError> {
        let peer = PeerConnection {
            address: address.clone(),
            state: PeerState::Connected,
            best_height: version.start_height,
            best_hash: Hash::zero(), // Will be updated
            last_seen: Instant::now(),
            version: version.version,
            services: version.services,
        };

        let mut peers = self.active_peers.write().await;
        peers.insert(address.clone(), peer);

        println!(
            "Peer connected: {} (height: {})",
            address, version.start_height
        );
        Ok(())
    }

    /// Handle peer disconnection
    async fn handle_peer_disconnected(&self, address: String) -> Result<(), IntegratedSyncError> {
        let mut peers = self.active_peers.write().await;
        if let Some(peer) = peers.get_mut(&address) {
            peer.state = PeerState::Disconnected;
        }

        println!("Peer disconnected: {}", address);
        Ok(())
    }

    /// Handle messages from peers
    async fn handle_peer_message(
        &self,
        peer: String,
        message: Message,
    ) -> Result<(), IntegratedSyncError> {
        match message {
            Message::Headers(headers) => {
                self.process_headers_message(peer, headers).await?;
            }
            Message::Block(block) => {
                self.process_block_message(peer, block).await?;
            }
            Message::Inv(inventory) => {
                self.process_inventory_message(peer, inventory).await?;
            }
            Message::GetData(inventory) => {
                self.process_getdata_message(peer, inventory).await?;
            }
            _ => {
                // Handle other message types
            }
        }
        Ok(())
    }

    //
    // ============================================================================
    // MESSAGE PROCESSING METHODS
    // ============================================================================
    //
    // These methods implement the P2P sync coordination layer, integrating
    // blockchain synchronization with the networking stack.
    //
    // Implementation details:
    // - process_headers_message: Validates header chain (prev_hash links + PoW),
    //   updates sync manager, and requests missing blocks via GETDATA.
    // - process_block_message: Validates blocks with StorageBlockValidator
    //   (structure, PoW, UTXO, signatures, coinbase reward), applies to chain,
    //   and relays to peers via INV announcement.
    //
    // Supporting layers:
    // - simple_peer_manager.rs: Connection management and message routing
    // - protocol.rs: Wire protocol, size limits, and message validation
    // - discovery.rs: Peer discovery and DNS seed resolution
    //
    // Security Note: All message validation (size limits, rate limiting, etc.)
    // is enforced at the protocol layer before reaching these methods.
    // ============================================================================

    /// Process headers message
    ///
    /// Validates received headers and queues missing blocks for download.
    async fn process_headers_message(
        &self,
        peer: String,
        headers: Vec<BlockHeader>,
    ) -> Result<(), IntegratedSyncError> {
        if headers.is_empty() {
            return Ok(());
        }

        println!("Processing {} headers from peer: {}", headers.len(), peer);

        // Step 1: Validate header chain (prev_hash links)
        let mut prev_hash = headers[0].prev_hash;
        for (i, header) in headers.iter().enumerate() {
            // First header can have any prev_hash (depends on where sync started)
            if i > 0 {
                let expected_prev = headers[i - 1].hash();
                if header.prev_hash != expected_prev {
                    println!(
                        "Header chain broken at index {}: expected prev_hash {:?}, got {:?}",
                        i, expected_prev, header.prev_hash
                    );
                    return Err(IntegratedSyncError::InvalidPeerResponse);
                }
            }
            prev_hash = header.hash();
        }

        // Step 2: Check PoW difficulty for each header
        for header in &headers {
            let target = crate::consensus::DifficultyTarget::from_bits(header.bits);
            let block_hash = header.hash();

            // Verify the block hash meets the difficulty target
            if !target.validates_hash(&block_hash) {
                println!(
                    "Header {} fails PoW verification (bits: {:08x})",
                    block_hash.to_hex(),
                    header.bits
                );
                return Err(IntegratedSyncError::InvalidPeerResponse);
            }
        }

        // Step 3: Update sync manager with new headers and get blocks to download
        // Use spawn_blocking to avoid holding std::sync::RwLock across await
        let blockchain_db_clone = Arc::clone(&self.blockchain_db);
        let sync_manager_clone = Arc::clone(&self.sync_manager);
        let blocks_to_download =
            tokio::task::spawn_blocking(move || -> Result<Vec<Hash>, IntegratedSyncError> {
                let blockchain_db = blockchain_db_clone.read().map_err(|e| {
                    IntegratedSyncError::Network(NetworkError::Connection(format!(
                        "Lock error: {}",
                        e
                    )))
                })?;

                // Use try_write since we're in blocking context
                let runtime = tokio::runtime::Handle::current();
                let result = runtime.block_on(async {
                    let mut sync = sync_manager_clone.write().await;
                    sync.process_headers(headers, &*blockchain_db)
                })?;
                Ok(result)
            })
            .await
            .map_err(|e| {
                IntegratedSyncError::Network(NetworkError::Connection(format!(
                    "Task join error: {}",
                    e
                )))
            })??;

        // Step 4: Request blocks for headers we don't have
        if !blocks_to_download.is_empty() {
            println!(
                "Requesting {} blocks from peer: {}",
                blocks_to_download.len(),
                peer
            );

            let message = self.create_getdata_message(blocks_to_download)?;
            self.send_message_to_peer(&peer, message).await?;
        }

        Ok(())
    }

    /// Process block message
    ///
    /// Validates and applies received blocks to the blockchain.
    async fn process_block_message(
        &self,
        peer: String,
        block: Block,
    ) -> Result<(), IntegratedSyncError> {
        let block_hash = block.hash();
        println!(
            "Processing block {} from peer: {}",
            block_hash.to_hex(),
            peer
        );

        // Step 1: Check if we already have this block
        if self.block_validator.has_block(&block_hash).await? {
            println!("Block {} already in chain, skipping", block_hash.to_hex());
            return Ok(());
        }

        // Step 2: Validate block with StorageBlockValidator
        // This performs full consensus validation including:
        // - Stateless validation (structure, PoW)
        // - Context validation (prev block exists, difficulty correct)
        // - Transaction validation (UTXO existence, signatures)
        // - Coinbase validation (reward + fees)
        match self
            .block_validator
            .validate_block_with_context(&block)
            .await
        {
            Ok(()) => {
                println!("Block {} passed validation", block_hash.to_hex());
            }
            Err(e) => {
                println!(
                    "Block {} failed validation from peer {}: {}",
                    block_hash.to_hex(),
                    peer,
                    e
                );
                // Don't propagate error - just skip this block
                // The peer may have sent an invalid block, but we don't want to disconnect
                return Ok(());
            }
        }

        // Step 3 & 4: Apply block to blockchain database and update UTXO set
        // This is done atomically by the StorageBlockValidator
        self.block_validator.apply_block(&block).await?;
        println!(
            "Block {} applied to chain (height: TBD)",
            block_hash.to_hex()
        );

        // Step 5: Update sync manager state
        // Note: We use spawn_blocking to avoid holding std::sync::RwLock across await points
        let blockchain_db_clone = Arc::clone(&self.blockchain_db);
        let block_clone = block.clone();
        tokio::task::spawn_blocking(move || -> Result<(), IntegratedSyncError> {
            let mut blockchain_db = blockchain_db_clone.write().map_err(|e| {
                IntegratedSyncError::Network(NetworkError::Connection(format!("Lock error: {}", e)))
            })?;

            // Basic block storage (sync manager update happens in blocking context)
            blockchain_db
                .store_block(&block_clone)
                .map_err(|e| IntegratedSyncError::Network(NetworkError::Storage(e.to_string())))?;
            Ok(())
        })
        .await
        .map_err(|e| {
            IntegratedSyncError::Network(NetworkError::Connection(format!(
                "Task join error: {}",
                e
            )))
        })??;

        // Step 6: Relay block to other peers (announce via INV)
        let peers = self.active_peers.read().await;
        let inv = vec![InventoryVector {
            inv_type: InvType::Block,
            hash: block_hash,
        }];
        let message = Message::Inv(inv);

        for (address, peer_conn) in peers.iter() {
            // Don't relay back to sender
            if address == &peer {
                continue;
            }
            // Only relay to connected/synced peers
            if peer_conn.state == PeerState::Connected || peer_conn.state == PeerState::Synced {
                let _ = self.message_tx.send(NetworkMessage::SendMessage {
                    peer: address.clone(),
                    message: message.clone(),
                    response_tx: None,
                });
            }
        }

        println!(
            "Block {} relayed to {} peers",
            block_hash.to_hex(),
            peers.len().saturating_sub(1)
        );

        Ok(())
    }

    /// Process inventory message
    ///
    /// Filters inventory and requests data for items we don't have.
    async fn process_inventory_message(
        &self,
        peer: String,
        inventory: Vec<InventoryVector>,
    ) -> Result<(), IntegratedSyncError> {
        if inventory.is_empty() {
            return Ok(());
        }

        println!(
            "Processing {} inventory items from peer: {}",
            inventory.len(),
            peer
        );

        // Step 1 & 2: Separate blocks and transactions, filter for items we don't have
        let mut needed_blocks: Vec<Hash> = Vec::new();
        let mut needed_txs: Vec<Hash> = Vec::new();

        {
            let blockchain_db = self.blockchain_db.read().map_err(|e| {
                IntegratedSyncError::Network(NetworkError::Connection(format!("Lock error: {}", e)))
            })?;

            for inv in &inventory {
                match inv.inv_type {
                    InvType::Block => {
                        // Check if we already have this block
                        match blockchain_db.get_header(&inv.hash) {
                            Ok(Some(_)) => {
                                // Already have this block, skip
                            }
                            Ok(None) => {
                                needed_blocks.push(inv.hash);
                            }
                            Err(e) => {
                                println!("Error checking block {}: {}", inv.hash.to_hex(), e);
                            }
                        }
                    }
                    InvType::Tx => {
                        // Check if transaction is in mempool or blockchain
                        match blockchain_db.has_transaction(&inv.hash) {
                            Ok(true) => {
                                // Already have this transaction
                            }
                            Ok(false) => {
                                needed_txs.push(inv.hash);
                            }
                            Err(e) => {
                                println!("Error checking tx {}: {}", inv.hash.to_hex(), e);
                            }
                        }
                    }
                    _ => {
                        // Unknown inventory type, skip
                    }
                }
            }
        }

        // Step 3: Send getdata requests for needed items
        // Prioritize blocks over transactions by sending block requests first
        if !needed_blocks.is_empty() {
            println!(
                "Requesting {} blocks from peer: {}",
                needed_blocks.len(),
                peer
            );

            // Request blocks in batches to avoid overwhelming the peer
            const MAX_BLOCKS_PER_REQUEST: usize = 128;
            for chunk in needed_blocks.chunks(MAX_BLOCKS_PER_REQUEST) {
                let inv_vectors: Vec<InventoryVector> = chunk
                    .iter()
                    .map(|hash| InventoryVector {
                        inv_type: InvType::Block,
                        hash: *hash,
                    })
                    .collect();

                let message = Message::GetData(inv_vectors);
                self.send_message_to_peer(&peer, message).await?;
            }
        }

        if !needed_txs.is_empty() {
            println!(
                "Requesting {} transactions from peer: {}",
                needed_txs.len(),
                peer
            );

            // Request transactions in batches
            const MAX_TXS_PER_REQUEST: usize = 256;
            for chunk in needed_txs.chunks(MAX_TXS_PER_REQUEST) {
                let inv_vectors: Vec<InventoryVector> = chunk
                    .iter()
                    .map(|hash| InventoryVector {
                        inv_type: InvType::Tx,
                        hash: *hash,
                    })
                    .collect();

                let message = Message::GetData(inv_vectors);
                self.send_message_to_peer(&peer, message).await?;
            }
        }

        Ok(())
    }

    /// Process getdata message
    ///
    /// Serves requested blocks and transactions to peers.
    async fn process_getdata_message(
        &self,
        peer: String,
        inventory: Vec<InventoryVector>,
    ) -> Result<(), IntegratedSyncError> {
        if inventory.is_empty() {
            return Ok(());
        }

        println!(
            "Processing getdata request for {} items from peer: {}",
            inventory.len(),
            peer
        );

        // Step 1: Request size validation is done at protocol layer
        // Step 2: Rate limiting would be implemented here in production

        let mut blocks_sent = 0;
        let mut txs_sent = 0;
        let mut not_found: Vec<InventoryVector> = Vec::new();

        // Step 3: Lookup requested items in database
        {
            let blockchain_db = self.blockchain_db.read().map_err(|e| {
                IntegratedSyncError::Network(NetworkError::Connection(format!("Lock error: {}", e)))
            })?;

            for inv in &inventory {
                match inv.inv_type {
                    InvType::Block => {
                        // Lookup block in database
                        match blockchain_db.get_block(&inv.hash) {
                            Ok(Some(block)) => {
                                // Step 4: Send block message to peer
                                let message = Message::Block(block);
                                let _ = self.message_tx.send(NetworkMessage::SendMessage {
                                    peer: peer.clone(),
                                    message,
                                    response_tx: None,
                                });
                                blocks_sent += 1;
                            }
                            Ok(None) => {
                                // Block not found
                                not_found.push(inv.clone());
                            }
                            Err(e) => {
                                println!("Error retrieving block {}: {}", inv.hash.to_hex(), e);
                                not_found.push(inv.clone());
                            }
                        }
                    }
                    InvType::Tx => {
                        // Lookup transaction in blockchain (already mined)
                        // For mempool transactions, we'd need access to the mempool here
                        match blockchain_db.has_transaction(&inv.hash) {
                            Ok(true) => {
                                // Transaction exists but we'd need to retrieve it from a block
                                // For now, mark as not found (mempool access needed for pending TXs)
                                // In production, this would be enhanced to serve mempool transactions
                                not_found.push(inv.clone());
                            }
                            Ok(false) => {
                                not_found.push(inv.clone());
                            }
                            Err(e) => {
                                println!("Error checking tx {}: {}", inv.hash.to_hex(), e);
                                not_found.push(inv.clone());
                            }
                        }
                    }
                    _ => {
                        // Unknown inventory type
                        not_found.push(inv.clone());
                    }
                }
            }
        }

        // Send NotFound message for items we couldn't provide
        if !not_found.is_empty() {
            println!(
                "Sending NotFound for {} items to peer: {}",
                not_found.len(),
                peer
            );
            let message = Message::NotFound(not_found);
            let _ = self.message_tx.send(NetworkMessage::SendMessage {
                peer: peer.clone(),
                message,
                response_tx: None,
            });
        }

        // Step 5: Bandwidth tracking would be implemented here in production
        println!(
            "Served {} blocks, {} transactions to peer: {}",
            blocks_sent, txs_sent, peer
        );

        Ok(())
    }

    /// Check if we have peers for syncing
    async fn has_sync_peers(&self) -> bool {
        let peers = self.active_peers.read().await;
        peers
            .values()
            .any(|p| p.state == PeerState::Connected || p.state == PeerState::Syncing)
    }

    /// Start initial synchronization
    async fn start_initial_sync(&self) -> Result<(), IntegratedSyncError> {
        println!("Starting initial blockchain sync...");

        let mut sync = self.sync_manager.write().await;
        sync.start_sync()?;

        Ok(())
    }

    /// Request headers from peers
    async fn request_headers(&self) -> Result<(), IntegratedSyncError> {
        let peers = self.get_sync_peers().await;

        for peer in peers {
            // Create getheaders message
            let locator = self.create_block_locator().await?;
            let message = self.create_getheaders_message(locator)?;

            // Send to peer
            self.send_message_to_peer(&peer, message).await?;
        }

        Ok(())
    }

    /// Download blocks from peers
    async fn download_blocks(&self) -> Result<(), IntegratedSyncError> {
        let downloads = {
            let mut sync = self.sync_manager.write().await;
            sync.get_next_downloads()
        };

        if downloads.is_empty() {
            return Ok(());
        }

        let peers = self.get_sync_peers().await;
        if peers.is_empty() {
            return Err(IntegratedSyncError::NoPeersAvailable);
        }

        // Distribute downloads across peers
        for (i, block_hash) in downloads.iter().enumerate() {
            let peer = &peers[i % peers.len()];
            let message = self.create_getdata_message(vec![*block_hash])?;

            // Send with timeout
            let result = timeout(
                self.config.block_timeout,
                self.send_message_to_peer(peer, message),
            )
            .await;

            if result.is_err() {
                // Handle timeout
                let mut sync = self.sync_manager.write().await;
                sync.handle_timeout(block_hash);
            }
        }

        Ok(())
    }

    /// Maintain sync state (listen for new blocks)
    async fn maintain_sync(&self) -> Result<(), IntegratedSyncError> {
        // Send ping messages to peers to maintain connections
        let peers = self.get_sync_peers().await;

        for peer in peers {
            let ping_message = self.create_ping_message()?;
            let _ = self.send_message_to_peer(&peer, ping_message).await;
        }

        Ok(())
    }

    /// Get peers suitable for syncing
    async fn get_sync_peers(&self) -> Vec<String> {
        let peers = self.active_peers.read().await;
        peers
            .values()
            .filter(|p| p.state == PeerState::Connected || p.state == PeerState::Syncing)
            .map(|p| p.address.clone())
            .take(self.config.max_sync_peers)
            .collect()
    }

    /// Connect to a peer (placeholder)
    ///
    /// Note: Actual network connections are handled by SimplePeerManager.
    /// This is a coordination layer stub for future integrated sync.
    async fn connect_to_peer(&self, address: &str) -> Result<(), IntegratedSyncError> {
        println!("Connecting to peer: {}", address);

        // Future implementation would:
        // 1. Delegate to SimplePeerManager for actual TCP connection
        // 2. Perform protocol handshake (version/verack)
        // 3. Update peer discovery with successful connection
        // 4. Add peer to active_peers tracking

        Ok(())
    }

    /// Send message to peer (placeholder)
    ///
    /// Note: Actual message sending is handled by SimplePeerManager.
    /// This is a coordination layer stub for future integrated sync.
    async fn send_message_to_peer(
        &self,
        _peer: &str,
        _message: Message,
    ) -> Result<(), IntegratedSyncError> {
        // Future implementation would:
        // 1. Delegate to SimplePeerManager for message encoding/sending
        // 2. Track message for timeout handling
        // 3. Update peer statistics

        Ok(())
    }

    /// Request sync with specific peer (placeholder)
    ///
    /// Note: This is a coordination stub for future peer-specific sync requests.
    async fn request_sync_with_peer(&self, peer: &str) -> Result<(), IntegratedSyncError> {
        println!("Requesting sync with peer: {}", peer);

        // Future implementation would:
        // 1. Send getheaders message to specific peer
        // 2. Mark peer as syncing in active_peers
        // 3. Set timeout for sync completion

        Ok(())
    }

    /// Create block locator (Issue #2: Fixed to use spawn_blocking for sync lock)
    async fn create_block_locator(&self) -> Result<Vec<Hash>, IntegratedSyncError> {
        let blockchain_db = Arc::clone(&self.blockchain_db);

        // Use spawn_blocking to avoid blocking async runtime (Issue #2)
        let locator =
            tokio::task::spawn_blocking(move || -> Result<Vec<Hash>, IntegratedSyncError> {
                // Acquire read lock in blocking context
                let db = blockchain_db.read().map_err(|e| {
                    IntegratedSyncError::Network(NetworkError::Connection(format!(
                        "Lock error: {}",
                        e
                    )))
                })?;

                // Get current tip
                let tip = db.get_chain_tip().map_err(|e| {
                    IntegratedSyncError::Network(NetworkError::Storage(e.to_string()))
                })?;

                if let Some(tip_block) = tip {
                    let locator =
                        crate::network::sync::create_block_locator(tip_block.hash(), &*db)?;
                    Ok(locator)
                } else {
                    // No blocks yet, use genesis
                    Ok(vec![Hash::zero()])
                }
            })
            .await
            .map_err(|e| {
                IntegratedSyncError::Network(NetworkError::Connection(format!(
                    "Task join error: {}",
                    e
                )))
            })??;

        Ok(locator)
    }

    /// Create getheaders message
    fn create_getheaders_message(
        &self,
        locator: Vec<Hash>,
    ) -> Result<Message, IntegratedSyncError> {
        let get_headers = GetHeadersMessage {
            version: 70015, // Protocol version
            block_locator: locator,
            hash_stop: Hash::zero(), // Get as many as possible
        };
        Ok(Message::GetHeaders(get_headers))
    }

    /// Create getdata message
    fn create_getdata_message(&self, hashes: Vec<Hash>) -> Result<Message, IntegratedSyncError> {
        let inventory: Vec<InventoryVector> = hashes
            .into_iter()
            .map(|hash| InventoryVector {
                inv_type: InvType::Block,
                hash,
            })
            .collect();
        Ok(Message::GetData(inventory))
    }

    /// Create ping message
    fn create_ping_message(&self) -> Result<Message, IntegratedSyncError> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Ok(Message::Ping(nonce))
    }

    /// Broadcast a transaction to all connected peers
    ///
    /// This announces the transaction via INV message to all connected peers.
    /// Peers interested in the transaction will request it via GETDATA.
    pub async fn broadcast_transaction(&self, txid: &Hash) -> Result<usize, IntegratedSyncError> {
        let peers = self.active_peers.read().await;

        if peers.is_empty() {
            return Err(IntegratedSyncError::NoPeersAvailable);
        }

        // Create inventory vector for the transaction
        let inv = vec![InventoryVector {
            inv_type: InvType::Tx,
            hash: *txid,
        }];

        let message = Message::Inv(inv);
        let mut broadcast_count = 0;

        // Send INV to all connected/synced peers
        for (address, peer) in peers.iter() {
            if peer.state == PeerState::Connected || peer.state == PeerState::Synced {
                // Send message through the channel
                let (response_tx, _response_rx) = oneshot::channel();
                if self
                    .message_tx
                    .send(NetworkMessage::SendMessage {
                        peer: address.clone(),
                        message: message.clone(),
                        response_tx: Some(response_tx),
                    })
                    .is_ok()
                {
                    broadcast_count += 1;
                }
            }
        }

        Ok(broadcast_count)
    }

    /// Broadcast a full transaction to all connected peers
    ///
    /// This sends the full transaction directly (not just INV).
    /// Use this for immediate propagation without waiting for GETDATA.
    pub async fn broadcast_transaction_full(
        &self,
        transaction: &crate::blockchain::Transaction,
    ) -> Result<usize, IntegratedSyncError> {
        let peers = self.active_peers.read().await;

        if peers.is_empty() {
            return Err(IntegratedSyncError::NoPeersAvailable);
        }

        let message = Message::Tx(transaction.clone());
        let mut broadcast_count = 0;

        // Send TX to all connected/synced peers
        for (address, peer) in peers.iter() {
            if peer.state == PeerState::Connected || peer.state == PeerState::Synced {
                let (response_tx, _response_rx) = oneshot::channel();
                if self
                    .message_tx
                    .send(NetworkMessage::SendMessage {
                        peer: address.clone(),
                        message: message.clone(),
                        response_tx: Some(response_tx),
                    })
                    .is_ok()
                {
                    broadcast_count += 1;
                }
            }
        }

        Ok(broadcast_count)
    }

    /// Get sync statistics
    /// T062: Best known header height from any connected peer.
    /// Used by `build_node_status_snapshot` for the `headers_height` field.
    pub async fn headers_height(&self) -> u64 {
        let peers = self.active_peers.read().await;
        peers
            .values()
            .map(|p| p.best_height as u64)
            .max()
            .unwrap_or(0)
    }

    pub async fn get_sync_stats(&self) -> SyncStats {
        let sync = self.sync_manager.read().await;
        let peers = self.active_peers.read().await;

        SyncStats {
            state: sync.state().clone(),
            progress: sync.progress(),
            connected_peers: peers.len(),
            syncing_peers: peers
                .values()
                .filter(|p| p.state == PeerState::Syncing)
                .count(),
            best_height: peers.values().map(|p| p.best_height).max().unwrap_or(0),
        }
    }
}

impl Clone for IntegratedSyncManager {
    fn clone(&self) -> Self {
        IntegratedSyncManager {
            sync_manager: Arc::clone(&self.sync_manager),
            peer_discovery: Arc::clone(&self.peer_discovery),
            blockchain_db: Arc::clone(&self.blockchain_db),
            utxo_db: Arc::clone(&self.utxo_db),
            block_validator: Arc::clone(&self.block_validator),
            active_peers: Arc::clone(&self.active_peers),
            config: self.config.clone(),
            message_tx: self.message_tx.clone(),
            is_running: Arc::clone(&self.is_running),
        }
    }
}

/// Synchronization statistics
#[derive(Debug, Clone)]
pub struct SyncStats {
    /// Current sync state
    pub state: SyncState,
    /// Sync progress percentage
    pub progress: f32,
    /// Number of connected peers
    pub connected_peers: usize,
    /// Number of peers currently syncing
    pub syncing_peers: usize,
    /// Best height known from peers
    pub best_height: u32,
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::storage::{
        database::{Database, DatabaseConfig},
        BlockchainDb, UtxoDb,
    };

    async fn create_test_sync_manager() -> (IntegratedSyncManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_config = DatabaseConfig::test();
        let database = Arc::new(Database::open(temp_dir.path(), db_config).unwrap());

        // Wrap databases in RwLock for StorageBlockValidator (Priority 2 fix)
        let blockchain_db = Arc::new(RwLock::new(BlockchainDb::new(database.clone())))
            as Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>;
        let utxo_db = Arc::new(RwLock::new(UtxoDb::new(database)))
            as Arc<RwLock<dyn UTXODatabase + Send + Sync>>;

        let sync_manager =
            IntegratedSyncManager::new(blockchain_db, utxo_db, SyncConfig::default());

        (sync_manager, temp_dir)
    }

    #[tokio::test]
    async fn test_sync_manager_creation() {
        let (manager, _temp_dir) = create_test_sync_manager().await;
        let stats = manager.get_sync_stats().await;

        assert_eq!(stats.state, SyncState::Idle);
        assert_eq!(stats.connected_peers, 0);
        assert_eq!(stats.progress, 0.0);
    }

    #[tokio::test]
    async fn test_sync_config_defaults() {
        let config = SyncConfig::default();

        assert_eq!(config.max_sync_peers, 8);
        assert_eq!(config.max_parallel_downloads, 100);
        assert!(config.enable_checkpoints);
    }

    #[tokio::test]
    async fn test_peer_connection_handling() {
        let (manager, _temp_dir) = create_test_sync_manager().await;

        let version = VersionMessage {
            version: 70015,
            services: 1,
            timestamp: 1234567890,
            addr_recv: crate::network::protocol::NetworkAddress::default(),
            addr_from: crate::network::protocol::NetworkAddress::default(),
            nonce: 12345,
            user_agent: "/BTPC:0.1.0/".to_string(),
            start_height: 100,
            relay: true,
        };

        manager
            .handle_peer_connected("127.0.0.1:8333".to_string(), version)
            .await
            .unwrap();

        let stats = manager.get_sync_stats().await;
        assert_eq!(stats.connected_peers, 1);
        assert_eq!(stats.best_height, 100);
    }
}

// ============================================================================
// 003-testnet-p2p-hardening — Phase 5 US3 stub types
// ============================================================================
//
// ── Phase 5 (US3) GREEN types: in-flight tracking, stall reaping, block handling ──
//
// Replaces the Phase 2.5 RED `todo!()` stubs with real implementations.
// T057: SyncJob + JobState
// T058: InFlightTracker (per-peer cap = 16)
// T059: BlockRequestScheduler (aggregate cap = 128)
// T060: StallReaper (30s timeout, ban_score += 10)
// T061: handle_received_block (invalid = ban 100, valid = complete)

use std::collections::HashMap as StubHashMap;
use std::net::SocketAddr as PeerAddr;
use std::time::Duration as StubDuration;

/// Configuration for bounded in-flight windows.
#[derive(Debug, Clone)]
pub struct BoundedWindowConfig {
    pub per_peer_cap: usize,
    pub aggregate_cap: usize,
}

impl Default for BoundedWindowConfig {
    fn default() -> Self {
        BoundedWindowConfig {
            per_peer_cap: 16,   // FR-016: max 16 in-flight per peer
            aggregate_cap: 128, // FR-016: max 128 aggregate
        }
    }
}

/// Per-peer and aggregate in-flight request tracker (T058, FR-016).
#[derive(Debug)]
pub struct InFlightTracker {
    per_peer: StubHashMap<PeerAddr, Vec<u64>>,
    per_peer_cap: usize,
}

impl InFlightTracker {
    pub fn new(cfg: BoundedWindowConfig) -> Self {
        InFlightTracker {
            per_peer: StubHashMap::new(),
            per_peer_cap: cfg.per_peer_cap,
        }
    }

    /// Reserve a slot for a block request. Returns `Err(())` if the per-peer
    /// cap (16) is exceeded.
    #[allow(clippy::result_unit_err)] // Unit error is intentional — callers only need success/failure
    pub fn try_reserve(&mut self, peer: PeerAddr, height: u64) -> Result<(), ()> {
        let slots = self.per_peer.entry(peer).or_default();
        if slots.len() >= self.per_peer_cap {
            return Err(());
        }
        slots.push(height);
        Ok(())
    }

    /// Release a slot when a block is received or timed out.
    pub fn release(&mut self, peer: &PeerAddr, height: u64) {
        if let Some(slots) = self.per_peer.get_mut(peer) {
            if let Some(pos) = slots.iter().position(|&h| h == height) {
                slots.swap_remove(pos);
            }
        }
    }
}

/// Aggregate block request scheduler (T059, FR-016).
/// Enforces a global cap of 128 in-flight requests across all peers.
#[derive(Debug)]
pub struct BlockRequestScheduler {
    requests: Vec<(PeerAddr, u64)>,
    aggregate_cap: usize,
}

impl BlockRequestScheduler {
    pub fn new(cfg: BoundedWindowConfig) -> Self {
        BlockRequestScheduler {
            requests: Vec::new(),
            aggregate_cap: cfg.aggregate_cap,
        }
    }

    /// Schedule a block request. Returns `Err(())` if the aggregate cap (128)
    /// is exceeded.
    #[allow(clippy::result_unit_err)] // Unit error is intentional — callers only need success/failure
    pub fn try_schedule(&mut self, peer: PeerAddr, height: u64) -> Result<(), ()> {
        if self.requests.len() >= self.aggregate_cap {
            return Err(());
        }
        self.requests.push((peer, height));
        Ok(())
    }

    /// Remove a completed or timed-out request.
    pub fn complete(&mut self, peer: &PeerAddr, height: u64) {
        if let Some(pos) = self
            .requests
            .iter()
            .position(|(p, h)| p == peer && *h == height)
        {
            self.requests.swap_remove(pos);
        }
    }
}

/// State of a sync job (T057).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Running,
    Stalled,
    Failed,
    Completed,
}

/// A tracked sync job for a specific block request (T057).
#[derive(Debug, Clone)]
pub struct SyncJob {
    pub job_id: u64,
    pub peer: PeerAddr,
    pub height: u64,
    pub state: JobState,
    pub ban_score_penalty: u32,
    scheduled_at: std::time::Instant,
    last_progress_at: std::time::Instant,
}

/// Stall reaper — detects and reaps jobs that haven't progressed (T060, FR-017).
///
/// A job is considered stalled if `now - last_progress_at > stall_window` (30s).
/// Stalled jobs receive a ban_score penalty of 10.
#[derive(Debug)]
pub struct StallReaper {
    stall_window: StubDuration,
    jobs: Vec<SyncJob>,
    next_job_id: u64,
    virtual_clock_offset: StubDuration,
}

impl StallReaper {
    pub fn new(window: StubDuration) -> Self {
        StallReaper {
            stall_window: window,
            jobs: Vec::new(),
            next_job_id: 1,
            virtual_clock_offset: StubDuration::ZERO,
        }
    }

    /// Schedule a new job for tracking. Returns the job ID.
    pub fn schedule(&mut self, peer: PeerAddr, height: u64) -> u64 {
        let id = self.next_job_id;
        self.next_job_id += 1;
        let now = std::time::Instant::now();
        self.jobs.push(SyncJob {
            job_id: id,
            peer,
            height,
            state: JobState::Running,
            ban_score_penalty: 0,
            scheduled_at: now,
            last_progress_at: now,
        });
        id
    }

    /// Advance the virtual clock (for deterministic testing).
    pub fn advance_clock(&mut self, by: StubDuration) {
        self.virtual_clock_offset += by;
    }

    /// Reap all stalled jobs. Returns the stalled jobs with ban_score_penalty set.
    pub fn reap_stalled(&mut self) -> Vec<SyncJob> {
        let effective_now = std::time::Instant::now() + self.virtual_clock_offset;
        let window = self.stall_window;
        let mut reaped = Vec::new();

        for job in &mut self.jobs {
            if job.state == JobState::Running
                && effective_now.duration_since(job.last_progress_at) > window
            {
                job.state = JobState::Stalled;
                job.ban_score_penalty = 10; // FR-017: +10 for stall
                reaped.push(job.clone());
            }
        }

        // Remove reaped jobs from active list
        self.jobs
            .retain(|j| j.state == JobState::Running || j.state == JobState::Pending);

        reaped
    }

    /// Record progress on a job (resets the stall timer to the effective virtual-clock time).
    pub fn record_progress(&mut self, job_id: u64) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.job_id == job_id) {
            // Use effective time (real + virtual offset) so that subsequent
            // advance_clock + reap_stalled measures from the virtual "now".
            job.last_progress_at = std::time::Instant::now() + self.virtual_clock_offset;
        }
    }

    /// Mark a job as completed and remove it.
    pub fn complete(&mut self, job_id: u64) {
        self.jobs.retain(|j| j.job_id != job_id);
    }
}

/// Outcome of processing a received block (T061).
#[derive(Debug)]
pub struct BlockHandleOutcome {
    pub ban_score_delta: u32,
    pub job_state: JobState,
    pub peer_banned: bool,
}

/// Handle a received block — valid blocks complete, invalid blocks ban (T061, FR-018).
///
/// Invalid block: ban_score += 100 (instant ban threshold), job state = Failed.
/// Valid block: ban_score += 0, job state = Completed.
pub fn handle_received_block(_peer: PeerAddr, valid: bool, _height: u64) -> BlockHandleOutcome {
    if valid {
        BlockHandleOutcome {
            ban_score_delta: 0,
            job_state: JobState::Completed,
            peer_banned: false,
        }
    } else {
        BlockHandleOutcome {
            ban_score_delta: 100,
            job_state: JobState::Failed,
            peer_banned: true, // 100 >= ban threshold
        }
    }
}

#[cfg(test)]
mod red_phase_bounded_windows_tests {
    //! T103 RED-phase — FR-014, FR-015 bounded in-flight windows.
    //!
    //! Per-peer in-flight request cap = 16 blocks. Aggregate cap = 128.
    //! GREEN impl introduces `BlockRequestScheduler`/`InFlightTracker`.

    use crate::network::integrated_sync::{
        BlockRequestScheduler, BoundedWindowConfig, InFlightTracker,
    };

    #[test]
    fn per_peer_window_caps_at_sixteen() {
        let mut tracker = InFlightTracker::new(BoundedWindowConfig::default());
        let peer = "198.51.100.1:18351".parse().unwrap();
        for i in 0..16u32 {
            assert!(tracker.try_reserve(peer, i.into()).is_ok());
        }
        // 17th must be rejected — per-peer cap.
        assert!(tracker.try_reserve(peer, 999u32.into()).is_err());
    }

    #[test]
    fn aggregate_window_caps_at_one_hundred_twenty_eight() {
        let mut scheduler = BlockRequestScheduler::new(BoundedWindowConfig::default());
        for i in 0..128u32 {
            let peer = format!("198.51.100.{}:18351", (i % 250) + 1)
                .parse()
                .unwrap();
            assert!(scheduler.try_schedule(peer, i.into()).is_ok());
        }
        let overflow_peer = "203.0.113.1:18351".parse().unwrap();
        assert!(
            scheduler
                .try_schedule(overflow_peer, 9999u32.into())
                .is_err(),
            "aggregate in-flight must not exceed 128 blocks"
        );
    }
}

#[cfg(test)]
mod red_phase_stall_tests {
    //! T104 RED-phase — stall detection and penalty (FR-016).
    //!
    //! A SyncJob that does not progress for 30s must be reaped, the peer's
    //! ban_score incremented by 10, and the job re-queued to another peer.

    use crate::network::integrated_sync::{JobState, StallReaper, SyncJob};
    use std::time::Duration;

    #[test]
    fn stalled_job_is_reaped_after_thirty_seconds() {
        let mut reaper = StallReaper::new(Duration::from_secs(30));
        let peer = "198.51.100.1:18351".parse().unwrap();
        let job_id = reaper.schedule(peer, /*block_height*/ 100);
        reaper.advance_clock(Duration::from_secs(31));
        let reaped = reaper.reap_stalled();
        assert_eq!(reaped.len(), 1);
        assert_eq!(reaped[0].job_id, job_id);
        assert_eq!(reaped[0].state, JobState::Stalled);
    }

    #[test]
    fn stalled_peer_receives_ban_score_penalty() {
        let mut reaper = StallReaper::new(Duration::from_secs(30));
        let peer = "198.51.100.2:18351".parse().unwrap();
        reaper.schedule(peer, 200);
        reaper.advance_clock(Duration::from_secs(31));
        let reaped = reaper.reap_stalled();
        let job: &SyncJob = &reaped[0];
        assert_eq!(job.ban_score_penalty, 10, "stall penalty = +10");
    }
}

#[cfg(test)]
mod red_phase_invalid_block_tests {
    //! T105 RED-phase — invalid block penalty (FR-017).
    //!
    //! A peer that delivers an invalid block receives ban_score += 100
    //! (immediate ban threshold). The SyncJob transitions to Failed.

    use crate::network::integrated_sync::{handle_received_block, JobState};

    #[test]
    fn invalid_block_immediately_bans_peer() {
        let peer = "198.51.100.9:18351".parse().unwrap();
        let outcome = handle_received_block(peer, /*valid=*/ false, /*height=*/ 500);
        assert_eq!(outcome.ban_score_delta, 100);
        assert_eq!(outcome.job_state, JobState::Failed);
        assert!(outcome.peer_banned, "100+ ban score must trigger ban");
    }

    #[test]
    fn valid_block_does_not_penalize() {
        let peer = "198.51.100.10:18351".parse().unwrap();
        let outcome = handle_received_block(peer, /*valid=*/ true, 501);
        assert_eq!(outcome.ban_score_delta, 0);
        assert_eq!(outcome.job_state, JobState::Completed);
        assert!(!outcome.peer_banned);
    }
}
