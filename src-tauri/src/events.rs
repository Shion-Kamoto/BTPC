use serde::{Deserialize, Serialize};
use tauri::Emitter;

// ============================================================================
// Blockchain Events (for integration tests - events_tests.rs)
// ============================================================================

/// Blockchain event types for node synchronization and block notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload")]
pub enum BlockchainEvent {
    /// New block added to the blockchain
    BlockAdded {
        height: u64,
        hash: String,
        timestamp: u64,
        tx_count: usize,
    },
    /// Sync progress updated
    SyncProgressUpdated {
        current_height: u64,
        target_height: u64,
        is_syncing: bool,
        connected_peers: u32,
    },
}

/// Mining event types for hashrate and block discovery notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload")]
pub enum MiningEvent {
    /// Hashrate statistics updated (every 5 seconds)
    HashrateUpdated {
        total_hashrate: f64,
        cpu_hashrate: f64,
        gpu_hashrate: f64,
        blocks_found: u64,
    },
    /// Block successfully mined
    BlockMined {
        block_hash: String,
        block_height: u64,
        reward_btpc: f64,
    },
}

// ============================================================================
// Wallet Events (for integration tests - test_wallet_events.rs)
// ============================================================================

/// Event emitted when a wallet is created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletCreatedEvent {
    pub wallet_id: String,
    pub address: String,
    pub version: String,
    pub created_at: String,
}

/// Event emitted when a wallet is recovered from mnemonic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletRecoveredEvent {
    pub wallet_id: String,
    pub address: String,
    pub version: String,
    pub recovered_at: String,
}

/// Emit a wallet event (placeholder for test compatibility)
/// In production, this uses Tauri's event system
#[cfg(test)]
pub fn emit_wallet_event<T: Serialize>(_event_name: &str, _payload: T) -> Result<(), String> {
    // Test stub - actual implementation uses Tauri's emit
    Ok(())
}

#[cfg(not(test))]
pub fn emit_wallet_event<T: Serialize>(_event_name: &str, _payload: T) -> Result<(), String> {
    // Production: would use app_handle.emit() but requires Tauri context
    // This function exists for API compatibility
    Ok(())
}

// ============================================================================
// Transaction Events (Article XI compliance)
// ============================================================================

/// Transaction event types for Article XI compliance
/// Backend emits these events for frontend state synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload")]
pub enum TransactionEvent {
    /// Transaction creation initiated by user
    TransactionInitiated {
        wallet_id: String,
        recipient: String,
        amount: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Transaction parameters validated
    TransactionValidated {
        transaction_id: String,
        inputs_count: usize,
        outputs_count: usize,
        fee: u64,
        change_amount: u64,
        total_input: u64,
        total_output: u64,
    },

    /// Fee estimated for transaction (T018)
    FeeEstimated {
        transaction_id: Option<String>,
        estimated_fee: u64,
        fee_rate: u64,
        estimated_size: usize,
    },

    /// ML-DSA signature generation started
    SigningStarted {
        transaction_id: String,
        inputs_to_sign: usize,
    },

    /// Individual input signed
    InputSigned {
        transaction_id: String,
        input_index: usize,
        signature_algorithm: String,
    },

    /// All inputs signed successfully
    TransactionSigned {
        transaction_id: String,
        signatures_count: usize,
        ready_to_broadcast: bool,
    },

    /// Transaction sent to network
    TransactionBroadcast {
        transaction_id: String,
        broadcast_to_peers: usize,
        network_response: String,
    },

    /// Transaction accepted into mempool
    MempoolAccepted {
        transaction_id: String,
        mempool_size: usize,
        position: usize,
    },

    /// Transaction included in block
    TransactionConfirmed {
        transaction_id: String,
        block_height: u64,
        block_hash: String,
        confirmations: u32,
    },

    /// Confirmation count updated
    ConfirmationUpdate {
        transaction_id: String,
        confirmations: u32,
        is_final: bool,
    },

    /// Transaction failed at any stage
    TransactionFailed {
        transaction_id: Option<String>,
        stage: TransactionStage,
        error_type: String,
        error_message: String,
        recoverable: bool,
        suggested_action: Option<String>,
    },

    /// Transaction cancelled by user
    TransactionCancelled {
        transaction_id: String,
        utxos_released: usize,
    },
}

/// UTXO reservation events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload")]
pub enum UTXOEvent {
    /// UTXOs reserved for transaction
    UTXOReserved {
        reservation_token: String,
        transaction_id: Option<String>,
        utxo_count: usize,
        total_amount: u64,
        expires_at: chrono::DateTime<chrono::Utc>,
    },

