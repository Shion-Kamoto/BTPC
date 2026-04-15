//! Peer discovery mechanisms
//!
//! Handles discovering and connecting to network peers.

use std::{
    collections::{HashMap, HashSet},
    fs,
    net::{IpAddr, SocketAddr},
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use thiserror::Error;
use tokio::time::Instant;

use crate::network::{NetworkAddress, NetworkError, ServiceFlags};

/// Peer discovery manager
pub struct PeerDiscovery {
    /// Known peer addresses
    addresses: HashMap<SocketAddr, PeerInfo>,
    /// DNS seed addresses
    dns_seeds: Vec<String>,
    /// Maximum addresses to store
    max_addresses: usize,
    /// Last DNS query time
    last_dns_query: Option<Instant>,
    /// DNS query interval
    dns_query_interval: Duration,
    /// Default P2P port for this network (used when resolving DNS seeds)
    default_port: u16,
}

/// Information about a peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Network address
    pub address: NetworkAddress,
    /// Last seen timestamp
    pub last_seen: SystemTime,
    /// First seen timestamp (for longevity calculation)
    pub first_seen: SystemTime,
    /// Connection attempts
    pub attempts: u32,
    /// Failed connection attempts
    pub failed_attempts: u32,
    /// Last attempt timestamp
    pub last_attempt: Option<SystemTime>,
    /// Services provided
    pub services: ServiceFlags,
    /// Connection success rate
    pub success_rate: f32,
    /// Average latency (if known)
    pub avg_latency: Option<Duration>,
    /// Misbehavior score (for penalty calculation)
    pub misbehavior_score: u32,
}

/// Discovery errors
#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error("DNS resolution failed: {0}")]
    DnsError(String),
    #[error("No peers available")]
    NoPeers,
    #[error("Address parsing error: {0}")]
    AddressParse(String),
    #[error("Network error: {0}")]
    Network(String),
}

impl PeerDiscovery {
    /// Create a new peer discovery manager (defaults to mainnet seeds)
    pub fn new(max_addresses: usize) -> Self {
        Self::new_for_network(max_addresses, crate::Network::Mainnet)
    }

    /// Get the default P2P port for this network
    pub fn default_port(&self) -> u16 {
        self.default_port
    }

    /// Create a peer discovery manager with network-specific DNS seeds.
    ///
    /// - **Mainnet**: 3 DNS seeds (`seed.btpc.org`, `seed1/2.btpc.network`)
    /// - **Testnet**: 2 DNS seeds (`testnet-seed.btpc.org`, `testnet-seed1.btpc.network`)
    /// - **Regtest**: no DNS seeds (local-only network)
    pub fn new_for_network(max_addresses: usize, network: crate::Network) -> Self {
        let default_port = match network {
            crate::Network::Mainnet => 18341,
            crate::Network::Testnet => 18351,
            crate::Network::Regtest => 18361,
        };
        PeerDiscovery {
            addresses: HashMap::new(),
            dns_seeds: Self::dns_seeds_for_network(network),
            max_addresses,
            last_dns_query: None,
            dns_query_interval: Duration::from_secs(300), // 5 minutes
            default_port,
        }
    }

    /// DNS seed addresses per network
    fn dns_seeds_for_network(network: crate::Network) -> Vec<String> {
        match network {
            crate::Network::Mainnet => vec![
                "seed.btpc.org".to_string(),
                "seed1.btpc.network".to_string(),
                "seed2.btpc.network".to_string(),
            ],
            crate::Network::Testnet => vec![
                "testnet-seed.btpc.org".to_string(),
                "testnet-seed1.btpc.network".to_string(),
            ],
            crate::Network::Regtest => vec![
                // Regtest is local-only — no DNS seeds
            ],
        }
    }

    /// Default DNS seed addresses for mainnet (legacy helper)
    fn default_dns_seeds() -> Vec<String> {
        Self::dns_seeds_for_network(crate::Network::Mainnet)
    }

    /// Add a peer address
    pub fn add_address(&mut self, addr: SocketAddr, services: ServiceFlags) {
        // If we're at capacity, remove oldest address
        if self.addresses.len() >= self.max_addresses {
            self.evict_oldest();
        }

        let now = SystemTime::now();
        let network_addr = NetworkAddress::new(addr.ip(), addr.port(), services);
        let peer_info = PeerInfo {
            address: network_addr,
            last_seen: now,
            first_seen: now,
            attempts: 0,
            failed_attempts: 0,
            last_attempt: None,
            services,
            success_rate: 0.0,
            avg_latency: None,
            misbehavior_score: 0,
        };

        self.addresses.insert(addr, peer_info);
    }

