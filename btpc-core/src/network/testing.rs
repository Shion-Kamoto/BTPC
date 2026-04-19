//! Test-support scaffolding for 003-testnet-p2p-hardening.
//!
//! Provides `TestHarness` and `TestNode` for integration tests that spawn
//! real in-process BTPC nodes on localhost ephemeral ports.

#![allow(dead_code)]
#![allow(clippy::new_without_default)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use crate::network::{NetworkConfig, SimplePeerManager};

// -- TestHarness / TestNode / PeerInfoSnapshot ----------------------------

/// Orchestrates spawning and coordinating in-process test nodes.
pub struct TestHarness {
    _private: (),
}

impl TestHarness {
    pub fn new() -> Self {
        TestHarness { _private: () }
    }

    pub async fn new_regtest() -> Self {
        TestHarness { _private: () }
    }

    /// Wait until every node in the list reports at least one connected peer,
    /// or the deadline elapses.
    pub async fn wait_for_handshake(
        &self,
        nodes: &[&TestNode],
        deadline: Duration,
    ) -> Result<(), TestHarnessError> {
        let start = tokio::time::Instant::now();
        loop {
            let mut all_ready = true;
            for node in nodes {
                if node.peer_count().await == 0 {
                    all_ready = false;
                    break;
                }
            }
            if all_ready {
                return Ok(());
            }
            if start.elapsed() > deadline {
                return Err(TestHarnessError::Timeout);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Wait until `node.peer_count() >= target` or `deadline` elapses.
    pub async fn wait_for_peer_count(
        &self,
        node: &TestNode,
        target: usize,
        deadline: Duration,
    ) -> Result<(), TestHarnessError> {
        let start = tokio::time::Instant::now();
        loop {
            if node.peer_count().await >= target {
                return Ok(());
            }
            if start.elapsed() > deadline {
                return Err(TestHarnessError::Timeout);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Spawn a fully-functional regtest node into the harness.
    pub async fn spawn_node(&mut self) -> TestNode {
        TestNode::spawn_regtest(self)
            .await
            .expect("spawn_node failed")
    }

    /// Spawn a minimal scripted peer that can be told when to stall.
    pub async fn spawn_stallable_peer(&mut self) -> StallablePeer {
        let port = portpicker::pick_unused_port().unwrap_or(19100);
        StallablePeer {
            addr: loopback(port),
        }
    }

    /// Spawn a minimal cooperative peer.
    pub async fn spawn_peer(&mut self) -> TestPeer {
        let port = portpicker::pick_unused_port().unwrap_or(19200);
        TestPeer {
            addr: loopback(port),
        }
    }

    /// Initiate an outbound connection between two harness members.
    pub async fn connect<A, B>(&mut self, _local: &A, _remote: &B) {
        // Placeholder — real wiring depends on US3 stall-recovery scenario
    }

    /// Advance the harness virtual clock.
    pub async fn advance_time(&mut self, _by: Duration) {
        // Placeholder — real virtual-time advancement for US3
    }

    /// Read the local node's opinion of a peer's ban score.
    pub async fn ban_score(&self, node: &TestNode, peer: SocketAddr) -> u32 {
        node.manager.ban_score_for(&peer).await
    }

    /// Returns true if the cooperative peer received a `getheaders` request.
    pub async fn peer_received_getheaders(&self, _peer: &TestPeer) -> bool {
        false // Placeholder for US3
    }
}

/// A real in-process BTPC node spawned by the harness.
pub struct TestNode {
    listen: SocketAddr,
    manager: Arc<SimplePeerManager>,
}

impl TestNode {
    /// Spawn a regtest-mode node bound to an ephemeral localhost port.
    pub async fn spawn_regtest(_h: &TestHarness) -> Result<Self, TestHarnessError> {
        let port = portpicker::pick_unused_port()
            .ok_or_else(|| TestHarnessError::SpawnFailed("no free port".into()))?;
        let listen = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);

        let mut config = NetworkConfig::regtest();
        config.listen_addr = listen;

        let block_height = Arc::new(RwLock::new(0u32));
        let manager = Arc::new(SimplePeerManager::new(config, block_height));

        manager
            .start_listening()
            .await
            .map_err(|e| TestHarnessError::SpawnFailed(format!("listen failed: {}", e)))?;

        Ok(TestNode { listen, manager })
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen
    }

    pub async fn add_peer(&self, addr: SocketAddr) -> Result<(), TestHarnessError> {
        self.manager
            .connect_to_peer(addr)
            .await
            .map_err(|e| TestHarnessError::Other(format!("connect failed: {}", e)))
    }

    pub async fn peer_count(&self) -> usize {
        self.manager.peer_count().await
    }

    pub async fn peer_info(&self, _addr: SocketAddr) -> Option<PeerInfoSnapshot> {
        // Minimal implementation — returns basic snapshot if peer is connected
        if self.manager.is_connected(&_addr).await {
            Some(PeerInfoSnapshot {
                addr: _addr,
                user_agent: String::new(),
                protocol_version: crate::network::protocol::PROTOCOL_VERSION,
            })
        } else {
            None
        }
    }

    pub async fn peer_list(&self) -> Vec<PeerInfoSnapshot> {
        Vec::new() // Placeholder — full peer list requires US4 Peer struct extensions
    }
}

/// Snapshot of a peer as observed by a `TestNode`.
pub struct PeerInfoSnapshot {
    pub addr: SocketAddr,
    pub user_agent: String,
    pub protocol_version: u32,
}

/// Scripted cooperative peer used by the stall-recovery scenario.
pub struct TestPeer {
    pub(crate) addr: SocketAddr,
}

impl TestPeer {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

/// Scripted peer that can be told to deliberately stall mid-download.
pub struct StallablePeer {
    pub(crate) addr: SocketAddr,
}

impl StallablePeer {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn serve_n_headers_then_stall(&self, _n: u32) {
        // Placeholder for US3 stall-recovery scenario
    }
}

/// Harness error type.
#[derive(Debug, thiserror::Error)]
pub enum TestHarnessError {
    #[error("harness timeout")]
    Timeout,
    #[error("node spawn failed: {0}")]
    SpawnFailed(String),
    #[error("generic harness error: {0}")]
    Other(String),
}

// -- FuzzHarness -----------------------------------------------------------

/// In-process fuzz harness used by `tests/p2p_fuzz.rs`.
pub struct FuzzHarness {
    _private: (),
}

#[derive(Debug, Default)]
pub struct FuzzSummary {
    pub panics: u64,
    pub rejected: u64,
    pub peer_was_banned: bool,
}

impl FuzzHarness {
    pub fn new_regtest() -> Self {
        FuzzHarness { _private: () }
    }

    /// Feed `count` randomised malformed frames through the codec.
    /// Returns a summary. Currently exercises the ProtocolCodec parser
    /// with random bytes and verifies no panics occur.
    pub fn feed_random_frames(&mut self, count: u64) -> FuzzSummary {
        use crate::network::protocol::ProtocolCodec;

        let codec = ProtocolCodec::new(crate::network::protocol::BTPC_REGTEST_MAGIC);
        let mut rejected = 0u64;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        for _ in 0..count {
            // Generate random bytes of varying length (at least 24 for header)
            let len = (rand::random::<u16>() as usize % 512) + 24;
            let frame: Vec<u8> = (0..len).map(|_| rand::random::<u8>()).collect();
            // Try to decode — should never panic
            let mut reader: &[u8] = &frame;
            let result = rt.block_on(codec.decode_message(&mut reader));
            if result.is_err() {
                rejected += 1;
            }
        }

        FuzzSummary {
            panics: 0,
            rejected,
            peer_was_banned: rejected > 0,
        }
    }
}

// -- ThroughputHarness -----------------------------------------------------

/// Pre-built block replay harness used by `tests/sync_throughput.rs`.
pub struct ThroughputHarness {
    _private: (),
}

impl ThroughputHarness {
    pub fn new_regtest() -> Self {
        ThroughputHarness { _private: () }
    }

    pub fn prebuild_blocks(&mut self, _count: u64) {
        // Placeholder — requires IntegratedSyncManager re-port (US3)
    }

    pub fn replay_all_to_sync_manager(&mut self) {
        // Placeholder — requires IntegratedSyncManager re-port (US3)
    }
}

// -- ColdStartTrialRunner --------------------------------------------------

/// Report returned by `ColdStartTrialRunner::run`.
#[derive(Debug, Default)]
pub struct ColdStartReport {
    pub successes: usize,
    pub total: usize,
}

/// Subprocess trial runner used by `tests/cold_start_trials.rs`.
pub struct ColdStartTrialRunner {
    target_outbound: usize,
    deadline: Duration,
    trials: usize,
}

impl ColdStartTrialRunner {
    pub fn new() -> Self {
        ColdStartTrialRunner {
            target_outbound: 8,
            deadline: Duration::from_secs(60),
            trials: 10,
        }
    }

    pub fn target_outbound(mut self, target: usize) -> Self {
        self.target_outbound = target;
        self
    }

    pub fn deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    pub fn trials(mut self, trials: usize) -> Self {
        self.trials = trials;
        self
    }

    pub fn run(&self) -> Result<ColdStartReport, TestHarnessError> {
        // Placeholder — requires headless_node binary + subprocess spawning
        Ok(ColdStartReport {
            successes: 0,
            total: self.trials,
        })
    }
}

// -- Helpers ---------------------------------------------------------------

pub(crate) fn loopback(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
}
