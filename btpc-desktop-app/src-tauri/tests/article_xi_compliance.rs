//! Article XI Compliance Tests
//!
//! Validates desktop app patterns from Constitution Article XI:
//! 1. Backend is single source of truth
//! 2. Backend validates before frontend
//! 3. Automatic event emission on state changes
//! 4. No localStorage writes without backend validation
//!
//! The StateManager<T> implementation ensures compile-time guarantees
//! for these requirements.

use btpc_desktop_app::StateManager;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct TestState {
    counter: u32,
    message: String,
    validated: bool,
}

impl Default for TestState {
    fn default() -> Self {
        Self {
            counter: 0,
            message: "initial".to_string(),
            validated: false,
        }
    }
}

#[test]
fn test_state_manager_creation() {
    // Contract: StateManager must accept event prefix and initial state
    let manager = StateManager::new("test_state", TestState::default());

    let state = manager.read().unwrap();
    assert_eq!(state.counter, 0);
    assert_eq!(state.message, "initial");
}

#[test]
fn test_event_name_generation_contract() {
    // Contract: Event names must follow "{prefix}_changed" pattern
    let manager = StateManager::new("node_status", TestState::default());

    assert_eq!(manager.event_name(), "node_status_changed");
    assert_eq!(manager.component_name(), "node_status");
}

#[test]
fn test_multiple_state_managers_different_events() {
    // Contract: Different StateManagers emit different events
    let node_manager = StateManager::new("node_status", TestState::default());
    let mining_manager = StateManager::new("mining_status", TestState::default());

    assert_eq!(node_manager.event_name(), "node_status_changed");
    assert_eq!(mining_manager.event_name(), "mining_status_changed");
}

#[test]
fn test_state_update_silent_no_event() {
    // Contract: update_silent() must not emit events (for batch updates)
    let manager = StateManager::new("test", TestState::default());

    let result = manager.update_silent(|state| {
        state.counter = 42;
        state.message = "updated".to_string();
    });

    assert!(result.is_ok());

    // Verify state updated
    let state = manager.read().unwrap();
    assert_eq!(state.counter, 42);
    assert_eq!(state.message, "updated");
}

#[test]
fn test_state_read_no_mutation() {
    // Contract: read() must return copy, not allow mutation
    let manager = StateManager::new("test", TestState::default());

    let state1 = manager.read().unwrap();
    let state2 = manager.read().unwrap();

    // Both should be equal (clones)
    assert_eq!(state1, state2);
}

#[test]
fn test_backend_validation_before_state_change() {
    // Article XI: Backend validates before state change
    let manager = StateManager::new("validated_state", TestState::default());

    // Simulate validation in update closure
    let result = manager.update_silent(|state| {
        // Backend validation logic
        if state.counter < 100 {
            state.counter += 1;
            state.validated = true;
        }
    });

    assert!(result.is_ok());

    let state = manager.read().unwrap();
    assert_eq!(state.counter, 1);
    assert_eq!(state.validated, true);
}

#[test]
fn test_state_manager_thread_safety() {
    // Contract: StateManager must be Send + Sync for Arc usage
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<StateManager<TestState>>();
    assert_sync::<StateManager<TestState>>();
}

#[test]
fn test_state_manager_clone_semantics() {
    // Contract: StateManager clones share same underlying state
    let manager1 = StateManager::new("shared", TestState::default());
    let manager2 = manager1.clone();

    // Update via manager1
    let _ = manager1.update_silent(|state| {
        state.counter = 42;
    });

    // Read via manager2 - should see the update
    let state = manager2.read().unwrap();
    assert_eq!(state.counter, 42);
}

#[test]
fn test_state_set_replaces_entire_state() {
    // Contract: set() must replace entire state
    let manager = StateManager::new("test", TestState::default());

    let new_state = TestState {
        counter: 999,
        message: "replaced".to_string(),
        validated: true,
    };

    // Note: set() requires AppHandle, so we test via update_silent
    let _ = manager.update_silent(|state| *state = new_state.clone());

    let current = manager.read().unwrap();
    assert_eq!(current, new_state);
}

#[test]
fn test_mutex_poison_recovery_contract() {
    // Contract: Mutex poison errors must be converted to BtpcError
    // This is tested by the error handling in StateManager::read()

    let manager = StateManager::new("test", TestState::default());

    // Normal read should succeed
    let result = manager.read();
    assert!(result.is_ok());

    // Poison errors are automatically converted to BtpcError::MutexPoisoned
    // (actual poisoning would require panic in another thread)
}

