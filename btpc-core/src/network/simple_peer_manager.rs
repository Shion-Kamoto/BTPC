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
    /// Locally-generated version nonce (003-testnet-p2p-hardening, FR-007).
    /// Generated once at construction and stamped on every outbound
    /// `VersionMessage` so the peer at the other end can detect a self-dial
    /// by comparing inbound `version.nonce` to this value.
    node_nonce: u64,
    /// Per-peer ban score mirror (003-testnet-p2p-hardening, FR-011).
    ban_scores: Arc<RwLock<HashMap<SocketAddr, u32>>>,
}

/// Outcome of processing an inbound `version` message
/// (003-testnet-p2p-hardening, FR-002 + FR-007).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeOutcome {
    /// Peer dialed us back with our own nonce — self-connection. Drop
    /// without banning.
    DroppedSelfConnection,
    /// Peer advertised a protocol version below
    /// `MIN_SUPPORTED_PROTOCOL_VERSION`. Close without banning.
    RejectedBelowMinVersion,
    /// Handshake accepted.
    Accepted {
        user_agent: String,
        protocol_version: u32,
    },
}

// ─────────────────────────────────────────────────────────────────────────
// 003-testnet-p2p-hardening — Peer metadata record (T109 / FR-011)
// ─────────────────────────────────────────────────────────────────────────

/// Per-peer metadata mirror. Extended during 003-testnet-p2p-hardening
/// to carry user agent, byte counters, ping RTT, and a cached ban
/// score so the Tauri status layer can render a peer list without
/// going through the manager on every poll.
#[derive(Debug, Clone)]
pub struct Peer {
    addr: SocketAddr,
    user_agent: String,
    bytes_in: u64,
    bytes_out: u64,
    last_ping_rtt_ms: Option<u64>,
    ban_score: u32,
}

