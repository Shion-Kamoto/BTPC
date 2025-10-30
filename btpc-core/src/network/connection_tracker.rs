//! Connection tracking and per-IP limits (Issue #4 - HIGH)
//!
//! Tracks active connections by IP address and subnet to prevent eclipse attacks.
//! Enforces limits on:
//! - Connections per individual IP address
//! - Connections per /24 subnet (e.g., 192.168.1.0/24)
//! - Connections per /16 subnet (e.g., 192.168.0.0/16)

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

use crate::network::NetworkConfig;

/// Tracks connections by IP and subnet to enforce limits
pub struct ConnectionTracker {
    /// Number of connections per IP address
    by_ip: HashMap<IpAddr, usize>,
    /// Number of connections per /24 subnet (IPv4)
    by_subnet_24: HashMap<u32, usize>,
    /// Number of connections per /16 subnet (IPv4)
    by_subnet_16: HashMap<u32, usize>,
    /// Number of connections per /64 subnet (IPv6)
    by_subnet_64: HashMap<u128, usize>,
    /// Total connection count
    total_count: usize,
}

impl ConnectionTracker {
    /// Create a new connection tracker
    pub fn new() -> Self {
        ConnectionTracker {
            by_ip: HashMap::new(),
            by_subnet_24: HashMap::new(),
            by_subnet_16: HashMap::new(),
            by_subnet_64: HashMap::new(),
            total_count: 0,
        }
    }

    /// Check if a connection from this address would be accepted
    ///
    /// Returns Ok(()) if within limits, Err with reason if over limit.
    pub fn can_accept(&self, addr: &SocketAddr, config: &NetworkConfig) -> Result<(), ConnectionLimitError> {
        let ip = addr.ip();

        // Check total connection limit
        if self.total_count >= config.max_connections {
            return Err(ConnectionLimitError::TotalLimitExceeded {
                current: self.total_count,
                limit: config.max_connections,
            });
        }

        // Check per-IP limit
        let ip_count = self.by_ip.get(&ip).unwrap_or(&0);
        if *ip_count >= config.max_per_ip {
            return Err(ConnectionLimitError::PerIpLimitExceeded {
                ip,
                current: *ip_count,
                limit: config.max_per_ip,
            });
        }

        // Check subnet limits based on IP version
        match ip {
            IpAddr::V4(ipv4) => {
                self.check_ipv4_subnet_limits(ipv4, config)?;
            }
            IpAddr::V6(ipv6) => {
                self.check_ipv6_subnet_limits(ipv6, config)?;
            }
        }

        Ok(())
    }

    /// Check IPv4 subnet limits
    fn check_ipv4_subnet_limits(&self, ipv4: Ipv4Addr, config: &NetworkConfig) -> Result<(), ConnectionLimitError> {
        let ip_u32 = u32::from(ipv4);

        // Check /24 subnet (e.g., 192.168.1.0/24)
        let subnet_24 = ip_u32 & 0xFFFFFF00;
        let count_24 = self.by_subnet_24.get(&subnet_24).unwrap_or(&0);
        if *count_24 >= config.max_per_subnet_24 {
            return Err(ConnectionLimitError::SubnetLimitExceeded {
                subnet: format!("{}.{}.{}.0/24",
                    (subnet_24 >> 24) & 0xFF,
                    (subnet_24 >> 16) & 0xFF,
                    (subnet_24 >> 8) & 0xFF,
                ),
                current: *count_24,
                limit: config.max_per_subnet_24,
            });
        }

        // Check /16 subnet (e.g., 192.168.0.0/16)
        let subnet_16 = ip_u32 & 0xFFFF0000;
        let count_16 = self.by_subnet_16.get(&subnet_16).unwrap_or(&0);
        if *count_16 >= config.max_per_subnet_16 {
            return Err(ConnectionLimitError::SubnetLimitExceeded {
                subnet: format!("{}.{}.0.0/16",
                    (subnet_16 >> 24) & 0xFF,
                    (subnet_16 >> 16) & 0xFF,
                ),
                current: *count_16,
                limit: config.max_per_subnet_16,
            });
        }

        Ok(())
    }

