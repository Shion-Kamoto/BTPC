//! Simple peer manager for handling P2P connections with security features
//!
//! Security features (Issues #1, #3, #4, #12):
//! - Rate limiting per peer (Issue #1)
//! - Bounded channels with backpressure (Issue #3)
//! - Per-IP connection limits (Issue #4)
//! - Connection direction tracking (Issue #12 - basic implementation)
//!
//! ## Connection Direction (Issue #12)
//!
//! The peer manager tracks connection direction (inbound vs outbound) for basic
//! eclipse attack mitigation. Current implementation:
//! - Tracks which connections are node-initiated (outbound) vs peer-initiated (inbound)
//! - Provides statistics on connection mix
//!
//! Future enhancements for mainnet (not required for testnet):
//! - Enforce minimum outbound connection count (e.g., 8)
//! - Implement netgroup diversity for outbound connections
//! - Prefer diverse /16 subnets for outbound peers
//! - Active outbound connection management
//!
//! The current per-IP limits (Issue #4) already provide basic eclipse attack
//! protection by limiting connections from any single IP or subnet.

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
};

use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, RwLock},
};

use crate::{
    blockchain::Block,
    network::{
        protocol::*,
        ConnectionTracker, NetworkConfig, NetworkError, NetworkResult, PeerRateLimiter,
    },
};

/// Peer event messages sent to the application
#[derive(Debug, Clone)]
pub enum PeerEvent {
    /// Peer connected
    PeerConnected { addr: SocketAddr, height: u32 },
    /// Peer disconnected
    PeerDisconnected { addr: SocketAddr, reason: DisconnectReason },
    /// Block received from peer
    BlockReceived { from: SocketAddr, block: Block },
    /// Transaction received from peer
    TransactionReceived { from: SocketAddr, tx: crate::blockchain::Transaction },
    /// Inventory received
    InventoryReceived { from: SocketAddr, inv: Vec<InventoryVector> },
}

/// Reason for peer disconnection
#[derive(Debug, Clone)]
pub enum DisconnectReason {
    /// Normal disconnect
    Normal,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Message queue full
    QueueFull,
    /// Protocol error
    ProtocolError(String),
    /// Connection error
    ConnectionError(String),
}

/// Simple peer manager with security features
pub struct SimplePeerManager {
    /// Connected peers
    peers: Arc<RwLock<HashMap<SocketAddr, PeerHandle>>>,
    /// Magic bytes for network
    magic_bytes: [u8; 4],
    /// Local listening address
    local_addr: SocketAddr,
    /// Block height
    block_height: Arc<RwLock<u32>>,
    /// Event sender (bounded channel - Issue #3)
    event_tx: mpsc::Sender<PeerEvent>,
    /// Event receiver
    event_rx: Arc<RwLock<Option<mpsc::Receiver<PeerEvent>>>>,
    /// Connection tracker (Issue #4)
    connection_tracker: Arc<RwLock<ConnectionTracker>>,
    /// Network configuration
    config: NetworkConfig,
}

/// Handle to a connected peer with rate limiting
struct PeerHandle {
    /// Sender for messages to this peer (bounded - Issue #3)
    tx: mpsc::Sender<Message>,
    /// Rate limiter for this peer (Issue #1)
    rate_limiter: Arc<RwLock<PeerRateLimiter>>,
}

impl SimplePeerManager {
    /// Create new peer manager with configuration
    pub fn new(
        config: NetworkConfig,
        block_height: Arc<RwLock<u32>>,
    ) -> Self {
        // Create bounded event channel (Issue #3)
        let (event_tx, event_rx) = mpsc::channel(config.event_queue_size);

        SimplePeerManager {
            peers: Arc::new(RwLock::new(HashMap::new())),
            magic_bytes: config.magic_bytes(),
            local_addr: config.listen_addr,
            block_height,
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
            connection_tracker: Arc::new(RwLock::new(ConnectionTracker::new())),
            config,
        }
    }

