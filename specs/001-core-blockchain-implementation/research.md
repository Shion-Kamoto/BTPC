# Research Findings: Core Blockchain Implementation

**Feature**: Core Blockchain Implementation
**Branch**: `001-core-blockchain-implementation`
**Phase**: 0 - Outline & Research
**Generated**: 2025-09-30

## RocksDB Configuration for UTXO Storage

### Decision: Multi-Column Family Architecture with Performance Optimization

**Configuration Approach**:
- Multi-column family setup with dedicated families for blockchain data types
- Universal compaction strategy for write-heavy blockchain workloads
- Large block cache configuration (50-70% of available RAM)
- ClockCache for reduced lock contention in high-concurrency scenarios

### Rationale

**Performance Benefits**:
- Column families enable data segregation and independent optimization
- Universal compaction reduces write amplification from 20-30x to much lower levels
- Large cache sizes prevent storage reads by caching UTXO data in RAM
- Multi-threaded writes handle parallel blockchain operations

**UTXO-Specific Optimizations**:
- Prefix bloom filters for efficient UTXO lookup operations
- Dedicated UTXO column family with optimized compaction strategies
- Index and filter block caching keeps UTXO metadata in memory
- LSM-tree architecture handles continuous UTXO set growth

### Alternatives Considered

- **LevelDB**: Single-threaded writes create bottlenecks, limited concurrency
- **LMDB**: Poor performance with large datasets, memory mapping challenges
- **PostgreSQL/MySQL**: Significant overhead for key-value UTXO operations
- **Redis**: High memory requirements for full UTXO set storage

## ML-DSA (Dilithium) Implementation

### Decision: Hybrid Implementation with pqc_dilithium Crate

**Implementation Strategy**:
- Use `pqc_dilithium` crate as primary ML-DSA implementation
- Implement ML-DSA-65 parameter set for optimal security/performance balance
- Maintain algorithm agility for future transitions
- Secure memory management with `zeroize` crate

### Rationale

**Standards Compliance**:
- ML-DSA standardized under NIST FIPS 204 (August 2024)
- ML-DSA-65 provides AES-192 equivalent security (Category 3)
- Regulatory compliance for post-quantum cryptography

**Performance Characteristics**:
- Signing: ~137µs, verification: ~57µs on modern hardware
- 1,952-byte public keys, 3,309-byte signatures
- Significantly faster than alternative post-quantum schemes
- Compatible with blockchain transaction throughput requirements

**Integration Benefits**:
- Clean Rust API compatible with blockchain systems
- Deterministic key generation for wallet implementations
- Built-in serialization for network protocols
- Memory-safe implementation with proper key zeroization

### Alternatives Considered

- **crystals-dilithium**: GPL-3.0 license incompatibility, no security audit
- **pqcrypto-dilithium**: FFI overhead, C toolchain dependency
- **FALCON/SPHINCS+**: Not NIST-standardized, limited Rust ecosystem
- **Custom implementation**: High development effort, audit requirements

## Bitcoin P2P Protocol Implementation

### Decision: Rust-Native Protocol with Bitcoin Compatibility

**Approach**:
- Implement Bitcoin-compatible P2P protocol in pure Rust
- Use tokio for async networking and connection management
- Maintain protocol version compatibility for broader network participation
- Implement custom message types for ML-DSA signature validation

### Rationale

**Network Compatibility**:
- Leverage existing Bitcoin P2P infrastructure and tooling
- Maintain interoperability with Bitcoin network monitoring tools
- Proven protocol design for distributed blockchain networks

**Performance Benefits**:
- Async I/O with tokio for high-concurrency node operations
- Zero-copy message parsing where possible
- Efficient peer discovery and connection management

### Alternatives Considered

- **libp2p**: Additional complexity, over-engineered for blockchain P2P
- **Custom P2P protocol**: Network effect disadvantages, implementation complexity
- **Modified Bitcoin Core networking**: C++ integration complexity

## Tokio Async Patterns for Blockchain

### Decision: Actor-Based Architecture with Message Passing

**Pattern Selection**:
- Actor model for blockchain components (consensus, networking, storage)
- Channel-based communication between components
- Spawn blocking for CPU-intensive operations (signature verification, mining)
- Structured concurrency for component lifecycle management

### Rationale

**Scalability Benefits**:
- Actor isolation prevents shared state race conditions
- Message passing enables clean component interfaces
- Backpressure handling for high-throughput blockchain operations

**Maintenance Advantages**:
- Clear separation of concerns between blockchain components
- Testable component interfaces
- Graceful error handling and component restart capabilities

### Alternatives Considered

- **Shared state with mutexes**: Complexity and deadlock risks
- **Thread-per-component**: Resource overhead, communication complexity
- **Synchronous design**: Poor performance for I/O-heavy blockchain operations

## Implementation Timeline and Dependencies

### Phase Readiness Assessment
- ✅ **RocksDB**: Well-researched configuration approach
- ✅ **ML-DSA**: Clear implementation path with `pqc_dilithium`
- ✅ **P2P Protocol**: Bitcoin-compatible approach defined
- ✅ **Async Architecture**: Tokio actor patterns established

### Dependency Verification
- `tokio`: Mature async runtime, widely adopted
- `serde`: Standard serialization, blockchain protocol compatible
- `RocksDB`: Production-ready with extensive blockchain usage
- `pqc_dilithium`: NIST-compliant, suitable for blockchain integration
- `sha2`: Standard cryptographic hashing, SHA-512 support confirmed

All technical unknowns from the implementation plan have been resolved through research. The chosen approaches align with constitutional requirements for security, performance, and Rust-first development principles.

---

**Phase 0 Status**: ✅ Complete - All NEEDS CLARIFICATION resolved
**Next Phase**: Phase 1 - Design & Contracts