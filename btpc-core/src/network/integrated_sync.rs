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
        let sync_manager = Arc::new(TokioRwLock::new(SyncManager::new(config.max_parallel_downloads)));
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
    async fn start_message_processor(&self) {
        let (_, mut message_rx) = mpsc::unbounded_channel::<NetworkMessage>();
        let manager = self.clone();

        tokio::spawn(async move {
            while let Some(message) = message_rx.recv().await {
                if let Err(e) = manager.process_network_message(message).await {
                    eprintln!("Error processing network message: {}", e);
                }
            }
        });
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
    // Note: The following message processing methods are currently stubs for the
    // integrated sync coordination layer. This layer is designed to coordinate
    // P2P networking with blockchain synchronization.
    //
    // The actual P2P message handling is implemented in:
    // - simple_peer_manager.rs: Connection management and message routing
    // - protocol.rs: Wire protocol and message validation
    // - discovery.rs: Peer discovery and selection
    //
    // These methods are placeholders for future enhancement when full P2P sync
    // coordination is required. Current deployment uses simple peer management.
    //
    // For production deployment, these would be implemented to:
    // - Process and validate headers in sync order
    // - Apply blocks with full consensus validation
    // - Manage inventory requests and bandwidth
    // - Serve data to requesting peers
    //
    // Security Note: All message validation (size limits, rate limiting, etc.)
    // is enforced at the protocol layer before reaching these methods.
    // ============================================================================

    /// Process headers message
    ///
    /// Placeholder: Full implementation would validate headers, update sync state,
    /// and request missing blocks.
    async fn process_headers_message(
        &self,
        _peer: String,
        headers: Vec<BlockHeader>,
    ) -> Result<(), IntegratedSyncError> {
        // Log received headers for monitoring
        println!("Received {} headers from peer: {}", headers.len(), _peer);

        // Future implementation:
        // 1. Validate header chain (prev_hash links)
        // 2. Check PoW difficulty for each header
        // 3. Update sync manager with new headers
        // 4. Request blocks for headers we don't have

        Ok(())
    }

    /// Process block message
    ///
    /// Placeholder: Full implementation would validate and apply blocks to chain.
    async fn process_block_message(
        &self,
        _peer: String,
        block: Block,
    ) -> Result<(), IntegratedSyncError> {
        // Log received block for monitoring
        println!(
            "Received block {} from peer: {}",
            block.hash().to_hex(),
            _peer
        );

        // Future implementation:
        // 1. Validate block with StorageBlockValidator
        // 2. Check block is part of best chain
        // 3. Apply block to blockchain database
        // 4. Update UTXO set
        // 5. Update sync manager state
        // 6. Relay block to other peers if valid

        Ok(())
    }

    /// Process inventory message
    ///
    /// Placeholder: Full implementation would track inventory and request needed data.
    async fn process_inventory_message(
        &self,
        peer: String,
        inventory: Vec<InventoryVector>,
    ) -> Result<(), IntegratedSyncError> {
        // Log received inventory for monitoring
        println!(
            "Received {} inventory items from peer: {}",
            inventory.len(),
            peer
        );

        // Future implementation:
        // 1. Filter inventory for items we don't have
        // 2. Prioritize blocks over transactions
        // 3. Send getdata requests for needed items
        // 4. Track requested items to avoid duplicates

        Ok(())
    }

    /// Process getdata message
    ///
    /// Placeholder: Full implementation would serve requested data to peers.
    async fn process_getdata_message(
        &self,
        peer: String,
        inventory: Vec<InventoryVector>,
    ) -> Result<(), IntegratedSyncError> {
        // Log received getdata request for monitoring
        println!("Received getdata request for {} items from peer: {}", inventory.len(), peer);

        // Future implementation:
        // 1. Validate request size (already done at protocol layer)
        // 2. Check rate limits for this peer
        // 3. Lookup requested blocks/transactions in database
        // 4. Send block/tx messages to requesting peer
        // 5. Track bandwidth usage

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
        let locator = tokio::task::spawn_blocking(move || -> Result<Vec<Hash>, IntegratedSyncError> {
            // Acquire read lock in blocking context
            let db = blockchain_db.read()
                .map_err(|e| IntegratedSyncError::Network(
                    NetworkError::Connection(format!("Lock error: {}", e))
                ))?;

            // Get current tip
            let tip = db
                .get_chain_tip()
                .map_err(|e| IntegratedSyncError::Network(NetworkError::Storage(e.to_string())))?;

            if let Some(tip_block) = tip {
                let locator = crate::network::sync::create_block_locator(
                    tip_block.hash(),
                    &*db,
                )?;
                Ok(locator)
            } else {
                // No blocks yet, use genesis
                Ok(vec![Hash::zero()])
            }
        })
        .await
        .map_err(|e| IntegratedSyncError::Network(
            NetworkError::Connection(format!("Task join error: {}", e))
        ))??;

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

    /// Get sync statistics
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
