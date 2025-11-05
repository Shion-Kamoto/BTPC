//! Network layer for BTPC blockchain
//!
//! Provides Bitcoin-compatible P2P networking with quantum-resistant enhancements.

pub mod connection_tracker;
pub mod discovery;
pub mod integrated_sync;
pub mod peer_ban_manager;
pub mod protocol;
pub mod rate_limiter;
pub mod simple_peer_manager;
pub mod sync;

use std::{net::SocketAddr, time::Duration};

pub use connection_tracker::*;
pub use discovery::*;
pub use integrated_sync::{IntegratedSyncError, IntegratedSyncManager, SyncConfig, SyncStats};
pub use peer_ban_manager::*;
pub use protocol::*;
pub use rate_limiter::*;
pub use simple_peer_manager::{DisconnectReason, PeerEvent, SimplePeerManager};
pub use sync::*;
use thiserror::Error;

/// Progressive connection timeouts (Issue #8 - Slowloris attack prevention)
///
/// Implements different timeout values for different connection stages to prevent
/// slowloris-style attacks where an attacker holds connection slots with slow connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionTimeouts {
    /// TCP connection establishment timeout
    /// Default: 10 seconds
    pub tcp_connect: Duration,

    /// Protocol handshake timeout (version + verack exchange)
    /// Default: 15 seconds
    pub handshake_complete: Duration,

    /// Time to wait for first message after handshake
    /// Default: 30 seconds
    pub first_message: Duration,

    /// Interval for sending ping messages to detect dead connections
    /// Default: 120 seconds (2 minutes)
    pub ping_interval: Duration,

    /// Time to wait for pong response after ping
    /// Default: 20 seconds
    pub ping_timeout: Duration,

    /// Idle timeout - disconnect if no messages received
    /// Default: 300 seconds (5 minutes)
    pub idle_timeout: Duration,
}

impl ConnectionTimeouts {
    /// Create default timeout configuration
    ///
    /// Based on Bitcoin Core timeout values with adjustments for
    /// quantum signature processing overhead.
    pub fn default() -> Self {
        ConnectionTimeouts {
            tcp_connect: Duration::from_secs(10),
            handshake_complete: Duration::from_secs(15),
            first_message: Duration::from_secs(30),
            ping_interval: Duration::from_secs(120),
            ping_timeout: Duration::from_secs(20),
            idle_timeout: Duration::from_secs(300),
        }
    }

    /// Create aggressive timeout configuration for high-security environments
    ///
    /// Reduces all timeouts by ~50% to be more aggressive against slow attacks.
    pub fn aggressive() -> Self {
        ConnectionTimeouts {
            tcp_connect: Duration::from_secs(5),
            handshake_complete: Duration::from_secs(8),
            first_message: Duration::from_secs(15),
            ping_interval: Duration::from_secs(60),
            ping_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(150),
        }
    }

    /// Create relaxed timeout configuration for slow/unreliable networks
    ///
    /// Doubles timeouts to accommodate higher latency connections.
    pub fn relaxed() -> Self {
        ConnectionTimeouts {
            tcp_connect: Duration::from_secs(20),
            handshake_complete: Duration::from_secs(30),
            first_message: Duration::from_secs(60),
            ping_interval: Duration::from_secs(240),
            ping_timeout: Duration::from_secs(40),
            idle_timeout: Duration::from_secs(600),
        }
    }
}

impl Default for ConnectionTimeouts {
    fn default() -> Self {
        ConnectionTimeouts::default()
    }
}

/// Connection state tracking for timeout enforcement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// TCP connection being established
    TcpConnecting {
        /// When connection attempt started
        started_at: std::time::Instant,
    },

    /// Protocol handshake in progress (version/verack exchange)
    HandshakeStarted {
        /// When handshake started
        started_at: std::time::Instant,
    },

    /// Handshake complete, waiting for first substantive message
    AwaitingFirstMessage {
        /// When handshake completed
        completed_at: std::time::Instant,
    },

    /// Connection active and healthy
    Active {
        /// Last time a message was received
        last_message_at: std::time::Instant,
        /// Last time a ping was sent (None if no ping sent yet)
        last_ping_at: Option<std::time::Instant>,
    },
}