    /// UTXOs released
    UTXOReleased {
        reservation_token: String,
        reason: ReleaseReason,
        utxo_count: usize,
    },
}

/// Wallet balance events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload")]
pub enum WalletEvent {
    /// Wallet balance changed
    BalanceUpdated {
        wallet_id: String,
        balance: WalletBalance,
        change_type: BalanceChangeType,
    },
}

/// Fee estimation events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload")]
pub enum FeeEvent {
    /// Fee calculation completed
    FeeEstimated {
        transaction_size: usize,
        fee_rate: u64,
        total_fee: u64,
        priority: FeePriority,
    },
}

/// Transaction stages for error reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStage {
    Validation,
    UTXOSelection,
    FeeCalculation,
    Signing,
    Broadcasting,
    Confirmation,
}

/// Reasons for UTXO release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReleaseReason {
    TransactionCancelled,
    TransactionFailed,
    ReservationExpired,
    TransactionConfirmed,
}

/// Types of balance changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BalanceChangeType {
    TransactionSent,
    TransactionReceived,
    TransactionConfirmed,
    UTXOReserved,
    UTXOReleased,
}

/// Fee priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeePriority {
    Low,
    Normal,
    High,
    Custom(u64),
}

/// Wallet balance structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub confirmed: u64,
    pub pending: u64,
    pub reserved: u64,
    pub total: u64,
}

/// Event names for Tauri emission (Article XI compliance)
pub mod event_names {
    pub const TRANSACTION_INITIATED: &str = "transaction:initiated";
    pub const TRANSACTION_VALIDATED: &str = "transaction:validated";
    pub const TRANSACTION_SIGNING_STARTED: &str = "transaction:signing_started";
    pub const TRANSACTION_INPUT_SIGNED: &str = "transaction:input_signed";
    pub const TRANSACTION_SIGNED: &str = "transaction:signed";
    pub const TRANSACTION_BROADCAST: &str = "transaction:broadcast";
    pub const TRANSACTION_MEMPOOL_ACCEPTED: &str = "transaction:mempool_accepted";
    pub const TRANSACTION_CONFIRMED: &str = "transaction:confirmed";
    pub const TRANSACTION_CONFIRMATION_UPDATE: &str = "transaction:confirmation_update";
    pub const TRANSACTION_FAILED: &str = "transaction:failed";
    pub const TRANSACTION_CANCELLED: &str = "transaction:cancelled";

    pub const UTXO_RESERVED: &str = "utxo:reserved";
    pub const UTXO_RELEASED: &str = "utxo:released";

    pub const WALLET_BALANCE_UPDATED: &str = "wallet:balance_updated";

    pub const FEE_ESTIMATED: &str = "fee:estimated";
}

/// Helper trait for emitting events via Tauri
pub trait EmitTransactionEvent {
    fn emit_transaction_event(&self, event: TransactionEvent) -> Result<(), String>;
    fn emit_utxo_event(&self, event: UTXOEvent) -> Result<(), String>;
    fn emit_wallet_event(&self, event: WalletEvent) -> Result<(), String>;
    fn emit_fee_event(&self, event: FeeEvent) -> Result<(), String>;
}