    /// Check IPv6 subnet limits
    fn check_ipv6_subnet_limits(&self, ipv6: Ipv6Addr, config: &NetworkConfig) -> Result<(), ConnectionLimitError> {
        let ip_u128 = u128::from(ipv6);

        // Check /64 subnet (standard IPv6 subnet)
        let subnet_64 = ip_u128 & 0xFFFFFFFFFFFFFFFF_0000000000000000;
        let count_64 = self.by_subnet_64.get(&subnet_64).unwrap_or(&0);

        // Use max_per_subnet_24 as limit for IPv6 /64 subnets (reasonable default)
        if *count_64 >= config.max_per_subnet_24 {
            return Err(ConnectionLimitError::SubnetLimitExceeded {
                subnet: format!("{:x}::/64", subnet_64 >> 64),
                current: *count_64,
                limit: config.max_per_subnet_24,
            });
        }

        Ok(())
    }

    /// Register a new connection from this address
    ///
    /// Should only be called after can_accept() returns Ok.
    /// Panics if limits would be exceeded (caller should check first).
    pub fn add_connection(&mut self, addr: &SocketAddr) {
        let ip = addr.ip();

        // Increment total count
        self.total_count += 1;

        // Increment per-IP count
        *self.by_ip.entry(ip).or_insert(0) += 1;

        // Increment subnet counts
        match ip {
            IpAddr::V4(ipv4) => {
                let ip_u32 = u32::from(ipv4);
                let subnet_24 = ip_u32 & 0xFFFFFF00;
                let subnet_16 = ip_u32 & 0xFFFF0000;

                *self.by_subnet_24.entry(subnet_24).or_insert(0) += 1;
                *self.by_subnet_16.entry(subnet_16).or_insert(0) += 1;
            }
            IpAddr::V6(ipv6) => {
                let ip_u128 = u128::from(ipv6);
                let subnet_64 = ip_u128 & 0xFFFFFFFFFFFFFFFF_0000000000000000;

                *self.by_subnet_64.entry(subnet_64).or_insert(0) += 1;
            }
        }
    }

    /// Remove a connection for this address
    pub fn remove_connection(&mut self, addr: &SocketAddr) {
        let ip = addr.ip();

        // Decrement total count
        self.total_count = self.total_count.saturating_sub(1);

        // Decrement per-IP count
        if let Some(count) = self.by_ip.get_mut(&ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.by_ip.remove(&ip);
            }
        }

        // Decrement subnet counts
        match ip {
            IpAddr::V4(ipv4) => {
                let ip_u32 = u32::from(ipv4);
                let subnet_24 = ip_u32 & 0xFFFFFF00;
                let subnet_16 = ip_u32 & 0xFFFF0000;

                Self::decrement_subnet(&mut self.by_subnet_24, subnet_24);
                Self::decrement_subnet(&mut self.by_subnet_16, subnet_16);
            }
            IpAddr::V6(ipv6) => {
                let ip_u128 = u128::from(ipv6);
                let subnet_64 = ip_u128 & 0xFFFFFFFFFFFFFFFF_0000000000000000;

                Self::decrement_subnet_u128(&mut self.by_subnet_64, subnet_64);
            }
        }
    }

    /// Helper to decrement subnet count
    fn decrement_subnet<K: Eq + std::hash::Hash>(map: &mut HashMap<K, usize>, key: K) {
        if let Some(count) = map.get_mut(&key) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                map.remove(&key);
            }
        }
    }

    /// Helper to decrement u128 subnet count
    fn decrement_subnet_u128(map: &mut HashMap<u128, usize>, key: u128) {
        Self::decrement_subnet(map, key);
    }

    /// Get total number of active connections
    pub fn total_connections(&self) -> usize {
        self.total_count
    }

    /// Get number of connections from a specific IP
    pub fn connections_from_ip(&self, ip: &IpAddr) -> usize {
        *self.by_ip.get(ip).unwrap_or(&0)
    }

    /// Get statistics for monitoring
    pub fn stats(&self) -> ConnectionTrackerStats {
        ConnectionTrackerStats {
            total_connections: self.total_count,
            unique_ips: self.by_ip.len(),
            ipv4_subnets_24: self.by_subnet_24.len(),
            ipv4_subnets_16: self.by_subnet_16.len(),
            ipv6_subnets_64: self.by_subnet_64.len(),
        }
    }

    /// Clear all connection tracking (for testing/reset)
    pub fn clear(&mut self) {
        self.by_ip.clear();
        self.by_subnet_24.clear();
        self.by_subnet_16.clear();
        self.by_subnet_64.clear();
        self.total_count = 0;
    }
}

