//! State Management Module - Article XI Compliance
//!
//! This module provides the `StateManager<T>` wrapper pattern that ensures
//! automatic Tauri event emission whenever state is modified. This implements
//! the "backend is the single source of truth" principle from Article XI.
//!
//! # Architecture
//!
//! - `StateManager<T>`: Wraps Arc<Mutex<T>> and auto-emits events on update
//! - Events are emitted AFTER successful state mutation
//! - Mutex poison errors are handled gracefully with BtpcError
//! - Type-safe event names derived from type T
//!
//! # Usage
//!
//! ```rust
//! use state_management::StateManager;
//!
//! // Create managed state
//! let node_status = StateManager::new("node_status", NodeStatus::default());
//!
//! // Update state - automatically emits "node_status_changed" event
//! node_status.update(|status| {
//!     status.running = true;
//!     status.start_time = Some(Utc::now());
//! }, &app_handle)?;
//! ```

use std::sync::{Arc, Mutex};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use crate::error::{BtpcError, BtpcResult};

/// StateManager wraps Arc<Mutex<T>> and provides automatic event emission
///
/// This type ensures that every state mutation triggers a Tauri event,
/// keeping the frontend automatically synchronized with backend state.
#[derive(Clone)]
pub struct StateManager<T: Clone + Serialize> {
    /// The underlying shared state
    inner: Arc<Mutex<T>>,
    /// Event name to emit on state changes (e.g., "node_status_changed")
    event_name: String,
    /// Component name for error reporting
    component_name: String,
}

impl<T: Clone + Serialize + Send + 'static> StateManager<T> {
    /// Create a new StateManager with the given event name prefix
    ///
    /// # Arguments
    /// * `event_prefix` - Base name for events (e.g., "node_status" becomes "node_status_changed")
    /// * `initial_state` - The initial value for the managed state
    ///
    /// # Example
    /// ```rust
    /// let node_status = StateManager::new("node_status", NodeStatus::default());
    /// ```
    pub fn new(event_prefix: impl Into<String>, initial_state: T) -> Self {
        let prefix = event_prefix.into();
        Self {
            inner: Arc::new(Mutex::new(initial_state)),
            event_name: format!("{}_changed", prefix),
            component_name: prefix,
        }
    }

    /// Get a read-only copy of the current state
    ///
    /// Returns `BtpcError::MutexPoisoned` if another thread panicked while holding the lock.
    ///
    /// # Example
    /// ```rust
    /// let status = node_status.read()?;
    /// println!("Node running: {}", status.running);
    /// ```
    pub fn read(&self) -> BtpcResult<T> {
        self.inner
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| BtpcError::mutex_poison(&self.component_name, "read"))
    }

    /// Update the state using a closure and emit change event
    ///
    /// The closure receives a mutable reference to the state and can modify it.
    /// After successful modification, a Tauri event is automatically emitted.
    ///
    /// # Arguments
    /// * `update_fn` - Closure that mutates the state
    /// * `app` - Tauri AppHandle for event emission
    ///
    /// # Returns
    /// Returns the updated state copy on success, or `BtpcError::MutexPoisoned` on lock failure.
    ///
    /// # Example
    /// ```rust
    /// node_status.update(|status| {
    ///     status.running = true;
    ///     status.block_height = 12345;
    /// }, &app_handle)?;
    /// // "node_status_changed" event automatically emitted to frontend
    /// ```
    pub fn update<F>(&self, update_fn: F, app: &AppHandle) -> BtpcResult<T>
    where
        F: FnOnce(&mut T),
    {
        // Acquire lock with poison handling
        let mut guard = self.inner
            .lock()
            .map_err(|_| BtpcError::mutex_poison(&self.component_name, "update"))?;

        // Apply the update
        update_fn(&mut *guard);

        // Clone updated state for return and event emission
        let updated_state = guard.clone();

        // Emit event (Article XI: automatic frontend synchronization)
        if let Err(_e) = app.emit(&self.event_name, &updated_state) {
            eprintln!("⚠️ Failed to emit '{}' event: {}", self.event_name, _e);
            // Don't fail the operation just because event emission failed
            // The state is still updated correctly
        }

        Ok(updated_state)
    }

    /// Update state without emitting an event (use sparingly)
    ///
    /// This method should only be used when you need to batch multiple updates
    /// and emit a single event manually afterwards.
    ///
    /// **Warning**: Bypasses automatic event emission - use with caution!
    pub fn update_silent<F>(&self, update_fn: F) -> BtpcResult<T>
    where
        F: FnOnce(&mut T),
    {
        let mut guard = self.inner
            .lock()
            .map_err(|_| BtpcError::mutex_poison(&self.component_name, "update_silent"))?;

        update_fn(&mut *guard);
        Ok(guard.clone())
    }

    /// Manually emit the state change event with current state
    ///
    /// Useful after using `update_silent()` for batch updates.
    pub fn emit_current(&self, app: &AppHandle) -> BtpcResult<()> {
        let current = self.read()?;
        app.emit(&self.event_name, &current)
            .map_err(|e| BtpcError::Application(format!("Event emission failed: {}", e)))
    }

    /// Replace the entire state with a new value and emit event
    ///
    /// # Example
    /// ```rust
    /// let new_status = NodeStatus { running: true, ..Default::default() };
    /// node_status.set(new_status, &app_handle)?;
    /// ```
    pub fn set(&self, new_state: T, app: &AppHandle) -> BtpcResult<T> {
        self.update(|state| *state = new_state.clone(), app)
    }

    /// Get the event name that this StateManager emits
    pub fn event_name(&self) -> &str {
        &self.event_name
    }

    /// Get the component name
    pub fn component_name(&self) -> &str {
        &self.component_name
    }

    /// Create a raw Arc<Mutex<T>> for cases where direct mutex access is needed
    ///
    /// **Use with caution**: This bypasses automatic event emission.
    /// Prefer using `update()` whenever possible.
    pub fn inner_arc(&self) -> Arc<Mutex<T>> {
        Arc::clone(&self.inner)
    }
}

