//! Tauri Application Builder
//!
//! Contains the main Tauri app setup including:
//! - Single instance lock management
//! - Command handler registration
//! - Background task initialization (health monitoring, transaction monitor, etc.)

use std::fs;
use tauri::Manager;

use crate::app_state::AppState;
use crate::auth_commands;
use crate::auth_state;
use crate::commands;
use crate::commands::blockchain::bits_to_difficulty;
use crate::gpu_stats_commands;
use crate::lock_manager;
use crate::mining_commands;
use crate::transaction_commands;
use crate::transaction_monitor;
use crate::wallet_commands;

// ============================================================================
// Single Instance Lock
// ============================================================================

/// Ensure only one instance of the desktop app is running
/// Returns a lock guard that must be kept alive for the app lifetime
fn ensure_single_instance() -> Result<lock_manager::FileLockGuard, String> {
    use lock_manager::LockManager;

    let btpc_home = dirs::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?
        .join(".btpc");

    // Create .btpc directory if it doesn't exist
    fs::create_dir_all(&btpc_home)
        .map_err(|e| format!("Failed to create .btpc directory: {}", e))?;

    // Create lock manager for .btpc/locks directory
    let lock_dir = btpc_home.join("locks");
    let lock_mgr =
        LockManager::new(lock_dir).map_err(|e| format!("Failed to create lock manager: {}", e))?;

    // Try to acquire exclusive lock (cross-platform, safe implementation)
    match lock_mgr.try_lock_exclusive("btpc_desktop_app") {
        Ok(Some(guard)) => {
            let pid = std::process::id();
            eprintln!("[BTPC::App] Single instance lock acquired (PID: {})", pid);
            Ok(guard)
        }
        Ok(None) => {
            let lock_path = btpc_home.join("locks/btpc_desktop_app.lock");
            Err(format!(
                "Another instance of BTPC desktop app is already running.\n\
                 If you're sure no other instance is running, delete: {}",
                lock_path.display()
            ))
        }
        Err(e) => Err(format!("Failed to acquire single instance lock: {}", e)),
    }
}

// ============================================================================
// Main Application Runner
// ============================================================================