    /// Add a peer address with an explicit `last_seen` timestamp
    /// (003-testnet-p2p-hardening — used by `addr` gossip ingestion).
    pub fn add_address_with_last_seen(
        &mut self,
        addr: SocketAddr,
        services: ServiceFlags,
        last_seen: SystemTime,
    ) {
        if self.addresses.len() >= self.max_addresses {
            self.evict_oldest();
        }
        let network_addr = NetworkAddress::new(addr.ip(), addr.port(), services);
        let peer_info = PeerInfo {
            address: network_addr,
            last_seen,
            first_seen: last_seen,
            attempts: 0,
            failed_attempts: 0,
            last_attempt: None,
            services,
            success_rate: 0.0,
            avg_latency: None,
            misbehavior_score: 0,
        };
        self.addresses.insert(addr, peer_info);
    }

    /// Add multiple addresses from addr message
    pub fn add_addresses(&mut self, addresses: Vec<NetworkAddress>) {
        for addr in addresses {
            let socket_addr = SocketAddr::new(addr.ip, addr.port);
            let services = ServiceFlags(addr.services);
            self.add_address(socket_addr, services);
        }
    }

    /// Get a list of peer addresses to connect to
    pub fn get_peers(&mut self, count: usize, required_services: ServiceFlags) -> Vec<SocketAddr> {
        let now = SystemTime::now();
        let mut candidates: Vec<_> = self
            .addresses
            .iter()
            .filter(|(_, info)| {
                // Filter by required services
                ServiceFlags(info.services.0 & required_services.0).0 == required_services.0
            })
            .filter(|(_, info)| {
                // Don't retry recently failed attempts
                match info.last_attempt {
                    Some(last_attempt) => {
                        now.duration_since(last_attempt)
                            .unwrap_or(Duration::ZERO)
                            .as_secs()
                            > 60 // Wait at least 1 minute
                    }
                    None => true,
                }
            })
            .map(|(addr, info)| (*addr, info))
            .collect();

        // Sort by success rate and last seen
        candidates.sort_by(|a, b| {
            let score_a = self.calculate_peer_score(a.1);
            let score_b = self.calculate_peer_score(b.1);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates
            .into_iter()
            .take(count)
            .map(|(addr, _)| addr)
            .collect()
    }

    /// Calculate peer score for selection (Enhanced algorithm - Issue #9)
    ///
    /// Implements sophisticated scoring that is harder to game:
    /// - Longevity bonus: Rewards peers seen for long periods (harder to fake)
    /// - Latency scoring: Prefers low-latency peers
    /// - Misbehavior penalty: Heavy penalty for bad behavior
    /// - Diversity bonus: Rewards peers from underrepresented subnets
    /// - Squared attempt penalty: Exponentially penalizes repeated failures
    fn calculate_peer_score(&self, info: &PeerInfo) -> f32 {
        // 1. Base score from success rate (0-100)
        let base_score = info.success_rate * 100.0;

        // 2. Longevity bonus: Reward long uptime (harder to fake)
        // First seen age in days, capped at 30 days
        let longevity_bonus = match info.first_seen.elapsed() {
            Ok(elapsed) => {
                let days = elapsed.as_secs() as f32 / 86400.0;
                days.min(30.0)
            }
            Err(_) => 0.0,
        };

        // 3. Latency score: Prefer low-latency peers
        let latency_score = match info.avg_latency {
            Some(latency) if latency < Duration::from_millis(100) => 20.0,
            Some(latency) if latency < Duration::from_millis(500) => 10.0,
            Some(_) => 0.0,
            None => 5.0, // Neutral for unknown
        };

        // 4. Heavy misbehavior penalty
        let misbehavior_penalty = -(info.misbehavior_score as f32 * 10.0);

        // 5. Network diversity bonus
        // Check if peer is from underrepresented subnet
        let diversity_bonus = if self.is_diverse_peer(info) {
            15.0
        } else {
            0.0
        };

        // 6. Squared penalty for repeated failed attempts (exponential)
        let attempt_penalty = (info.failed_attempts as f32).powi(2) * 0.1;

        // Calculate final score
        let final_score =
            base_score + longevity_bonus + latency_score + misbehavior_penalty + diversity_bonus
                - attempt_penalty;

        // Ensure score is non-negative
        final_score.max(0.0)
    }

    /// Check if peer is from an underrepresented subnet (for diversity bonus)
    fn is_diverse_peer(&self, info: &PeerInfo) -> bool {
        let peer_subnet = match info.address.ip {
            IpAddr::V4(ipv4) => {
                // Get /24 subnet
                let octets = ipv4.octets();
                u32::from_be_bytes(octets) & 0xFFFFFF00
            }
            IpAddr::V6(_) => {
                // For IPv6, we use simplified diversity check
                return true; // IPv6 peers get diversity bonus by default
            }
        };

        // Count peers from same /24 subnet
        let subnet_count = self
            .addresses
            .values()
            .filter(|p| match p.address.ip {
                IpAddr::V4(ipv4) => {
                    let octets = ipv4.octets();
                    let subnet = u32::from_be_bytes(octets) & 0xFFFFFF00;
                    subnet == peer_subnet
                }
                _ => false,
            })
            .count();

        // Peer is diverse if fewer than 10 connections from same /24
        subnet_count < 10
    }

    /// Record connection attempt
    pub fn record_attempt(&mut self, addr: &SocketAddr) {
        if let Some(info) = self.addresses.get_mut(addr) {
            info.attempts += 1;
            info.last_attempt = Some(SystemTime::now());
        }
    }

    /// Record successful connection
    pub fn record_success(&mut self, addr: &SocketAddr) {
        if let Some(info) = self.addresses.get_mut(addr) {
            info.last_seen = SystemTime::now();

            // Update success rate using exponential moving average
            let success_weight = 0.1;
            info.success_rate = info.success_rate * (1.0 - success_weight) + success_weight;
        }
    }

    /// Record failed connection
    pub fn record_failure(&mut self, addr: &SocketAddr) {
        if let Some(info) = self.addresses.get_mut(addr) {
            // Update success rate using exponential moving average
            let failure_weight = 0.1;
            info.success_rate *= 1.0 - failure_weight;

            // Increment failed attempts for exponential penalty
            info.failed_attempts += 1;
        }
    }

    /// Update peer latency (for latency scoring)
    pub fn record_latency(&mut self, addr: &SocketAddr, latency: Duration) {
        if let Some(info) = self.addresses.get_mut(addr) {
            // Use exponential moving average for latency
            match info.avg_latency {
                Some(avg) => {
                    // Weight: 0.2 new, 0.8 old
                    let new_avg_ms =
                        (avg.as_millis() as f32 * 0.8 + latency.as_millis() as f32 * 0.2) as u64;
                    info.avg_latency = Some(Duration::from_millis(new_avg_ms));
                }
                None => {
                    info.avg_latency = Some(latency);
                }
            }
        }
    }

    /// Update peer misbehavior score (for heavy penalty)
    pub fn record_misbehavior(&mut self, addr: &SocketAddr, points: u32) {
        if let Some(info) = self.addresses.get_mut(addr) {
            info.misbehavior_score += points;
        }
    }

    /// Query DNS seeds for new addresses
    pub async fn query_dns_seeds(&mut self) -> Result<Vec<SocketAddr>, DiscoveryError> {
        let now = Instant::now();

        // Check if enough time has passed since last query
        if let Some(last_query) = self.last_dns_query {
            if now.duration_since(last_query) < self.dns_query_interval {
                return Ok(Vec::new());
            }
        }

        let mut addresses = Vec::new();

        for seed in &self.dns_seeds {
            match self.resolve_dns_seed(seed).await {
                Ok(mut seed_addresses) => addresses.append(&mut seed_addresses),
                Err(e) => {
                    // Log error but continue with other seeds
                    eprintln!("DNS seed query failed for {}: {}", seed, e);
                }
            }
        }

        self.last_dns_query = Some(now);

        // Add discovered addresses
        for addr in &addresses {
            self.add_address(*addr, ServiceFlags::NETWORK);
        }

        Ok(addresses)
    }

    /// Resolve a single DNS seed
    async fn resolve_dns_seed(&self, seed: &str) -> Result<Vec<SocketAddr>, DiscoveryError> {
        use tokio::net::lookup_host;

        // Use the correct port for this network (mainnet=18341, testnet=18351, regtest=18361)
        let seed_with_port = if seed.contains(':') {
            seed.to_string()
        } else {
            format!("{}:{}", seed, self.default_port)
        };

        let addresses: Vec<SocketAddr> = lookup_host(&seed_with_port)
            .await
            .map_err(|e| DiscoveryError::DnsError(e.to_string()))?
            .collect();

        Ok(addresses)
    }

    /// Remove oldest addresses to make room
    fn evict_oldest(&mut self) {
        let oldest_addr = self
            .addresses
            .iter()
            .min_by_key(|(_, info)| info.last_seen)
            .map(|(addr, _)| *addr);

        if let Some(addr) = oldest_addr {
            self.addresses.remove(&addr);
        }
    }

    /// Get addresses for sharing with peers
    pub fn get_addresses_for_sharing(&self, count: usize) -> Vec<NetworkAddress> {
        let now = SystemTime::now();

        let mut candidates: Vec<_> = self
            .addresses
            .values()
            .filter(|info| {
                // Only share recently seen addresses
                now.duration_since(info.last_seen)
                    .unwrap_or(Duration::from_secs(u64::MAX))
                    .as_secs()
                    < 10800 // 3 hours
            })
            .collect();

        // Sort by last seen (most recent first)
        candidates.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));

        candidates
            .into_iter()
            .take(count)
            .map(|info| info.address.clone())
            .collect()
    }

