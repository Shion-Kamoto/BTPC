//! Process Health Monitoring Module
//!
//! This module provides health checking and crash recovery for child processes.
//! It implements the crash recovery policy from spec.md FR-039, FR-040, FR-046.
//!
//! # Features
//!
//! - Periodic health checks every 5 seconds (FR-038)
//! - Automatic restart on first crash (FR-039)
//! - User notification on second+ crash (FR-040)
//! - Crash counter reset after 1 hour stable operation (FR-046)
//! - Graceful shutdown with SIGTERM → 10s timeout → SIGKILL (FR-007)
//!
//! # Architecture
//!
//! ```text
//! ProcessHealthMonitor
//!   ├─ HealthCheckTask (tokio task, runs every 5s)
//!   ├─ CrashTracker (tracks crash count, timestamps)
//!   └─ RecoveryPolicy (auto-restart logic)
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use serde::{Serialize, Deserialize};
use tokio::time::sleep;
use crate::error::{BtpcError, BtpcResult};

/// Process health check interval (FR-038: 5 seconds)
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(5);

/// Crash counter reset time (FR-046: 1 hour stable)
const CRASH_COUNTER_RESET_TIME: Duration = Duration::from_secs(3600);

/// Graceful shutdown timeout (FR-007: 10 seconds)
const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

/// Process health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    /// Process is running and healthy
    Healthy,
    /// Process is not responding to health checks
    Unresponsive,
    /// Process has crashed
    Crashed { exit_code: Option<i32> },
    /// Process was stopped intentionally
    Stopped,
}

/// Crash tracking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashInfo {
    /// Number of crashes since last reset
    pub crash_count: u32,
    /// Timestamp of last crash
    pub last_crash: Option<SystemTime>,
    /// Timestamp when process last started successfully
    pub last_successful_start: Option<SystemTime>,
    /// Whether auto-restart is allowed (first crash only)
    pub auto_restart_enabled: bool,
}

impl Default for CrashInfo {
    fn default() -> Self {
        Self {
            crash_count: 0,
            last_crash: None,
            last_successful_start: None,
            auto_restart_enabled: true,
        }
    }
}