impl<R: tauri::Runtime> EmitTransactionEvent for tauri::AppHandle<R> {
    fn emit_transaction_event(&self, event: TransactionEvent) -> Result<(), String> {
        let event_name = match &event {
            TransactionEvent::TransactionInitiated { .. } => event_names::TRANSACTION_INITIATED,
            TransactionEvent::TransactionValidated { .. } => event_names::TRANSACTION_VALIDATED,
            TransactionEvent::FeeEstimated { .. } => event_names::FEE_ESTIMATED,
            TransactionEvent::SigningStarted { .. } => event_names::TRANSACTION_SIGNING_STARTED,
            TransactionEvent::InputSigned { .. } => event_names::TRANSACTION_INPUT_SIGNED,
            TransactionEvent::TransactionSigned { .. } => event_names::TRANSACTION_SIGNED,
            TransactionEvent::TransactionBroadcast { .. } => event_names::TRANSACTION_BROADCAST,
            TransactionEvent::MempoolAccepted { .. } => event_names::TRANSACTION_MEMPOOL_ACCEPTED,
            TransactionEvent::TransactionConfirmed { .. } => event_names::TRANSACTION_CONFIRMED,
            TransactionEvent::ConfirmationUpdate { .. } => {
                event_names::TRANSACTION_CONFIRMATION_UPDATE
            }
            TransactionEvent::TransactionFailed { .. } => event_names::TRANSACTION_FAILED,
            TransactionEvent::TransactionCancelled { .. } => event_names::TRANSACTION_CANCELLED,
        };

        self.emit(event_name, event)
            .map_err(|e| format!("Failed to emit transaction event: {}", e))
    }

    fn emit_utxo_event(&self, event: UTXOEvent) -> Result<(), String> {
        let event_name = match &event {
            UTXOEvent::UTXOReserved { .. } => event_names::UTXO_RESERVED,
            UTXOEvent::UTXOReleased { .. } => event_names::UTXO_RELEASED,
        };

        self.emit(event_name, event)
            .map_err(|e| format!("Failed to emit UTXO event: {}", e))
    }

    fn emit_wallet_event(&self, event: WalletEvent) -> Result<(), String> {
        let event_name = match &event {
            WalletEvent::BalanceUpdated { .. } => event_names::WALLET_BALANCE_UPDATED,
        };

        self.emit(event_name, event)
            .map_err(|e| format!("Failed to emit wallet event: {}", e))
    }

    fn emit_fee_event(&self, event: FeeEvent) -> Result<(), String> {
        let event_name = match &event {
            FeeEvent::FeeEstimated { .. } => event_names::FEE_ESTIMATED,
        };

        self.emit(event_name, event)
            .map_err(|e| format!("Failed to emit fee event: {}", e))
    }
}

// ============================================================================
// Disk Space Events (FR-058)
// ============================================================================

/// Disk space monitoring events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload")]
pub enum DiskSpaceEvent {
    /// Warning: disk space below 10GB threshold
    DiskSpaceWarning {
        partition: String,
        available_bytes: u64,
        available_formatted: String,
        threshold_bytes: u64,
    },
    /// Critical: sync paused due to low disk space (< 5GB)
    SyncPausedLowSpace {
        partition: String,
        available_bytes: u64,
        available_formatted: String,
    },
    /// Critical: mining prevented due to low disk space (< 2GB)
    MiningPreventedLowSpace {
        partition: String,
        available_bytes: u64,
        available_formatted: String,
    },
    /// Disk space recovered above threshold
    DiskSpaceRecovered {
        partition: String,
        available_bytes: u64,
        previous_alert_level: String,
    },
}

/// Event names for disk space events
pub mod disk_space_event_names {
    pub const DISK_SPACE_WARNING: &str = "disk:space_warning";
    pub const SYNC_PAUSED_LOW_SPACE: &str = "disk:sync_paused";
    pub const MINING_PREVENTED_LOW_SPACE: &str = "disk:mining_prevented";
    pub const DISK_SPACE_RECOVERED: &str = "disk:space_recovered";
}

// ============================================================================
// Chain Reorganization Events (FR-057)
// ============================================================================

