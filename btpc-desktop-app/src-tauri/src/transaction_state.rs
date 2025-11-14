//! Transaction State Management - Feature 007
//!
//! Provides in-memory transaction state tracking for the desktop application.
//! Moved from transaction_commands.rs for TD-001 refactoring (testability).

use chrono::Utc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Transaction from utxo_manager module
/// Re-exported to avoid circular dependency
pub use crate::utxo_manager::Transaction;

/// Transaction state tracking with full transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionState {
    pub transaction_id: String,
    pub status: TransactionStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub error: Option<String>,
    /// The actual transaction (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction: Option<Transaction>,
    /// UTXO reservation token for cleanup
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reservation_token: Option<String>,
    /// UTXO keys that were reserved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utxo_keys: Option<Vec<String>>,
    /// Wallet ID for this transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Creating,
    Validating,
    Signing,
    Signed,
    Broadcasting,
    Broadcast,
    Confirming,
    Confirmed,
    Failed,
    Cancelled,
}

/// Global transaction state manager with transaction storage
pub struct TransactionStateManager {
    transactions: Arc<Mutex<std::collections::HashMap<String, TransactionState>>>,
}

impl TransactionStateManager {
    pub fn new() -> Self {
        Self {
            transactions: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn set_state(&self, tx_id: String, status: TransactionStatus, error: Option<String>) {
        let mut txs = self.transactions.lock();
        let now = Utc::now().timestamp();

        txs.entry(tx_id.clone())
            .and_modify(|state| {
                state.status = status.clone();
                state.updated_at = now;
                state.error = error.clone();
            })
            .or_insert(TransactionState {
                transaction_id: tx_id,
                status,
                created_at: now,
                updated_at: now,
                error,
                transaction: None,
                reservation_token: None,
                utxo_keys: None,
                wallet_id: None,
            });
    }

    pub fn set_transaction(&self, tx_id: String, transaction: Transaction, status: TransactionStatus) {
        let mut txs = self.transactions.lock();
        let now = Utc::now().timestamp();

        txs.entry(tx_id.clone())
            .and_modify(|state| {
                state.transaction = Some(transaction.clone());
                state.status = status.clone();
                state.updated_at = now;
            })
            .or_insert(TransactionState {
                transaction_id: tx_id,
                status,
                created_at: now,
                updated_at: now,
                error: None,
                transaction: Some(transaction),
                reservation_token: None,
                utxo_keys: None,
                wallet_id: None,
            });
    }

    /// Set transaction with full details including reservation info
    pub fn set_transaction_with_reservation(
        &self,
        tx_id: String,
        transaction: Transaction,
        status: TransactionStatus,
        reservation_token: String,
        utxo_keys: Vec<String>,
        wallet_id: String,
    ) {
        let mut txs = self.transactions.lock();
        let now = Utc::now().timestamp();

        txs.entry(tx_id.clone())
            .and_modify(|state| {
                state.transaction = Some(transaction.clone());
                state.status = status.clone();
                state.updated_at = now;
                state.reservation_token = Some(reservation_token.clone());
                state.utxo_keys = Some(utxo_keys.clone());
                state.wallet_id = Some(wallet_id.clone());
            })
            .or_insert(TransactionState {
                transaction_id: tx_id,
                status,
                created_at: now,
                updated_at: now,
                error: None,
                transaction: Some(transaction),
                reservation_token: Some(reservation_token),
                utxo_keys: Some(utxo_keys),
                wallet_id: Some(wallet_id),
            });
    }

    pub fn get_state(&self, tx_id: &str) -> Option<TransactionState> {
        self.transactions.lock().get(tx_id).cloned()
    }

    pub fn get_transaction(&self, tx_id: &str) -> Option<Transaction> {
        self.transactions.lock().get(tx_id).and_then(|state| state.transaction.clone())
    }

    pub fn remove_state(&self, tx_id: &str) {
        self.transactions.lock().remove(tx_id);
    }

    /// Get all transactions that are in Broadcast or Confirming state
    pub fn get_pending_transactions(&self) -> Vec<TransactionState> {
        self.transactions.lock()
            .values()
            .filter(|state| {
                state.status == TransactionStatus::Broadcast ||
                state.status == TransactionStatus::Confirming
            })
            .cloned()
            .collect()
    }
}

impl Default for TransactionStateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_state_manager() {
        let manager = TransactionStateManager::new();

        // Set state
        manager.set_state(
            "tx_123".to_string(),
            TransactionStatus::Creating,
            None,
        );

        // Get state
        let state = manager.get_state("tx_123").unwrap();
        assert_eq!(state.transaction_id, "tx_123");
        assert_eq!(state.status, TransactionStatus::Creating);
        assert!(state.error.is_none());

        // Update state
        manager.set_state(
            "tx_123".to_string(),
            TransactionStatus::Signed,
            None,
        );

        let state = manager.get_state("tx_123").unwrap();
        assert_eq!(state.status, TransactionStatus::Signed);

        // Remove state
        manager.remove_state("tx_123");
        assert!(manager.get_state("tx_123").is_none());
    }

    #[test]
    fn test_transaction_status_transitions() {
        let manager = TransactionStateManager::new();
        let tx_id = "tx_test".to_string();

        // Valid transition: Creating -> Validating -> Signing -> Signed -> Broadcasting -> Broadcast
        let transitions = vec![
            TransactionStatus::Creating,
            TransactionStatus::Validating,
            TransactionStatus::Signing,
            TransactionStatus::Signed,
            TransactionStatus::Broadcasting,
            TransactionStatus::Broadcast,
            TransactionStatus::Confirming,
            TransactionStatus::Confirmed,
        ];

        for status in transitions {
            manager.set_state(tx_id.clone(), status.clone(), None);
            let state = manager.get_state(&tx_id).unwrap();
            assert_eq!(state.status, status);
        }
    }
}