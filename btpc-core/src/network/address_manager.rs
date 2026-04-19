//! Bitcoin-style bucketed peer address manager (003-testnet-p2p-hardening).
//!
//! Provides an addrman modelled after Bitcoin Core:
//!
//! * Addresses live in one of two pools — `new` (freshly learned) or
//!   `tried` (we have successfully connected at least once).
//! * Eleven consecutive failures evict an entry.
//! * Outbound peer selection biases across `/16` network groups so a
//!   single subnet cannot eclipse the node (FR-009).
//!
//! Persistence uses a standalone RocksDB instance at a caller-supplied
//! path; the in-memory flavour is used by unit tests and fast paths.
//!
//! Constitution Art. VI mandates RocksDB for persistent state; plain
//! JSON is not acceptable here.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};

/// Number of "new" buckets (matches Bitcoin Core addrman).
pub const NEW_BUCKET_COUNT: usize = 64;
/// Number of "tried" buckets (matches Bitcoin Core addrman).
pub const TRIED_BUCKET_COUNT: usize = 16;
/// Maximum consecutive failures before an entry is evicted.
pub const MAX_CONSECUTIVE_FAILURES: u32 = 10;

/// State of a peer address within the manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PeerState {
    /// Newly learned — never successfully contacted.
    New,
    /// At least one successful connection recorded.
    Tried,
}

/// `/16` (IPv4) or `/32` (IPv6) network group used for eclipse-resistance
/// bucketing in outbound peer selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetworkGroup(pub [u8; 4]);

impl NetworkGroup {
    /// Derive the network group from a socket address. IPv4 uses the
    /// leading `/16` (first two octets); IPv6 uses the leading `/32`
    /// (first four octets) — same as Bitcoin Core's addrman.
    pub fn from_socket_addr(addr: &SocketAddr) -> Self {
        match addr.ip() {
            IpAddr::V4(v4) => {
                let octs = v4.octets();
                NetworkGroup([octs[0], octs[1], 0, 0])
            }
            IpAddr::V6(v6) => {
                let segs = v6.segments();
                let hi = segs[0].to_be_bytes();
                let lo = segs[1].to_be_bytes();
                NetworkGroup([hi[0], hi[1], lo[0], lo[1]])
            }
        }
    }
}

/// A persisted entry in the address manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAddressEntry {
    addr: SocketAddr,
    services: u64,
    state: PeerState,
    success_count: u32,
    failure_count: u32,
    consecutive_failures: u32,
    last_seen_unix: u64,
}

impl PeerAddressEntry {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
    pub fn services(&self) -> u64 {
        self.services
    }
    pub fn state(&self) -> PeerState {
        self.state
    }
    pub fn success_count(&self) -> u32 {
        self.success_count
    }
    pub fn failure_count(&self) -> u32 {
        self.failure_count
    }
    pub fn last_seen_unix(&self) -> u64 {
        self.last_seen_unix
    }
    pub fn network_group(&self) -> NetworkGroup {
        NetworkGroup::from_socket_addr(&self.addr)
    }
}

/// Lightweight public view used by callers that only need the socket
/// address + service flags (kept as a thin alias so the rest of the code
/// can evolve independently).
pub type PeerAddress = PeerAddressEntry;

/// Errors produced by the address manager.
#[derive(Debug, thiserror::Error)]
pub enum AddressManagerError {
    #[error("rocksdb error: {0}")]
    RocksDb(String),
    #[error("serialisation error: {0}")]
    Codec(String),
}

impl From<rocksdb::Error> for AddressManagerError {
    fn from(e: rocksdb::Error) -> Self {
        AddressManagerError::RocksDb(e.to_string())
    }
}

/// Bitcoin-style bucketed address manager.
pub struct AddressManager {
    entries: HashMap<SocketAddr, PeerAddressEntry>,
    new_buckets: Vec<Vec<SocketAddr>>,
    tried_buckets: Vec<Vec<SocketAddr>>,
    db: Option<rocksdb::DB>,
    db_path: Option<PathBuf>,
}

impl std::fmt::Debug for AddressManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddressManager")
            .field("entries", &self.entries.len())
            .field("db_path", &self.db_path)
            .finish()
    }
}

impl AddressManager {
    /// Construct an in-memory address manager (no persistence).
    pub fn in_memory() -> Self {
        AddressManager {
            entries: HashMap::new(),
            new_buckets: vec![Vec::new(); NEW_BUCKET_COUNT],
            tried_buckets: vec![Vec::new(); TRIED_BUCKET_COUNT],
            db: None,
            db_path: None,
        }
    }

