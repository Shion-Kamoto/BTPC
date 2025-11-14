//! Lock Manager - Safe File Locking Module
//!
//! Provides safe, cross-platform file locking using the fs2 crate.
//! Replaces unsafe `libc::flock()` calls with proper error handling.
//!
//! # Features
//!
//! - Cross-platform file locking (Unix and Windows)
//! - RAII-based lock guards (automatic unlock on drop)
//! - Try-lock support for non-blocking operations
//! - Single instance protection for desktop app (FR-008)
//! - Database lock conflict detection (NFR-001)

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use fs2::FileExt;
use crate::error::{BtpcError, BtpcResult};

/// File lock guard - automatically unlocks on drop (RAII pattern)
pub struct FileLockGuard {
    file: File,
    path: PathBuf,
    exclusive: bool,
}

impl FileLockGuard {
    /// Get the path of the locked file
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if lock is exclusive
    pub fn is_exclusive(&self) -> bool {
        self.exclusive
    }
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        // Unlock happens automatically when file is dropped
        let _ = self.file.unlock();
        println!("🔓 Released lock on: {}", self.path.display());
    }
}

/// Lock manager for safe file locking operations
pub struct LockManager {
    /// Directory where lock files are stored
    lock_dir: PathBuf,
}

impl LockManager {
    /// Create a new lock manager
    ///
    /// # Arguments
    /// * `lock_dir` - Directory where lock files will be created
    ///
    /// # Example
    /// ```rust
    /// let lock_mgr = LockManager::new("~/.btpc/locks")?;
    /// ```
    pub fn new(lock_dir: impl Into<PathBuf>) -> BtpcResult<Self> {
        let dir = lock_dir.into();

        // Create lock directory if it doesn't exist
        if !dir.exists() {
            std::fs::create_dir_all(&dir).map_err(|e| {
                BtpcError::Application(format!("Failed to create lock directory: {}", e))
            })?;
        }

        Ok(Self { lock_dir: dir })
    }

    /// Acquire an exclusive lock (blocking)
    ///
    /// This will wait indefinitely until the lock can be acquired.
    ///
    /// # Arguments
    /// * `name` - Lock name (e.g., "app", "database")
    ///
    /// # Returns
    /// `FileLockGuard` which will automatically release the lock when dropped
    ///
    /// # Example
    /// ```rust
    /// let _guard = lock_mgr.lock_exclusive("database")?;
    /// // Lock is held here
    /// // Lock automatically released when _guard goes out of scope
    /// ```
    pub fn lock_exclusive(&self, name: impl AsRef<str>) -> BtpcResult<FileLockGuard> {
        let lock_path = self.lock_path(name.as_ref());

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&lock_path)
            .map_err(|e| {
                BtpcError::Application(format!("Failed to open lock file '{}': {}", lock_path.display(), e))
            })?;

        file.lock_exclusive().map_err(|e| {
            BtpcError::Application(format!("Failed to acquire exclusive lock on '{}': {}", lock_path.display(), e))
        })?;

        println!("🔒 Acquired exclusive lock on: {}", lock_path.display());