    /// Remove invalid or misbehaving addresses
    pub fn remove_address(&mut self, addr: &SocketAddr) {
        self.addresses.remove(addr);
    }

    /// Get total number of known addresses
    pub fn address_count(&self) -> usize {
        self.addresses.len()
    }

    /// Check if an address is known
    /// Alias for [`PeerDiscovery::has_address`] (003-testnet-p2p-hardening).
    pub fn contains_address(&self, addr: &SocketAddr) -> bool {
        self.has_address(addr)
    }

    pub fn has_address(&self, addr: &SocketAddr) -> bool {
        self.addresses.contains_key(addr)
    }

    /// Get addresses matching specific criteria
    pub fn filter_addresses<F>(&self, predicate: F) -> Vec<SocketAddr>
    where
        F: Fn(&PeerInfo) -> bool,
    {
        self.addresses
            .iter()
            .filter(|(_, info)| predicate(info))
            .map(|(addr, _)| *addr)
            .collect()
    }

    /// Save known peers to disk (peers.dat equivalent).
    ///
    /// Only saves peers seen in the last 3 hours with fewer than 5 failed
    /// attempts, capped at 1000 entries.  The format is a JSON array so it
    /// can be inspected and hand-edited easily.
    pub fn save_to_disk(&self, path: &Path) -> std::io::Result<()> {
        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let cutoff = now_unix.saturating_sub(10_800); // 3 hours

        let saved: Vec<SavedPeer> = self
            .addresses
            .iter()
            .filter(|(_, info)| {
                let last_seen = info
                    .last_seen
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                last_seen >= cutoff && info.failed_attempts < 5
            })
            .map(|(addr, info)| SavedPeer {
                addr: addr.to_string(),
                services: info.services.0,
                last_seen_unix: info
                    .last_seen
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            })
            .take(1000)
            .collect();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&saved)
            .map_err(std::io::Error::other)?;
        fs::write(path, json)
    }

    /// Load known peers from disk (peers.dat equivalent).
    ///
    /// Silently ignores a missing file (first run).  Entries older than
    /// 7 days are discarded so stale addresses don't clog the address book.
    pub fn load_from_disk(&mut self, path: &Path) {
        let data = match fs::read_to_string(path) {
            Ok(d) => d,
            Err(_) => return,
        };

        let saved: Vec<SavedPeer> = match serde_json::from_str(&data) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("⚠️  Failed to parse peers.dat: {}", e);
                return;
            }
        };

        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let max_age = 86_400 * 7; // 7 days

        let mut loaded = 0usize;
        for entry in saved {
            if now_unix.saturating_sub(entry.last_seen_unix) > max_age {
                continue;
            }

            let addr: SocketAddr = match entry.addr.parse() {
                Ok(a) => a,
                Err(_) => continue,
            };

            let services = ServiceFlags(entry.services);
            let last_seen = UNIX_EPOCH + Duration::from_secs(entry.last_seen_unix);
            let network_addr = NetworkAddress::new(addr.ip(), addr.port(), services);

            let peer_info = PeerInfo {
                address: network_addr,
                last_seen,
                first_seen: last_seen,
                attempts: 0,
                failed_attempts: 0,
                last_attempt: None,
                services,
                success_rate: 0.5, // neutral starting score
                avg_latency: None,
                misbehavior_score: 0,
            };

            self.addresses.insert(addr, peer_info);
            loaded += 1;
        }

        if loaded > 0 {
            println!("📋 Loaded {} peer(s) from peers.dat", loaded);
        }
    }
}