/// Timeout error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutError {
    /// TCP connection took too long
    TcpConnect,
    /// Handshake took too long
    Handshake,
    /// No first message after handshake
    FirstMessage,
    /// Ping response took too long
    PingTimeout,
    /// No messages received for too long
    Idle,
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeoutError::TcpConnect => write!(f, "TCP connection timeout"),
            TimeoutError::Handshake => write!(f, "Handshake timeout"),
            TimeoutError::FirstMessage => write!(f, "First message timeout"),
            TimeoutError::PingTimeout => write!(f, "Ping response timeout"),
            TimeoutError::Idle => write!(f, "Idle connection timeout"),
        }
    }
}

impl std::error::Error for TimeoutError {}

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Listen address
    pub listen_addr: SocketAddr,
    /// Maximum number of connections (total)
    pub max_connections: usize,
    /// Maximum connections per IP address (Issue #4 - Eclipse attack prevention)
    pub max_per_ip: usize,
    /// Maximum connections per /24 subnet (Issue #4)
    pub max_per_subnet_24: usize,
    /// Maximum connections per /16 subnet (Issue #4)
    pub max_per_subnet_16: usize,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Enable testnet
    pub testnet: bool,
    /// User agent string
    pub user_agent: String,
    /// Initial seed nodes
    pub seed_nodes: Vec<SocketAddr>,
    /// Rate limiter configuration
    pub rate_limiter: RateLimiterConfig,
    /// Event queue size (Issue #3 - bounded channels)
    pub event_queue_size: usize,
    /// Per-peer message queue size (Issue #3)
    pub peer_message_queue_size: usize,
    /// Progressive connection timeouts (Issue #8 - Slowloris attack prevention)
    pub timeouts: ConnectionTimeouts,
}

impl NetworkConfig {
    /// Create default mainnet configuration
    pub fn mainnet() -> Self {
        NetworkConfig {
            listen_addr: "0.0.0.0:8333"
                .parse()
                .expect("Valid mainnet listen address"),
            max_connections: 125,
            max_per_ip: 3,           // Bitcoin-style: max 3 connections per IP
            max_per_subnet_24: 10,   // Max 10 connections per /24 subnet
            max_per_subnet_16: 20,   // Max 20 connections per /16 subnet
            connection_timeout: 30,
            testnet: false,
            user_agent: "/BTPC:0.1.0/".to_string(),
            seed_nodes: vec![
                // Add mainnet seed nodes here
            ],
            rate_limiter: RateLimiterConfig::default(),
            event_queue_size: 10_000,      // 10K events max in queue
            peer_message_queue_size: 1000, // 1K messages per peer
            timeouts: ConnectionTimeouts::default(),
        }
    }

    /// Create default testnet configuration
    pub fn testnet() -> Self {
        NetworkConfig {
            listen_addr: "0.0.0.0:18333"
                .parse()
                .expect("Valid testnet listen address"),
            max_connections: 125,
            max_per_ip: 3,
            max_per_subnet_24: 10,
            max_per_subnet_16: 20,
            connection_timeout: 30,
            testnet: true,
            user_agent: "/BTPC:0.1.0-testnet/".to_string(),
            seed_nodes: vec![
                // Add testnet seed nodes here
            ],
            rate_limiter: RateLimiterConfig::default(),
            event_queue_size: 10_000,
            peer_message_queue_size: 1000,
            timeouts: ConnectionTimeouts::default(),
        }
    }

    /// Get network magic bytes
    pub fn magic_bytes(&self) -> [u8; 4] {
        if self.testnet {
            protocol::BTPC_TESTNET_MAGIC
        } else {
            protocol::BTPC_MAINNET_MAGIC
        }
    }
}