/// Helper macro for creating multiple StateManagers
///
/// # Example
/// ```rust
/// state_managers! {
///     app_handle,
///     node_status: NodeStatus = NodeStatus::default(),
///     mining_status: MiningStatus = MiningStatus::default(),
///     wallet_balance: WalletBalance = WalletBalance::default()
/// }
/// ```
#[macro_export]
macro_rules! state_managers {
    ($app:expr, $( $name:ident : $type:ty = $init:expr ),* $(,)?) => {
        $(
            let $name = StateManager::new(stringify!($name), $init);
        )*
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    struct TestState {
        counter: u32,
        message: String,
    }

    impl Default for TestState {
        fn default() -> Self {
            Self {
                counter: 0,
                message: "initial".to_string(),
            }
        }
    }

    #[test]
    fn test_state_manager_read() {
        let manager = StateManager::new("test", TestState::default());
        let state = manager.read().expect("Failed to read state");
        assert_eq!(state.counter, 0);
        assert_eq!(state.message, "initial");
    }

    #[test]
    fn test_state_manager_update_silent() {
        let manager = StateManager::new("test", TestState::default());

        let updated = manager.update_silent(|state| {
            state.counter = 42;
            state.message = "updated".to_string();
        }).expect("Failed to update state");

        assert_eq!(updated.counter, 42);
        assert_eq!(updated.message, "updated");

        // Verify state persisted
        let current = manager.read().expect("Failed to read state");
        assert_eq!(current.counter, 42);
    }

    #[test]
    fn test_event_name_generation() {
        let manager = StateManager::new("node_status", TestState::default());
        assert_eq!(manager.event_name(), "node_status_changed");
        assert_eq!(manager.component_name(), "node_status");
    }

    #[test]
    fn test_state_manager_set_silent() {
        let manager = StateManager::new("test", TestState::default());

        let new_state = TestState {
            counter: 100,
            message: "replaced".to_string(),
        };

        manager.update_silent(|state| *state = new_state.clone())
            .expect("Failed to set state");

        let current = manager.read().expect("Failed to read state");
        assert_eq!(current, new_state);
    }
}