/// Lightweight peer data serialised to peers.dat between restarts.
///
/// Transient fields (latency, scores, ban state) are intentionally omitted —
/// they are rebuilt from live connections.
#[derive(Serialize, Deserialize)]
struct SavedPeer {
    /// Socket address as "ip:port"
    addr: String,
    /// Raw service-flags bitmask
    services: u64,
    /// Unix timestamp of last successful contact
    last_seen_unix: u64,
}

/// Bootstrap peer discovery from hardcoded seed nodes
pub async fn bootstrap_from_seeds(
    seed_nodes: &[SocketAddr],
    discovery: &mut PeerDiscovery,
) -> Result<(), DiscoveryError> {
    for addr in seed_nodes {
        discovery.add_address(*addr, ServiceFlags::NETWORK);
    }

    // Also query DNS seeds
    discovery.query_dns_seeds().await?;

    Ok(())
}

/// Address manager for efficient peer address storage
pub struct AddressManager {
    /// IPv4 addresses
    ipv4_addresses: HashMap<u32, Vec<PeerInfo>>,
    /// IPv6 addresses
    ipv6_addresses: HashMap<u128, Vec<PeerInfo>>,
    /// Maximum addresses per bucket
    max_per_bucket: usize,
}

impl AddressManager {
    /// Create a new address manager
    pub fn new(max_per_bucket: usize) -> Self {
        AddressManager {
            ipv4_addresses: HashMap::new(),
            ipv6_addresses: HashMap::new(),
            max_per_bucket,
        }
    }

