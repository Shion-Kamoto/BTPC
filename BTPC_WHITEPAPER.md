# BTPC: Bitcoin-Time Protocol Chain

## A Quantum-Resistant Proof-of-Work Cryptocurrency with Sustainable Economics

**Version 2.2 — November 2026**

---

## Abstract

BTPC (Bitcoin-Time Protocol Chain) is a quantum-resistant, proof-of-work cryptocurrency that addresses two structural weaknesses of Bitcoin-class systems: vulnerability to cryptographically relevant quantum computers, and the long-term economic fragility of geometric halving schedules.

BTPC signs transactions with ML-DSA (Dilithium5, NIST post-quantum signature level 5), hashes with SHA-512 (64-byte digests), and derives keys with SHAKE-256 under domain separation. Its monetary policy replaces Bitcoin's stepwise halving with a continuous linear decay: the block reward starts at 32.375 BTPC and decreases linearly (an average of ≈2,525 credits per block) over 24 years (1,262,304 blocks), reaching a permanent tail emission of 0.5 BTPC. Total supply at the end of the decay period is approximately 20.75 million BTPC, after which annual issuance is fixed at ~26,298 BTPC and the inflation rate asymptotically approaches zero.

The core is a clean-room Rust implementation following Bitcoin's proven architecture — the UTXO model, proof-of-work consensus, 10-minute target block times, and 2,016-block difficulty adjustment — with every parameter verified against the codebase (see Appendix A).

---

## Table of Contents

1. Introduction
2. Design Principles
3. Cryptographic Foundations
4. Address Format
5. Economic Model: Linear Decay
6. Consensus Mechanism
7. Transaction System
8. Block Structure
9. Network Architecture
10. Security Considerations
11. Comparison with Bitcoin
12. Implementation Status
13. Conclusion
Appendix A — Parameter Reference
Appendix B — Mathematical Specification

---

## 1. Introduction

### 1.1 The Quantum Threat

Bitcoin and nearly all deployed blockchains secure ownership with ECDSA over secp256k1. Shor's algorithm breaks the elliptic-curve discrete-logarithm problem in polynomial time on a sufficiently large fault-tolerant quantum computer, allowing an adversary to recover private keys from exposed public keys and forge signatures. Grover's algorithm halves the effective security of symmetric primitives: SHA-256's 256-bit classical security drops to 128-bit against quantum search.

Blockchain transactions are permanent and publicly visible. The "harvest now, decrypt later" model means funds secured by quantum-vulnerable signatures are exposed today for attack the moment a capable machine arrives. BTPC's design premise is that long-term value storage cannot wait for that event.

### 1.2 The Halving Problem

Bitcoin's halving reduces the block subsidy by 50% every 210,000 blocks (~4 years). This creates abrupt, discontinuous revenue shocks for miners, transient hash-rate instability after each event, and — over the long run — a transition to a fee-only security model whose viability is unproven. No major blockchain has yet operated at scale under a fee-dominant security regime.

### 1.3 BTPC's Approach

BTPC addresses both problems in a single coherent design: post-quantum cryptography throughout, and a smooth linear-decay reward schedule that removes halving shocks while guaranteeing a perpetual minimum miner subsidy.

---

## 2. Design Principles

- **Quantum-first.** Every cryptographic primitive is post-quantum. No classical public-key cryptography (RSA, ECDSA, DSA) appears anywhere in the consensus-critical path.
- **Bitcoin-compatible architecture.** BTPC deliberately retains the structures that have survived 15 years of adversarial testing: the UTXO model, SHA-based proof-of-work, 10-minute target block times, and 2,016-block difficulty adjustment.
- **Predictable economics.** The emission schedule is a closed-form arithmetic function of block height. Any participant can compute the exact reward at any future height with integer arithmetic — no governance votes, oracles, or parameter updates.
- **Security through simplicity.** BTPC supports only Pay-to-Public-Key-Hash (P2PKH) transactions. There is no scripting language and no Turing-complete execution environment.
- **Memory safety.** The core is written in Rust, eliminating buffer overflows, use-after-free, and data races at the language level.