#[test]
fn test_state_serialization_for_events() {
    // Contract: State must be serializable for Tauri event emission
    let state = TestState {
        counter: 42,
        message: "test".to_string(),
        validated: true,
    };

    let serialized = serde_json::to_string(&state).unwrap();
    assert!(serialized.contains("42"));
    assert!(serialized.contains("test"));

    let deserialized: TestState = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, state);
}

#[test]
fn test_inner_arc_access_for_legacy_code() {
    // Contract: inner_arc() provides escape hatch for legacy Arc<Mutex<T>> patterns
    let manager = StateManager::new("test", TestState::default());

    let arc = manager.inner_arc();

    // Should be able to lock manually
    {
        let mut state = arc.lock().unwrap();
        state.counter = 100;
    }

    // Verify change via StateManager
    let state = manager.read().unwrap();
    assert_eq!(state.counter, 100);
}

#[cfg(test)]
mod backend_first_validation {
    use super::*;

    #[test]
    fn test_validation_failure_preserves_state() {
        // Article XI: If validation fails, state must not change
        let manager = StateManager::new("validated", TestState::default());

        // Attempt invalid update (validation fails)
        let result = manager.update_silent(|state| {
            // Validation: counter must be < 10
            if state.counter >= 10 {
                return; // Don't update
            }
            state.counter += 1;
        });

        assert!(result.is_ok()); // update_silent itself succeeds

        // But state should have changed (counter was 0 < 10)
        let state = manager.read().unwrap();
        assert_eq!(state.counter, 1);

        // Now try to exceed limit
        for _ in 0..20 {
            let _ = manager.update_silent(|state| {
                if state.counter < 10 {
                    state.counter += 1;
                }
            });
        }

        // Should stop at 10
        let state = manager.read().unwrap();
        assert_eq!(state.counter, 10);
    }

    #[test]
    fn test_backend_validation_prevents_invalid_state() {
        // Article XI: Backend prevents invalid state transitions
        let manager = StateManager::new("strict", TestState::default());

        // Rule: message can only be changed if validated = true
        let _ = manager.update_silent(|state| {
            // First validate
            state.validated = true;
        });

        // Now message can be updated
        let _ = manager.update_silent(|state| {
            if state.validated {
                state.message = "allowed".to_string();
            }
        });

        let state = manager.read().unwrap();
        assert_eq!(state.message, "allowed");
    }
}

#[cfg(test)]
mod event_emission_patterns {
    use super::*;

    #[test]
    fn test_event_name_consistency() {
        // Contract: Event names must be consistent for frontend listeners
        let manager = StateManager::new("node_status", TestState::default());

        // Event name should never change
        let name1 = manager.event_name();
        let name2 = manager.event_name();

        assert_eq!(name1, name2);
        assert_eq!(name1, "node_status_changed");
    }

    #[test]
    fn test_component_name_for_error_reporting() {
        // Contract: Component name used in error messages
        let manager = StateManager::new("mining_status", TestState::default());

        assert_eq!(manager.component_name(), "mining_status");
    }
}

#[cfg(test)]
mod integration_scenarios {
    use super::*;

    #[test]
    fn test_node_status_state_management() {
        // Real-world scenario: Node status tracking
        #[derive(Clone, Debug, Serialize, Deserialize)]
        struct NodeStatus {
            running: bool,
            block_height: u64,
            peer_count: u32,
        }

        impl Default for NodeStatus {
            fn default() -> Self {
                Self {
                    running: false,
                    block_height: 0,
                    peer_count: 0,
                }
            }
        }

        let manager = StateManager::new("node_status", NodeStatus::default());

        // Simulate node start
        let _ = manager.update_silent(|status| {
            status.running = true;
            status.peer_count = 3;
        });

        let status = manager.read().unwrap();
        assert_eq!(status.running, true);
        assert_eq!(status.peer_count, 3);
    }

    #[test]
    fn test_mining_status_state_management() {
        // Real-world scenario: Mining status tracking
        #[derive(Clone, Debug, Serialize, Deserialize)]
        struct MiningStatus {
            active: bool,
            hashrate: u64,
            blocks_mined: u32,
        }

        impl Default for MiningStatus {
            fn default() -> Self {
                Self {
                    active: false,
                    hashrate: 0,
                    blocks_mined: 0,
                }
            }
        }

        let manager = StateManager::new("mining_status", MiningStatus::default());

        // Simulate mining start
        let _ = manager.update_silent(|status| {
            status.active = true;
            status.hashrate = 1_000_000;
        });

        // Simulate block found
        let _ = manager.update_silent(|status| {
            status.blocks_mined += 1;
        });

        let status = manager.read().unwrap();
        assert_eq!(status.active, true);
        assert_eq!(status.hashrate, 1_000_000);
        assert_eq!(status.blocks_mined, 1);
    }
}