    /// Get bucket for IPv4 address
    fn ipv4_bucket(&self, ip: &IpAddr) -> Option<u32> {
        match ip {
            IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                Some(u32::from_be_bytes(octets) >> 16) // Use first 16 bits
            }
            _ => None,
        }
    }

    /// Get bucket for IPv6 address
    fn ipv6_bucket(&self, ip: &IpAddr) -> Option<u128> {
        match ip {
            IpAddr::V6(ipv6) => {
                let segments = ipv6.segments();
                let high = u64::from_be_bytes([
                    (segments[0] >> 8) as u8,
                    segments[0] as u8,
                    (segments[1] >> 8) as u8,
                    segments[1] as u8,
                    (segments[2] >> 8) as u8,
                    segments[2] as u8,
                    (segments[3] >> 8) as u8,
                    segments[3] as u8,
                ]);
                Some(high as u128) // Use first 64 bits
            }
            _ => None,
        }
    }

    /// Add address to appropriate bucket
    pub fn add_address(&mut self, peer_info: PeerInfo) {
        let ip = peer_info.address.ip;

        if let Some(bucket) = self.ipv4_bucket(&ip) {
            let bucket_vec = self.ipv4_addresses.entry(bucket).or_default();
            AddressManager::add_to_bucket_static(bucket_vec, peer_info, self.max_per_bucket);
        } else if let Some(bucket) = self.ipv6_bucket(&ip) {
            let bucket_vec = self.ipv6_addresses.entry(bucket).or_default();
            AddressManager::add_to_bucket_static(bucket_vec, peer_info, self.max_per_bucket);
        }
    }

    /// Add peer to bucket, evicting if necessary
    fn add_to_bucket(&mut self, bucket: &mut Vec<PeerInfo>, peer_info: PeerInfo) {
        AddressManager::add_to_bucket_static(bucket, peer_info, self.max_per_bucket);
    }

    /// Static version of add_to_bucket to avoid borrowing issues
    fn add_to_bucket_static(
        bucket: &mut Vec<PeerInfo>,
        peer_info: PeerInfo,
        max_per_bucket: usize,
    ) {
        // Check if address already exists
        if let Some(pos) = bucket
            .iter()
            .position(|p| p.address.ip == peer_info.address.ip)
        {
            bucket[pos] = peer_info;
            return;
        }

        // Add new address
        bucket.push(peer_info);

        // Evict oldest if bucket is full
        if bucket.len() > max_per_bucket {
            bucket.sort_by_key(|p| p.last_seen);
            bucket.remove(0);
        }
    }

    /// Get random addresses from different buckets
    pub fn get_random_addresses(&self, count: usize) -> Vec<PeerInfo> {
        let mut addresses = Vec::new();

        // Get from IPv4 buckets
        for bucket in self.ipv4_addresses.values() {
            if addresses.len() >= count {
                break;
            }
            if !bucket.is_empty() {
                let idx = rand::random::<usize>() % bucket.len();
                addresses.push(bucket[idx].clone());
            }
        }

        // Get from IPv6 buckets if needed
        for bucket in self.ipv6_addresses.values() {
            if addresses.len() >= count {
                break;
            }
            if !bucket.is_empty() {
                let idx = rand::random::<usize>() % bucket.len();
                addresses.push(bucket[idx].clone());
            }
        }

        addresses.truncate(count);
        addresses
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    #[test]
    fn test_peer_discovery_creation() {
        let discovery = PeerDiscovery::new(1000);
        assert_eq!(discovery.address_count(), 0);
    }

    #[test]
    fn test_add_address() {
        let mut discovery = PeerDiscovery::new(1000);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8333);

        discovery.add_address(addr, ServiceFlags::NETWORK);
        assert_eq!(discovery.address_count(), 1);
        assert!(discovery.has_address(&addr));
    }

    #[test]
    fn test_peer_score_calculation() {
        let discovery = PeerDiscovery::new(1000);
        let network_addr = NetworkAddress::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8333,
            ServiceFlags::NETWORK,
        );

        let now = SystemTime::now();
        let peer_info = PeerInfo {
            address: network_addr,
            last_seen: now,
            first_seen: now,
            attempts: 0,
            failed_attempts: 0,
            last_attempt: None,
            services: ServiceFlags::NETWORK,
            success_rate: 0.8,
            avg_latency: None,
            misbehavior_score: 0,
        };

        let score = discovery.calculate_peer_score(&peer_info);
        // Enhanced scoring: base (80) + longevity (0) + latency (5) + diversity bonus (0-15)
        assert!(score > 0.0);
    }

    #[test]
    fn test_enhanced_peer_scoring() {
        let mut discovery = PeerDiscovery::new(1000);

        // Add a high-quality peer
        let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8333);
        discovery.add_address(addr1, ServiceFlags::NETWORK);
        discovery.record_success(&addr1);
        discovery.record_latency(&addr1, Duration::from_millis(50));

        // Add a low-latency peer with misbehavior
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), 8333);
        discovery.add_address(addr2, ServiceFlags::NETWORK);
        discovery.record_misbehavior(&addr2, 5); // 50 point penalty

        // Add a peer with failed attempts
        let addr3 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3)), 8333);
        discovery.add_address(addr3, ServiceFlags::NETWORK);
        discovery.record_failure(&addr3);
        discovery.record_failure(&addr3);
        discovery.record_failure(&addr3);

        // Get peer scores
        let peers = discovery.get_peers(3, ServiceFlags::NETWORK);
        // Peer 1 should rank highest (good success rate + low latency)
        assert_eq!(peers[0], addr1);
    }

    #[test]
    fn test_diversity_bonus() {
        let mut discovery = PeerDiscovery::new(1000);

        // Add peers from same subnet
        for i in 1..=5 {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, i)), 8333);
            discovery.add_address(addr, ServiceFlags::NETWORK);
        }

        // Check diversity for peer in same subnet (should get bonus - only 5 peers)
        let test_addr = NetworkAddress::new(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
            8333,
            ServiceFlags::NETWORK,
        );

        let now = SystemTime::now();
        let peer_info = PeerInfo {
            address: test_addr,
            last_seen: now,
            first_seen: now,
            attempts: 0,
            failed_attempts: 0,
            last_attempt: None,
            services: ServiceFlags::NETWORK,
            success_rate: 0.5,
            avg_latency: None,
            misbehavior_score: 0,
        };

        assert!(discovery.is_diverse_peer(&peer_info)); // <10 peers in subnet
    }

    #[test]
    fn test_latency_tracking() {
        let mut discovery = PeerDiscovery::new(1000);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8333);

        discovery.add_address(addr, ServiceFlags::NETWORK);
        discovery.record_latency(&addr, Duration::from_millis(100));
        discovery.record_latency(&addr, Duration::from_millis(200));

        let info = &discovery.addresses[&addr];
        assert!(info.avg_latency.is_some());
        // EMA: 100 * 0.8 + 200 * 0.2 = 120ms
        assert!((info.avg_latency.unwrap().as_millis() as i64 - 120).abs() < 10);
    }

    #[test]
    fn test_misbehavior_penalty() {
        let mut discovery = PeerDiscovery::new(1000);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8333);

        discovery.add_address(addr, ServiceFlags::NETWORK);
        discovery.record_misbehavior(&addr, 10); // 100 point penalty

        let info = &discovery.addresses[&addr];
        assert_eq!(info.misbehavior_score, 10);

        let score = discovery.calculate_peer_score(info);
        // Score should be heavily penalized
        assert!(score < 5.0); // Base 0 + neutral 5 - penalty 100 = -95, clamped to 0
    }

    #[test]
    fn test_failed_attempts_exponential_penalty() {
        let discovery = PeerDiscovery::new(1000);
        let network_addr = NetworkAddress::new(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            8333,
            ServiceFlags::NETWORK,
        );

        let now = SystemTime::now();

        // Peer with 0 failed attempts
        let peer_0 = PeerInfo {
            address: network_addr.clone(),
            last_seen: now,
            first_seen: now,
            attempts: 0,
            failed_attempts: 0,
            last_attempt: None,
            services: ServiceFlags::NETWORK,
            success_rate: 0.5,
            avg_latency: None,
            misbehavior_score: 0,
        };

        // Peer with 5 failed attempts (5^2 * 0.1 = 2.5 penalty)
        let peer_5 = PeerInfo {
            address: network_addr.clone(),
            last_seen: now,
            first_seen: now,
            attempts: 5,
            failed_attempts: 5,
            last_attempt: None,
            services: ServiceFlags::NETWORK,
            success_rate: 0.5,
            avg_latency: None,
            misbehavior_score: 0,
        };

        let score_0 = discovery.calculate_peer_score(&peer_0);
        let score_5 = discovery.calculate_peer_score(&peer_5);

        // Score should be lower with more failed attempts (squared penalty)
        assert!(score_0 > score_5);
        assert!((score_0 - score_5 - 2.5).abs() < 0.1); // ~2.5 difference
    }

    #[test]
    fn test_address_manager() {
        let mut manager = AddressManager::new(10);
        let network_addr = NetworkAddress::new(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            8333,
            ServiceFlags::NETWORK,
        );

        let now = SystemTime::now();
        let peer_info = PeerInfo {
            address: network_addr,
            last_seen: now,
            first_seen: now,
            attempts: 0,
            failed_attempts: 0,
            last_attempt: None,
            services: ServiceFlags::NETWORK,
            success_rate: 0.0,
            avg_latency: None,
            misbehavior_score: 0,
        };

        manager.add_address(peer_info);
        let addresses = manager.get_random_addresses(5);
        assert!(!addresses.is_empty());
    }
}

