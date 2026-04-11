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

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, RwLock},
};

use crate::{
    blockchain::Block,
    network::{
        discovery::PeerDiscovery, protocol::*, ConnectionTracker, NetworkConfig, NetworkError,
        NetworkResult, PeerRateLimiter,
    },
};

/// Peer event messages sent to the application
#[derive(Debug, Clone)]
pub enum PeerEvent {
    /// Peer connected
    PeerConnected { addr: SocketAddr, height: u32, user_agent: String },
    /// Peer disconnected
    PeerDisconnected {
        addr: SocketAddr,
        reason: DisconnectReason,
    },
    /// Block received from peer
    BlockReceived { from: SocketAddr, block: Block },
    /// Transaction received from peer
    TransactionReceived {
        from: SocketAddr,
        tx: crate::blockchain::Transaction,
    },
    /// Inventory received
    InventoryReceived {
        from: SocketAddr,
        inv: Vec<InventoryVector>,
    },
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

/// Minimum outbound connections to maintain for eclipse attack resistance.
/// Bitcoin uses 8; we match that for mainnet security.
pub const MIN_OUTBOUND_CONNECTIONS: usize = 8;

/// Simple peer manager with security features
pub struct SimplePeerManager {
    /// Connected peers
    peers: Arc<RwLock<HashMap<SocketAddr, PeerHandle>>>,
    /// Outbound peer addresses (node-initiated connections)
    outbound_peers: Arc<RwLock<std::collections::HashSet<SocketAddr>>>,
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
    /// Peer address book — Bitcoin-style AddrMan equivalent
    pub discovery: Arc<RwLock<PeerDiscovery>>,
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
    pub fn new(config: NetworkConfig, block_height: Arc<RwLock<u32>>) -> Self {
        // Create bounded event channel (Issue #3)
        let (event_tx, event_rx) = mpsc::channel(config.event_queue_size);

        // Initialise the address book for the correct network
        let network = if config.regtest {
            crate::Network::Regtest
        } else if config.testnet {
            crate::Network::Testnet
        } else {
            crate::Network::Mainnet
        };
        let mut discovery = PeerDiscovery::new_for_network(2048, network);

        // Seed the address book with hardcoded fallback peers from config
        for addr in &config.hardcoded_seeds {
            discovery.add_address(*addr, ServiceFlags::NETWORK);
        }

        SimplePeerManager {
            peers: Arc::new(RwLock::new(HashMap::new())),
            outbound_peers: Arc::new(RwLock::new(std::collections::HashSet::new())),
            magic_bytes: config.magic_bytes(),
            local_addr: config.listen_addr,
            block_height,
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
            connection_tracker: Arc::new(RwLock::new(ConnectionTracker::new())),
            config,
            discovery: Arc::new(RwLock::new(discovery)),
        }
    }

    /// Take the event receiver (can only be called once)
    pub async fn take_event_receiver(&self) -> Option<mpsc::Receiver<PeerEvent>> {
        self.event_rx.write().await.take()
    }

