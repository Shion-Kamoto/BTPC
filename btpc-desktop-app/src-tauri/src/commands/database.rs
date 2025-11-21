//! T182: Database Backup/Restore/Integrity Tauri Commands
//!
//! Exposes database operations to the frontend

use btpc_desktop_app::unified_database::{BackupInfo, UnifiedDatabase};
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

use crate::AppState;

/// Response for database integrity check
#[derive(Debug, Clone, Serialize)]
pub struct IntegrityCheckResponse {
    pub success: bool,
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub checked_at: u64,
    pub error_message: Option<String>,
}

/// Response for backup creation
#[derive(Debug, Clone, Serialize)]
pub struct CreateBackupResponse {
    pub success: bool,
    pub backup_path: Option<String>,
    pub error_message: Option<String>,
}

/// Response for backup restore
#[derive(Debug, Clone, Serialize)]
pub struct RestoreBackupResponse {
    pub success: bool,
    pub message: Option<String>,
    pub error_message: Option<String>,
}

/// Response for listing backups
#[derive(Debug, Clone, Serialize)]
pub struct ListBackupsResponse {
    pub success: bool,
    pub backups: Vec<BackupInfo>,
    pub error_message: Option<String>,
}

/// T182-001: Check database integrity
///
/// # FR-056
/// Validates database integrity and detects corruption
///
/// # Returns
/// IntegrityCheckResponse with validation results
#[tauri::command]
pub async fn check_database_integrity(
    state: State<'_, AppState>,
) -> Result<IntegrityCheckResponse, String> {
    eprintln!("🔍 Frontend requested database integrity check");

    let node = state.embedded_node.read().await;
    let db = node.get_database();

    match db.check_integrity() {
        Ok(result) => Ok(IntegrityCheckResponse {
            success: true,
            is_valid: result.is_valid,
            errors: result.errors,
            checked_at: result.checked_at,
            error_message: None,
        }),
        Err(e) => {
            eprintln!("❌ Integrity check failed: {}", e);
            Ok(IntegrityCheckResponse {
                success: false,
                is_valid: false,
                errors: vec![],
                checked_at: 0,
                error_message: Some(e.to_string()),
            })
        }
    }
}

/// T182-002: Create database backup
///
/// # FR-055
/// Creates atomic backup using RocksDB checkpoint
///
/// # Arguments
/// * `backup_name` - Name for the backup (e.g., "backup_2024-11-19")
///
/// # Returns
/// CreateBackupResponse with backup path
#[tauri::command]
pub async fn create_database_backup(
    state: State<'_, AppState>,
    backup_name: String,
) -> Result<CreateBackupResponse, String> {
    eprintln!("📦 Frontend requested database backup: {}", backup_name);

    // Create backups directory in user's home
    let home_dir = match dirs::home_dir() {
        Some(dir) => dir,
        None => {
            return Ok(CreateBackupResponse {
                success: false,
                backup_path: None,
                error_message: Some("Failed to determine home directory".to_string()),
            });
        }
    };

    let backups_root = home_dir.join(".btpc").join("backups");
    let backup_path = backups_root.join(&backup_name);

    let node = state.embedded_node.read().await;
    let db = node.get_database();

    match db.create_backup(&backup_path) {
        Ok(path) => {
            let path_str = path.to_string_lossy().to_string();
            eprintln!("✅ Backup created at: {}", path_str);

            Ok(CreateBackupResponse {
                success: true,
                backup_path: Some(path_str),
                error_message: None,
            })
        }
        Err(e) => {
            eprintln!("❌ Backup creation failed: {}", e);
            Ok(CreateBackupResponse {
                success: false,
                backup_path: None,
                error_message: Some(e.to_string()),
            })
        }
    }
}

/// T182-003: Restore database from backup
///
/// # FR-055, FR-056
/// Restores database from backup checkpoint
///
/// # Arguments
/// * `backup_path` - Path to backup directory
///
/// # Returns
/// RestoreBackupResponse with success/failure status
///
/// # Safety
/// This will shut down the application after restore.
/// The database must be reopened.
#[tauri::command]
pub async fn restore_database_backup(
    backup_path: String,
) -> Result<RestoreBackupResponse, String> {
    eprintln!("📂 Frontend requested database restore from: {}", backup_path);

    // Determine database path
    let home_dir = match dirs::home_dir() {
        Some(dir) => dir,
        None => {
            return Ok(RestoreBackupResponse {
                success: false,
                message: None,
                error_message: Some("Failed to determine home directory".to_string()),
            });
        }
    };

    let db_path = home_dir.join(".btpc").join("blockchain.db");
    let backup_path_buf = PathBuf::from(backup_path);

    // Perform restore
    match UnifiedDatabase::restore_from_backup(&backup_path_buf, &db_path) {
        Ok(_) => {
            eprintln!("✅ Database restored successfully");
            eprintln!("⚠️ Application must be restarted to use restored database");

            Ok(RestoreBackupResponse {
                success: true,
                message: Some("Database restored successfully. Please restart the application.".to_string()),
                error_message: None,
            })
        }
        Err(e) => {
            eprintln!("❌ Database restore failed: {}", e);
            Ok(RestoreBackupResponse {
                success: false,
                message: None,
                error_message: Some(e.to_string()),
            })
        }
    }
}

/// T182-004: List available backups
///
/// # FR-055
/// Lists all available database backups
///
/// # Returns
/// ListBackupsResponse with backup metadata
#[tauri::command]
pub async fn list_database_backups() -> Result<ListBackupsResponse, String> {
    eprintln!("📋 Frontend requested backup list");

    // Get backups directory
    let home_dir = match dirs::home_dir() {
        Some(dir) => dir,
        None => {
            return Ok(ListBackupsResponse {
                success: false,
                backups: vec![],
                error_message: Some("Failed to determine home directory".to_string()),
            });
        }
    };

    let backups_root = home_dir.join(".btpc").join("backups");

    match UnifiedDatabase::list_backups(&backups_root) {
        Ok(backups) => {
            eprintln!("✅ Found {} backup(s)", backups.len());
            Ok(ListBackupsResponse {
                success: true,
                backups,
                error_message: None,
            })
        }
        Err(e) => {
            eprintln!("❌ Failed to list backups: {}", e);
            Ok(ListBackupsResponse {
                success: false,
                backups: vec![],
                error_message: Some(e.to_string()),
            })
        }
    }
}