// ============================================================================
// 003-testnet-p2p-hardening — addr gossip handlers (T022–T024, FR-005)
// ============================================================================

/// Maximum number of entries returned by [`handle_getaddr`].
pub const GETADDR_MAX_ENTRIES: usize = 1000;

/// Maximum age of an `addr` entry that can be gossiped back out (10 days,
/// matching Bitcoin Core's `ADDRMAN_ADDRMAN_FRESH_WINDOW`).
pub const GETADDR_MAX_AGE: Duration = Duration::from_secs(10 * 24 * 60 * 60);

/// Threshold below which an inbound `addr` message is considered a
/// gossip relay candidate (Bitcoin convention: ≤10 entries).
pub const ADDR_RELAY_THRESHOLD: usize = 10;

/// Decision returned by [`handle_inbound_addr`] telling the caller what
/// to do with a received `addr` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddrRelayDecision {
    /// Small addr — ingest and forward to `peers` other peers.
    Forward { peers: usize },
    /// Bulk addr — ingest only, do not relay.
    IngestOnly,
}

/// Build a `getaddr` response from a peer discovery state. Filters
/// stale entries (>10 days) and caps the response at 1000 addresses.
pub fn handle_getaddr(disc: &PeerDiscovery) -> Vec<NetworkAddress> {
    let now = SystemTime::now();
    let mut out = Vec::new();
    for info in disc.addresses.values() {
        if let Ok(age) = now.duration_since(info.last_seen) {
            if age > GETADDR_MAX_AGE {
                continue;
            }
        }
        out.push(info.address.clone());
        if out.len() >= GETADDR_MAX_ENTRIES {
            break;
        }
    }
    out
}