impl Default for ConnectionTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection tracker statistics
#[derive(Debug, Clone)]
pub struct ConnectionTrackerStats {
    /// Total number of connections
    pub total_connections: usize,
    /// Number of unique IP addresses connected
    pub unique_ips: usize,
    /// Number of unique IPv4 /24 subnets
    pub ipv4_subnets_24: usize,
    /// Number of unique IPv4 /16 subnets
    pub ipv4_subnets_16: usize,
    /// Number of unique IPv6 /64 subnets
    pub ipv6_subnets_64: usize,
}

/// Connection limit errors
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionLimitError {
    /// Total connection limit exceeded
    TotalLimitExceeded { current: usize, limit: usize },
    /// Per-IP limit exceeded
    PerIpLimitExceeded { ip: IpAddr, current: usize, limit: usize },
    /// Subnet limit exceeded
    SubnetLimitExceeded { subnet: String, current: usize, limit: usize },
}

impl std::fmt::Display for ConnectionLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionLimitError::TotalLimitExceeded { current, limit } => {
                write!(f, "Total connection limit exceeded: {} >= {}", current, limit)
            }
            ConnectionLimitError::PerIpLimitExceeded { ip, current, limit } => {
                write!(f, "Per-IP limit exceeded for {}: {} >= {}", ip, current, limit)
            }
            ConnectionLimitError::SubnetLimitExceeded { subnet, current, limit } => {
                write!(f, "Subnet limit exceeded for {}: {} >= {}", subnet, current, limit)
            }
        }
    }
}