---

## 3. Cryptographic Foundations

### 3.1 Signatures: ML-DSA (Dilithium5)

BTPC uses ML-DSA at NIST security level 5 (the Dilithium5 parameter set). It is the only signature scheme used for transaction authorisation.

| Parameter | Size |
|---|---|
| Private key | 4,864 bytes |
| Public key | 2,592 bytes |
| Signature | 4,595 bytes |

The security basis is the Module-Learning-With-Errors (MLWE) and Module-Short-Integer-Solution (MSIS) lattice problems. The large signatures and keys are the unavoidable cost of post-quantum security; BTPC accepts this in exchange for eliminating the entire class of quantum signature-forgery attacks.

### 3.2 Hashing: SHA-512

All hashing uses SHA-512, producing 64-byte digests. Block hashes are double-hashed, following Bitcoin's pattern:

```
block_hash = SHA-512(SHA-512(block_header))
```

SHA-512 provides 256-bit security against Grover's algorithm (versus 128-bit for SHA-256), and the double hash prevents length-extension attacks.

### 3.3 Key Derivation: SHAKE-256

Wallet seed expansion uses SHAKE-256 (the SHA-3 extendable-output function) with a domain-separation tag so that the same BIP-39 mnemonic in another system yields different keys. The pipeline is: 32-byte entropy → BIP-39 mnemonic → SHAKE-256 expansion (domain-separated) → ML-DSA key generation. All-zero seeds are rejected.

### 3.4 Wallet Encryption

Wallet files are encrypted at rest with Argon2id (memory-hard password hashing, 64 MB / 3 iterations / 4 lanes) feeding AES-256-GCM authenticated encryption. Sensitive material (passwords, seeds, private keys) is held in zeroizing containers that wipe memory on drop.

### 3.5 Constant-Time Operations

Difficulty-target comparisons and credential verification use constant-time equality checks to avoid timing side channels.

---

## 4. Address Format

BTPC addresses use Base58Check encoding with a network version byte, mirroring Bitcoin:

| Network | Version byte | Prefix |
|---|---|---|
| Mainnet | 0x00 | `1` |
| Testnet | 0x6f | `m`/`n` |
| Regtest | 0x6f | `m`/`n` |

An address is derived from the public key as:

```
address = Base58Check(version_byte || RIPEMD-160(SHA-512(public_key)))
```