    /// Start listening for incoming connections
    pub async fn start_listening(&self) -> NetworkResult<()> {
        let listener = TcpListener::bind(self.local_addr)
            .await
            .map_err(|e| NetworkError::Connection(format!("Failed to bind: {}", e)))?;

        println!("📡 Listening for P2P connections on {}", self.local_addr);

        let peers = Arc::clone(&self.peers);
        let magic_bytes = self.magic_bytes;
        let block_height = Arc::clone(&self.block_height);
        let event_tx = self.event_tx.clone();
        let connection_tracker = Arc::clone(&self.connection_tracker);
        let config = self.config.clone();
        let discovery = Arc::clone(&self.discovery);

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
                                let discovery_clone = Arc::clone(&discovery);

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
                                        discovery_clone,
                                    )
                                    .await
                                    {
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
        discovery: Arc<RwLock<PeerDiscovery>>,
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
                println!(
                    "✅ Handshake complete with {} (height: {})",
                    addr, peer_version.start_height
                );

                // Record peer in address book
                {
                    let mut disc = discovery.write().await;
                    disc.add_address(addr, ServiceFlags::NETWORK);
                    disc.record_success(&addr);
                }

                // Send peer connected event (with backpressure handling - Issue #3)
                if let Err(e) = event_tx.try_send(PeerEvent::PeerConnected {
                    addr,
                    height: peer_version.start_height,
                    user_agent: peer_version.user_agent.clone(),
                }) {
                    eprintln!(
                        "⚠️ Event queue full when notifying connection from {}: {}",
                        addr, e
                    );
                }

                // Create bounded message channel (Issue #3)
                let (tx, mut rx) = mpsc::channel(config.peer_message_queue_size);

                // Keep a sender clone for use inside the message-handler loop
                // (needed to reply to GetAddr without going through the HashMap)
                let peer_tx_for_handler = tx.clone();

                // Create rate limiter for this peer (Issue #1)
                let rate_limiter = Arc::new(RwLock::new(PeerRateLimiter::with_config(
                    config.rate_limiter.clone(),
                )));

                // Store peer
                {
                    let mut peers_write = peers.write().await;
                    peers_write.insert(
                        addr,
                        PeerHandle {
                            tx,
                            rate_limiter: Arc::clone(&rate_limiter),
                        },
                    );
                }

                // Ask the inbound peer for their address list — Bitcoin does this
                // immediately after handshake for both inbound and outbound peers.
                let _ = peer_tx_for_handler.try_send(Message::GetAddr);

                // Spawn message handler
                let peers_clone = Arc::clone(&peers);
                let event_tx_clone = event_tx.clone();
                let tracker_clone = Arc::clone(&connection_tracker);
                let discovery_clone = Arc::clone(&discovery);
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
                                                Self::handle_peer_message(
                                                    &msg,
                                                    addr,
                                                    &event_tx_clone,
                                                    &peer_tx_for_handler,
                                                    &discovery_clone,
                                                )
                                                .await;
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

        let stream = TcpStream::connect(addr).await.map_err(|e| {
            // Unregister on connection failure
            let tracker = self.connection_tracker.clone();
            let disc = Arc::clone(&self.discovery);
            tokio::spawn(async move {
                tracker.write().await.remove_connection(&addr);
                disc.write().await.record_failure(&addr);
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
                println!(
                    "✅ Connected to {} (height: {})",
                    addr, peer_version.start_height
                );

                // Record in address book and update success stats
                {
                    let mut disc = self.discovery.write().await;
                    disc.add_address(addr, ServiceFlags::NETWORK);
                    disc.record_success(&addr);
                }

                // Send peer connected event (with backpressure)
                let _ = self.event_tx.try_send(PeerEvent::PeerConnected {
                    addr,
                    height: peer_version.start_height,
                    user_agent: peer_version.user_agent.clone(),
                });

                // Create bounded message channel (Issue #3)
                let (tx, mut rx) = mpsc::channel(self.config.peer_message_queue_size);

                // Keep a sender clone for responding to GetAddr inside the loop
                let peer_tx_for_handler = tx.clone();

                // Create rate limiter for this peer (Issue #1)
                let rate_limiter = Arc::new(RwLock::new(PeerRateLimiter::with_config(
                    self.config.rate_limiter.clone(),
                )));

                // Store peer and mark as outbound
                {
                    let mut peers_write = self.peers.write().await;
                    peers_write.insert(
                        addr,
                        PeerHandle {
                            tx,
                            rate_limiter: Arc::clone(&rate_limiter),
                        },
                    );
                }
                {
                    let mut outbound = self.outbound_peers.write().await;
                    outbound.insert(addr);
                }

                // Bitcoin-style post-handshake address exchange:
                // 1. Ask the peer for its address list
                let _ = peer_tx_for_handler.try_send(Message::GetAddr);
                // 2. Announce our own listening address so the peer can store and
                //    relay it — this is how new nodes propagate into the network.
                let our_addr_msg = Message::Addr(vec![NetworkAddress::new(
                    self.local_addr.ip(),
                    self.local_addr.port(),
                    ServiceFlags::NETWORK,
                )]);
                let _ = peer_tx_for_handler.try_send(our_addr_msg);

                // Spawn message handler
                let peers_clone = Arc::clone(&self.peers);
                let outbound_clone = Arc::clone(&self.outbound_peers);
                let event_tx_clone = self.event_tx.clone();
                let tracker_clone = Arc::clone(&self.connection_tracker);
                let discovery_clone = Arc::clone(&self.discovery);
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
                                                Self::handle_peer_message(
                                                    &msg,
                                                    addr,
                                                    &event_tx_clone,
                                                    &peer_tx_for_handler,
                                                    &discovery_clone,
                                                )
                                                .await;
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
                    outbound_clone.write().await.remove(&addr);

                    // Unregister connection (Issue #4)
                    {
                        let mut tracker = tracker_clone.write().await;
                        tracker.remove_connection(&addr);
                    }

                    // Update failure stats on disconnect
                    discovery_clone.write().await.record_failure(&addr);

                    // Notify disconnect
                    let _ = event_tx_clone.try_send(PeerEvent::PeerDisconnected {
                        addr,
                        reason: disconnect_reason,
                    });

                    println!("Peer {} disconnected", addr);
                });

                Ok(())
            }
            Err(e) => {
                eprintln!("Handshake failed with {}: {}", addr, e);

                // Update stats and unregister connection on handshake failure
                self.discovery.write().await.record_failure(&addr);
                let mut tracker = self.connection_tracker.write().await;
                tracker.remove_connection(&addr);

                Err(e.into())
            }
        }
    }