impl std::error::Error for ConnectionLimitError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> NetworkConfig {
        NetworkConfig {
            listen_addr: "0.0.0.0:8333".parse().unwrap(),
            max_connections: 10,
            max_per_ip: 2,
            max_per_subnet_24: 5,
            max_per_subnet_16: 8,
            connection_timeout: 30,
            testnet: false,
            user_agent: "/BTPC:0.1.0/".to_string(),
            seed_nodes: vec![],
            rate_limiter: crate::network::RateLimiterConfig::default(),
            event_queue_size: 1000,
            peer_message_queue_size: 100,
            timeouts: crate::network::ConnectionTimeouts::default(),
        }
    }

    #[test]
    fn test_connection_tracker_creation() {
        let tracker = ConnectionTracker::new();
        assert_eq!(tracker.total_connections(), 0);
    }

    #[test]
    fn test_per_ip_limit() {
        let mut tracker = ConnectionTracker::new();
        let config = test_config();

        let addr1 = "192.168.1.100:8333".parse().unwrap();
        let addr2 = "192.168.1.100:8334".parse().unwrap(); // Same IP, different port

        // First connection from IP should be accepted
        assert!(tracker.can_accept(&addr1, &config).is_ok());
        tracker.add_connection(&addr1);

        // Second connection from same IP should be accepted (limit is 2)
        assert!(tracker.can_accept(&addr2, &config).is_ok());
        tracker.add_connection(&addr2);

        // Third connection from same IP should be rejected
        let addr3 = "192.168.1.100:8335".parse().unwrap();
        assert!(matches!(
            tracker.can_accept(&addr3, &config),
            Err(ConnectionLimitError::PerIpLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_subnet_24_limit() {
        let mut tracker = ConnectionTracker::new();
        let config = test_config();

        // Add 5 connections from different IPs in same /24 subnet (192.168.1.0/24)
        for i in 1..=5 {
            let addr: SocketAddr = format!("192.168.1.{}:8333", i).parse().unwrap();
            assert!(tracker.can_accept(&addr, &config).is_ok(), "Connection {} should be accepted", i);
            tracker.add_connection(&addr);
        }

        // 6th connection from same /24 subnet should be rejected
        let addr6 = "192.168.1.100:8333".parse().unwrap();
        assert!(matches!(
            tracker.can_accept(&addr6, &config),
            Err(ConnectionLimitError::SubnetLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_subnet_16_limit() {
        let mut tracker = ConnectionTracker::new();
        let config = test_config();

        // Add 8 connections from different /24 subnets within same /16 subnet (192.168.0.0/16)
        // We need 2 per /24 to reach 8 total without hitting /24 limit (5)
        for i in 1..=4 {
            for j in 1..=2 {
                let addr: SocketAddr = format!("192.168.{}.{}:8333", i, j).parse().unwrap();
                assert!(tracker.can_accept(&addr, &config).is_ok());
                tracker.add_connection(&addr);
            }
        }

        assert_eq!(tracker.total_connections(), 8);

        // 9th connection from same /16 subnet should be rejected
        let addr9 = "192.168.5.1:8333".parse().unwrap();
        assert!(matches!(
            tracker.can_accept(&addr9, &config),
            Err(ConnectionLimitError::SubnetLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_total_limit() {
        let mut tracker = ConnectionTracker::new();
        let config = test_config();

        // Add 10 connections from different /16 subnets to avoid subnet limits
        // Use 10.0.x.1, 11.0.x.1, 12.0.x.1, etc. (different /16 subnets)
        for i in 0..10 {
            let addr: SocketAddr = format!("{}.0.0.1:8333", 10 + i).parse().unwrap();
            assert!(tracker.can_accept(&addr, &config).is_ok());
            tracker.add_connection(&addr);
        }

        // 11th connection should be rejected (total limit exceeded)
        let addr11 = "20.0.0.1:8333".parse().unwrap();
        assert!(matches!(
            tracker.can_accept(&addr11, &config),
            Err(ConnectionLimitError::TotalLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_remove_connection() {
        let mut tracker = ConnectionTracker::new();
        let config = test_config();

        let addr = "192.168.1.100:8333".parse().unwrap();

        tracker.add_connection(&addr);
        assert_eq!(tracker.total_connections(), 1);
        assert_eq!(tracker.connections_from_ip(&"192.168.1.100".parse().unwrap()), 1);

        tracker.remove_connection(&addr);
        assert_eq!(tracker.total_connections(), 0);
        assert_eq!(tracker.connections_from_ip(&"192.168.1.100".parse().unwrap()), 0);
    }

    #[test]
    fn test_ipv6_support() {
        let mut tracker = ConnectionTracker::new();
        let config = test_config();

        let addr1 = "[2001:db8::1]:8333".parse().unwrap();
        let addr2 = "[2001:db8::2]:8333".parse().unwrap();

        assert!(tracker.can_accept(&addr1, &config).is_ok());
        tracker.add_connection(&addr1);

        assert!(tracker.can_accept(&addr2, &config).is_ok());
        tracker.add_connection(&addr2);

        assert_eq!(tracker.total_connections(), 2);
    }

    #[test]
    fn test_stats() {
        let mut tracker = ConnectionTracker::new();
        let config = test_config();

        let addr1 = "192.168.1.1:8333".parse().unwrap();
        let addr2 = "192.168.1.2:8333".parse().unwrap();
        let addr3 = "192.168.2.1:8333".parse().unwrap();

        tracker.add_connection(&addr1);
        tracker.add_connection(&addr2);
        tracker.add_connection(&addr3);

        let stats = tracker.stats();
        assert_eq!(stats.total_connections, 3);
        assert_eq!(stats.unique_ips, 3);
        assert_eq!(stats.ipv4_subnets_24, 2); // 192.168.1.0/24 and 192.168.2.0/24
        assert_eq!(stats.ipv4_subnets_16, 1); // 192.168.0.0/16
    }
}