/// Network errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Peer discovery error: {0}")]
    Discovery(String),
    #[error("Sync error: {0}")]
    Sync(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Rate limit error: {0}")]
    RateLimit(#[from] RateLimitError),
    #[error("Connection limit error: {0}")]
    ConnectionLimit(#[from] ConnectionLimitError),
    #[error("Too many peers connected")]
    TooManyPeers,
    #[error("Too many connections from IP: {ip} (limit: {limit})")]
    TooManyFromIp { ip: std::net::IpAddr, limit: usize },
    #[error("Too many connections from subnet (limit: {limit})")]
    TooManyFromSubnet { limit: usize },
    #[error("Event queue full")]
    EventQueueFull,
    #[error("Peer message queue full")]
    PeerQueueFull,
    #[error("Peer is banned: {reason}")]
    PeerBanned { reason: BanReason },
    #[error("Connection timeout: {0}")]
    Timeout(#[from] TimeoutError),
}

/// Result type for network operations
pub type NetworkResult<T> = Result<T, NetworkError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_connection_timeouts_default() {
        let timeouts = ConnectionTimeouts::default();
        assert_eq!(timeouts.tcp_connect, Duration::from_secs(10));
        assert_eq!(timeouts.handshake_complete, Duration::from_secs(15));
        assert_eq!(timeouts.first_message, Duration::from_secs(30));
        assert_eq!(timeouts.ping_interval, Duration::from_secs(120));
        assert_eq!(timeouts.ping_timeout, Duration::from_secs(20));
        assert_eq!(timeouts.idle_timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_connection_timeouts_aggressive() {
        let timeouts = ConnectionTimeouts::aggressive();
        assert_eq!(timeouts.tcp_connect, Duration::from_secs(5));
        assert_eq!(timeouts.handshake_complete, Duration::from_secs(8));
        assert_eq!(timeouts.first_message, Duration::from_secs(15));
        assert_eq!(timeouts.ping_interval, Duration::from_secs(60));
        assert_eq!(timeouts.ping_timeout, Duration::from_secs(10));
        assert_eq!(timeouts.idle_timeout, Duration::from_secs(150));

        // Verify aggressive is ~50% of default
        let default = ConnectionTimeouts::default();
        assert!(timeouts.tcp_connect < default.tcp_connect);
        assert!(timeouts.handshake_complete < default.handshake_complete);
        assert!(timeouts.first_message < default.first_message);
    }

    #[test]
    fn test_connection_timeouts_relaxed() {
        let timeouts = ConnectionTimeouts::relaxed();
        assert_eq!(timeouts.tcp_connect, Duration::from_secs(20));
        assert_eq!(timeouts.handshake_complete, Duration::from_secs(30));
        assert_eq!(timeouts.first_message, Duration::from_secs(60));
        assert_eq!(timeouts.ping_interval, Duration::from_secs(240));
        assert_eq!(timeouts.ping_timeout, Duration::from_secs(40));
        assert_eq!(timeouts.idle_timeout, Duration::from_secs(600));

        // Verify relaxed is ~2x default
        let default = ConnectionTimeouts::default();
        assert!(timeouts.tcp_connect > default.tcp_connect);
        assert!(timeouts.handshake_complete > default.handshake_complete);
        assert!(timeouts.first_message > default.first_message);
    }

    #[test]
    fn test_connection_state_tcp_connecting() {
        let now = Instant::now();
        let state = ConnectionState::TcpConnecting { started_at: now };

        match state {
            ConnectionState::TcpConnecting { started_at } => {
                assert_eq!(started_at, now);
            }
            _ => panic!("Expected TcpConnecting state"),
        }
    }

    #[test]
    fn test_connection_state_handshake_started() {
        let now = Instant::now();
        let state = ConnectionState::HandshakeStarted { started_at: now };

        match state {
            ConnectionState::HandshakeStarted { started_at } => {
                assert_eq!(started_at, now);
            }
            _ => panic!("Expected HandshakeStarted state"),
        }
    }

    #[test]
    fn test_connection_state_awaiting_first_message() {
        let now = Instant::now();
        let state = ConnectionState::AwaitingFirstMessage { completed_at: now };

        match state {
            ConnectionState::AwaitingFirstMessage { completed_at } => {
                assert_eq!(completed_at, now);
            }
            _ => panic!("Expected AwaitingFirstMessage state"),
        }
    }

    #[test]
    fn test_connection_state_active() {
        let now = Instant::now();
        let state = ConnectionState::Active {
            last_message_at: now,
            last_ping_at: None,
        };

        match state {
            ConnectionState::Active {
                last_message_at,
                last_ping_at,
            } => {
                assert_eq!(last_message_at, now);
                assert_eq!(last_ping_at, None);
            }
            _ => panic!("Expected Active state"),
        }

        // Test with ping time
        let ping_time = Instant::now();
        let state_with_ping = ConnectionState::Active {
            last_message_at: now,
            last_ping_at: Some(ping_time),
        };

        match state_with_ping {
            ConnectionState::Active {
                last_message_at,
                last_ping_at,
            } => {
                assert_eq!(last_message_at, now);
                assert_eq!(last_ping_at, Some(ping_time));
            }
            _ => panic!("Expected Active state"),
        }
    }

    #[test]
    fn test_timeout_error_display() {
        assert_eq!(
            TimeoutError::TcpConnect.to_string(),
            "TCP connection timeout"
        );
        assert_eq!(TimeoutError::Handshake.to_string(), "Handshake timeout");
        assert_eq!(
            TimeoutError::FirstMessage.to_string(),
            "First message timeout"
        );
        assert_eq!(
            TimeoutError::PingTimeout.to_string(),
            "Ping response timeout"
        );
        assert_eq!(
            TimeoutError::Idle.to_string(),
            "Idle connection timeout"
        );
    }

    #[test]
    fn test_network_config_includes_timeouts() {
        let mainnet_config = NetworkConfig::mainnet();
        assert_eq!(
            mainnet_config.timeouts.tcp_connect,
            Duration::from_secs(10)
        );
        assert_eq!(
            mainnet_config.timeouts.handshake_complete,
            Duration::from_secs(15)
        );

        let testnet_config = NetworkConfig::testnet();
        assert_eq!(
            testnet_config.timeouts.tcp_connect,
            Duration::from_secs(10)
        );
        assert_eq!(
            testnet_config.timeouts.handshake_complete,
            Duration::from_secs(15)
        );
    }

    #[test]
    fn test_timeout_error_conversion() {
        // Test that TimeoutError converts to NetworkError
        let timeout_err = TimeoutError::Handshake;
        let network_err: NetworkError = timeout_err.into();

        match network_err {
            NetworkError::Timeout(te) => {
                assert_eq!(te, TimeoutError::Handshake);
            }
            _ => panic!("Expected NetworkError::Timeout"),
        }
    }

    #[test]
    fn test_connection_state_equality() {
        let now = Instant::now();

        let state1 = ConnectionState::TcpConnecting { started_at: now };
        let state2 = ConnectionState::TcpConnecting { started_at: now };
        assert_eq!(state1, state2);

        let state3 = ConnectionState::HandshakeStarted { started_at: now };
        assert_ne!(state1, state3);
    }

    #[test]
    fn test_timeout_comparison() {
        let default = ConnectionTimeouts::default();
        let aggressive = ConnectionTimeouts::aggressive();
        let relaxed = ConnectionTimeouts::relaxed();

        // Aggressive < Default < Relaxed
        assert!(aggressive.tcp_connect < default.tcp_connect);
        assert!(default.tcp_connect < relaxed.tcp_connect);

        assert!(aggressive.handshake_complete < default.handshake_complete);
        assert!(default.handshake_complete < relaxed.handshake_complete);

        assert!(aggressive.ping_interval < default.ping_interval);
        assert!(default.ping_interval < relaxed.ping_interval);
    }
}