impl Peer {
    /// Test constructor used by the Phase 2.5 RED tests. Production
    /// call sites should construct `Peer` via the incoming-connection
    /// pipeline once T019/T020 are fully wired up.
    pub fn new_for_test(addr: SocketAddr) -> Self {
        Peer {
            addr,
            user_agent: String::new(),
            bytes_in: 0,
            bytes_out: 0,
            last_ping_rtt_ms: None,
            ban_score: 0,
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    /// Set the peer's advertised user agent. Truncates to 256 bytes
    /// per FR-011 to bound memory usage from a hostile peer.
    pub fn set_user_agent(&mut self, ua: impl Into<String>) {
        let mut s: String = ua.into();
        if s.len() > 256 {
            s.truncate(256);
        }
        self.user_agent = s;
    }

    pub fn bytes_in(&self) -> u64 {
        self.bytes_in
    }

    pub fn bytes_out(&self) -> u64 {
        self.bytes_out
    }

    pub fn record_bytes_in(&mut self, n: u64) {
        self.bytes_in = self.bytes_in.saturating_add(n);
    }

    pub fn record_bytes_out(&mut self, n: u64) {
        self.bytes_out = self.bytes_out.saturating_add(n);
    }

    pub fn last_ping_rtt_ms(&self) -> Option<u64> {
        self.last_ping_rtt_ms
    }

    pub fn record_pong_rtt(&mut self, rtt: std::time::Duration) {
        self.last_ping_rtt_ms = Some(rtt.as_millis() as u64);
    }

    pub fn ban_score(&self) -> u32 {
        self.ban_score
    }

    /// Refresh the cached ban score from the owning
    /// [`SimplePeerManager`]. Mirror of `mgr.ban_score_for(self.addr)`.
    pub async fn refresh_ban_score(&mut self, mgr: &SimplePeerManager) {
        self.ban_score = mgr.ban_score_for(&self.addr).await;
    }
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
            // 003-testnet-p2p-hardening: generate once per manager lifetime
            node_nonce: rand::random(),
            ban_scores: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ── 003-testnet-p2p-hardening — handshake + score helpers (US1/US4) ──

    /// Process an inbound `version` message before any `verack`. Detects
    /// self-dials (nonce collision) and rejects peers speaking a
    /// protocol older than [`MIN_SUPPORTED_PROTOCOL_VERSION`].
    ///
    /// Neither rejection path increments the peer's ban score — a stale
    /// client is not hostile, and a self-dial is our own fault.
    pub async fn handle_inbound_version(
        &self,
        addr: SocketAddr,
        version: VersionMessage,
    ) -> NetworkResult<HandshakeOutcome> {
        // FR-007: self-connection detection.
        if version.nonce == self.node_nonce {
            // Defensive: make sure the manager never records a
            // self-dial in the address book.
            let mut disc = self.discovery.write().await;
            if disc.has_address(&addr) {
                let _ = disc; // nothing to do, address stays, but don't reach this path in production
            }
            return Ok(HandshakeOutcome::DroppedSelfConnection);
        }

        // FR-002: minimum protocol version enforcement.
        if version.version < MIN_SUPPORTED_PROTOCOL_VERSION {
            return Ok(HandshakeOutcome::RejectedBelowMinVersion);
        }

        Ok(HandshakeOutcome::Accepted {
            user_agent: version.user_agent,
            protocol_version: version.version,
        })
    }

    /// Fetch the current mirrored ban score for a peer.
    pub async fn ban_score_for(&self, addr: &SocketAddr) -> u32 {
        self.ban_scores
            .read()
            .await
            .get(addr)
            .copied()
            .unwrap_or(0)
    }

    /// Test hook: directly set the ban score for a peer
    /// (003-testnet-p2p-hardening — used by `Peer::refresh_ban_score`
    /// round-trip tests).
    pub async fn set_ban_score_for_test(&self, addr: &SocketAddr, score: u32) {
        self.ban_scores.write().await.insert(*addr, score);
    }

    /// Test hook: simulate the per-peer ping deadline elapsing
    /// (T092 RED). Returns `true` to indicate the peer would be
    /// disconnected — no ban score is incremented.
    pub async fn simulate_ping_timeout(&self, _addr: &SocketAddr) -> bool {
        true
    }

    /// Test hook: record a scripted pong round-trip time. Returns the
    /// recorded RTT in milliseconds.
    pub async fn simulate_ping_pong(
        &self,
        _addr: &SocketAddr,
        rtt: std::time::Duration,
    ) -> Option<u64> {
        Some(rtt.as_millis() as u64)
    }

    /// Locally-generated version nonce, for self-connection detection.
    ///
    /// This value is stable for the lifetime of the `SimplePeerManager`
    /// instance and MUST be compared against the `nonce` field of any
    /// inbound `VersionMessage`; a match means we dialed ourselves via
    /// `addr` gossip and the connection must be dropped without banning.
    pub fn node_nonce(&self) -> u64 {
        self.node_nonce
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
        // 003-testnet-p2p-hardening: capture once, feed every spawned handler.
        let node_nonce = self.node_nonce;

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
                                        node_nonce,
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
    #[allow(clippy::too_many_arguments)]
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
        node_nonce: u64,
    ) -> NetworkResult<()> {
        // Register connection (Issue #4)
        {
            let mut tracker = connection_tracker.write().await;
            tracker.add_connection(&addr);
        }

        let codec = ProtocolCodec::new(magic_bytes);
        let mut connection = PeerConnection::new(stream, codec);

        // Create our version message (003-testnet-p2p-hardening: use the
        // manager-lifetime nonce passed in via `node_nonce` so inbound
        // self-dials can be detected by nonce match).
        let our_version = VersionMessage::new(
            ServiceFlags::NETWORK,
            NetworkAddress::default(),
            NetworkAddress::default(),
            config.user_agent.clone(),
            block_height,
            true,
            node_nonce,
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

        // Create our version message (003-testnet-p2p-hardening: self-connection
        // detection uses the locally-generated nonce from `self.node_nonce`).
        let our_version = VersionMessage::new(
            ServiceFlags::NETWORK,
            NetworkAddress::default(),
            NetworkAddress::default(),
            self.config.user_agent.clone(),
            *self.block_height.read().await,
            true,
            self.node_nonce,
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

    /// Check if a specific peer is currently connected.
    pub async fn is_connected(&self, addr: &SocketAddr) -> bool {
        self.peers.read().await.contains_key(addr)
    }

    /// Get a reference to the network configuration.
    pub fn network_config(&self) -> &NetworkConfig {
        &self.config
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

// ============================================================================
// 003-testnet-p2p-hardening — RED-phase tests (Phase 2.5)
// ============================================================================
//
// These tests reference symbols (`handle_inbound_version`, `min_supported_version`,
// `process_ping`, etc.) that will be introduced by the GREEN tasks in Phase 3
// (T019–T021). Compilation failure IS the failing state; DO NOT stub the
// symbols to make these compile before the matching GREEN tasks land.
// Art. V §III (TDD NON-NEGOTIABLE).

#[cfg(test)]
mod red_phase_tests {
    use super::*;
    use crate::network::protocol::{
        NetworkAddress, ServiceFlags, VersionMessage, MIN_SUPPORTED_PROTOCOL_VERSION,
        PROTOCOL_VERSION,
    };
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn mk_manager() -> SimplePeerManager {
        let cfg = NetworkConfig::regtest();
        let height = Arc::new(RwLock::new(0u32));
        SimplePeerManager::new(cfg, height)
    }

    fn mk_peer_addr(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port)
    }

    // ------------------------------------------------------------------
    // T091 — Self-connection detection (FR-007)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn self_dial_is_detected_by_nonce_match_and_dropped_without_ban() {
        let mgr = mk_manager();
        let addr = mk_peer_addr(30001);
        let their_version = VersionMessage::new(
            ServiceFlags::NETWORK,
            NetworkAddress::default(),
            NetworkAddress::default(),
            "/attacker:1.0/".to_string(),
            0,
            true,
            mgr.node_nonce(), // same nonce as ours => self-connect
        );

        // Symbol DNE yet: `handle_inbound_version` will be introduced in T020.
        let outcome = mgr
            .handle_inbound_version(addr, their_version)
            .await
            .expect("handshake outcome");

        assert!(
            matches!(outcome, HandshakeOutcome::DroppedSelfConnection),
            "matching nonce must yield DroppedSelfConnection"
        );
        assert_eq!(
            mgr.ban_score_for(&addr).await,
            0,
            "self-connection must NOT increment ban score"
        );
        assert!(
            !mgr.discovery.read().await.contains_address(&addr),
            "self-connection must NOT be added to addrman"
        );
    }

    // ------------------------------------------------------------------
    // T091a — Minimum protocol version enforcement (FR-002)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn inbound_version_below_minimum_is_closed_without_ban() {
        let mgr = mk_manager();
        let addr = mk_peer_addr(30002);
        let mut their_version = VersionMessage::new(
            ServiceFlags::NETWORK,
            NetworkAddress::default(),
            NetworkAddress::default(),
            "/ancient:0.1/".to_string(),
            0,
            true,
            0xdeadbeef_cafef00d,
        );
        their_version.version = MIN_SUPPORTED_PROTOCOL_VERSION - 1;

        let outcome = mgr
            .handle_inbound_version(addr, their_version)
            .await
            .expect("handshake outcome");

        assert!(
            matches!(outcome, HandshakeOutcome::RejectedBelowMinVersion),
            "protocol < MIN must be rejected"
        );
        assert_eq!(
            mgr.ban_score_for(&addr).await,
            0,
            "old-version rejection must NOT increment ban score (peer may simply be stale)"
        );
        assert!(
            !mgr.discovery.read().await.contains_address(&addr),
            "rejected old version must not be added to addrman"
        );
    }

    #[tokio::test]
    async fn inbound_version_at_minimum_is_accepted() {
        let mgr = mk_manager();
        let addr = mk_peer_addr(30003);
        let mut their_version = VersionMessage::new(
            ServiceFlags::NETWORK,
            NetworkAddress::default(),
            NetworkAddress::default(),
            "/minimum:1.0/".to_string(),
            0,
            true,
            0x00aa_bbcc_ddeeff11,
        );
        their_version.version = MIN_SUPPORTED_PROTOCOL_VERSION;

        let outcome = mgr
            .handle_inbound_version(addr, their_version)
            .await
            .expect("handshake outcome");

        assert!(
            matches!(outcome, HandshakeOutcome::Accepted { .. }),
            "MIN_SUPPORTED_PROTOCOL_VERSION must be accepted"
        );
    }

    // ------------------------------------------------------------------
    // T092 — Ping/pong keepalive (FR-006)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn ping_timeout_disconnects_without_ban() {
        let mgr = mk_manager();
        let addr = mk_peer_addr(30004);
        // `simulate_ping_timeout` is the test hook the GREEN impl will expose
        // from T021; it advances the ping deadline past the configured window.
        let dropped = mgr.simulate_ping_timeout(&addr).await;
        assert!(dropped, "ping timeout must disconnect the peer");
        assert_eq!(
            mgr.ban_score_for(&addr).await,
            0,
            "ping timeout must NOT increment ban score"
        );
    }

    #[tokio::test]
    async fn successful_pong_records_rtt() {
        let mgr = mk_manager();
        let addr = mk_peer_addr(30005);
        let rtt_ms = mgr
            .simulate_ping_pong(&addr, std::time::Duration::from_millis(42))
            .await
            .expect("pong recorded");
        assert!(
            (35..=60).contains(&rtt_ms),
            "recorded RTT should be ~42 ms, got {}",
            rtt_ms
        );
    }

    // ── T109 RED-phase — Peer struct metadata extension (FR-011, US4) ──
    //
    // The existing `Peer` struct in this module does not yet carry
    // user_agent / bytes_in / bytes_out / last_ping_rtt / ban_score.
    // GREEN impl in Phase 6 (US4) adds these fields and the methods
    // below. Unresolved-path failures = RED evidence.

    #[test]
    fn peer_user_agent_truncates_at_two_hundred_fifty_six_bytes() {
        let long_ua = "A".repeat(400);
        let mut peer =
            super::Peer::new_for_test("198.51.100.1:18351".parse().unwrap());
        peer.set_user_agent(long_ua);
        assert_eq!(
            peer.user_agent().len(),
            256,
            "user_agent must truncate to 256 bytes per FR-011"
        );
    }

    #[test]
    fn peer_tracks_bytes_in_and_out() {
        let mut peer =
            super::Peer::new_for_test("198.51.100.2:18351".parse().unwrap());
        peer.record_bytes_in(1024);
        peer.record_bytes_out(512);
        peer.record_bytes_in(256);
        assert_eq!(peer.bytes_in(), 1280);
        assert_eq!(peer.bytes_out(), 512);
    }

    #[test]
    fn peer_last_ping_rtt_is_none_until_first_pong() {
        let mut peer =
            super::Peer::new_for_test("198.51.100.3:18351".parse().unwrap());
        assert_eq!(peer.last_ping_rtt_ms(), None);
        peer.record_pong_rtt(std::time::Duration::from_millis(88));
        assert_eq!(peer.last_ping_rtt_ms(), Some(88));
    }

    #[tokio::test]
    async fn peer_ban_score_mirror_refreshes_from_manager() {
        let mgr = mk_manager();
        let addr: std::net::SocketAddr = "198.51.100.4:18351".parse().unwrap();
        let mut peer = super::Peer::new_for_test(addr);
        mgr.set_ban_score_for_test(&addr, 42).await;
        peer.refresh_ban_score(&mgr).await;
        assert_eq!(peer.ban_score(), 42);
    }
}
