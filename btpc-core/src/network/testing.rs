//! Test-support scaffolding for 003-testnet-p2p-hardening.
//!
//! This module exposes the public API surface that the integration
//! tests under `btpc-core/tests/` reference (`TestHarness`, `TestNode`,
//! `StallablePeer`, `FuzzHarness`, `ColdStartTrialRunner`,
//! `ThroughputHarness`). The goal of GREEN US1 (T030) is to bring these
//! types to a **compile-green** state so the remaining integration
//! tests can run or be `#[ignore]`-gated without blocking `cargo check
//! --tests -p btpc-core`.
//!
//! Richer runtime behaviour is introduced incrementally by the later
//! GREEN tasks:
//!
//! * T030 — real two-node handshake + addr-gossip harness (US1)
//! * T031 — fuzz frame generator (SC-003)
//! * T032 — stall reaper + bounded window integration (US3)
//! * T032a — throughput harness (SC-002)
//! * T032b — cold-start trial runner (SC-001)
//!
//! Every method that has not yet been implemented for runtime use
//! intentionally calls [`todo!`] — integration tests that exercise it
//! will panic with a clear message at runtime, while the whole module
//! still type-checks.

#![allow(dead_code)]
#![allow(clippy::new_without_default)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

// -- TestHarness / TestNode / PeerInfoSnapshot ----------------------------

/// Orchestrates spawning and coordinating in-process test nodes.
pub struct TestHarness {
    _private: (),
}

impl TestHarness {
    /// Construct a fresh harness. Synchronous variant used by the
    /// handshake + addr-gossip integration tests.
    pub fn new() -> Self {
        TestHarness { _private: () }
    }

    /// Construct a harness configured for regtest-mode. Async flavour
    /// is used by the stall-recovery scenario.
    pub async fn new_regtest() -> Self {
        TestHarness { _private: () }
    }

    /// Wait until every node in the list reports a completed handshake
    /// with at least one peer. Returns once the quorum is reached or
    /// the deadline elapses.
    pub async fn wait_for_handshake(
        &self,
        _nodes: &[&TestNode],
        _deadline: Duration,
    ) -> Result<(), TestHarnessError> {
        todo!("T030: real handshake wait loop")
    }

    /// Wait until `node.peer_count() >= target` or `deadline` elapses.
    pub async fn wait_for_peer_count(
        &self,
        _node: &TestNode,
        _target: usize,
        _deadline: Duration,
    ) -> Result<(), TestHarnessError> {
        todo!("T030: real peer-count wait loop")
    }

    /// Spawn a fully-functional regtest node into the harness
    /// (stall-recovery scenario).
    pub async fn spawn_node(&mut self) -> TestNode {
        todo!("T032: real node spawn for stall-recovery harness")
    }

    /// Spawn a minimal scripted peer that can be told when to stall.
    pub async fn spawn_stallable_peer(&mut self) -> StallablePeer {
        todo!("T032: spawn stallable scripted peer")
    }

    /// Spawn a minimal cooperative peer.
    pub async fn spawn_peer(&mut self) -> TestPeer {
        todo!("T032: spawn cooperative scripted peer")
    }

    /// Initiate an outbound connection between two harness members.
    pub async fn connect<A, B>(&mut self, _local: &A, _remote: &B) {
        todo!("T032: driven connect")
    }

    /// Advance the harness virtual clock (used by stall-recovery to
    /// fast-forward past the 30-second stall window).
    pub async fn advance_time(&mut self, _by: Duration) {
        todo!("T032: virtual-time advancement")
    }

    /// Read the local node's opinion of a peer's ban score.
    pub async fn ban_score(&self, _node: &TestNode, _peer: SocketAddr) -> u32 {
        todo!("T032: ban-score lookup")
    }

    /// Returns true if the cooperative peer received a `getheaders`
    /// request from the local node since connection.
    pub async fn peer_received_getheaders(&self, _peer: &TestPeer) -> bool {
        todo!("T032: observe getheaders re-issue")
    }
}

/// A real in-process BTPC node spawned by the harness.
pub struct TestNode {
    listen: SocketAddr,
}

impl TestNode {
    /// Spawn a regtest-mode node bound to an ephemeral localhost port.
    pub async fn spawn_regtest(_h: &TestHarness) -> Result<Self, TestHarnessError> {
        // Deliberate compile-only stub so the integration test can be
        // linked; runtime impl lands in T030 GREEN US1 follow-up.
        todo!("T030: spawn real regtest TestNode")
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen
    }

    pub async fn add_peer(&self, _addr: SocketAddr) -> Result<(), TestHarnessError> {
        todo!("T030: manual peer seed")
    }

    pub async fn peer_count(&self) -> usize {
        todo!("T030: peer count")
    }

    pub async fn peer_info(&self, _addr: SocketAddr) -> Option<PeerInfoSnapshot> {
        todo!("T030: peer info lookup")
    }

    pub async fn peer_list(&self) -> Vec<PeerInfoSnapshot> {
        todo!("T030: peer list")
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
        todo!("T032: scripted header delivery + stall")
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

    /// Feed `count` randomised malformed frames through the codec and
    /// return a summary describing what happened.
    pub fn feed_random_frames(&mut self, _count: u64) -> FuzzSummary {
        todo!("T032a: fuzz frame generator + codec feeder")
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
        todo!("T032a: pre-build block chain for throughput replay")
    }

    pub fn replay_all_to_sync_manager(&mut self) {
        todo!("T032a: replay pre-built blocks to IntegratedSyncManager")
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
        todo!("T032b: run 10 subprocess cold-start trials")
    }
}

// -- Helpers ---------------------------------------------------------------

pub(crate) fn loopback(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
}