    /// Open (or create) a RocksDB-backed address manager at `path`.
    ///
    /// The DB is a standalone instance owned by this manager; callers
    /// that want to colocate it inside a larger `UnifiedDatabase` should
    /// use a dedicated column family instead.
    pub fn open_at(path: impl AsRef<Path>) -> Result<Self, AddressManagerError> {
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        let db = rocksdb::DB::open(&opts, path.as_ref())?;
        let mut am = AddressManager {
            entries: HashMap::new(),
            new_buckets: vec![Vec::new(); NEW_BUCKET_COUNT],
            tried_buckets: vec![Vec::new(); TRIED_BUCKET_COUNT],
            db: None,
            db_path: Some(path.as_ref().to_path_buf()),
        };
        // Load existing entries from disk into the in-memory index.
        for item in db.iterator(rocksdb::IteratorMode::Start) {
            let (_, value) = item?;
            if let Ok(entry) = bincode::deserialize::<PeerAddressEntry>(&value) {
                am.insert_into_buckets(&entry);
                am.entries.insert(entry.addr, entry);
            }
        }
        am.db = Some(db);
        Ok(am)
    }

    /// Number of entries currently tracked.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the manager has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Add a newly-learned address. If the address already exists the
    /// existing entry is left untouched.
    pub fn add_new(&mut self, addr: SocketAddr, services: u64) {
        if self.entries.contains_key(&addr) {
            return;
        }
        let entry = PeerAddressEntry {
            addr,
            services,
            state: PeerState::New,
            success_count: 0,
            failure_count: 0,
            consecutive_failures: 0,
            last_seen_unix: now_unix(),
        };
        self.insert_into_buckets(&entry);
        self.persist(&entry);
        self.entries.insert(addr, entry);
    }

    /// Record a successful connection. Promotes `New` → `Tried`.
    pub fn record_success(&mut self, addr: &SocketAddr) {
        if let Some(entry) = self.entries.get_mut(addr) {
            let was_new = entry.state == PeerState::New;
            entry.state = PeerState::Tried;
            entry.success_count = entry.success_count.saturating_add(1);
            entry.consecutive_failures = 0;
            entry.last_seen_unix = now_unix();
            let snapshot = entry.clone();
            if was_new {
                remove_from_bucket_vec(&mut self.new_buckets, new_bucket_index(addr), addr);
                self.tried_buckets[tried_bucket_index(addr)].push(*addr);
            }
            self.persist(&snapshot);
        }
    }

    /// Record a failed connection. Evicts the entry once
    /// `consecutive_failures` exceeds [`MAX_CONSECUTIVE_FAILURES`].
    pub fn record_failure(&mut self, addr: &SocketAddr) {
        let should_evict = if let Some(entry) = self.entries.get_mut(addr) {
            entry.failure_count = entry.failure_count.saturating_add(1);
            entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
            entry.last_seen_unix = now_unix();
            let exceeded = entry.consecutive_failures > MAX_CONSECUTIVE_FAILURES;
            if !exceeded {
                let snapshot = entry.clone();
                self.persist(&snapshot);
            }
            exceeded
        } else {
            false
        };
        if should_evict {
            self.remove(addr);
        }
    }

    /// Remove an address from every tracking structure.
    pub fn remove(&mut self, addr: &SocketAddr) {
        if self.entries.remove(addr).is_some() {
            remove_from_bucket_vec(&mut self.new_buckets, new_bucket_index(addr), addr);
            remove_from_bucket_vec(&mut self.tried_buckets, tried_bucket_index(addr), addr);
            if let Some(db) = &self.db {
                let _ = db.delete(addr_key(addr));
            }
        }
    }

    /// Fetch an entry by socket address.
    pub fn get(&self, addr: &SocketAddr) -> Option<&PeerAddressEntry> {
        self.entries.get(addr)
    }

    /// Pick `count` outbound candidates, biased across distinct `/16`
    /// network groups. Tried entries are preferred; new entries fill any
    /// remaining slots. FR-009 — eclipse attack resistance.
    pub fn pick_outbound_candidates(&self, count: usize) -> Vec<SocketAddr> {
        if count == 0 || self.entries.is_empty() {
            return Vec::new();
        }

        // Group by network group. Within each group, prefer `Tried` over
        // `New` so good peers win when available.
        let mut by_group: HashMap<NetworkGroup, Vec<&PeerAddressEntry>> = HashMap::new();
        for entry in self.entries.values() {
            by_group
                .entry(entry.network_group())
                .or_default()
                .push(entry);
        }
        for bucket in by_group.values_mut() {
            bucket.sort_by_key(|e| match e.state {
                PeerState::Tried => 0u8,
                PeerState::New => 1u8,
            });
        }

        // Deterministic ordering for reproducibility in tests.
        let mut groups: Vec<NetworkGroup> = by_group.keys().copied().collect();
        groups.sort_by_key(|g| g.0);

        let mut picks = Vec::with_capacity(count);
        let mut cursors: HashMap<NetworkGroup, usize> = HashMap::new();

        // Round-robin across groups until we hit `count` or exhaust entries.
        while picks.len() < count {
            let before = picks.len();
            for g in &groups {
                if picks.len() >= count {
                    break;
                }
                let idx = cursors.entry(*g).or_insert(0);
                if let Some(entries) = by_group.get(g) {
                    if *idx < entries.len() {
                        picks.push(entries[*idx].addr);
                        *idx += 1;
                    }
                }
            }
            if picks.len() == before {
                break; // No forward progress — every group exhausted.
            }
        }

        picks
    }