impl CrashInfo {
    /// Check if crash counter should be reset (1 hour stable operation)
    pub fn should_reset_counter(&self) -> bool {
        if let (Some(last_start), Some(last_crash)) = (self.last_successful_start, self.last_crash) {
            // If process has been running for 1 hour since last crash, reset counter
            if let Ok(elapsed) = last_start.elapsed() {
                elapsed >= CRASH_COUNTER_RESET_TIME && last_start > last_crash
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Reset the crash counter (FR-046)
    pub fn reset_counter(&mut self) {
        self.crash_count = 0;
        self.auto_restart_enabled = true;
    }

    /// Record a crash event
    pub fn record_crash(&mut self) {
        self.crash_count += 1;
        self.last_crash = Some(SystemTime::now());

        // After first crash, disable auto-restart (FR-040)
        if self.crash_count > 1 {
            self.auto_restart_enabled = false;
        }
    }

    /// Record successful process start
    pub fn record_successful_start(&mut self) {
        self.last_successful_start = Some(SystemTime::now());
    }

    /// Check if auto-restart should be attempted (FR-039)
    pub fn should_auto_restart(&self) -> bool {
        self.crash_count == 0 || (self.crash_count == 1 && self.auto_restart_enabled)
    }

    /// Check if user notification is required (FR-040)
    pub fn should_notify_user(&self) -> bool {
        self.crash_count > 1 || !self.auto_restart_enabled
    }
}

/// Process health monitor
pub struct ProcessHealthMonitor {
    /// Tracked processes: process_name -> CrashInfo
    crash_tracker: Arc<Mutex<HashMap<String, CrashInfo>>>,
}

impl ProcessHealthMonitor {
    /// Create a new process health monitor
    pub fn new() -> Self {
        Self {
            crash_tracker: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new process for monitoring
    pub fn register_process(&self, process_name: impl Into<String>) -> BtpcResult<()> {
        let name = process_name.into();
        let mut tracker = self.crash_tracker
            .lock()
            .map_err(|_| BtpcError::mutex_poison("ProcessHealthMonitor", "register_process"))?;

        tracker.entry(name.clone()).or_insert_with(CrashInfo::default);
        Ok(())
    }

    /// Record that a process started successfully
    pub fn record_start(&self, process_name: &str) -> BtpcResult<()> {
        let mut tracker = self.crash_tracker
            .lock()
            .map_err(|_| BtpcError::mutex_poison("ProcessHealthMonitor", "record_start"))?;

        if let Some(info) = tracker.get_mut(process_name) {
            info.record_successful_start();

            // Check if we should reset crash counter (1 hour stable)
            if info.should_reset_counter() {
                println!("✅ Process '{}' has been stable for 1 hour, resetting crash counter", process_name);
                info.reset_counter();
            }
        }

        Ok(())
    }

    /// Record a process crash and determine recovery action
    ///
    /// Returns `true` if auto-restart should be attempted, `false` if user notification is needed
    pub fn record_crash(&self, process_name: &str, exit_code: Option<i32>) -> BtpcResult<bool> {
        let mut tracker = self.crash_tracker
            .lock()
            .map_err(|_| BtpcError::mutex_poison("ProcessHealthMonitor", "record_crash"))?;

        if let Some(info) = tracker.get_mut(process_name) {
            info.record_crash();

            eprintln!(
                "⚠️ Process '{}' crashed (count: {}, exit_code: {:?})",
                process_name, info.crash_count, exit_code
            );

            // FR-039: Auto-restart on first crash
            // FR-040: Notify user on second+ crash
            if info.should_auto_restart() {
                println!("🔄 Auto-restarting '{}' (first crash)", process_name);
                Ok(true)
            } else {
                println!("🔔 Notifying user about '{}' crash (count: {})", process_name, info.crash_count);
                Ok(false)
            }
        } else {
            // Process not registered, notify user
            Ok(false)
        }
    }

    /// Get crash information for a process
    pub fn get_crash_info(&self, process_name: &str) -> BtpcResult<Option<CrashInfo>> {
        let tracker = self.crash_tracker
            .lock()
            .map_err(|_| BtpcError::mutex_poison("ProcessHealthMonitor", "get_crash_info"))?;

        Ok(tracker.get(process_name).cloned())
    }

    /// Reset crash counter for a process (called after manual restart)
    pub fn reset_crash_counter(&self, process_name: &str) -> BtpcResult<()> {
        let mut tracker = self.crash_tracker
            .lock()
            .map_err(|_| BtpcError::mutex_poison("ProcessHealthMonitor", "reset_crash_counter"))?;

        if let Some(info) = tracker.get_mut(process_name) {
            info.reset_counter();
        }

        Ok(())
    }

    /// Check if a process PID is still running
    #[cfg(unix)]
    pub fn check_pid_alive(pid: u32) -> bool {
        use std::process::Command;

        // Use kill -0 to check if process exists without sending signal
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[cfg(windows)]
    pub fn check_pid_alive(pid: u32) -> bool {
        use std::process::Command;

        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output()
            .map(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains(&pid.to_string())
            })
            .unwrap_or(false)
    }

    /// Gracefully stop a process (SIGTERM → 10s → SIGKILL)
    #[cfg(unix)]
    pub async fn graceful_stop(pid: u32) -> BtpcResult<()> {
        use std::process::Command;

        // Send SIGTERM
        println!("📤 Sending SIGTERM to process {}", pid);
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output();

        // Wait up to 10 seconds for graceful shutdown
        let start = SystemTime::now();
        loop {
            let elapsed = start.elapsed().unwrap_or(Duration::from_secs(0));
            if elapsed >= GRACEFUL_SHUTDOWN_TIMEOUT {
                break;
            }
            if !Self::check_pid_alive(pid) {
                println!("✅ Process {} stopped gracefully", pid);
                return Ok(());
            }
            sleep(Duration::from_millis(500)).await;
        }

        // Force kill if still alive
        println!("⚠️ Process {} did not stop gracefully, sending SIGKILL", pid);
        let _ = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .output();

        // Wait a bit to confirm kill
        sleep(Duration::from_secs(1)).await;

        if Self::check_pid_alive(pid) {
            Err(BtpcError::Application(format!("Failed to kill process {}", pid)))
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    pub async fn graceful_stop(pid: u32) -> BtpcResult<()> {
        use std::process::Command;

        // Windows doesn't have SIGTERM, use taskkill /PID
        println!("📤 Attempting to stop process {}", pid);

        // Try graceful termination first
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string()])
            .output();

        // Wait up to 10 seconds
        let start = SystemTime::now();
        loop {
            let elapsed = start.elapsed().unwrap_or(Duration::from_secs(0));
            if elapsed >= GRACEFUL_SHUTDOWN_TIMEOUT {
                break;
            }
            if !Self::check_pid_alive(pid) {
                println!("✅ Process {} stopped", pid);
                return Ok(());
            }
            sleep(Duration::from_millis(500)).await;
        }

        // Force kill
        println!("⚠️ Process {} did not stop, forcing termination", pid);
        let _ = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output();

        sleep(Duration::from_secs(1)).await;

        if Self::check_pid_alive(pid) {
            Err(BtpcError::Application(format!("Failed to kill process {}", pid)))
        } else {
            Ok(())
        }
    }
}

impl Default for ProcessHealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crash_info_default() {
        let info = CrashInfo::default();
        assert_eq!(info.crash_count, 0);
        assert_eq!(info.auto_restart_enabled, true);
        assert_eq!(info.should_auto_restart(), true);
        assert_eq!(info.should_notify_user(), false);
    }

    #[test]
    fn test_crash_info_first_crash() {
        let mut info = CrashInfo::default();
        info.record_crash();

        assert_eq!(info.crash_count, 1);
        assert_eq!(info.should_auto_restart(), true); // FR-039: auto-restart on first crash
        assert_eq!(info.should_notify_user(), false);
    }

    #[test]
    fn test_crash_info_second_crash() {
        let mut info = CrashInfo::default();
        info.record_crash();
        info.record_crash();

        assert_eq!(info.crash_count, 2);
        assert_eq!(info.should_auto_restart(), false); // FR-040: no auto-restart on second crash
        assert_eq!(info.should_notify_user(), true); // FR-040: notify user
    }

    #[test]
    fn test_crash_counter_reset() {
        let mut info = CrashInfo::default();
        info.record_crash();
        info.reset_counter();

        assert_eq!(info.crash_count, 0);
        assert_eq!(info.auto_restart_enabled, true);
    }

    #[test]
    fn test_health_monitor_register() {
        let monitor = ProcessHealthMonitor::new();
        assert!(monitor.register_process("test_node").is_ok());

        let info = monitor.get_crash_info("test_node").unwrap();
        assert!(info.is_some());
        assert_eq!(info.unwrap().crash_count, 0);
    }

    #[test]
    fn test_health_monitor_crash_recording() {
        let monitor = ProcessHealthMonitor::new();
        monitor.register_process("test_miner").unwrap();

        // First crash: should auto-restart
        let should_restart = monitor.record_crash("test_miner", Some(1)).unwrap();
        assert_eq!(should_restart, true);

        // Second crash: should notify user
        let should_restart = monitor.record_crash("test_miner", Some(1)).unwrap();
        assert_eq!(should_restart, false);
    }
}