    /// Handle message from peer.
    ///
    /// `peer_tx` is the sender for messages going back to *this* peer (used to
    /// reply to `GetAddr` without taking the global peers lock).
    /// `discovery` is the shared address book (written on incoming `Addr` msgs).
    async fn handle_peer_message(
        msg: &Message,
        from: SocketAddr,
        event_tx: &mpsc::Sender<PeerEvent>,
        peer_tx: &mpsc::Sender<Message>,
        discovery: &Arc<RwLock<PeerDiscovery>>,
    ) {
        match msg {
            Message::Ping(nonce) => {
                println!("📌 Ping from {}: {}", from, nonce);
                // Pong is sent automatically in the message loop
            }
            Message::Pong(nonce) => {
                println!("📍 Pong from {}: {}", from, nonce);
            }

            // ── Bitcoin-style peer exchange ──────────────────────────────
            //
            // GetAddr: peer wants our address list — reply with up to 1000
            // addresses we know about (same as Bitcoin Core does).
            Message::GetAddr => {
                let addrs = discovery
                    .read()
                    .await
                    .get_addresses_for_sharing(1000);
                if !addrs.is_empty() {
                    println!(
                        "📬 GetAddr from {} — sending {} address(es)",
                        from,
                        addrs.len()
                    );
                    let _ = peer_tx.try_send(Message::Addr(addrs));
                }
            }

            // Addr: peer is sharing addresses it knows about — add them all
            // to our address book so we can connect to them later.
            Message::Addr(addresses) => {
                let count = addresses.len();
                println!("📬 Addr from {} — {} address(es)", from, count);
                if count > 0 {
                    discovery
                        .write()
                        .await
                        .add_addresses(addresses.clone());
                }
            }
            // ────────────────────────────────────────────────────────────

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
                println!(
                    "💳 Received transaction from {}: {}",
                    from,
                    tx.hash().to_hex()
                );
                let _ = event_tx.try_send(PeerEvent::TransactionReceived {
                    from,
                    tx: tx.clone(),
                });
            }
            _ => {}
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

        println!(
            "📡 Broadcasting block {} to {} peer(s)",
            block_hash.to_hex(),
            peers.len()
        );

        // Send inventory announcement
        let inv = Message::Inv(vec![InventoryVector {
            inv_type: InvType::Block,
            hash: block_hash,
        }]);

        for (addr, handle) in peers.iter() {
            // Use try_send to handle full queues (Issue #3)
            if let Err(e) = handle.tx.try_send(inv.clone()) {
                eprintln!(
                    "⚠️ Failed to send inv to {} (queue full or closed): {}",
                    addr, e
                );
            }
        }

        // Send the full block data to all peers
        let block_msg = Message::Block(block.clone());
        for (addr, handle) in peers.iter() {
            if let Err(e) = handle.tx.try_send(block_msg.clone()) {
                eprintln!(
                    "⚠️ Failed to send block to {} (queue full or closed): {}",
                    addr, e
                );
            }
        }
    }

