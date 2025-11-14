# Data Model: Core Blockchain Implementation

**Feature**: Core Blockchain Implementation
**Branch**: `001-core-blockchain-implementation`
**Phase**: 1 - Design & Contracts
**Generated**: 2025-09-30

## Overview

This data model defines the core blockchain entities for BTPC (Bitcoin-Time Protocol Chain), a quantum-resistant cryptocurrency with linear decay economics. All entities follow Bitcoin-compatible structures while incorporating ML-DSA signatures and SHA-512 hashing.

## Core Blockchain Entities

### Block

**Purpose**: Represents a single block in the blockchain containing validated transactions.

**Attributes**:
- `header: BlockHeader` - Block metadata and validation info
- `transactions: Vec<Transaction>` - List of transactions in the block
- `size: u32` - Block size in bytes (max 1MB per constitution)
- `hash: Hash` - SHA-512 hash of the block header

**Validation Rules**:
- Block size MUST NOT exceed 1MB (1,048,576 bytes)
- All transactions MUST be valid and reference valid UTXOs
- Block header MUST contain valid SHA-512 proof-of-work
- Merkle root MUST match calculated root of all transactions
- Timestamp MUST be within acceptable network time variance

**State Transitions**:
- New → Validated (when PoW and transaction validation succeeds)
- Validated → Accepted (when added to blockchain)
- Accepted → Finalized (after sufficient confirmations)

### BlockHeader

**Purpose**: Contains block metadata and proof-of-work validation data.

**Attributes**:
- `prev_hash: Hash` - SHA-512 hash of previous block header
- `merkle_root: Hash` - SHA-512 merkle root of all transactions
- `timestamp: u32` - Block creation timestamp (Unix time)
- `bits: u32` - Difficulty target in compact notation
- `nonce: u64` - Proof-of-work nonce value
- `version: u32` - Block version for protocol upgrades

**Validation Rules**:
- `prev_hash` MUST reference valid previous block
- `merkle_root` MUST match calculated merkle tree root
- `timestamp` MUST be greater than median of last 11 blocks
- `bits` MUST represent valid difficulty target
- Block hash MUST be less than difficulty target

### Transaction

**Purpose**: Represents a transfer of value using Bitcoin-compatible UTXO model with ML-DSA signatures.

**Attributes**:
- `version: u32` - Transaction version for format compatibility
- `inputs: Vec<TransactionInput>` - UTXOs being spent
- `outputs: Vec<TransactionOutput>` - New UTXOs being created
- `lock_time: u32` - Earliest time transaction can be mined
- `hash: Hash` - SHA-512 hash of transaction data

**Validation Rules**:
- Sum of input values MUST equal sum of output values plus fees
- All inputs MUST reference valid, unspent UTXOs
- All ML-DSA signatures MUST be valid and authorize spending
- Transaction size MUST NOT exceed reasonable limits
- No double-spending of UTXOs

**State Transitions**:
- Created → Validated (when signatures and UTXOs verified)
- Validated → Mempool (when added to transaction pool)
- Mempool → Confirmed (when included in valid block)

### TransactionInput

**Purpose**: References a UTXO being spent with authorization signature.

**Attributes**:
- `previous_output: OutPoint` - Reference to UTXO being spent
- `signature_script: MLDSASignature` - ML-DSA signature authorizing spend
- `sequence: u32` - Transaction sequence number for replacement

**Validation Rules**:
- `previous_output` MUST reference existing, unspent UTXO
- `signature_script` MUST be valid ML-DSA signature
- Signature MUST authorize spending of referenced UTXO
- UTXO value MUST be positive

### TransactionOutput

**Purpose**: Creates a new UTXO that can be spent in future transactions.

**Attributes**:
- `value: u64` - Amount in base units (1 BTPC = 100,000,000 base units)
- `script_pubkey: PublicKey` - ML-DSA public key that can spend this output
- `address: String` - BTPC address derived from public key

**Validation Rules**:
- `value` MUST be positive and not exceed total supply
- `script_pubkey` MUST be valid ML-DSA public key
- `address` MUST be properly formatted BTPC address

### UTXO (Unspent Transaction Output)

**Purpose**: Represents spendable coins in the UTXO set for double-spend prevention.

**Attributes**:
- `outpoint: OutPoint` - Transaction hash and output index
- `output: TransactionOutput` - The actual output data
- `height: u32` - Block height where UTXO was created
- `coinbase: bool` - Whether this UTXO came from block reward