        Ok(FileLockGuard {
            file,
            path: lock_path,
            exclusive: true,
        })
    }

    /// Try to acquire an exclusive lock (non-blocking)
    ///
    /// Returns immediately if lock cannot be acquired.
    ///
    /// # Arguments
    /// * `name` - Lock name
    ///
    /// # Returns
    /// * `Ok(Some(guard))` - Lock acquired successfully
    /// * `Ok(None)` - Lock is held by another process
    /// * `Err(...)` - Error occurred
    ///
    /// # Example
    /// ```rust
    /// match lock_mgr.try_lock_exclusive("app")? {
    ///     Some(guard) => {
    ///         // We got the lock
    ///         println!("Application started");
    ///     }
    ///     None => {
    ///         // Another instance is running
    ///         return Err("Another instance is already running");
    ///     }
    /// }
    /// ```
    pub fn try_lock_exclusive(&self, name: impl AsRef<str>) -> BtpcResult<Option<FileLockGuard>> {
        let lock_path = self.lock_path(name.as_ref());

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&lock_path)
            .map_err(|e| {
                BtpcError::Application(format!("Failed to open lock file '{}': {}", lock_path.display(), e))
            })?;

        match file.try_lock_exclusive() {
            Ok(_) => {
                println!("🔒 Acquired exclusive lock on: {}", lock_path.display());
                Ok(Some(FileLockGuard {
                    file,
                    path: lock_path,
                    exclusive: true,
                }))
            }
            Err(_) => {
                // Lock is held by another process (fs2::lock_contended_error)
                // fs2 doesn't expose error kind, so we assume contention on any error
                Ok(None)
            }
        }
    }

    /// Acquire a shared lock (blocking)
    ///
    /// Multiple processes can hold shared locks simultaneously.
    /// Shared locks are compatible with other shared locks but not exclusive locks.
    ///
    /// # Arguments
    /// * `name` - Lock name
    pub fn lock_shared(&self, name: impl AsRef<str>) -> BtpcResult<FileLockGuard> {
        let lock_path = self.lock_path(name.as_ref());

        let file = OpenOptions::new()
            .read(true)
            .write(true)  // Need write permission to create the file
            .create(true)
            .open(&lock_path)
            .map_err(|e| {
                BtpcError::Application(format!("Failed to open lock file '{}': {}", lock_path.display(), e))
            })?;

        file.lock_shared().map_err(|e| {
            BtpcError::Application(format!("Failed to acquire shared lock on '{}': {}", lock_path.display(), e))
        })?;

        println!("🔒 Acquired shared lock on: {}", lock_path.display());

        Ok(FileLockGuard {
            file,
            path: lock_path,
            exclusive: false,
        })
    }

    /// Try to acquire a shared lock (non-blocking)
    pub fn try_lock_shared(&self, name: impl AsRef<str>) -> BtpcResult<Option<FileLockGuard>> {
        let lock_path = self.lock_path(name.as_ref());

        let file = OpenOptions::new()
            .read(true)
            .write(true)  // Need write permission to create the file
            .create(true)
            .open(&lock_path)
            .map_err(|e| {
                BtpcError::Application(format!("Failed to open lock file '{}': {}", lock_path.display(), e))
            })?;

        match file.try_lock_shared() {
            Ok(_) => {
                println!("🔒 Acquired shared lock on: {}", lock_path.display());
                Ok(Some(FileLockGuard {
                    file,
                    path: lock_path,
                    exclusive: false,
                }))
            }
            Err(_) => {
                // Lock is held exclusively by another process
                Ok(None)
            }
        }
    }

    /// Check if a lock file exists and is locked
    pub fn is_locked(&self, name: impl AsRef<str>) -> bool {
        let lock_path = self.lock_path(name.as_ref());

        if !lock_path.exists() {
            return false;
        }

        // Try to acquire lock non-blocking
        if let Ok(file) = OpenOptions::new().read(true).open(&lock_path) {
            file.try_lock_exclusive().is_err()
        } else {
            false
        }
    }

    /// Get the full path for a lock file
    fn lock_path(&self, name: &str) -> PathBuf {
        self.lock_dir.join(format!("{}.lock", name))
    }

    /// Remove stale lock files (use with caution!)
    ///
    /// Only call this if you're certain no other process is using the lock.
    pub fn remove_lock(&self, name: impl AsRef<str>) -> BtpcResult<()> {
        let lock_path = self.lock_path(name.as_ref());

        if lock_path.exists() {
            std::fs::remove_file(&lock_path).map_err(|e| {
                BtpcError::Application(format!("Failed to remove lock file '{}': {}", lock_path.display(), e))
            })?;
            println!("🗑️ Removed lock file: {}", lock_path.display());
        }

        Ok(())
    }
}

/// Ensure single instance of desktop application (FR-008)
///
/// # Example
/// ```rust
/// let app_lock = ensure_single_instance("~/.btpc/locks")?;
/// // Keep app_lock alive for the entire application lifetime
/// ```
pub fn ensure_single_instance(lock_dir: impl Into<PathBuf>) -> BtpcResult<FileLockGuard> {
    let lock_mgr = LockManager::new(lock_dir)?;

    match lock_mgr.try_lock_exclusive("btpc_desktop_app") {
        Ok(Some(guard)) => Ok(guard),
        Ok(None) => Err(BtpcError::Application(
            "Another instance of BTPC Desktop App is already running".to_string()
        )),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_lock_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let lock_mgr = LockManager::new(temp_dir.path()).unwrap();
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_exclusive_lock() {
        let temp_dir = TempDir::new().unwrap();
        let lock_mgr = LockManager::new(temp_dir.path()).unwrap();

        let guard = lock_mgr.lock_exclusive("test").unwrap();
        assert!(guard.is_exclusive());

        // Try to acquire same lock should fail
        let result = lock_mgr.try_lock_exclusive("test").unwrap();
        assert!(result.is_none());

        // Drop guard to release lock
        drop(guard);

        // Now we should be able to acquire it
        let result = lock_mgr.try_lock_exclusive("test").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_shared_lock() {
        let temp_dir = TempDir::new().unwrap();
        let lock_mgr = LockManager::new(temp_dir.path()).unwrap();

        let guard1 = lock_mgr.lock_shared("test").unwrap();
        assert!(!guard1.is_exclusive());

        // Multiple shared locks should succeed
        let guard2 = lock_mgr.try_lock_shared("test").unwrap();
        assert!(guard2.is_some());

        // Exclusive lock should fail while shared locks are held
        let result = lock_mgr.try_lock_exclusive("test").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_is_locked() {
        let temp_dir = TempDir::new().unwrap();
        let lock_mgr = LockManager::new(temp_dir.path()).unwrap();

        assert!(!lock_mgr.is_locked("test"));

        let _guard = lock_mgr.lock_exclusive("test").unwrap();
        assert!(lock_mgr.is_locked("test"));
    }

    #[test]
    fn test_remove_lock() {
        let temp_dir = TempDir::new().unwrap();
        let lock_mgr = LockManager::new(temp_dir.path()).unwrap();

        {
            let _guard = lock_mgr.lock_exclusive("test").unwrap();
        } // Guard dropped, lock released

        lock_mgr.remove_lock("test").unwrap();
        assert!(!lock_mgr.lock_path("test").exists());
    }
}
