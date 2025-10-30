//! Comprehensive test suite for wallet manager functionality
//!
//! This module contains unit tests, integration tests, and performance benchmarks
//! for the wallet manager system.

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::security::SecurityManager;
    use crate::btpc_integration::BtpcIntegration;
    use std::fs;
    use tempfile::TempDir;
    use chrono::Utc;

    fn setup_test_environment() -> (TempDir, WalletManager, BtpcIntegration) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = WalletManagerConfig {
            wallets_dir: temp_dir.path().join("wallets"),
            backups_dir: temp_dir.path().join("backups"),
            max_wallets: 50,
            default_category: "Test".to_string(),
            backup_interval_hours: 24,
            encrypt_metadata: true,
        };

        let security = SecurityManager::new(temp_dir.path().join("security"))
            .expect("Failed to create security manager");

        let wallet_manager = WalletManager::new(config, security)
            .expect("Failed to create wallet manager");

        let btpc_integration = BtpcIntegration::new(temp_dir.path().join("btpc"));

        (temp_dir, wallet_manager, btpc_integration)
    }

    #[test]
    fn test_wallet_manager_creation() {
        let (_temp_dir, wallet_manager, _btpc) = setup_test_environment();
        assert_eq!(wallet_manager.list_wallets().len(), 0);
    }

    #[test]
    fn test_create_wallet_with_nickname() {
        let (_temp_dir, mut wallet_manager) = setup_test_environment();

        let request = CreateWalletRequest {
            nickname: "Test Wallet".to_string(),
            description: "Test wallet description".to_string(),
            category: Some("Personal".to_string()),
            color: Some("#3b82f6".to_string()),
            is_favorite: false,
            is_default: true,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000),
            import_data: None,
        };

        let result = wallet_manager.create_wallet(request);
        assert!(result.is_ok());

        let wallet = result.unwrap();
        assert_eq!(wallet.nickname, "Test Wallet");
        assert_eq!(wallet.metadata.description, Some("Test wallet description".to_string()));
        assert!(wallet.is_default);
        assert_eq!(wallet_manager.list_wallets().len(), 1);
    }

    #[test]
    fn test_create_multiple_wallets() {
        let (_temp_dir, mut wallet_manager) = setup_test_environment();

        // Create first wallet
        let request1 = CreateWalletRequest {
            nickname: "Wallet 1".to_string(),
            description: "First wallet".to_string(),
            category: Some("Personal".to_string()),
            color: Some("#3b82f6".to_string()),
            is_favorite: false,
            is_default: true,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000),
            import_data: None,
        };

        // Create second wallet
        let request2 = CreateWalletRequest {
            nickname: "Wallet 2".to_string(),
            description: "Second wallet".to_string(),
            category: Some("Mining".to_string()),
            color: Some("#10b981".to_string()),
            is_favorite: true,
            is_default: false,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(15000),
            import_data: None,
        };

        let wallet1 = wallet_manager.create_wallet(request1).unwrap();
        let wallet2 = wallet_manager.create_wallet(request2).unwrap();

        assert_eq!(wallet_manager.list_wallets().len(), 2);
        assert_ne!(wallet1.id, wallet2.id);
        assert_ne!(wallet1.address, wallet2.address);

        // First wallet should remain default
        assert!(wallet1.is_default);
        assert!(!wallet2.is_default);
    }

    #[test]
    fn test_duplicate_nickname_prevention() {
        let (_temp_dir, mut wallet_manager) = setup_test_environment();

        let request1 = CreateWalletRequest {
            nickname: "Duplicate Name".to_string(),
            description: "First wallet".to_string(),
            category: Some("Personal".to_string()),
            color: Some("#3b82f6".to_string()),
            is_favorite: false,
            is_default: true,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000),
            import_data: None,
        };

        let request2 = CreateWalletRequest {
            nickname: "Duplicate Name".to_string(),
            description: "Second wallet with same name".to_string(),
            category: Some("Mining".to_string()),
            color: Some("#10b981".to_string()),
            is_favorite: false,
            is_default: false,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000),
            import_data: None,
        };

        // First wallet should succeed
        assert!(wallet_manager.create_wallet(request1).is_ok());

        // Second wallet with same nickname should fail
        assert!(wallet_manager.create_wallet(request2).is_err());
        assert_eq!(wallet_manager.list_wallets().len(), 1);
    }

    #[test]
    fn test_wallet_retrieval() {
        let (_temp_dir, mut wallet_manager) = setup_test_environment();

        let request = CreateWalletRequest {
            nickname: "Retrieval Test".to_string(),
            description: "Test wallet for retrieval".to_string(),
            category: Some("Personal".to_string()),
            color: Some("#3b82f6".to_string()),
            is_favorite: false,
            is_default: true,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000),
            import_data: None,
        };

        let created_wallet = wallet_manager.create_wallet(request).unwrap();

        // Test get_wallet by ID
        let retrieved_wallet = wallet_manager.get_wallet(&created_wallet.id);
        assert!(retrieved_wallet.is_some());
        assert_eq!(retrieved_wallet.unwrap().nickname, "Retrieval Test");

        // Test get_wallet_by_nickname
        let wallet_by_nickname = wallet_manager.get_wallet_by_nickname("Retrieval Test");
        assert!(wallet_by_nickname.is_some());
        assert_eq!(wallet_by_nickname.unwrap().id, created_wallet.id);

        // Test get_default_wallet
        let default_wallet = wallet_manager.get_default_wallet();
        assert!(default_wallet.is_some());
        assert_eq!(default_wallet.unwrap().id, created_wallet.id);
    }

    #[test]
    fn test_wallet_update() {
        let (_temp_dir, mut wallet_manager) = setup_test_environment();

        let create_request = CreateWalletRequest {
            nickname: "Update Test".to_string(),
            description: "Original description".to_string(),
            category: Some("Personal".to_string()),
            color: Some("#3b82f6".to_string()),
            is_favorite: false,
            is_default: true,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000),
            import_data: None,
        };

        let wallet = wallet_manager.create_wallet(create_request).unwrap();

        let update_request = UpdateWalletRequest {
            wallet_id: wallet.id.clone(),
            nickname: Some("Updated Name".to_string()),
            description: Some("Updated description".to_string()),
            category: Some("Mining".to_string()),
            color: Some("#10b981".to_string()),
            is_favorite: Some(true),
            is_default: None,
            auto_backup: Some(false),
            notifications_enabled: Some(false),
            default_fee_credits: Some(20000),
        };

        let updated_wallet = wallet_manager.update_wallet(update_request).unwrap();

        assert_eq!(updated_wallet.nickname, "Updated Name");
        assert_eq!(updated_wallet.metadata.description, Some("Updated description".to_string()));
        assert_eq!(updated_wallet.metadata.category, "Mining");
        assert_eq!(updated_wallet.metadata.color, "#10b981");
        assert!(updated_wallet.metadata.is_favorite);
        assert!(!updated_wallet.metadata.auto_backup);
        assert!(!updated_wallet.metadata.notifications_enabled);
        assert_eq!(updated_wallet.metadata.default_fee_credits, Some(20000));
    }

    #[test]
    fn test_wallet_deletion() {
        let (_temp_dir, mut wallet_manager) = setup_test_environment();

        let request = CreateWalletRequest {
            nickname: "Delete Test".to_string(),
            description: "Test wallet for deletion".to_string(),
            category: Some("Personal".to_string()),
            color: Some("#3b82f6".to_string()),
            is_favorite: false,
            is_default: true,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000),
            import_data: None,
        };

        let wallet = wallet_manager.create_wallet(request).unwrap();
        assert_eq!(wallet_manager.list_wallets().len(), 1);

        let result = wallet_manager.delete_wallet(&wallet.id);
        assert!(result.is_ok());
        assert_eq!(wallet_manager.list_wallets().len(), 0);

        // Verify wallet file is deleted
        assert!(!wallet.file_path.exists());
    }

    #[test]
    fn test_balance_update() {
        let (_temp_dir, mut wallet_manager) = setup_test_environment();

        let request = CreateWalletRequest {
            nickname: "Balance Test".to_string(),
            description: "Test wallet for balance updates".to_string(),
            category: Some("Personal".to_string()),
            color: Some("#3b82f6".to_string()),
            is_favorite: false,
            is_default: true,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000),
            import_data: None,
        };

        let wallet = wallet_manager.create_wallet(request).unwrap();
        assert_eq!(wallet.cached_balance_credits, 0);

        let result = wallet_manager.update_wallet_balance(&wallet.id, 1000000000); // 10 BTP
        assert!(result.is_ok());

        let updated_wallet = wallet_manager.get_wallet(&wallet.id).unwrap();
        assert_eq!(updated_wallet.cached_balance_credits, 1000000000);
        assert_eq!(updated_wallet.cached_balance_btp, 10.0);
    }

    #[test]
    fn test_wallet_summary() {
        let (_temp_dir, mut wallet_manager) = setup_test_environment();

        // Create multiple wallets with different balances
        let requests = vec![
            CreateWalletRequest {
                nickname: "Wallet 1".to_string(),
                description: "First wallet".to_string(),
                category: Some("Personal".to_string()),
                color: Some("#3b82f6".to_string()),
                is_favorite: true,
                is_default: true,
                auto_backup: true,
                notifications_enabled: true,
                default_fee_credits: Some(10000),
                import_data: None,
            },
            CreateWalletRequest {
                nickname: "Wallet 2".to_string(),
                description: "Second wallet".to_string(),
                category: Some("Mining".to_string()),
                color: Some("#10b981".to_string()),
                is_favorite: false,
                is_default: false,
                auto_backup: true,
                notifications_enabled: true,
                default_fee_credits: Some(10000),
                import_data: None,
            },
            CreateWalletRequest {
                nickname: "Wallet 3".to_string(),
                description: "Third wallet".to_string(),
                category: Some("Trading".to_string()),
                color: Some("#f59e0b".to_string()),
                is_favorite: true,
                is_default: false,
                auto_backup: true,
                notifications_enabled: true,
                default_fee_credits: Some(10000),
                import_data: None,
            },
        ];

        let mut wallets = Vec::new();
        for request in requests {
            wallets.push(wallet_manager.create_wallet(request).unwrap());
        }

        // Update balances
        wallet_manager.update_wallet_balance(&wallets[0].id, 500000000).unwrap(); // 5 BTP
        wallet_manager.update_wallet_balance(&wallets[1].id, 1000000000).unwrap(); // 10 BTP
        wallet_manager.update_wallet_balance(&wallets[2].id, 1500000000).unwrap(); // 15 BTP

        let summary = wallet_manager.get_summary();

        assert_eq!(summary.total_wallets, 3);
        assert_eq!(summary.total_balance_credits, 3000000000); // 30 BTP total
        assert_eq!(summary.total_balance_btp, 30.0);
        assert_eq!(summary.favorite_wallets, 2);
        assert!(summary.most_recent_wallet.is_some());
        assert!(summary.highest_balance_wallet.is_some());
        assert!(summary.default_wallet.is_some());

        // Highest balance should be wallet 3 (15 BTP)
        let highest = summary.highest_balance_wallet.unwrap();
        assert_eq!(highest.nickname, "Wallet 3");
        assert_eq!(highest.cached_balance_credits, 1500000000);
    }

    #[test]
    fn test_wallet_backup() {
        let (_temp_dir, mut wallet_manager) = setup_test_environment();

        let request = CreateWalletRequest {
            nickname: "Backup Test".to_string(),
            description: "Test wallet for backup".to_string(),
            category: Some("Personal".to_string()),
            color: Some("#3b82f6".to_string()),
            is_favorite: false,
            is_default: true,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000),
            import_data: None,
        };

        let wallet = wallet_manager.create_wallet(request).unwrap();

        let backup_result = wallet_manager.backup_wallet(&wallet.id);
        assert!(backup_result.is_ok());

        let backup_path = backup_result.unwrap();
        assert!(backup_path.exists());

        // Verify backup content
        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert!(backup_content.contains("Backup Test"));
        assert!(backup_content.contains(&wallet.address));
    }

    #[test]
    fn test_default_wallet_management() {
        let (_temp_dir, mut wallet_manager) = setup_test_environment();

        // Create first wallet as default
        let request1 = CreateWalletRequest {
            nickname: "First Default".to_string(),
            description: "First default wallet".to_string(),
            category: Some("Personal".to_string()),
            color: Some("#3b82f6".to_string()),
            is_favorite: false,
            is_default: true,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000),
            import_data: None,
        };

        let wallet1 = wallet_manager.create_wallet(request1).unwrap();
        assert!(wallet1.is_default);

        // Create second wallet and set as default
        let request2 = CreateWalletRequest {
            nickname: "Second Default".to_string(),
            description: "Second default wallet".to_string(),
            category: Some("Mining".to_string()),
            color: Some("#10b981".to_string()),
            is_favorite: false,
            is_default: false,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000),
            import_data: None,
        };

        let wallet2 = wallet_manager.create_wallet(request2).unwrap();

        // Update second wallet to be default
        let update_request = UpdateWalletRequest {
            wallet_id: wallet2.id.clone(),
            nickname: None,
            description: None,
            category: None,
            color: None,
            is_favorite: None,
            is_default: Some(true),
            auto_backup: None,
            notifications_enabled: None,
            default_fee_credits: None,
        };

        wallet_manager.update_wallet(update_request).unwrap();

        // First wallet should no longer be default
        let updated_wallet1 = wallet_manager.get_wallet(&wallet1.id).unwrap();
        let updated_wallet2 = wallet_manager.get_wallet(&wallet2.id).unwrap();

        assert!(!updated_wallet1.is_default);
        assert!(updated_wallet2.is_default);

        // get_default_wallet should return the second wallet
        let default_wallet = wallet_manager.get_default_wallet().unwrap();
        assert_eq!(default_wallet.id, wallet2.id);
    }

    #[test]
    fn test_persistence() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = WalletManagerConfig {
            data_dir: temp_dir.path().to_path_buf(),
            default_category: "Test".to_string(),
            backup_enabled: true,
            auto_save: true,
        };

        let security = SecurityManager::new(temp_dir.path().join("security"))
            .expect("Failed to create security manager");

        let wallet_id;

        // Create wallet in first instance
        {
            let mut wallet_manager = WalletManager::new(config.clone(), security.clone()).unwrap();

            let request = CreateWalletRequest {
                nickname: "Persistence Test".to_string(),
                description: "Test wallet persistence".to_string(),
                category: Some("Personal".to_string()),
                color: Some("#3b82f6".to_string()),
                is_favorite: true,
                is_default: true,
                auto_backup: true,
                notifications_enabled: true,
                default_fee_credits: Some(10000),
                import_data: None,
            };

            let wallet = wallet_manager.create_wallet(request).unwrap();
            wallet_id = wallet.id.clone();

            wallet_manager.update_wallet_balance(&wallet_id, 500000000).unwrap();
        }

        // Create new instance and verify persistence
        {
            let wallet_manager = WalletManager::new(config, security).unwrap();

            assert_eq!(wallet_manager.list_wallets().len(), 1);

            let persisted_wallet = wallet_manager.get_wallet(&wallet_id).unwrap();
            assert_eq!(persisted_wallet.nickname, "Persistence Test");
            assert_eq!(persisted_wallet.cached_balance_credits, 500000000);
            assert!(persisted_wallet.metadata.is_favorite);
            assert!(persisted_wallet.is_default);
        }
    }

    #[test]
    fn test_error_handling() {
        let (_temp_dir, mut wallet_manager) = setup_test_environment();

        // Test invalid wallet ID
        let result = wallet_manager.get_wallet("invalid_id");
        assert!(result.is_none());

        // Test updating non-existent wallet
        let update_request = UpdateWalletRequest {
            wallet_id: "non_existent".to_string(),
            nickname: Some("Updated".to_string()),
            description: None,
            category: None,
            color: None,
            is_favorite: None,
            is_default: None,
            auto_backup: None,
            notifications_enabled: None,
            default_fee_credits: None,
        };

        let result = wallet_manager.update_wallet(update_request);
        assert!(result.is_err());

        // Test deleting non-existent wallet
        let result = wallet_manager.delete_wallet("non_existent");
        assert!(result.is_err());

        // Test updating balance for non-existent wallet
        let result = wallet_manager.update_wallet_balance("non_existent", 1000000);
        assert!(result.is_err());
    }

    #[test]
    fn test_concurrent_operations() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = WalletManagerConfig {
            data_dir: temp_dir.path().to_path_buf(),
            default_category: "Test".to_string(),
            backup_enabled: true,
            auto_save: true,
        };

        let security = SecurityManager::new(temp_dir.path().join("security"))
            .expect("Failed to create security manager");

        let wallet_manager = Arc::new(Mutex::new(
            WalletManager::new(config, security).unwrap()
        ));

        let mut handles = vec![];

        // Spawn multiple threads creating wallets
        for i in 0..5 {
            let wm = Arc::clone(&wallet_manager);
            let handle = thread::spawn(move || {
                let request = CreateWalletRequest {
                    nickname: format!("Concurrent Wallet {}", i),
                    description: format!("Wallet created by thread {}", i),
                    category: Some("Personal".to_string()),
                    color: Some("#3b82f6".to_string()),
                    is_favorite: false,
                    is_default: false,
                    auto_backup: true,
                    notifications_enabled: true,
                    default_fee_credits: Some(10000),
                    import_data: None,
                };

                let mut manager = wm.lock().unwrap();
                manager.create_wallet(request)
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        let results: Vec<_> = handles.into_iter()
            .map(|h| h.join().unwrap())
            .collect();

        // All operations should succeed
        for result in &results {
            assert!(result.is_ok());
        }

        // Should have 5 wallets
        let manager = wallet_manager.lock().unwrap();
        assert_eq!(manager.list_wallets().len(), 5);
    }

    #[test]
    fn test_wallet_import_functionality() {
        let (_temp_dir, mut wallet_manager) = setup_test_environment();

        let import_data = ImportData {
            address: "btpc1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
            private_key_hex: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            password: "test_password".to_string(),
        };

        let request = CreateWalletRequest {
            nickname: "Imported Wallet".to_string(),
            description: "Test imported wallet".to_string(),
            category: Some("imported".to_string()),
            color: Some("#6366f1".to_string()),
            is_favorite: false,
            is_default: false,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000),
            import_data: Some(import_data.clone()),
        };

        let result = wallet_manager.create_wallet(request);
        assert!(result.is_ok());

        let wallet = result.unwrap();
        assert_eq!(wallet.nickname, "Imported Wallet");
        assert_eq!(wallet.address, import_data.address);
        assert_eq!(wallet.metadata.category, "imported");
        assert_eq!(wallet.source, WalletSource::ImportedFromKey {
            import_date: wallet.created_at
        });
    }

    #[test]
    fn test_performance_benchmarks() {
        let (_temp_dir, mut wallet_manager) = setup_test_environment();

        let start = std::time::Instant::now();

        // Create 100 wallets
        for i in 0..100 {
            let request = CreateWalletRequest {
                nickname: format!("Benchmark Wallet {}", i),
                description: format!("Benchmark test wallet {}", i),
                category: Some("Personal".to_string()),
                color: Some("#3b82f6".to_string()),
                is_favorite: i % 5 == 0, // Every 5th wallet is favorite
                is_default: i == 0, // First wallet is default
                auto_backup: true,
                notifications_enabled: true,
                default_fee_credits: Some(10000),
                import_data: None,
            };

            let result = wallet_manager.create_wallet(request);
            assert!(result.is_ok());
        }

        let creation_time = start.elapsed();
        println!("Created 100 wallets in {:?}", creation_time);

        // Test retrieval performance
        let start = std::time::Instant::now();

        for i in 0..100 {
            let nickname = format!("Benchmark Wallet {}", i);
            let wallet = wallet_manager.get_wallet_by_nickname(&nickname);
            assert!(wallet.is_some());
        }

        let retrieval_time = start.elapsed();
        println!("Retrieved 100 wallets by nickname in {:?}", retrieval_time);

        // Test summary generation performance
        let start = std::time::Instant::now();
        let summary = wallet_manager.get_summary();
        let summary_time = start.elapsed();

        println!("Generated summary for 100 wallets in {:?}", summary_time);
        assert_eq!(summary.total_wallets, 100);
        assert_eq!(summary.favorite_wallets, 20); // Every 5th wallet

        // Performance assertions (these may need adjustment based on hardware)
        assert!(creation_time.as_millis() < 5000, "Wallet creation took too long: {:?}", creation_time);
        assert!(retrieval_time.as_millis() < 1000, "Wallet retrieval took too long: {:?}", retrieval_time);
        assert!(summary_time.as_millis() < 100, "Summary generation took too long: {:?}", summary_time);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::super::*;
    use crate::security::SecurityManager;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_full_wallet_lifecycle() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = WalletManagerConfig {
            data_dir: temp_dir.path().to_path_buf(),
            default_category: "Integration".to_string(),
            backup_enabled: true,
            auto_save: true,
        };

        let security = SecurityManager::new(temp_dir.path().join("security"))
            .expect("Failed to create security manager");

        let mut wallet_manager = WalletManager::new(config, security)
            .expect("Failed to create wallet manager");

        // 1. Create wallet
        let create_request = CreateWalletRequest {
            nickname: "Integration Test Wallet".to_string(),
            description: "Full lifecycle test".to_string(),
            category: Some("Personal".to_string()),
            color: Some("#3b82f6".to_string()),
            is_favorite: false,
            is_default: true,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000),
            import_data: None,
        };

        let wallet = wallet_manager.create_wallet(create_request)
            .expect("Failed to create wallet");

        // 2. Update wallet balance
        wallet_manager.update_wallet_balance(&wallet.id, 2500000000)
            .expect("Failed to update balance");

        // 3. Update wallet metadata
        let update_request = UpdateWalletRequest {
            wallet_id: wallet.id.clone(),
            nickname: Some("Updated Integration Wallet".to_string()),
            description: Some("Updated description".to_string()),
            category: Some("Mining".to_string()),
            color: Some("#10b981".to_string()),
            is_favorite: Some(true),
            is_default: None,
            auto_backup: Some(false),
            notifications_enabled: Some(false),
            default_fee_credits: Some(20000),
        };

        let updated_wallet = wallet_manager.update_wallet(update_request)
            .expect("Failed to update wallet");

        // 4. Create backup
        let backup_path = wallet_manager.backup_wallet(&wallet.id)
            .expect("Failed to backup wallet");

        // 5. Verify all changes
        assert_eq!(updated_wallet.nickname, "Updated Integration Wallet");
        assert_eq!(updated_wallet.cached_balance_credits, 2500000000);
        assert_eq!(updated_wallet.cached_balance_btp, 25.0);
        assert_eq!(updated_wallet.metadata.category, "Mining");
        assert!(updated_wallet.metadata.is_favorite);
        assert!(!updated_wallet.metadata.auto_backup);
        assert!(backup_path.exists());

        // 6. Delete wallet
        wallet_manager.delete_wallet(&wallet.id)
            .expect("Failed to delete wallet");

        // 7. Verify deletion
        assert!(wallet_manager.get_wallet(&wallet.id).is_none());
        assert_eq!(wallet_manager.list_wallets().len(), 0);
        assert!(!wallet.file_path.exists());
    }

    #[test]
    fn test_wallet_file_corruption_recovery() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = WalletManagerConfig {
            data_dir: temp_dir.path().to_path_buf(),
            default_category: "Recovery".to_string(),
            backup_enabled: true,
            auto_save: true,
        };

        let security = SecurityManager::new(temp_dir.path().join("security"))
            .expect("Failed to create security manager");

        let mut wallet_manager = WalletManager::new(config.clone(), security.clone())
            .expect("Failed to create wallet manager");

        // Create a wallet
        let request = CreateWalletRequest {
            nickname: "Corruption Test".to_string(),
            description: "Test corruption recovery".to_string(),
            category: Some("Personal".to_string()),
            color: Some("#3b82f6".to_string()),
            is_favorite: false,
            is_default: true,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000),
            import_data: None,
        };

        let wallet = wallet_manager.create_wallet(request).unwrap();
        let wallet_file_path = wallet.file_path.clone();

        // Corrupt the wallet file
        fs::write(&wallet_file_path, "corrupted data").unwrap();

        // Create new wallet manager instance
        let wallet_manager2 = WalletManager::new(config, security);

        // Should handle corrupted file gracefully
        assert!(wallet_manager2.is_ok());
        let wallet_manager2 = wallet_manager2.unwrap();

        // The corrupted wallet should not be loaded
        assert_eq!(wallet_manager2.list_wallets().len(), 0);
    }

    #[test]
    fn test_encrypted_wallet_persistence() {
        use btpc_core::crypto::SecurePassword;

        let (_temp_dir, mut wallet_manager, btpc) = setup_test_environment();

        // Create wallet
        let request = CreateWalletRequest {
            nickname: "Encrypted Test".to_string(),
            description: "Test encrypted storage".to_string(),
            category: Some("Test".to_string()),
            color: Some("#00ff00".to_string()),
            is_favorite: false,
            is_default: true,
            auto_backup: false,
            notifications_enabled: false,
            default_fee_credits: None,
            password: "wallet_password_123".to_string(),
            import_data: None,
        };

        let wallet_response = wallet_manager.create_wallet(request, &btpc).unwrap();
        let original_wallet_id = wallet_response.wallet_info.id.clone();

        // Save with encryption
        let password = SecurePassword::new("test_password_123".to_string());
        wallet_manager.save_wallets_encrypted(&password).unwrap();

        // Create new manager (simulates restart)
        let security2 = SecurityManager::new(_temp_dir.path().join("security2")).unwrap();
        let config2 = wallet_manager.config.clone();
        let mut wallet_manager2 = WalletManager::new(config2, security2).unwrap();

        // Load with same password
        wallet_manager2.load_wallets_encrypted(&password).unwrap();

        // Verify
        let loaded = wallet_manager2.list_wallets();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, original_wallet_id);
    }

    #[test]
    fn test_encrypted_wallet_wrong_password() {
        use btpc_core::crypto::SecurePassword;

        let (_temp_dir, mut wallet_manager, btpc) = setup_test_environment();

        let request = CreateWalletRequest {
            nickname: "Password Test".to_string(),
            description: "Test wrong password".to_string(),
            category: Some("Test".to_string()),
            color: None,
            is_favorite: false,
            is_default: true,
            auto_backup: false,
            notifications_enabled: false,
            default_fee_credits: None,
            password: "wallet_password_456".to_string(),
            import_data: None,
        };

        wallet_manager.create_wallet(request, &btpc).unwrap();

        let correct_password = SecurePassword::new("correct_pass".to_string());
        wallet_manager.save_wallets_encrypted(&correct_password).unwrap();

        // Wrong password
        let security2 = SecurityManager::new(_temp_dir.path().join("security3")).unwrap();
        let config2 = wallet_manager.config.clone();
        let mut wallet_manager2 = WalletManager::new(config2, security2).unwrap();

        let wrong_password = SecurePassword::new("wrong_pass".to_string());
        let result = wallet_manager2.load_wallets_encrypted(&wrong_password);

        assert!(result.is_err());
    }
}