**Validation Rules**:
- MUST NOT be already spent in any confirmed transaction
- Block height MUST be valid and confirmed
- Coinbase UTXOs require additional confirmation depth

**State Transitions**:
- Created → Confirmed (when block is confirmed)
- Confirmed → Spent (when referenced by valid transaction input)

### BlockReward

**Purpose**: Calculates block rewards using linear decay economics.

**Attributes**:
- `height: u32` - Block height for reward calculation
- `base_reward: u64` - Calculated reward amount
- `total_fees: u64` - Sum of transaction fees in block
- `total_reward: u64` - Base reward plus fees

**Validation Rules**:
- Reward MUST follow linear decay formula: starts at 32.375 BTPC
- Linear decay over 24 years (1,261,440 blocks)
- After block 1,261,440: permanent 0.5 BTPC tail emission
- Mathematical precision to 8 decimal places required

**Linear Decay Formula**:
```
if height <= 1_261_440:
    base_reward = 32.375 - (height * 32.375 / 1_261_440)
else:
    base_reward = 0.5
```

### DifficultyTarget

**Purpose**: Manages SHA-512 proof-of-work difficulty adjustment every 2016 blocks.

**Attributes**:
- `height: u32` - Block height of difficulty adjustment
- `target: [u8; 64]` - 64-byte SHA-512 difficulty target
- `bits: u32` - Compact representation of target
- `adjustment_factor: f64` - Multiplier applied for adjustment

**Validation Rules**:
- Adjustment every 2016 blocks (approximately 2 weeks)
- Target time: 10 minutes per block average
- Adjustment factor MUST be between 0.25 and 4.0 (max ±75% change)
- SHA-512 hash MUST be less than target for valid block

### NetworkState

**Purpose**: Tracks overall blockchain and consensus state.

**Attributes**:
- `best_block_hash: Hash` - Hash of current best block
- `best_block_height: u32` - Height of current best block
- `total_work: BigInt` - Cumulative proof-of-work in chain
- `utxo_set_hash: Hash` - Hash of current UTXO set state
- `total_supply: u64` - Current total BTPC in circulation
- `network_hashrate: u64` - Estimated network hash rate

**Validation Rules**:
- Best block MUST be valid and have most cumulative work
- Total supply MUST match sum of all unspent outputs
- UTXO set hash MUST match actual UTXO set
- Network hashrate estimated from recent block times

### MLDSAKeyPair

**Purpose**: Quantum-resistant key pair for transaction authorization.

**Attributes**:
- `private_key: PrivateKey` - ML-DSA private key (secured, zeroized)
- `public_key: PublicKey` - ML-DSA public key (1,952 bytes for ML-DSA-65)
- `address: String` - Derived BTPC address for receiving funds

**Validation Rules**:
- Private key MUST be securely generated and stored
- Public key MUST be valid ML-DSA-65 public key
- Address MUST be properly derived from public key hash

### MLDSASignature

**Purpose**: Quantum-resistant digital signature for transaction authorization.

**Attributes**:
- `signature: [u8; 3309]` - ML-DSA-65 signature (3,309 bytes)
- `sighash_type: u8` - Type of data being signed
- `public_key: PublicKey` - Public key for signature verification

**Validation Rules**:
- Signature MUST be valid ML-DSA-65 signature
- Signature verification MUST complete within 1.5ms
- Public key MUST match the one authorizing the spend

## Relationships

### Primary Relationships
- **Block** → contains multiple **Transaction**
- **Transaction** → has multiple **TransactionInput** and **TransactionOutput**
- **TransactionInput** → references one **UTXO**
- **TransactionOutput** → creates one **UTXO**
- **Block** → generates one **BlockReward**

### State Dependencies
- **NetworkState** tracks current **Block** and **UTXO** set
- **DifficultyTarget** adjusts based on **Block** timing
- **UTXO** set updated by **Transaction** inputs/outputs
- **BlockReward** calculated per **Block** height

## Performance Considerations

### Database Indexing
- Index blocks by height and hash
- Index transactions by hash and block
- Index UTXOs by outpoint for fast lookup
- Index addresses for balance queries

### Caching Strategy
- Cache recent blocks and transactions
- Cache UTXO set for fast validation
- Cache difficulty targets and network state
- Cache signature verification results

### Memory Management
- Use zeroization for private keys
- Implement proper cleanup for sensitive data
- Optimize serialization for network protocols
- Batch operations for database updates

---

**Status**: ✅ Complete - All blockchain entities defined
**Next**: Generate API contracts from functional requirements