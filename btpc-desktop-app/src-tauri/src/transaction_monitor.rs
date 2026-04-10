//! Transaction Monitoring Service - Feature 007
//!
//! Background service that monitors broadcast transactions for confirmations
//! and automatically releases UTXO reservations when transactions are confirmed.
//!
//! FIX 2025-11-27: Uses embedded node instead of external RPC for transaction queries

use btpc_desktop_app::embedded_node::{EmbeddedNode, TransactionInfo};
use crate::events::{ReleaseReason, TransactionEvent, UTXOEvent};
use crate::utxo_manager::UTXOManager;
use crate::AppState;
use btpc_desktop_app::transaction_state::{
    TransactionState, TransactionStateManager, TransactionStatus,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use tokio::time::sleep;

/// Transaction monitoring service that runs in the background
pub struct TransactionMonitor {
    /// Reference to transaction state manager
    tx_state_manager: Arc<TransactionStateManager>,
    /// Reference to UTXO manager for releasing reservations
    utxo_manager: Arc<Mutex<UTXOManager>>,
    /// Embedded node for checking transaction status (replaces RPC)
    embedded_node: Arc<RwLock<EmbeddedNode>>,
    /// App handle for emitting events
    app: AppHandle,
    /// Polling interval (seconds)
    poll_interval: u64,
}

impl TransactionMonitor {
    /// Create a new transaction monitor
    /// FIX 2025-11-27: Now uses embedded node instead of RPC
    pub async fn new(app_state: &AppState, app: AppHandle, poll_interval: u64) -> Self {
        Self {
            tx_state_manager: app_state.tx_state_manager.clone(),
            utxo_manager: app_state.utxo_manager.clone(),
            embedded_node: app_state.embedded_node.clone(),
            app,
            poll_interval,
        }
    }

    /// Start the monitoring service in the background
    pub async fn start(self) {
        println!(
            "🔎 Starting transaction monitor (polling every {}s, using embedded node)",
            self.poll_interval
        );

        tokio::spawn(async move {
            loop {
                // Sleep first to allow time for transactions to broadcast
                sleep(Duration::from_secs(self.poll_interval)).await;

                // Check pending transactions
                self.check_pending_transactions().await;
            }
        });
    }

    /// Check all pending transactions for confirmations
    /// FIX 2025-11-27: Uses embedded node instead of RPC
    async fn check_pending_transactions(&self) {
        // Get all transactions in Broadcast or Confirming state
        let pending_txs = self.tx_state_manager.get_pending_transactions();

        if pending_txs.is_empty() {
            return;
        }

        println!("🔎 Checking {} pending transactions (via embedded node)", pending_txs.len());

        // Check each transaction using embedded node
        for tx_state in pending_txs {
            self.check_transaction_status(&tx_state).await;
        }
    }

    /// Check the status of a single transaction
    /// FIX 2025-11-27: Uses embedded node get_transaction_info() instead of RPC
    async fn check_transaction_status(&self, tx_state: &TransactionState) {
        let tx_id = &tx_state.transaction_id;

        // Query embedded node for transaction info
        let node = self.embedded_node.read().await;
        match node.get_transaction_info(tx_id).await {
            Ok(Some(tx_info)) => {
                // Check confirmations
                let confirmations = tx_info.confirmations as u64;

                if confirmations >= 1 {
                    // Transaction is confirmed!
                    self.handle_confirmation(tx_state, confirmations, &tx_info)
                        .await;
                } else if tx_state.status == TransactionStatus::Broadcast {
                    // Move to Confirming state (waiting for first confirmation)
                    self.tx_state_manager.set_state(
                        tx_id.clone(),
                        TransactionStatus::Confirming,
                        None,
                    );
                    println!("⏳ Transaction {} is confirming (0 confirmations)", &tx_id[..16.min(tx_id.len())]);
                }
            }
            Ok(None) => {
                // Transaction not found in mempool or blockchain
                if tx_state.status == TransactionStatus::Broadcast {
                    println!("⚠️  Transaction {} not found in embedded node (may be pending)", &tx_id[..16.min(tx_id.len())]);
                    // Don't mark as failed immediately - might just be propagating
                }
            }
            Err(e) => {
                // Error querying node
                println!("⚠️  Error checking transaction {}: {}", &tx_id[..16.min(tx_id.len())], e);
            }
        }
    }

    /// Handle a confirmed transaction
    /// FIX 2025-11-27: Updated to use embedded node TransactionInfo
    async fn handle_confirmation(
        &self,
        tx_state: &TransactionState,
        confirmations: u64,
        tx_info: &TransactionInfo,
    ) {
        let tx_id = &tx_state.transaction_id;

        // Only process if not already marked as confirmed
        if tx_state.status == TransactionStatus::Confirmed {
            return;
        }

        println!(
            "✅ Transaction {} confirmed ({} confirmations)",
            &tx_id[..16.min(tx_id.len())], confirmations
        );

        // Update state to confirmed
        self.tx_state_manager
            .set_state(tx_id.clone(), TransactionStatus::Confirmed, None);

        // Emit confirmation event
        let _ = self.app.emit(
            "transaction:confirmed",
            TransactionEvent::TransactionConfirmed {
                transaction_id: tx_id.clone(),
                confirmations: confirmations as u32,
                block_height: tx_info.block_height.unwrap_or(0),
                block_hash: tx_info
                    .block_hash
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
            },
        );

        // Release UTXO reservations
        self.release_utxo_reservation(tx_state).await;

        println!("✅ Transaction {} fully processed and confirmed", &tx_id[..16.min(tx_id.len())]);
    }

    /// Release UTXO reservations for a confirmed transaction
    async fn release_utxo_reservation(&self, tx_state: &TransactionState) {
        if let (Some(reservation_token), Some(utxo_keys)) =
            (&tx_state.reservation_token, &tx_state.utxo_keys)
        {
            let utxo_manager = self.utxo_manager.lock().expect("Mutex poisoned");

            match utxo_manager.release_reservation(reservation_token) {
                Ok(_) => {
                    println!("✅ Released UTXO reservation: {}", reservation_token);

                    // Emit UTXO released event
                    let _ = self.app.emit(
                        "utxo:released",
                        UTXOEvent::UTXOReleased {
                            reservation_token: reservation_token.clone(),
                            reason: ReleaseReason::TransactionConfirmed,
                            utxo_count: utxo_keys.len(),
                        },
                    );
                }
                Err(e) => {
                    println!(
                        "⚠️  Failed to release UTXO reservation {}: {}",
                        reservation_token, e
                    );
                }
            }
        } else {
            println!(
                "⚠️  Transaction {} has no reservation info to release",
                tx_state.transaction_id
            );
        }
    }
}

/// Initialize and start the transaction monitor
pub async fn start_transaction_monitor(app_state: &AppState, app: AppHandle) {
    let monitor = TransactionMonitor::new(
        app_state, app, 30, // Poll every 30 seconds
    )
    .await;

    monitor.start().await;
    println!("✅ Transaction monitor started");
}