/// Decide how to process an inbound `addr` message. Small messages
/// (≤[`ADDR_RELAY_THRESHOLD`] entries) are relayed to exactly two other
/// peers; larger ones are ingested silently.
pub fn handle_inbound_addr(
    _disc: &PeerDiscovery,
    addrs: &[NetworkAddress],
) -> AddrRelayDecision {
    if addrs.len() <= ADDR_RELAY_THRESHOLD {
        AddrRelayDecision::Forward { peers: 2 }
    } else {
        AddrRelayDecision::IngestOnly
    }
}

// ============================================================================
// 003-testnet-p2p-hardening — RED-phase tests for T093 (Phase 2.5)
// ============================================================================
//
// These tests cover FR-005 `addr` gossip + the `getaddr` handler that does
// not yet exist in the live flat tree. GREEN implementation lands in T022–T024.
// Compilation failure IS the failing state; do NOT stub symbols.

#[cfg(test)]
mod red_phase_addr_gossip_tests {
    use super::*;
    use crate::network::discovery::{handle_getaddr, handle_inbound_addr, AddrRelayDecision};
    use std::time::{Duration, SystemTime};

    fn mk_discovery() -> PeerDiscovery {
        PeerDiscovery::new_for_network(2048, crate::Network::Regtest)
    }

