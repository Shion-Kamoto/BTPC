//! TestEnvironment - Main test infrastructure

use super::{MockRpcClient, TestWallet};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use serde_json::Value;

/// Test environment with all necessary mocks and fixtures
pub struct TestEnvironment {
    /// Temporary directory for test wallets
    pub temp_dir: TempDir,

    /// Mock RPC client
    pub rpc: Arc<MockRpcClient>,

    /// Created test wallets
    pub wallets: Arc<Mutex<HashMap<String, TestWallet>>>,

    /// Emitted events (for verification)
    events: Arc<Mutex<Vec<(String, Value)>>>,

    /// Active transactions
    transactions: Arc<Mutex<HashMap<String, TransactionState>>>,
}

/// Transaction state for testing
#[derive(Debug, Clone)]
pub struct TransactionState {
    pub id: String,
    pub wallet_id: String,
    pub status: String,
    pub amount: u64,
    pub fee: u64,
}

impl TestEnvironment {
    /// Create new test environment
    pub fn new() -> Result<Self, String> {
        let temp_dir = TempDir::new()
            .map_err(|e| format!("Failed to create temp dir: {:?}", e))?;

        Ok(Self {
            temp_dir,
            rpc: Arc::new(MockRpcClient::new()),
            wallets: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(Mutex::new(Vec::new())),
            transactions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Create test wallet with balance
    pub fn create_wallet_with_balance(
        &self,
        name: &str,
        balance: u64,
    ) -> Result<TestWallet, String> {
        let wallet = TestWallet::new_with_balance(&self.temp_dir, name, balance)?;

        // Add UTXOs to mock RPC
        for (index, amount) in wallet.utxos.iter().enumerate() {
            self.rpc.add_utxo(&wallet.address, index as u32, *amount);
        }

        // Store wallet
        self.wallets.lock().unwrap()
            .insert(wallet.id.clone(), wallet.clone());

        Ok(wallet)
    }

    /// Create test wallet with specific UTXOs
    pub fn create_wallet_with_utxos(
        &self,
        name: &str,
        utxo_amounts: Vec<u64>,
    ) -> Result<TestWallet, String> {
        let wallet = TestWallet::new_with_utxos(&self.temp_dir, name, utxo_amounts)?;

        // Add UTXOs to mock RPC
        for (index, amount) in wallet.utxos.iter().enumerate() {
            self.rpc.add_utxo(&wallet.address, index as u32, *amount);
        }

        // Store wallet
        self.wallets.lock().unwrap()
            .insert(wallet.id.clone(), wallet.clone());

        Ok(wallet)
    }

    /// Get wallet by ID
    pub fn get_wallet(&self, wallet_id: &str) -> Option<TestWallet> {
        self.wallets.lock().unwrap()
            .get(wallet_id)
            .cloned()
    }

    /// Track emitted event
    pub fn track_event(&self, event_name: &str, payload: Value) {
        self.events.lock().unwrap()
            .push((event_name.to_string(), payload));
    }

    /// Get all emitted events
    pub fn get_emitted_events(&self) -> Vec<String> {
        self.events.lock().unwrap()
            .iter()
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get events by name
    pub fn get_events_by_name(&self, event_name: &str) -> Vec<Value> {
        self.events.lock().unwrap()
            .iter()
            .filter(|(name, _)| name == event_name)
            .map(|(_, payload)| payload.clone())
            .collect()
    }

    /// Verify event sequence
    pub fn verify_event_sequence(&self, expected: &[&str]) -> bool {
        let actual = self.get_emitted_events();
        expected.iter().all(|e| actual.contains(&e.to_string()))
    }

    /// Clear events (for test isolation)
    pub fn clear_events(&self) {
        self.events.lock().unwrap().clear();
    }

    /// Add transaction to tracking
    pub fn track_transaction(&self, tx_id: &str, wallet_id: &str, amount: u64, fee: u64) {
        let state = TransactionState {
            id: tx_id.to_string(),
            wallet_id: wallet_id.to_string(),
            status: "pending".to_string(),
            amount,
            fee,
        };

        self.transactions.lock().unwrap()
            .insert(tx_id.to_string(), state);
    }

    /// Get transaction state
    pub fn get_transaction(&self, tx_id: &str) -> Option<TransactionState> {
        self.transactions.lock().unwrap()
            .get(tx_id)
            .cloned()
    }

    /// Update transaction status
    pub fn update_transaction_status(&self, tx_id: &str, status: &str) {
        if let Some(tx) = self.transactions.lock().unwrap().get_mut(tx_id) {
            tx.status = status.to_string();
        }
    }

    /// Get mock RPC client
    pub fn rpc_client(&self) -> Arc<MockRpcClient> {
        Arc::clone(&self.rpc)
    }

    /// Set fee rate for testing
    pub fn set_fee_rate(&self, rate: u64) {
        self.rpc.set_fee_rate(rate);
    }

    /// Mine blocks (advance blockchain)
    pub fn mine_blocks(&self, count: u32) {
        self.rpc.mine_blocks(count);
    }
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new().expect("Failed to create test environment")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_environment() {
        let env = TestEnvironment::new().unwrap();
        assert_eq!(env.get_emitted_events().len(), 0);
    }

    #[test]
    fn test_create_wallet_in_environment() {
        let env = TestEnvironment::new().unwrap();
        let wallet = env.create_wallet_with_balance("test", 100_000_000).unwrap();

        assert_eq!(wallet.balance, 100_000_000);
        assert!(env.get_wallet(&wallet.id).is_some());
    }

    #[test]
    fn test_event_tracking() {
        let env = TestEnvironment::new().unwrap();

        env.track_event("transaction:initiated", serde_json::json!({"tx_id": "123"}));
        env.track_event("transaction:signed", serde_json::json!({"tx_id": "123"}));

        let events = env.get_emitted_events();
        assert_eq!(events.len(), 2);
        assert!(env.verify_event_sequence(&["transaction:initiated", "transaction:signed"]));
    }
}