    /// Iterate all currently tracked entries.
    pub fn iter(&self) -> impl Iterator<Item = &PeerAddressEntry> {
        self.entries.values()
    }

    fn insert_into_buckets(&mut self, entry: &PeerAddressEntry) {
        match entry.state {
            PeerState::New => {
                self.new_buckets[new_bucket_index(&entry.addr)].push(entry.addr);
            }
            PeerState::Tried => {
                self.tried_buckets[tried_bucket_index(&entry.addr)].push(entry.addr);
            }
        }
    }

    fn persist(&self, entry: &PeerAddressEntry) {
        if let Some(db) = &self.db {
            if let Ok(bytes) = bincode::serialize(entry) {
                let _ = db.put(addr_key(&entry.addr), bytes);
            }
        }
    }
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn addr_key(addr: &SocketAddr) -> Vec<u8> {
    addr.to_string().into_bytes()
}

fn hash_group_bytes(addr: &SocketAddr) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    NetworkGroup::from_socket_addr(addr).hash(&mut hasher);
    // Mix in the full address so distinct hosts in the same /16 land in
    // different buckets while still being reachable via their group.
    addr.hash(&mut hasher);
    hasher.finish()
}

fn new_bucket_index(addr: &SocketAddr) -> usize {
    (hash_group_bytes(addr) as usize) % NEW_BUCKET_COUNT
}

fn tried_bucket_index(addr: &SocketAddr) -> usize {
    (hash_group_bytes(addr) as usize) % TRIED_BUCKET_COUNT
}

fn remove_from_bucket_vec(buckets: &mut [Vec<SocketAddr>], idx: usize, target: &SocketAddr) {
    if let Some(bucket) = buckets.get_mut(idx) {
        bucket.retain(|a| a != target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn mk_addr(a: u8, b: u8, c: u8, d: u8, port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(a, b, c, d)), port)
    }

    #[test]
    fn network_group_uses_slash_16_for_ipv4() {
        let a = NetworkGroup::from_socket_addr(&mk_addr(192, 168, 1, 1, 18351));
        let b = NetworkGroup::from_socket_addr(&mk_addr(192, 168, 9, 200, 18351));
        let c = NetworkGroup::from_socket_addr(&mk_addr(192, 169, 1, 1, 18351));
        assert_eq!(a, b, "same /16 should hash to the same group");
        assert_ne!(a, c, "adjacent /16 should hash to a different group");
    }

    #[test]
    fn new_entries_start_in_new_bucket() {
        let mut am = AddressManager::in_memory();
        let addr = mk_addr(10, 0, 0, 1, 18351);
        am.add_new(addr, 0);
        let entry = am
            .get(&addr)
            .expect("address should be present after add_new");
        assert_eq!(entry.state(), PeerState::New);
        assert_eq!(entry.success_count(), 0);
        assert_eq!(entry.failure_count(), 0);
    }

    #[test]
    fn successful_contact_promotes_new_to_tried() {
        let mut am = AddressManager::in_memory();
        let addr = mk_addr(10, 0, 0, 2, 18351);
        am.add_new(addr, 0);
        am.record_success(&addr);
        let entry = am.get(&addr).expect("entry exists");
        assert_eq!(entry.state(), PeerState::Tried);
        assert_eq!(entry.success_count(), 1);
    }

    #[test]
    fn entry_evicted_after_eleven_consecutive_failures() {
        let mut am = AddressManager::in_memory();
        let addr = mk_addr(10, 0, 0, 3, 18351);
        am.add_new(addr, 0);
        for _ in 0..11 {
            am.record_failure(&addr);
        }
        assert!(
            am.get(&addr).is_none(),
            "entry should be evicted after > 10 failures"
        );
    }

    #[test]
    fn outbound_selection_biases_across_network_groups() {
        let mut am = AddressManager::in_memory();
        am.add_new(mk_addr(192, 168, 1, 10, 18351), 0);
        am.add_new(mk_addr(192, 168, 1, 11, 18351), 0);
        am.add_new(mk_addr(192, 168, 1, 12, 18351), 0);
        am.add_new(mk_addr(10, 0, 0, 1, 18351), 0);
        am.add_new(mk_addr(172, 16, 0, 1, 18351), 0);
        let picks = am.pick_outbound_candidates(3);
        let groups: std::collections::HashSet<_> =
            picks.iter().map(NetworkGroup::from_socket_addr).collect();
        assert_eq!(
            groups.len(),
            3,
            "three picks should come from three distinct /16 groups"
        );
    }

    #[test]
    fn rocksdb_round_trip_preserves_entries() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().to_path_buf();
        {
            let mut am = AddressManager::open_at(&path).expect("open addrman");
            am.add_new(mk_addr(10, 0, 0, 4, 18351), 0);
            am.record_success(&mk_addr(10, 0, 0, 4, 18351));
        }
        let am = AddressManager::open_at(&path).expect("reopen addrman");
        let entry = am
            .get(&mk_addr(10, 0, 0, 4, 18351))
            .expect("entry survives reopen");
        assert_eq!(entry.state(), PeerState::Tried);
    }
}