    #[test]
    fn getaddr_response_is_capped_at_one_thousand_entries() {
        // FR-005 / Bitcoin-style: `getaddr` returns ≤1000 addresses even if
        // we know more, to avoid flooding the requester.
        let mut disc = mk_discovery();
        for i in 0..2500 {
            let a = std::net::SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, (i >> 8) as u8, (i & 0xff) as u8, 1)),
                18351,
            );
            disc.add_address(a, ServiceFlags::NETWORK);
        }
        let response = handle_getaddr(&disc);
        assert!(
            response.len() <= 1000,
            "getaddr must cap response at 1000, got {}",
            response.len()
        );
        assert!(!response.is_empty(), "getaddr must return at least some entries");
    }

    #[test]
    fn getaddr_filters_entries_older_than_ten_days() {
        // Stale peers (last_seen > 10 days ago) must NOT be shared via getaddr.
        let mut disc = mk_discovery();
        let eleven_days_ago = SystemTime::now() - Duration::from_secs(11 * 24 * 60 * 60);
        disc.add_address_with_last_seen(
            "10.0.0.1:18351".parse().unwrap(),
            ServiceFlags::NETWORK,
            eleven_days_ago,
        );
        disc.add_address(
            "10.0.0.2:18351".parse().unwrap(),
            ServiceFlags::NETWORK,
        );
        let response = handle_getaddr(&disc);
        assert!(
            response.iter().all(|na| !format!("{:?}", na).contains("10.0.0.1")),
            "stale entry must be filtered out"
        );
    }

    #[test]
    fn small_addr_message_is_relayed_to_two_peers() {
        // FR-005 Bitcoin rule: `addr` with ≤10 entries gets forwarded to
        // two randomly-selected peers (relay on gossip).
        let disc = mk_discovery();
        let addrs: Vec<NetworkAddress> = (0..8)
            .map(|i| NetworkAddress::from_socket_addr(
                &format!("10.1.0.{}:18351", i + 1).parse().unwrap(),
                ServiceFlags::NETWORK,
            ))
            .collect();
        let decision = handle_inbound_addr(&disc, &addrs);
        match decision {
            AddrRelayDecision::Forward { peers } => {
                assert_eq!(peers, 2, "small addr must relay to exactly 2 peers");
            }
            _ => panic!("small addr must trigger Forward decision, got {:?}", decision),
        }
    }

    #[test]
    fn bulk_addr_message_is_not_relayed() {
        // FR-005: large `addr` dumps (>10 entries) are treated as bulk-ingest,
        // stored but not relayed onward.
        let disc = mk_discovery();
        let addrs: Vec<NetworkAddress> = (0..500)
            .map(|i| NetworkAddress::from_socket_addr(
                &format!("10.2.{}.{}:18351", (i >> 8) & 0xff, i & 0xff).parse().unwrap(),
                ServiceFlags::NETWORK,
            ))
            .collect();
        let decision = handle_inbound_addr(&disc, &addrs);
        assert!(
            matches!(decision, AddrRelayDecision::IngestOnly),
            "bulk addr must NOT be relayed, got {:?}",
            decision
        );
    }
}