/// Run the Tauri application
///
/// This function initializes all application state, sets up the Tauri builder
/// with command handlers and background tasks, and runs the application.
pub fn run() {
    // Ensure only one instance is running
    let _app_lock = ensure_single_instance().expect("Failed to acquire single instance lock");

    let app_state = AppState::new().expect("Failed to initialize app state");
    let process_manager = app_state.process_manager.clone();

    // Clone embedded_node as NodeHandle for embedded_node commands
    // This allows commands to access node via State<NodeHandle> directly
    let node_handle: commands::embedded_node::NodeHandle = app_state.embedded_node.clone();

    // Initialize authentication session state (Feature 006: Application-Level Login/Logout)
    let auth_session = std::sync::RwLock::new(auth_state::SessionState::new());

    tauri::Builder::default()
        .manage(app_state)
        .manage(node_handle)
        .manage(auth_session)
        .setup(move |app| {
            // Scan for and adopt existing BTPC processes
            eprintln!("[BTPC::App] Scanning for existing BTPC processes...");
            let adopted = process_manager
                .scan_and_adopt(vec![("node", "btpc_node"), ("miner", "btpc_miner")]);

            if !adopted.is_empty() {
                eprintln!("[BTPC::App] Adopted {} existing process(es):", adopted.len());
                for process in &adopted {
                    eprintln!("[BTPC::App]   - {}", process);
                }
            } else {
                eprintln!("[BTPC::App] No existing BTPC processes found");
            }

            // Set app handle and wallet manager in embedded node
            {
                eprintln!("[BTPC::App] Setting up embedded node with app handle and wallet manager...");
                let app_state = app.state::<AppState>();
                let embedded_node = app_state.embedded_node.clone();
                let wallet_manager = app_state.wallet_manager.clone();
                let tx_storage = app_state.tx_storage.clone();
                let app_handle = app.handle().clone();

                tauri::async_runtime::block_on(async move {
                    let mut node = embedded_node.write().await;
                    node.set_app_handle(app_handle);
                    node.set_wallet_manager(wallet_manager);
                    node.set_tx_storage(tx_storage);
                    eprintln!("[BTPC::App] Embedded node setup complete");
                });
            }

            // Cleanup on window close - stop mining, shutdown node, stop processes
            let window = app.get_webview_window("main").unwrap();

            // Explicitly grab GTK keyboard focus for the WebView on Linux.
            // On some WebKitGTK configurations the OS window gets focus but the
            // WebView widget inside it does not, so keyboard events never reach JS.
            let _ = window.set_focus();


            let pm = process_manager.clone();
            let app_handle_for_close = app.handle().clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    eprintln!("[BTPC::App] Shutting down - stopping all services...");

                    let app_state = app_handle_for_close.state::<AppState>();

                    // 1. Stop background polling loops and network monitor
                    app_state.node_active.store(false, std::sync::atomic::Ordering::SeqCst);
                    app_state.network_monitor_active.store(false, std::sync::atomic::Ordering::SeqCst);

                    // 2. Stop mining pool
                    let mining_pool = app_state.mining_pool.clone();
                    let embedded_node = app_state.embedded_node.clone();
                    tauri::async_runtime::block_on(async {
                        // Stop mining
                        {
                            let mut pool_guard = mining_pool.write().await;
                            if let Some(ref mut pool) = *pool_guard {
                                if pool.get_stats().is_mining {
                                    eprintln!("[BTPC::App] Stopping mining...");
                                    let _ = pool.stop_all().await;
                                    eprintln!("[BTPC::App] Mining stopped");
                                }
                            }
                        }

                        // 3. Shutdown embedded node (flush DB, clear peers)
                        {
                            let mut node = embedded_node.write().await;
                            node.clear_peers();
                            if let Err(e) = node.shutdown().await {
                                eprintln!("[BTPC::App] Node shutdown error: {}", e);
                            } else {
                                eprintln!("[BTPC::App] Embedded node shutdown complete");
                            }
                        }
                    });

                    // 4. Stop external processes
                    pm.stop_all();
                    eprintln!("[BTPC::App] All services stopped");
                }
            });

            // Start process health monitoring (Article XI.5 - Process Lifecycle Management)
            let pm_health = process_manager.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(30));
                pm_health.health_check();
            });

            // Start transaction monitor service (Feature 007: Transaction Sending)
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let app_state = app_handle.state::<AppState>();
                transaction_monitor::start_transaction_monitor(&app_state, app_handle.clone())
                    .await;
            });

            // NOTE: Session timeout monitor REMOVED (2025-12-04)
            // Previously logged out users after 15 minutes of inactivity, which killed
            // mining operations. Users can manually lock wallets via Settings > Security
            // if they want to secure their session without stopping mining.
            // See: lock_wallets command in commands/wallet_encryption.rs

            // Start mining status updater (polls every 2 seconds)
            let app_handle_for_mining = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    let app_state = app_handle_for_mining.state::<AppState>();

                    // Skip polling if node is shutting down (logout/close)
                    if !app_state.node_active.load(std::sync::atomic::Ordering::SeqCst) {
                        continue;
                    }

                    let mining_stats = {
                        let mining_pool_guard = app_state.mining_pool.read().await;
                        if let Some(ref pool) = *mining_pool_guard {
                            pool.get_stats()
                        } else {
                            continue;
                        }
                    };

                    let (current_height, current_difficulty) = {
                        let node = app_state.embedded_node.read().await;
                        match node.get_blockchain_state().await {
                            Ok(state) => {
                                let difficulty = bits_to_difficulty(state.difficulty_bits);
                                (state.current_height, difficulty.to_string())
                            },
                            Err(_) => (0, "1.0".to_string()),
                        }
                    };

                    // Persist blocks_found to disk (for app restart persistence)
                    {
                        let mut persistent_stats = app_state.mining_stats.lock()
                            .expect("Failed to lock mining_stats");

                        // Only update and save if blocks_found has changed
                        if persistent_stats.blocks_found != mining_stats.blocks_found {
                            persistent_stats.blocks_found = mining_stats.blocks_found;
                            persistent_stats.save_to_disk();
                        }
                    }

                    let _ = app_state.mining_status.update(|status| {
                        status.active = mining_stats.is_mining;
                        status.hashrate = mining_stats.total_hashrate as u64;
                        status.blocks_mined = mining_stats.blocks_found as u32;
                        status.current_difficulty = current_difficulty;
                        status.threads = mining_stats.cpu_threads + mining_stats.gpu_devices;
                        status.current_height = current_height;
                    }, &app_handle_for_mining);
                }
            });

            // Start node status updater (polls every 2 seconds)
            // FIX 2025-11-24: Added to update Dashboard with blockchain difficulty
            let app_handle_for_node = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    let app_state = app_handle_for_node.state::<AppState>();

                    // Skip polling if node is shutting down (logout/close)
                    if !app_state.node_active.load(std::sync::atomic::Ordering::SeqCst) {
                        continue;
                    }

                    let node = app_state.embedded_node.read().await;

                    // Get blockchain state for difficulty and height
                    if let Ok(blockchain_state) = node.get_blockchain_state().await {
                        let difficulty = bits_to_difficulty(blockchain_state.difficulty_bits);
                        let sync_progress = node.get_sync_progress().unwrap_or({
                            btpc_desktop_app::embedded_node::SyncProgress {
                                current_height: 0,
                                target_height: 0,
                                is_syncing: false,
                                connected_peers: 0,
                                sync_percentage: 0.0,
                            }
                        });

                        let _ = app_state.node_status.update(|status| {
                            status.running = true; // Node is always running (embedded)
                            status.pid = None; // No separate process
                            status.block_height = blockchain_state.current_height;
                            status.peer_count = sync_progress.connected_peers;
                            status.sync_progress = sync_progress.sync_percentage / 100.0;
                            status.network = node.get_network();
                            status.difficulty = difficulty; // THIS is the fix!
                        }, &app_handle_for_node);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // System commands
            commands::system::test_command,
            commands::system::get_system_status,
            commands::system::setup_btpc,
            commands::system::get_logs,
            commands::system::get_all_settings,
            commands::system::get_network_config,
            commands::system::save_network_config,
            commands::system::debug_dump_persistence_state,
            commands::system::debug_dump_difficulty_timestamps,
            // Node commands
            commands::node::start_node,
            commands::node::stop_node,
            commands::node::get_node_status,
            commands::node::start_blockchain_sync,
            commands::node::stop_blockchain_sync,
            commands::node::get_sync_stats,
            commands::node::trigger_manual_sync,
            commands::node::get_address_balance_from_node,
            // Mining commands
            commands::mining_status::get_mining_status,
            commands::mining_logs::get_mining_logs,
            mining_commands::start_mining,
            mining_commands::stop_mining,
            mining_commands::get_mining_history,
            mining_commands::get_mining_stats,
            mining_commands::override_network_pause,
            // Wallet basic commands
            commands::wallet_basic::get_total_balance,
            commands::wallet_basic::list_addresses,
            commands::wallet_basic::create_wallet,
            commands::wallet_basic::get_wallet_balance,
            commands::wallet_basic::get_wallet_balance_with_mined,
            commands::wallet_basic::get_wallet_address,
            // FIX 2026-02-21 (H2): Removed send_btpc - password was never verified.
            // Use create_transaction→sign_transaction→broadcast_transaction flow instead.
            // UTXO commands
            commands::utxo::reload_utxos,
            commands::utxo::get_utxo_stats,
            commands::utxo::get_wallet_utxos,
            commands::utxo::get_spendable_utxos,
            commands::utxo::add_mining_utxo,
            commands::utxo::sync_wallet_utxos,
            // Transaction storage commands
            commands::tx_storage::get_transaction_history,
            commands::tx_storage::get_paginated_transaction_history,
            commands::tx_storage::add_transaction_to_storage,
            commands::tx_storage::get_transaction_from_storage,
            commands::tx_storage::get_wallet_balance_from_storage,
            commands::tx_storage::get_transaction_count_from_storage,
            commands::tx_storage::get_mining_history_from_storage,
            commands::tx_storage::migrate_json_transactions_to_rocksdb,
            commands::tx_storage::create_transaction_preview,
            // FIX 2025-12-11: Debug commands for transaction diagnostics
            commands::tx_storage::debug_tx_diagnostic,
            commands::tx_storage::debug_cleanup_pending_txs,
            // Transaction commands (Feature 007: Transaction Sending)
            transaction_commands::create_transaction,
            transaction_commands::sign_transaction,
            transaction_commands::broadcast_transaction,
            transaction_commands::get_transaction_status,
            transaction_commands::cancel_transaction,
            transaction_commands::estimate_fee,
            // Security commands
            commands::security::create_user,
            commands::security::login_user,
            commands::security::logout_user,
            commands::security::recover_account,
            commands::security::check_security_session,
            commands::security::get_session_info,
            commands::security::get_users,
            commands::security::user_exists,
            commands::security::decrypt_wallet_key,
            // Wallet encryption commands
            commands::wallet_encryption::check_wallet_lock_status,
            commands::wallet_encryption::unlock_wallets,
            commands::wallet_encryption::lock_wallets,
            commands::wallet_encryption::change_master_password,
            commands::wallet_encryption::migrate_to_encrypted,
            // Embedded node commands (Feature 013)
            commands::embedded_node::init_embedded_node,
            commands::embedded_node::get_blockchain_state,
            commands::embedded_node::get_sync_progress,
            commands::embedded_node::start_embedded_blockchain_sync,
            commands::embedded_node::stop_embedded_blockchain_sync,
            commands::embedded_node::shutdown_embedded_node,
            commands::embedded_node::get_peer_info,
            commands::embedded_node::add_simulated_peers, // P2P testing
            commands::embedded_node::clear_peers,         // P2P testing
            commands::embedded_node::connect_to_peer,     // Real P2P connection
            commands::embedded_node::get_disk_space_info, // FR-058: Disk space monitoring
            // Block explorer commands
            commands::blockchain::get_blockchain_info,
            commands::blockchain::get_network_health_info,
            commands::blockchain::get_recent_blocks,
            commands::blockchain::get_recent_transactions,
            commands::blockchain::search_blockchain,
            commands::blockchain::get_block_message,
            // Address book commands
            commands::address_book::add_address_book_entry,
            commands::address_book::list_address_book_entries,
            commands::address_book::get_address_book_entry,
            commands::address_book::update_address_book_entry,
            commands::address_book::delete_address_book_entry,
            commands::address_book::search_address_book_entries,
            // Multi-wallet management commands
            wallet_commands::create_wallet_with_nickname,
            wallet_commands::list_wallets,
            wallet_commands::get_wallet,
            wallet_commands::get_wallet_by_nickname,
            wallet_commands::get_default_wallet,
            wallet_commands::update_wallet,
            wallet_commands::delete_wallet,
            wallet_commands::get_wallet_summary,
            wallet_commands::update_wallet_balance,
            wallet_commands::get_wallet_balance_by_id,
            // FIX 2026-02-21 (H2): Removed send_btpc_from_wallet - same password bypass issue.
            wallet_commands::backup_wallet,
            wallet_commands::set_default_wallet,
            wallet_commands::toggle_wallet_favorite,
            wallet_commands::get_favorite_wallets,
            wallet_commands::refresh_all_wallet_balances,
            wallet_commands::import_wallet_from_key,
            wallet_commands::import_wallet_from_mnemonic,
            wallet_commands::import_wallet_from_backup,
            wallet_commands::export_wallet_to_json,
            wallet_commands::export_wallet_address,
            wallet_commands::export_all_wallets_summary,
            wallet_commands::generate_wallet_recovery_data,
            wallet_commands::migrate_utxo_addresses,
            wallet_commands::clean_orphaned_utxos,
            // Authentication commands (Feature 006)
            auth_commands::has_master_password,
            auth_commands::create_master_password,
            auth_commands::login,
            auth_commands::logout,
            auth_commands::check_session,
            // GPU Mining Dashboard commands (Feature 012)
            gpu_stats_commands::get_gpu_stats,
            gpu_stats_commands::is_gpu_stats_available,
            gpu_stats_commands::enumerate_gpus,
            gpu_stats_commands::get_gpu_mining_stats,
            gpu_stats_commands::get_gpu_health_metrics,
            gpu_stats_commands::set_temperature_threshold,
            gpu_stats_commands::get_temperature_threshold,
            gpu_stats_commands::set_gpu_fan_speed,
            gpu_stats_commands::get_gpu_dashboard_data,
            // Database commands (T182)
            commands::database::check_database_integrity,
            commands::database::create_database_backup,
            commands::database::restore_database_backup,
            commands::database::list_database_backups,
            commands::database::restart_app,
            // GPU detection commands
            commands::gpu_stats::get_available_gpus,
            commands::gpu_stats::check_gpu_available,
            commands::gpu_stats::get_recommended_gpu,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}