Note the double SHA-512 checksum (rather than Bitcoin's double SHA-256) and SHA-512 (rather than SHA-256) in the address hash, matching the 64-byte hash space used throughout.

---

## 5. Economic Model: Linear Decay

### 5.1 Constants

| Constant | Value |
|---|---|
| Initial reward (R0) | 3,237,500,000 credits (32.375 BTPC) |
| Tail emission (RT) | 50,000,000 credits (0.5 BTPC) |
| Blocks per year (B) | 52,596 (365.25 × 24 × 6) |
| Decay period (Y) | 24 years |
| Decay end height (N) | 1,262,304 blocks |
| Average decrease per block | ≈ 2,525.1445 credits (floor 2,525) |
| Base unit | 1 BTPC = 100,000,000 credits |

### 5.2 Reward Function

Let D = R0 − RT = 3,187,500,000 be the total decay amount and N = Y·B = 1,262,304 the decay-end height.

```
reward(h) = R0                            if h = 0
reward(h) = max(R0 − ⌊D·h/N⌋, RT)         if 0 < h < N
reward(h) = RT                            if h ≥ N
```

The reward is computed with integer-only arithmetic (u128 intermediates) so the result is bit-identical on every platform — a consensus requirement. The average decrease rate is Δ = D/N ≈ 2525.1445 credits per block; the stored constant `DECREASE_PER_BLOCK` = ⌊Δ⌋ = 2525 is its floor.

### 5.3 Reward Schedule

| Height | Reward (credits) | Reward (BTPC) |
|---|---|---|
| 0 (genesis) | 3,237,500,000 | 32.375 |
| 52,596 (year 1) | 3,104,687,500 | 31.047 |
| 631,152 (year 12) | 1,643,750,000 | 16.438 |
| 1,262,303 (last decay block) | 50,002,526 | 0.500 |
| 1,262,304 (first tail block) | 50,000,000 | 0.500 |

### 5.4 Supply and Inflation

Total supply at the end of the decay period (year 24) is approximately **20.75 million BTPC**. Thereafter, issuance is a fixed ~26,298 BTPC per year (0.5 BTPC × 52,596 blocks), and the inflation *rate* declines monotonically toward zero as the supply grows. The perpetual tail emission provides a permanent miner subsidy independent of fee-market conditions, and gradually replaces coins lost to key loss and user error.

---

## 6. Consensus Mechanism

### 6.1 Proof of Work

Proof-of-work is SHA-512 based. A block is valid when its double-SHA-512 header hash is numerically below the current difficulty target. The nonce is 4 bytes.

### 6.2 Difficulty Adjustment

Difficulty is recalculated every 2,016 blocks with a target timespan of 1,209,600 seconds (two weeks), retargeting toward the 600-second block interval. This preserves Bitcoin's proven responsiveness while operating in the 64-byte hash space.

### 6.3 Timestamp Rules

- Target block time: 600 seconds.
- Minimum spacing between consecutive blocks: 60 seconds.
- Maximum future timestamp: 7,200 seconds (2 hours) ahead of network time.
- Timestamps are validated against median-time-past, Bitcoin-style.

---

## 7. Transaction System

BTPC uses the UTXO model with Bitcoin-compatible structure. Each transaction consumes one or more outputs and creates new ones; ownership is transferred by satisfying a P2PKH script with a valid ML-DSA signature.

Only P2PKH is supported. There are no smart contracts, no multi-signature aggregation, and no scripting beyond the minimal `OP_DUP OP_HASH160 <hash> OP_EQUALVERIFY OP_CHECKSIG` pattern. A transaction carries a `fork_id` byte identifying the network (mainnet / testnet / regtest) to prevent cross-network replay.

---

## 8. Block Structure

The block header mirrors Bitcoin's, with the two hash fields expanded to 64 bytes for SHA-512:

| Field | Size |
|---|---|
| Version | 4 bytes |
| Previous block hash | 64 bytes |
| Merkle root | 64 bytes |
| Timestamp | 4 bytes |
| Bits (difficulty) | 4 bytes |
| Nonce | 4 bytes |
| **Total** | **144 bytes** |

The 144-byte header (versus Bitcoin's 80 bytes) is entirely attributable to the 64-byte digests replacing 32-byte ones.

---

## 9. Network Architecture

### 9.1 Network Parameters

| Network | P2P port | RPC port | Magic bytes |
|---|---|---|---|
| Mainnet | 18341 | 18340 | `B7 C0 1A 55` |
| Testnet | 18351 | 18350 | `F1 C0 BA 55` |
| Regtest | 18361 | 18360 | `DA B5 BF FA` |

Each network has a distinct P2P port and 4-byte magic value, preventing accidental cross-network connections. Nodes listen dual-stack on `[::]:<port>`.

### 9.2 Peer Discovery and Synchronisation

The network follows Bitcoin's bootstrap model:

- A persistent address manager (the analogue of Bitcoin's `addrman`) stores known peers in new/tried tables, bucketed by /16 (IPv4) and /32 (IPv6) network groups, and survives restarts via RocksDB.
- Bootstrap proceeds from a persisted `peers.dat` address book, LAN UDP discovery, DNS seeds, and manual peer-add — in that order. (At present the network is behind carrier-grade NAT with no hardcoded seed nodes, so fresh nodes rely on the address book and manual peer-add until a reachable seed is established.)
- Headers-first synchronisation is used, with per-peer and aggregate in-flight request caps, and a stall reaper that disconnects and penalises unresponsive peers.
- Self-connections are detected by comparing a random nonce exchanged in the version handshake, preventing a node from counting itself as a peer.

### 9.3 RPC Interface

Nodes expose an authenticated JSON-RPC 2.0 interface (mainnet 18340, testnet 18350, regtest 18360) used by the mining rig and operator tooling. Authentication is mandatory — the node refuses to start without valid credentials — and the interface can be secured with TLS (a self-signed certificate is generated by the one-command `--setup` flow).

---

## 10. Security Considerations

- **Post-quantum signatures** eliminate quantum forgery and key-recovery attacks against transaction ownership.
- **Constant-time operations** mitigate timing side channels in cryptographic comparisons.
- **Memory-safe implementation** (Rust) removes entire vulnerability classes.
- **Minimal scripting surface** (P2PKH only) sharply reduces the attack surface relative to general-purpose platforms.
- **Rate limiting and connection caps** (per-IP and per-subnet) mitigate denial-of-service and eclipse attacks.
- **Authenticated, TLS-capable RPC** protects the operator interface.

Known trade-offs, stated honestly: ML-DSA signatures and keys are orders of magnitude larger than ECDSA (4,595-byte signatures vs ~71 bytes), which increases on-chain storage and bandwidth costs; there is no native multi-signature aggregation; and ML-DSA is not yet supported in commercial hardware wallets.

---

## 11. Comparison with Bitcoin

| Property | Bitcoin | BTPC |
|---|---|---|
| Signature scheme | ECDSA (secp256k1) | ML-DSA (Dilithium5) |
| Quantum resistance | No | Yes (NIST level 5) |
| Hash function | SHA-256 (32 bytes) | SHA-512 (64 bytes) |
| Header size | 80 bytes | 144 bytes |
| Block time | ~10 minutes | ~10 minutes |
| Difficulty period | 2,016 blocks | 2,016 blocks |
| Reward model | Geometric halving | Linear decay + tail emission |
| Scripting | Full script | P2PKH only |
| Base unit | satoshi | credit |

---

## 12. Implementation Status

The core is implemented in Rust as a workspace: `btpc-core` (the library), `btpc_node` (headless full node), `btpc_wallet` (CLI wallet), `btpc_miner` (CPU miner), and a Tauri desktop wallet. A dedicated GPU mining rig (`BTPC_RIG`) mines against a node over authenticated HTTPS, and a BalenaEtcher-flashable OS image (`BTPC-OS`) boots commodity hardware straight into mining with NVIDIA and AMD GPU support.

Post-quantum cryptography, the linear-decay consensus, P2P networking with Bitcoin-style peer discovery, and the authenticated RPC layer are implemented and covered by automated tests. The project is near completion, with peer-discovery hardening and node↔rig security (mandatory auth + TLS + certificate pinning) being the most recent work.

---

## 13. Conclusion

BTPC demonstrates that quantum resistance and economic sustainability are achievable without abandoning the architectural lessons of Bitcoin. By pairing NIST-level post-quantum signatures and SHA-512 hashing with a continuous linear-decay monetary policy and a permanent tail emission, BTPC provides a store of value designed for the multi-decade horizon in which quantum computers will exist.

---

## Appendix A — Parameter Reference

Every protocol parameter in this document is verified against the implementation. Source locations (live tree, `/media/bob/HDD1/BTPC`):

| Parameter | Value | Source |
|---|---|---|
| P2P ports (mainnet/testnet/regtest) | 18341 / 18351 / 18361 | `btpc-core/src/network/mod.rs` |
| Magic bytes | `B7C01A55` / `F1C0BA55` / `DAB5BFFA` | `btpc-core/src/network/protocol.rs` |
| RPC ports (main/test/reg) | 18340 / 18350 / 18360 | `btpc-core/src/economics/constants.rs` |
| Protocol version | 70015 | `btpc-core/src/network/protocol.rs` |
| Target block time | 600 s | `btpc-core/src/consensus/mod.rs` |
| Min block spacing | 60 s | `btpc-core/src/consensus/mod.rs` |
| Max future block time | 7200 s | `btpc-core/src/consensus/mod.rs` |
| Difficulty period | 2,016 blocks | `btpc-core/src/consensus/difficulty.rs` |
| Genesis timestamp | 1735689600 (2025-01-01 UTC) | `btpc-core/src/blockchain/genesis.rs` |
| Initial reward | 3,237,500,000 credits | `btpc-core/src/consensus/rewards.rs` / `economics/rewards.rs` |
| Tail emission | 50,000,000 credits | (ibid.) |
| Decay end height | 1,262,304 blocks | (ibid.) |
| Average decrease per block | ≈ 2,525.1445 credits | `btpc-core/src/economics/constants.rs` |
| ML-DSA private key | 4,864 bytes | `btpc-core/src/crypto/mod.rs` |
| ML-DSA public key | 2,592 bytes | `btpc-core/src/crypto/mod.rs` |
| ML-DSA signature | 4,595 bytes | `btpc-core/src/crypto/mod.rs` |
| Address version bytes | 0x00 / 0x6f | `btpc-core/src/crypto/address.rs` |
| Credits per BTPC | 100,000,000 | economics module |

The economic model is independently verified by the `BTPC_VERIFICATION_ALGORITHM` script, which asserts the reward function at key heights, the decay-end supply (between 20.5 M and 21 M BTPC), and the year-over-year inflation schedule.

---

## Appendix B — Mathematical Specification

This appendix states the exact consensus and cryptographic calculations, matching the implementation. Let H(x) = SHA-512(x), and let ⌊·⌋ denote integer (floor) division.

### B.1 Hashing

Block and transaction identifiers use double SHA-512:

```
id(x) = H(H(x))
```

### B.2 Block Reward (Linear Decay)

With R0 = 3,237,500,000, RT = 50,000,000, B = 52,596, Y = 24, N = Y·B = 1,262,304, and D = R0 − RT = 3,187,500,000:

```
R(h) = R0                              if h = 0
R(h) = max(R0 − ⌊D·h/N⌋, RT)           if 0 < h < N
R(h) = RT                              if h ≥ N
```

The per-block decrease is Δ = D/N ≈ 2525.1445 credits/block; the stored constant `DECREASE_PER_BLOCK = ⌊Δ⌋ = 2525` is its floor.

### B.3 Total Supply

```
S(h) = Σ_{i=0}^{h} R(i)
```

Decay-period closed form (arithmetic series, used by `MAX_MONEY`):

```
S(N) ≈ N·(R0 + RT)/2 = 1,262,304 × 3,287,500,000 / 2 ≈ 2.075×10^15 credits ≈ 20.75M BTPC
```

### B.4 Inflation Rate

```
I(h) = (R(h) · B) / S(h) × 100%
```

### B.5 Maximum Supply

```
MAX_MONEY = N·(R0 + RT)/2 + 1000·B·RT
```

(the tail term uses 1000 years as the practical horizon).

### B.6 Difficulty Retarget

Every Nd = 2016 blocks, with target timespan T_target = 2016 × 600 s = 1,209,600 s:

```
ratio = T_actual / T_target
ratio = clamp(ratio, 1/4, 4)
target_next = target_prev × ratio
```

### B.7 Proof of Work

A block is valid if and only if its double-SHA-512 header hash is at or below the target (64-byte big-endian comparison):

```
H(H(header)) ≤ target
```

### B.8 Compact Target Encoding (bits)

The 64-byte target is serialised as a 32-bit compact "bits" value — a 1-byte exponent plus a 3-byte mantissa, Bitcoin-style, with the mantissa occupying the most-significant bytes of the 64-byte field.

### B.9 Merkle Root

Leaves are transaction IDs. Each internal node is the double SHA-512 of the concatenation of its two children; an odd node is duplicated (Bitcoin behaviour):

```
node = H(H(left ‖ right))
```

### B.10 Transaction Fee

```
fee = Σ input_UTXO_values − Σ output_values
```

### B.11 Address Derivation

```
address = Base58Check(version ‖ RIPEMD-160(H(public_key)))
checksum = H(H(payload))[0..4]
```

The Base58Check checksum is the first 4 bytes of the double SHA-512 of the payload.
