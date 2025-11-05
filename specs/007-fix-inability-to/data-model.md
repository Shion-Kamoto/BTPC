# Data Model: Transaction Sending Fix

## Core Entities

### Transaction
**Purpose**: Represents a blockchain transaction being created and sent

```rust
struct Transaction {
    id: String,              // Transaction hash
    inputs: Vec<TxInput>,     // UTXOs being spent
    outputs: Vec<TxOutput>,   // Recipients + change
    signatures: Vec<MLDSASignature>, // One per input
    fee: u64,                 // Calculated fee in satoshis
    status: TransactionStatus,
    created_at: Timestamp,
    broadcast_at: Option<Timestamp>,
}

enum TransactionStatus {
    Creating,      // Building inputs/outputs
    Signing,       // Generating ML-DSA signatures
    Broadcasting,  // Sending to network
    Pending,       // In mempool
    Confirmed,     // In block
    Failed(String), // Error message
}
```

### UTXO (Unspent Transaction Output)
**Purpose**: Track spendable outputs and their reservation status

```rust
struct UTXO {
    txid: String,           // Transaction ID
    vout: u32,              // Output index
    amount: u64,            // Value in satoshis
    address: String,        // Owner address
    public_key: Vec<u8>,    // ML-DSA public key
    lock_status: LockStatus,
    block_height: u32,      // Confirmation height
}

enum LockStatus {
    Available,                    // Can be spent
    Reserved(ReservationToken),   // Locked for transaction
    Spent,                        // Already used
}
```

### ReservationToken
**Purpose**: Track UTXO reservations to prevent double-spending

```rust
struct ReservationToken {
    id: Uuid,                   // Unique reservation ID
    transaction_id: Option<String>, // Associated transaction
    utxos: Vec<(String, u32)>, // (txid, vout) pairs
    created_at: Timestamp,
    expires_at: Timestamp,      // Auto-release after timeout
}
```

### WalletState
**Purpose**: Maintain wallet balance and transaction state (Article XI backend authority)

```rust
struct WalletState {
    wallet_id: String,
    balance: Balance,
    utxos: HashMap<(String, u32), UTXO>,
    pending_transactions: Vec<Transaction>,
    reserved_utxos: HashSet<(String, u32)>,
    last_sync_height: u32,
}

struct Balance {
    confirmed: u64,      // Spendable balance
    pending: u64,        // Unconfirmed incoming
    reserved: u64,       // Locked in transactions
    total: u64,          // confirmed + pending
}
```

### TransactionBuilder
**Purpose**: Temporary state during transaction creation

```rust
struct TransactionBuilder {
    inputs: Vec<SelectedUTXO>,
    outputs: Vec<PlannedOutput>,
    change_address: String,
    fee_rate: u64,           // Satoshis per byte
    total_input: u64,
    total_output: u64,
    estimated_size: usize,   // Bytes
}

struct SelectedUTXO {
    utxo: UTXO,
    private_key: Vec<u8>,    // Encrypted
    seed: Option<Vec<u8>>,   // For key regeneration
}

struct PlannedOutput {
    address: String,
    amount: u64,
    is_change: bool,
}
```

## Relationships

### Entity Relationships
```
WalletState --owns--> [UTXO]
WalletState --tracks--> [Transaction]
Transaction --consumes--> [UTXO]
Transaction --creates--> [UTXO] (outputs)
ReservationToken --locks--> [UTXO]
TransactionBuilder --reads--> WalletState
TransactionBuilder --produces--> Transaction
```

### State Transitions

#### UTXO Lifecycle
```
Available -> Reserved(token) -> Spent
    |            |
    |            v (timeout/cancel)
    <------------
```

#### Transaction Lifecycle
```
Creating -> Signing -> Broadcasting -> Pending -> Confirmed
    |          |            |            |
    v          v            v            v
  Failed    Failed       Failed       Failed
```

## Event Model (Article XI)

### Transaction Events
```typescript
interface TransactionInitiated {
    wallet_id: string;
    amount: number;
    recipient: string;
}

interface TransactionValidated {
    transaction_id: string;
    inputs_count: number;
    outputs_count: number;
    fee: number;
}

interface TransactionSigned {
    transaction_id: string;
    signatures_count: number;
}

interface TransactionBroadcast {
    transaction_id: string;
    network_response: string;
}

interface TransactionConfirmed {
    transaction_id: string;
    block_height: number;
    confirmations: number;
}

interface TransactionFailed {
    transaction_id?: string;
    error_type: string;
    error_message: string;
    recoverable: boolean;
}
```

## Storage Schema (RocksDB)

### Column Families
```
utxos: (txid, vout) -> UTXO
transactions: txid -> Transaction
reservations: token_id -> ReservationToken
wallet_state: wallet_id -> WalletState
```

### Indexes
```
utxos_by_address: address -> [(txid, vout)]
pending_transactions: wallet_id -> [txid]
reserved_utxos: token_id -> [(txid, vout)]
```

## Validation Rules

### Transaction Creation
1. Sum of inputs >= sum of outputs + fee
2. All input UTXOs must be Available or Reserved by this transaction
3. All output addresses must be valid BTPC addresses
4. Fee must be >= minimum network fee
5. Change output required if input > output + fee

### UTXO Selection
1. Prefer older UTXOs (more confirmations)
2. Minimize number of inputs (reduce tx size)
3. Avoid dust outputs (< 1000 satoshis)
4. Reserve selected UTXOs immediately

### Signature Generation
1. Must have private key for each input
2. Must have seed if key requires regeneration
3. Sign transaction hash with ML-DSA
4. Verify signature before broadcast

## Error Taxonomy

### Validation Errors
- `INVALID_ADDRESS`: Recipient address format incorrect
- `INVALID_AMOUNT`: Amount <= 0 or > max supply
- `INSUFFICIENT_FUNDS`: Balance < amount + fee
- `UTXO_LOCKED`: Required UTXOs already reserved

### Signing Errors
- `KEY_NOT_FOUND`: Private key missing for input
- `SEED_MISSING`: Cannot regenerate key without seed
- `SIGNATURE_FAILED`: ML-DSA signing error
- `WALLET_LOCKED`: Password required for decryption

### Network Errors
- `NODE_UNAVAILABLE`: Cannot connect to RPC
- `BROADCAST_FAILED`: Transaction rejected by network
- `MEMPOOL_FULL`: Transaction queued locally
- `FEE_TOO_LOW`: Network requires higher fee

### System Errors
- `STORAGE_ERROR`: RocksDB operation failed
- `TIMEOUT_ERROR`: Transaction took too long
- `CORRUPTION_ERROR`: Wallet file damaged