    /// Broadcast transaction to all peers
    pub async fn broadcast_transaction(&self, tx: &crate::blockchain::Transaction) {
        let tx_hash = tx.hash();
        let peers = self.peers.read().await;

        if peers.is_empty() {
            println!(
                "📡 No peers to broadcast transaction {} to",
                tx_hash.to_hex()
            );
            return;
        }

        println!(
            "📡 Broadcasting transaction {} to {} peer(s)",
            tx_hash.to_hex(),
            peers.len()
        );

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

    /// Send a message to a specific peer by address.
    ///
    /// Returns silently if the peer is not connected (fire-and-forget).
    pub async fn send_to_peer(&self, addr: &SocketAddr, msg: Message) {
        let peers = self.peers.read().await;
        if let Some(handle) = peers.get(addr) {
            if let Err(e) = handle.tx.try_send(msg) {
                eprintln!("⚠️ Failed to send message to {}: {}", addr, e);
            }
        }
    }

    /// Get connection statistics
    pub async fn connection_stats(&self) -> crate::network::ConnectionTrackerStats {
        self.connection_tracker.read().await.stats()
    }

    /// Disconnect a specific peer
    pub async fn disconnect_peer(&self, addr: &SocketAddr) -> bool {
        let removed = self.peers.write().await.remove(addr).is_some();
        if removed {
            self.outbound_peers.write().await.remove(addr);
            let mut tracker = self.connection_tracker.write().await;
            tracker.remove_connection(addr);
            println!("Manually disconnected peer: {}", addr);
        }
        removed
    }

    /// Get number of outbound (node-initiated) connections
    pub async fn outbound_count(&self) -> usize {
        self.outbound_peers.read().await.len()
    }

    /// Check if we need more outbound connections for eclipse attack resistance.
    /// Returns `true` if outbound count is below MIN_OUTBOUND_CONNECTIONS.
    pub async fn needs_outbound_connections(&self) -> bool {
        self.outbound_count().await < MIN_OUTBOUND_CONNECTIONS
    }

    // ── Bitcoin-style connection manager ────────────────────────────────────
    //
    // Runs as a background task.  Every 30 seconds it checks whether we have
    // fewer than MIN_OUTBOUND_CONNECTIONS (8) and tries to open new ones from
    // the address book.  This mirrors Bitcoin Core's `ThreadOpenConnections`.

    /// Start the background outbound-connection manager.
    ///
    /// Must be called with an `Arc<SimplePeerManager>` so the task can call
    /// `connect_to_peer` without requiring exclusive ownership.
    pub fn start_connection_manager(manager: Arc<SimplePeerManager>) {
        use tokio::time::{interval, Duration};

        tokio::spawn(async move {
            // Initial delay — give the node a moment to start listening first.
            tokio::time::sleep(Duration::from_secs(5)).await;

            let mut ticker = interval(Duration::from_secs(30));
            loop {
                ticker.tick().await;

                if !manager.needs_outbound_connections().await {
                    continue;
                }

                let needed = MIN_OUTBOUND_CONNECTIONS
                    .saturating_sub(manager.outbound_count().await);

                // Fetch a few extra candidates in case some fail
                let candidates = manager
                    .discovery
                    .write()
                    .await
                    .get_peers(needed + 5, ServiceFlags::NETWORK);

                let already_connected: std::collections::HashSet<SocketAddr> =
                    manager.peers.read().await.keys().cloned().collect();

                for addr in candidates {
                    if !manager.needs_outbound_connections().await {
                        break;
                    }
                    if already_connected.contains(&addr) {
                        continue;
                    }
                    // Fire-and-forget; errors are logged inside connect_to_peer
                    let _ = manager.connect_to_peer(addr).await;
                }

                // If we still have no peers at all, re-query DNS seeds
                if manager.peer_count().await == 0 {
                    println!("🔍 No peers — re-querying DNS seeds...");
                    let _ = manager.discovery.write().await.query_dns_seeds().await;
                }
            }
        });
    }

    // ── Peers.dat persistence ────────────────────────────────────────────────

    /// Save the current address book to `path` (peers.dat equivalent).
    ///
    /// Call this periodically (e.g. every 15 minutes) and on clean shutdown.
    pub async fn save_peers(&self, path: &std::path::Path) {
        match self.discovery.read().await.save_to_disk(path) {
            Ok(()) => println!("💾 Saved peers to {}", path.display()),
            Err(e) => eprintln!("⚠️  Failed to save peers.dat: {}", e),
        }
    }

    /// Load a previously-saved address book from `path` (peers.dat equivalent).
    ///
    /// Call this once at startup, before `start_connection_manager`.
    pub async fn load_peers(&self, path: &std::path::Path) {
        self.discovery.write().await.load_from_disk(path);
    }

    /// Spawn a background task that saves peers.dat every `interval_secs` seconds.
    pub fn start_peer_saver(manager: Arc<SimplePeerManager>, path: std::path::PathBuf) {
        use tokio::time::{interval, Duration};

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(900)); // every 15 minutes
            loop {
                ticker.tick().await;
                manager.save_peers(&path).await;
            }
        });
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
                let tx_size: usize = block
                    .transactions
                    .iter()
                    .map(|tx| 100 + (tx.inputs.len() * 150) + (tx.outputs.len() * 40))
                    .sum();
                base_size + tx_size
            }
            Message::Tx(tx) => 100 + (tx.inputs.len() * 150) + (tx.outputs.len() * 40),
            Message::GetHeaders(_) => 100,
            Message::Headers(headers) => 24 + (headers.len() * 80),
            Message::NotFound(inv) => 24 + (inv.len() * 36),
            Message::Reject(_) => 100,
            Message::MemPool => 24,
            Message::SendHeaders => 24,
        }
    }
}