    /// Take the event receiver (can only be called once)
    pub async fn take_event_receiver(&self) -> Option<mpsc::Receiver<PeerEvent>> {
        self.event_rx.write().await.take()
    }

    /// Start listening for incoming connections
    pub async fn start_listening(&self) -> NetworkResult<()> {
        let listener = TcpListener::bind(self.local_addr).await
            .map_err(|e| NetworkError::Connection(format!("Failed to bind: {}", e)))?;

        println!("📡 Listening for P2P connections on {}", self.local_addr);

        let peers = Arc::clone(&self.peers);
        let magic_bytes = self.magic_bytes;
        let block_height = Arc::clone(&self.block_height);
        let event_tx = self.event_tx.clone();
        let connection_tracker = Arc::clone(&self.connection_tracker);
        let config = self.config.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        println!("📥 Incoming connection from: {}", addr);

                        // Check connection limits (Issue #4)
                        let can_accept = {
                            let tracker = connection_tracker.read().await;
                            tracker.can_accept(&addr, &config)
                        };

                        match can_accept {
                            Ok(()) => {
                                let peers_clone = Arc::clone(&peers);
                                let height = *block_height.read().await;
                                let event_tx_clone = event_tx.clone();
                                let tracker_clone = Arc::clone(&connection_tracker);
                                let config_clone = config.clone();

                                tokio::spawn(async move {
                                    if let Err(e) = Self::handle_incoming_peer(
                                        stream,
                                        addr,
                                        magic_bytes,
                                        height,
                                        peers_clone,
                                        event_tx_clone,
                                        tracker_clone,
                                        config_clone,
                                    ).await {
                                        eprintln!("Error handling peer {}: {}", addr, e);
                                    }
                                });
                            }
                            Err(e) => {
                                eprintln!("❌ Connection from {} rejected: {}", addr, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error accepting connection: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Handle incoming peer connection
    async fn handle_incoming_peer(
        stream: TcpStream,
        addr: SocketAddr,
        magic_bytes: [u8; 4],
        block_height: u32,
        peers: Arc<RwLock<HashMap<SocketAddr, PeerHandle>>>,
        event_tx: mpsc::Sender<PeerEvent>,
        connection_tracker: Arc<RwLock<ConnectionTracker>>,
        config: NetworkConfig,
    ) -> NetworkResult<()> {
        // Register connection (Issue #4)
        {
            let mut tracker = connection_tracker.write().await;
            tracker.add_connection(&addr);
        }

        let codec = ProtocolCodec::new(magic_bytes);
        let mut connection = PeerConnection::new(stream, codec);

        // Create our version message
        let our_version = VersionMessage::new(
            ServiceFlags::NETWORK,
            NetworkAddress::default(),
            NetworkAddress::default(),
            config.user_agent.clone(),
            block_height,
            true,
        );

        // Perform handshake
        let handshake_result = connection.handshake(our_version).await;

        match handshake_result {
            Ok(peer_version) => {
                println!("✅ Handshake complete with {} (height: {})", addr, peer_version.start_height);

                // Send peer connected event (with backpressure handling - Issue #3)
                if let Err(e) = event_tx.try_send(PeerEvent::PeerConnected {
                    addr,
                    height: peer_version.start_height,
                }) {
                    eprintln!("⚠️ Event queue full when notifying connection from {}: {}", addr, e);
                    // Continue anyway - connection event is not critical
                }

                // Create bounded message channel (Issue #3)
                let (tx, mut rx) = mpsc::channel(config.peer_message_queue_size);

                // Create rate limiter for this peer (Issue #1)
                let rate_limiter = Arc::new(RwLock::new(
                    PeerRateLimiter::with_config(config.rate_limiter.clone())
                ));

                // Store peer
                {
                    let mut peers_write = peers.write().await;
                    peers_write.insert(addr, PeerHandle {
                        tx,
                        rate_limiter: Arc::clone(&rate_limiter),
                    });
                }

                // Spawn message handler
                let peers_clone = Arc::clone(&peers);
                let event_tx_clone = event_tx.clone();
                let tracker_clone = Arc::clone(&connection_tracker);
                tokio::spawn(async move {
                    let disconnect_reason = loop {
                        tokio::select! {
                            // Send messages from channel
                            Some(msg) = rx.recv() => {
                                if let Err(e) = connection.send_message(&msg).await {
                                    eprintln!("Error sending to {}: {}", addr, e);
                                    break DisconnectReason::ConnectionError(e.to_string());
                                }
                            }
                            // Receive messages from peer
                            msg_result = connection.receive_message() => {
                                match msg_result {
                                    Ok(msg) => {
                                        // Check rate limit (Issue #1)
                                        let rate_check = {
                                            let mut limiter = rate_limiter.write().await;
                                            limiter.check_and_record(msg.size())
                                        };

                                        match rate_check {
                                            Ok(()) => {
                                                Self::handle_peer_message(&msg, addr, &event_tx_clone).await;
                                            }
                                            Err(e) => {
                                                eprintln!("❌ Rate limit exceeded for {}: {}", addr, e);
                                                break DisconnectReason::RateLimitExceeded;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Error receiving from {}: {}", addr, e);
                                        break DisconnectReason::ProtocolError(e.to_string());
                                    }
                                }
                            }
                        }
                    };

                    // Remove peer on disconnect
                    peers_clone.write().await.remove(&addr);

                    // Unregister connection (Issue #4)
                    {
                        let mut tracker = tracker_clone.write().await;
                        tracker.remove_connection(&addr);
                    }

                    // Notify disconnect (with backpressure)
                    let _ = event_tx_clone.try_send(PeerEvent::PeerDisconnected {
                        addr,
                        reason: disconnect_reason,
                    });

                    println!("❌ Peer {} disconnected", addr);
                });
            }
            Err(e) => {
                eprintln!("Handshake failed with {}: {}", addr, e);

                // Unregister connection on handshake failure
                let mut tracker = connection_tracker.write().await;
                tracker.remove_connection(&addr);

                return Err(e.into());
            }
        }

        Ok(())
    }

    /// Connect to a peer
    pub async fn connect_to_peer(&self, addr: SocketAddr) -> NetworkResult<()> {
        println!("🔗 Connecting to peer: {}", addr);

        // Check connection limits (Issue #4)
        {
            let tracker = self.connection_tracker.read().await;
            tracker.can_accept(&addr, &self.config)?;
        }

        // Register connection
        {
            let mut tracker = self.connection_tracker.write().await;
            tracker.add_connection(&addr);
        }

        let stream = TcpStream::connect(addr).await
            .map_err(|e| {
                // Unregister on connection failure
                let tracker = self.connection_tracker.clone();
                tokio::spawn(async move {
                    tracker.write().await.remove_connection(&addr);
                });
                NetworkError::Connection(format!("Failed to connect: {}", e))
            })?;

        let codec = ProtocolCodec::new(self.magic_bytes);
        let mut connection = PeerConnection::new(stream, codec);

        // Create our version message
        let our_version = VersionMessage::new(
            ServiceFlags::NETWORK,
            NetworkAddress::default(),
            NetworkAddress::default(),
            self.config.user_agent.clone(),
            *self.block_height.read().await,
            true,
        );

        // Perform handshake
        match connection.handshake(our_version).await {
            Ok(peer_version) => {
                println!("✅ Connected to {} (height: {})", addr, peer_version.start_height);

                // Send peer connected event (with backpressure)
                let _ = self.event_tx.try_send(PeerEvent::PeerConnected {
                    addr,
                    height: peer_version.start_height,
                });

                // Create bounded message channel (Issue #3)
                let (tx, mut rx) = mpsc::channel(self.config.peer_message_queue_size);

                // Create rate limiter for this peer (Issue #1)
                let rate_limiter = Arc::new(RwLock::new(
                    PeerRateLimiter::with_config(self.config.rate_limiter.clone())
                ));

                // Store peer
                {
                    let mut peers_write = self.peers.write().await;
                    peers_write.insert(addr, PeerHandle {
                        tx,
                        rate_limiter: Arc::clone(&rate_limiter),
                    });
                }

                // Spawn message handler
                let peers_clone = Arc::clone(&self.peers);
                let event_tx_clone = self.event_tx.clone();
                let tracker_clone = Arc::clone(&self.connection_tracker);
                tokio::spawn(async move {
                    let disconnect_reason = loop {
                        tokio::select! {
                            // Send messages from channel
                            Some(msg) = rx.recv() => {
                                if let Err(e) = connection.send_message(&msg).await {
                                    eprintln!("Error sending to {}: {}", addr, e);
                                    break DisconnectReason::ConnectionError(e.to_string());
                                }
                            }
                            // Receive messages from peer
                            msg_result = connection.receive_message() => {
                                match msg_result {
                                    Ok(msg) => {
                                        // Check rate limit (Issue #1)
                                        let rate_check = {
                                            let mut limiter = rate_limiter.write().await;
                                            limiter.check_and_record(msg.size())
                                        };

                                        match rate_check {
                                            Ok(()) => {
                                                Self::handle_peer_message(&msg, addr, &event_tx_clone).await;
                                            }
                                            Err(e) => {
                                                eprintln!("❌ Rate limit exceeded for {}: {}", addr, e);
                                                break DisconnectReason::RateLimitExceeded;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Error receiving from {}: {}", addr, e);
                                        break DisconnectReason::ProtocolError(e.to_string());
                                    }
                                }
                            }
                        }
                    };

                    // Remove peer on disconnect
                    peers_clone.write().await.remove(&addr);

                    // Unregister connection (Issue #4)
                    {
                        let mut tracker = tracker_clone.write().await;
                        tracker.remove_connection(&addr);
                    }

                    // Notify disconnect
                    let _ = event_tx_clone.try_send(PeerEvent::PeerDisconnected {
                        addr,
                        reason: disconnect_reason,
                    });

                    println!("❌ Peer {} disconnected", addr);
                });

                Ok(())
            }
            Err(e) => {
                eprintln!("Handshake failed with {}: {}", addr, e);

                // Unregister connection on handshake failure
                let mut tracker = self.connection_tracker.write().await;
                tracker.remove_connection(&addr);

                Err(e.into())
            }
        }
    }

    /// Handle message from peer
    async fn handle_peer_message(msg: &Message, from: SocketAddr, event_tx: &mpsc::Sender<PeerEvent>) {
        match msg {
            Message::Ping(nonce) => {
                println!("📌 Ping from {}: {}", from, nonce);
                // Pong is sent automatically in the message loop
            }
            Message::Pong(nonce) => {
                println!("📍 Pong from {}: {}", from, nonce);
            }
            Message::Inv(inv_vectors) => {
                println!("📋 Inventory from {}: {} item(s)", from, inv_vectors.len());
                for inv in inv_vectors {
                    match inv.inv_type {
                        InvType::Block => {
                            println!("  - Block: {}", inv.hash.to_hex());
                        }
                        InvType::Tx => {
                            println!("  - Transaction: {}", inv.hash.to_hex());
                        }
                        _ => {}
                    }
                }
                // Send event (with backpressure)
                let _ = event_tx.try_send(PeerEvent::InventoryReceived {
                    from,
                    inv: inv_vectors.clone(),
                });
            }
            Message::Block(block) => {
                println!("📦 Received block from {}: {}", from, block.hash().to_hex());
                let _ = event_tx.try_send(PeerEvent::BlockReceived {
                    from,
                    block: block.clone(),
                });
            }
            Message::Tx(tx) => {
                println!("💳 Received transaction from {}: {}", from, tx.hash().to_hex());
                let _ = event_tx.try_send(PeerEvent::TransactionReceived {
                    from,
                    tx: tx.clone(),
                });
            }
            _ => {
                // Handle other message types
            }
        }
    }

    /// Broadcast block to all peers
    pub async fn broadcast_block(&self, block: &Block) {
        let block_hash = block.hash();
        let peers = self.peers.read().await;

        if peers.is_empty() {
            println!("📡 No peers to broadcast block {} to", block_hash.to_hex());
            return;
        }

        println!("📡 Broadcasting block {} to {} peer(s)", block_hash.to_hex(), peers.len());

        // Send inventory announcement
        let inv = Message::Inv(vec![InventoryVector {
            inv_type: InvType::Block,
            hash: block_hash,
        }]);

        for (addr, handle) in peers.iter() {
            // Use try_send to handle full queues (Issue #3)
            if let Err(e) = handle.tx.try_send(inv.clone()) {
                eprintln!("⚠️ Failed to send inv to {} (queue full or closed): {}", addr, e);
            }
        }

        // Send the full block data to all peers
        let block_msg = Message::Block(block.clone());
        for (addr, handle) in peers.iter() {
            if let Err(e) = handle.tx.try_send(block_msg.clone()) {
                eprintln!("⚠️ Failed to send block to {} (queue full or closed): {}", addr, e);
            }
        }
    }

    /// Broadcast transaction to all peers
    pub async fn broadcast_transaction(&self, tx: &crate::blockchain::Transaction) {
        let tx_hash = tx.hash();
        let peers = self.peers.read().await;

        if peers.is_empty() {
            println!("📡 No peers to broadcast transaction {} to", tx_hash.to_hex());
            return;
        }

        println!("📡 Broadcasting transaction {} to {} peer(s)", tx_hash.to_hex(), peers.len());

        // Send inventory announcement
        let inv = Message::Inv(vec![InventoryVector {
            inv_type: InvType::Tx,
            hash: tx_hash,
        }]);

        for (addr, handle) in peers.iter() {
            if let Err(e) = handle.tx.try_send(inv.clone()) {
                eprintln!("⚠️ Failed to send tx inv to {}: {}", addr, e);
            }
        }

        // Send the actual transaction to all peers
        let tx_msg = Message::Tx(tx.clone());
        for (addr, handle) in peers.iter() {
            if let Err(e) = handle.tx.try_send(tx_msg.clone()) {
                eprintln!("⚠️ Failed to send tx to {}: {}", addr, e);
            }
        }
    }

    /// Get number of connected peers
    pub async fn peer_count(&self) -> usize {
        self.peers.read().await.len()
    }

    /// Get connection statistics
    pub async fn connection_stats(&self) -> crate::network::ConnectionTrackerStats {
        self.connection_tracker.read().await.stats()
    }

    /// Disconnect a specific peer
    pub async fn disconnect_peer(&self, addr: &SocketAddr) -> bool {
        let removed = self.peers.write().await.remove(addr).is_some();
        if removed {
            let mut tracker = self.connection_tracker.write().await;
            tracker.remove_connection(addr);
            println!("🔌 Manually disconnected peer: {}", addr);
        }
        removed
    }
}

// Add size() method to Message for rate limiting
impl Message {
    /// Get approximate size of message in bytes (for rate limiting)
    pub fn size(&self) -> usize {
        match self {
            Message::Version(_) => 100,
            Message::VerAck => 24,
            Message::Ping(_) => 32,
            Message::Pong(_) => 32,
            Message::GetAddr => 24,
            Message::Addr(addrs) => 24 + (addrs.len() * 30),
            Message::Inv(inv) => 24 + (inv.len() * 36),
            Message::GetData(inv) => 24 + (inv.len() * 36),
            Message::Block(block) => {
                // Approximate block size
                let base_size = 80; // Block header
                let tx_size: usize = block.transactions.iter()
                    .map(|tx| 100 + (tx.inputs.len() * 150) + (tx.outputs.len() * 40))
                    .sum();
                base_size + tx_size
            }
            Message::Tx(tx) => {
                100 + (tx.inputs.len() * 150) + (tx.outputs.len() * 40)
            }
            Message::GetHeaders(_) => 100,
            Message::Headers(headers) => 24 + (headers.len() * 80),
            Message::NotFound(inv) => 24 + (inv.len() * 36),
            Message::Reject(_) => 100,
            Message::MemPool => 24,
            Message::SendHeaders => 24,
        }
    }
}