/// Chain reorganization events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload")]
pub enum ChainReorgEvent {
    /// Reorganization detected - processing started
    ReorgDetected {
        fork_point_hash: String,
        fork_point_height: u64,
        current_tip_hash: String,
        competing_tip_hash: String,
    },
    /// Reorganization in progress
    ReorgInProgress {
        blocks_to_disconnect: u32,
        blocks_to_connect: u32,
        current_progress: u32,
    },
    /// Reorganization completed successfully
    ReorgCompleted {
        fork_point_hash: String,
        fork_point_height: u64,
        blocks_disconnected: u32,
        blocks_connected: u32,
        transactions_returned_to_mempool: u32,
        new_tip_hash: String,
        new_tip_height: u64,
    },
    /// Reorganization failed
    ReorgFailed {
        fork_point_hash: String,
        error: String,
        rollback_successful: bool,
    },
    /// Transaction confirmations invalidated by reorg
    ConfirmationsInvalidated {
        transaction_id: String,
        previous_confirmations: u32,
        new_status: String,
    },
}

/// Event names for chain reorg events
pub mod chain_reorg_event_names {
    pub const REORG_DETECTED: &str = "chain:reorg_detected";
    pub const REORG_IN_PROGRESS: &str = "chain:reorg_in_progress";
    pub const REORG_COMPLETED: &str = "chain:reorg_completed";
    pub const REORG_FAILED: &str = "chain:reorg_failed";
    pub const CONFIRMATIONS_INVALIDATED: &str = "chain:confirmations_invalidated";
}

/// Helper trait for emitting disk space and reorg events
pub trait EmitSystemEvent {
    fn emit_disk_space_event(&self, event: DiskSpaceEvent) -> Result<(), String>;
    fn emit_chain_reorg_event(&self, event: ChainReorgEvent) -> Result<(), String>;
}

impl<R: tauri::Runtime> EmitSystemEvent for tauri::AppHandle<R> {
    fn emit_disk_space_event(&self, event: DiskSpaceEvent) -> Result<(), String> {
        let event_name = match &event {
            DiskSpaceEvent::DiskSpaceWarning { .. } => disk_space_event_names::DISK_SPACE_WARNING,
            DiskSpaceEvent::SyncPausedLowSpace { .. } => disk_space_event_names::SYNC_PAUSED_LOW_SPACE,
            DiskSpaceEvent::MiningPreventedLowSpace { .. } => disk_space_event_names::MINING_PREVENTED_LOW_SPACE,
            DiskSpaceEvent::DiskSpaceRecovered { .. } => disk_space_event_names::DISK_SPACE_RECOVERED,
        };

        self.emit(event_name, event)
            .map_err(|e| format!("Failed to emit disk space event: {}", e))
    }

    fn emit_chain_reorg_event(&self, event: ChainReorgEvent) -> Result<(), String> {
        let event_name = match &event {
            ChainReorgEvent::ReorgDetected { .. } => chain_reorg_event_names::REORG_DETECTED,
            ChainReorgEvent::ReorgInProgress { .. } => chain_reorg_event_names::REORG_IN_PROGRESS,
            ChainReorgEvent::ReorgCompleted { .. } => chain_reorg_event_names::REORG_COMPLETED,
            ChainReorgEvent::ReorgFailed { .. } => chain_reorg_event_names::REORG_FAILED,
            ChainReorgEvent::ConfirmationsInvalidated { .. } => chain_reorg_event_names::CONFIRMATIONS_INVALIDATED,
        };

        self.emit(event_name, event)
            .map_err(|e| format!("Failed to emit chain reorg event: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_event_serialization() {
        let event = TransactionEvent::TransactionInitiated {
            wallet_id: "wallet123".to_string(),
            recipient: "btpc1qxy...".to_string(),
            amount: 50000000,
            timestamp: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("TransactionInitiated"));
        assert!(json.contains("wallet123"));
    }

    #[test]
    fn test_wallet_balance_serialization() {
        let balance = WalletBalance {
            confirmed: 100000000,
            pending: 50000000,
            reserved: 10000000,
            total: 150000000,
        };

        let json = serde_json::to_string(&balance).unwrap();
        assert!(json.contains("100000000"));
        assert!(json.contains("confirmed"));
    }
}
