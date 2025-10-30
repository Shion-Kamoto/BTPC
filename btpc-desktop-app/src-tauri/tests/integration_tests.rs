//! Integration tests for BTPC Desktop Application
//!
//! These tests verify the core functionality of the desktop application
//! including security, BTPC integration, and UTXO management.
//!
//! **NOTE**: Temporarily disabled - btpc-desktop-app is a binary crate, not a library.
//! These tests need refactoring to work with the binary structure.
//! See: https://github.com/btpc/btpc/issues/TBD

#![cfg(feature = "integration_tests_disabled")]

use std::path::PathBuf;
use tempfile::TempDir;

/// Test utilities for setting up test environments
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub btpc_home: PathBuf,
    pub config: LauncherConfig,
}

impl TestEnvironment {
    /// Create a new test environment with temporary directories
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let btpc_home = temp_dir.path().to_path_buf();

        // Create necessary subdirectories
        std::fs::create_dir_all(btpc_home.join("bin"))?;
        std::fs::create_dir_all(btpc_home.join("data"))?;
        std::fs::create_dir_all(btpc_home.join("logs"))?;
        std::fs::create_dir_all(btpc_home.join("config"))?;

        let config = LauncherConfig {
            btpc_home: btpc_home.clone(),
            network: NetworkType::Testnet,
            data_dir: btpc_home.join("data"),
            log_dir: btpc_home.join("logs"),
            config_dir: btpc_home.join("config"),
            node: btpc_desktop_app::NodeConfig {
                port: 18333,
                rpc_port: 18332,
                sync_interval_secs: 5,
                max_connections: 10,
                enable_mining: false,
            },
            wallet: btpc_desktop_app::WalletConfig {
                auto_create: false,
                backup_interval_hours: 24,
                encryption_enabled: true,
            },
            mining: btpc_desktop_app::MiningConfig {
                enabled: false,
                threads: 1,
                target_address: None,
            },
            rpc: btpc_desktop_app::RpcConfig {
                enabled: true,
                bind_address: "127.0.0.1".to_string(),
                port: 8080,
                auth_token: None,
            },
        };

        Ok(Self {
            temp_dir,
            btpc_home,
            config,
        })
    }

    /// Create a mock binary in the bin directory
    pub fn create_mock_binary(&self, name: &str) -> anyhow::Result<()> {
        let binary_path = self.btpc_home.join("bin").join(name);

        #[cfg(unix)]
        {
            std::fs::write(&binary_path, "#!/bin/bash\necho 'Mock binary executed'\n")?;
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary_path, std::fs::Permissions::from_mode(0o755))?;
        }

        #[cfg(windows)]
        {
            std::fs::write(&binary_path.with_extension("bat"), "@echo Mock binary executed")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod btpc_integration_tests {
    use super::*;
    use btpc_desktop_app::btpc_integration::BtpcIntegration;

    #[test]
    fn test_btpc_integration_new() {
        let env = TestEnvironment::new().expect("Failed to create test environment");
        let integration = BtpcIntegration::new(env.btpc_home.clone());

        assert_eq!(integration.btpc_home, env.btpc_home);
        assert_eq!(integration.bin_dir, env.btpc_home.join("bin"));
    }

    #[test]
    fn test_binary_exists_when_present() {
        let env = TestEnvironment::new().expect("Failed to create test environment");
        env.create_mock_binary("test_binary").expect("Failed to create mock binary");

        let integration = BtpcIntegration::new(env.btpc_home);

        #[cfg(unix)]
        assert!(integration.binary_exists("test_binary"));

        #[cfg(windows)]
        assert!(integration.binary_exists("test_binary.bat"));
    }

    #[test]
    fn test_binary_exists_when_missing() {
        let env = TestEnvironment::new().expect("Failed to create test environment");
        let integration = BtpcIntegration::new(env.btpc_home);

        assert!(!integration.binary_exists("nonexistent_binary"));
    }

    #[test]
    fn test_get_binary_status() {
        let env = TestEnvironment::new().expect("Failed to create test environment");
        let integration = BtpcIntegration::new(env.btpc_home);

        let status = integration.get_binary_status();

        // All binaries should be missing in a fresh test environment
        assert!(!status.quantum_chain_exists);
        assert!(!status.wallet_exists);
        assert!(!status.miner_exists);
        assert!(!status.btpc_home_exists);
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;
    use btpc_desktop_app::security::{SecurityManager, UserCredentials};
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_security_manager_creation() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let security_manager = SecurityManager::new(temp_file.path().to_path_buf());

        assert!(security_manager.is_ok());
    }

    #[tokio::test]
    async fn test_create_user() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let mut security_manager = SecurityManager::new(temp_file.path().to_path_buf())
            .expect("Failed to create security manager");

        let result = security_manager.create_user(
            "testuser".to_string(),
            "testpassword".to_string(),
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string()
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_login_success() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let mut security_manager = SecurityManager::new(temp_file.path().to_path_buf())
            .expect("Failed to create security manager");

        // Create user first
        security_manager.create_user(
            "testuser".to_string(),
            "testpassword".to_string(),
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string()
        ).await.expect("Failed to create user");

        // Test login
        let result = security_manager.login("testuser".to_string(), "testpassword".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_login_failure() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let mut security_manager = SecurityManager::new(temp_file.path().to_path_buf())
            .expect("Failed to create security manager");

        // Create user first
        security_manager.create_user(
            "testuser".to_string(),
            "testpassword".to_string(),
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string()
        ).await.expect("Failed to create user");

        // Test login with wrong password
        let result = security_manager.login("testuser".to_string(), "wrongpassword".to_string()).await;
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod utxo_manager_tests {
    use super::*;
    use btpc_desktop_app::utxo_manager::{UTXOManager, UTXO};
    use chrono::Utc;

    #[test]
    fn test_utxo_manager_new() {
        let env = TestEnvironment::new().expect("Failed to create test environment");
        let utxo_manager = UTXOManager::new(env.btpc_home);

        assert!(utxo_manager.is_ok());
    }

    #[test]
    fn test_utxo_creation() {
        let utxo = UTXO {
            txid: "test_txid".to_string(),
            vout: 0,
            value_credits: 100000000, // 1 BTP
            value_btp: 1.0,
            address: "test_address".to_string(),
            block_height: 100,
            is_coinbase: false,
            created_at: Utc::now(),
            confirmations: 6,
            spendable: true,
        };

        assert_eq!(utxo.txid, "test_txid");
        assert_eq!(utxo.value_credits, 100000000);
        assert_eq!(utxo.value_btp, 1.0);
    }

    #[tokio::test]
    async fn test_utxo_manager_add_and_get() {
        let env = TestEnvironment::new().expect("Failed to create test environment");
        let mut utxo_manager = UTXOManager::new(env.btpc_home)
            .expect("Failed to create UTXO manager");

        let utxo = UTXO {
            txid: "test_txid".to_string(),
            vout: 0,
            value_credits: 100000000,
            value_btp: 1.0,
            address: "test_address".to_string(),
            block_height: 100,
            is_coinbase: false,
            created_at: Utc::now(),
            confirmations: 6,
            spendable: true,
        };

        let result = utxo_manager.add_utxo(utxo.clone()).await;
        assert!(result.is_ok());

        let utxos = utxo_manager.get_utxos_by_address("test_address").await;
        assert!(utxos.is_ok());
        let utxos = utxos.unwrap();
        assert_eq!(utxos.len(), 1);
        assert_eq!(utxos[0].txid, "test_txid");
    }
}