//! Transaction Monitoring Service - Feature 007
//!
//! Background service that monitors broadcast transactions for confirmations
//! and automatically releases UTXO reservations when transactions are confirmed.

use crate::rpc_client::RpcClient;
use crate::transaction_commands::{TransactionState, TransactionStateManager, TransactionStatus};
use crate::events::{TransactionEvent, UTXOEvent, ReleaseReason};
use crate::utxo_manager::UTXOManager;
use crate::AppState;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

/// Transaction monitoring service that runs in the background
pub struct TransactionMonitor {
    /// Reference to transaction state manager
    tx_state_manager: Arc<TransactionStateManager>,
    /// Reference to UTXO manager for releasing reservations
    utxo_manager: Arc<Mutex<UTXOManager>>,
    /// RPC client for checking transaction status
    rpc_port: u16,
    /// App handle for emitting events
    app: AppHandle,
    /// Polling interval (seconds)
    poll_interval: u64,
}

impl TransactionMonitor {
    /// Create a new transaction monitor
    pub async fn new(
        app_state: &AppState,
        app: AppHandle,
        poll_interval: u64,
    ) -> Self {
        let rpc_port = *app_state.active_rpc_port.read().await;

        Self {
            tx_state_manager: app_state.tx_state_manager.clone(),
            utxo_manager: app_state.utxo_manager.clone(),
            rpc_port,
            app,
            poll_interval,
        }
    }

    /// Start the monitoring service in the background
    pub async fn start(self) {
        println!("🔎 Starting transaction monitor (polling every {}s)", self.poll_interval);

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
    async fn check_pending_transactions(&self) {
        // Get all transactions in Broadcast or Confirming state
        let pending_txs = self.tx_state_manager.get_pending_transactions();

        if pending_txs.is_empty() {
            return;
        }

        println!("🔎 Checking {} pending transactions", pending_txs.len());

        // Connect to RPC node
        let rpc_client = RpcClient::new("127.0.0.1", self.rpc_port);

        // Check if node is available
        if !rpc_client.ping().await.unwrap_or(false) {
            println!("⚠️  Cannot connect to RPC node for transaction monitoring");
            return;
        }

        // Check each transaction
        for tx_state in pending_txs {
            self.check_transaction_status(&rpc_client, &tx_state).await;
        }
    }

    /// Check the status of a single transaction
    async fn check_transaction_status(&self, rpc_client: &RpcClient, tx_state: &TransactionState) {
        let tx_id = &tx_state.transaction_id;

        // Query RPC for transaction info
        match rpc_client.get_transaction(tx_id).await {
            Ok(tx_info) => {
                // Check confirmations
                let confirmations = tx_info.confirmations.unwrap_or(0);

                if confirmations >= 1 {
                    // Transaction is confirmed!
                    self.handle_confirmation(tx_state, confirmations, &tx_info).await;
                } else if tx_state.status == TransactionStatus::Broadcast {
                    // Move to Confirming state (waiting for first confirmation)
                    self.tx_state_manager.set_state(
                        tx_id.clone(),
                        TransactionStatus::Confirming,
                        None,
                    );
                    println!("⏳ Transaction {} is confirming (0 confirmations)", tx_id);
                }
            }
            Err(e) => {
                // Transaction not found in node (might have been dropped)
                if tx_state.status == TransactionStatus::Broadcast {
                    println!("⚠️  Transaction {} not found in node: {}", tx_id, e);
                    // Don't mark as failed immediately - might just be propagating
                }
            }
        }
    }

    /// Handle a confirmed transaction
    async fn handle_confirmation(&self, tx_state: &TransactionState, confirmations: u64, tx_info: &crate::rpc_client::TransactionInfo) {
        let tx_id = &tx_state.transaction_id;

        // Only process if not already marked as confirmed
        if tx_state.status == TransactionStatus::Confirmed {
            return;
        }

        println!("✅ Transaction {} confirmed ({} confirmations)", tx_id, confirmations);

        // Update state to confirmed
        self.tx_state_manager.set_state(
            tx_id.clone(),
            TransactionStatus::Confirmed,
            None,
        );

        // Emit confirmation event
        let _ = self.app.emit("transaction:confirmed", TransactionEvent::TransactionConfirmed {
            transaction_id: tx_id.clone(),
            confirmations: confirmations as u32,
            block_height: tx_info.blockheight.unwrap_or(0),
            block_hash: tx_info.blockhash.clone().unwrap_or_else(|| "unknown".to_string()),
        });

        // Release UTXO reservations
        self.release_utxo_reservation(tx_state).await;

        println!("✅ Transaction {} fully processed and confirmed", tx_id);
    }

    /// Release UTXO reservations for a confirmed transaction
    async fn release_utxo_reservation(&self, tx_state: &TransactionState) {
        if let (Some(reservation_token), Some(utxo_keys)) =
            (&tx_state.reservation_token, &tx_state.utxo_keys) {

            let utxo_manager = self.utxo_manager.lock().expect("Mutex poisoned");

            match utxo_manager.release_reservation(reservation_token) {
                Ok(_) => {
                    println!("✅ Released UTXO reservation: {}", reservation_token);

                    // Emit UTXO released event
                    let _ = self.app.emit("utxo:released", UTXOEvent::UTXOReleased {
                        reservation_token: reservation_token.clone(),
                        reason: ReleaseReason::TransactionConfirmed,
                        utxo_count: utxo_keys.len(),
                    });
                }
                Err(e) => {
                    println!("⚠️  Failed to release UTXO reservation {}: {}", reservation_token, e);
                }
            }
        } else {
            println!("⚠️  Transaction {} has no reservation info to release", tx_state.transaction_id);
        }
    }
}

/// Initialize and start the transaction monitor
pub async fn start_transaction_monitor(app_state: &AppState, app: AppHandle) {
    let monitor = TransactionMonitor::new(
        app_state,
        app,
        30, // Poll every 30 seconds
    ).await;

    monitor.start().await;
    println!("✅ Transaction monitor started");
}