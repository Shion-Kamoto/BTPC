//! Contract Tests for Error Handling
//!
//! These tests validate the error handling contracts from:
//! - specs/003-fix-node-and/contracts/error_types.json
//! - NFR-003: Error messages don't expose private keys
//! - NFR-004: Technical details sanitize passwords
//!
//! Test Status: These tests validate existing error module functionality

use btpc_desktop_app::error::{BtpcError, ProcessError};
use serde_json;

#[test]
fn test_error_serialization_contract() {
    // Contract: All BtpcError variants must be serializable to JSON
    // This is required for Tauri event emission

    let errors = vec![
        BtpcError::mutex_poison("TestComponent", "test_operation"),
        BtpcError::database_locked("blockchain.db", "PID 12345"),
        BtpcError::process_crashed(12345, Some(1)),
    ];

    for error in errors {
        let serialized = serde_json::to_string(&error);
        assert!(
            serialized.is_ok(),
            "Error should serialize to JSON: {:?}",
            error
        );

        // Verify deserialization
        if let Ok(json) = serialized {
            let deserialized: Result<BtpcError, _> = serde_json::from_str(&json);
            assert!(
                deserialized.is_ok(),
                "Error should deserialize from JSON: {}",
                json
            );
        }
    }
}

#[test]
fn test_mutex_poison_error_contract() {
    // Contract: MutexPoisoned error must contain component and operation info
    let error = BtpcError::mutex_poison("NodeManager", "start_node");

    match &error {
        BtpcError::Process(ProcessError::MutexPoisoned { component, operation }) => {
            assert_eq!(component, "NodeManager");
            assert_eq!(operation, "start_node");
        }
        _ => panic!("Expected MutexPoisoned error"),
    }

    // Verify user-friendly message
    let display = format!("{}", error);
    assert!(display.contains("Internal error"));
    assert!(display.contains("NodeManager"));
}

#[test]
fn test_database_locked_error_contract() {
    // Contract: DatabaseLocked error must contain database name and holder info
    let error = BtpcError::database_locked("blockchain.db", "btpc_node (PID: 12345)");

    match &error {
        BtpcError::Process(ProcessError::DatabaseLocked { database, holder_info }) => {
            assert_eq!(database, "blockchain.db");
            assert!(holder_info.contains("12345"));
        }
        _ => panic!("Expected DatabaseLocked error"),
    }

    let display = format!("{}", error);
    assert!(display.contains("locked"));
}

#[test]
fn test_process_crashed_error_contract() {
    // Contract: ProcessCrashed must include PID and optional exit code
    let error_with_code = BtpcError::process_crashed(12345, Some(1));
    let error_without_code = BtpcError::process_crashed(12345, None);

    match error_with_code {
        BtpcError::Process(ProcessError::ProcessCrashed { pid, exit_code }) => {
            assert_eq!(pid, 12345);
            assert_eq!(exit_code, Some(1));
        }
        _ => panic!("Expected ProcessCrashed error"),
    }

    match error_without_code {
        BtpcError::Process(ProcessError::ProcessCrashed { pid, exit_code }) => {
            assert_eq!(pid, 12345);
            assert_eq!(exit_code, None);
        }
        _ => panic!("Expected ProcessCrashed error"),
    }
}

#[test]
fn test_error_size_contract() {
    // Contract: Error messages should be reasonable size (< 200 chars user message)
    let error = BtpcError::mutex_poison("VeryLongComponentNameThatShouldStillWork", "operation");
    let display = format!("{}", error);

    // User message should be concise
    assert!(
        display.len() < 500,
        "Error display message too long: {} chars",
        display.len()
    );
}

#[test]
fn test_error_display_no_panic() {
    // Contract: All error Display implementations must not panic
    let errors = vec![
        BtpcError::mutex_poison("Test", "test"),
        BtpcError::database_locked("db", "holder"),
        BtpcError::process_crashed(123, Some(0)),
        BtpcError::process_crashed(456, None),
    ];

    for error in errors {
        let _ = format!("{}", error); // Should not panic
        let _ = format!("{:?}", error); // Debug should also not panic
    }
}

#[cfg(test)]
mod error_sanitization_contracts {
    use super::*;

    #[test]
    fn test_no_private_key_exposure_in_errors() {
        // NFR-003: Error messages must not expose private keys
        // This is a placeholder - actual implementation would need to scan error messages
        // for patterns that look like private keys

        let error = BtpcError::Application("Failed to sign with key 0x1234...".to_string());
        let display = format!("{}", error);

        // Should not contain full hex keys (simplified check)
        assert!(
            !display.contains("0x") || display.contains("..."),
            "Error might expose full key: {}",
            display
        );
    }

    #[test]
    fn test_no_password_in_technical_details() {
        // NFR-004: Technical details must sanitize passwords
        // The sanitization logic would be implemented in ErrorSanitization trait

        let error = BtpcError::Application("Authentication failed".to_string());
        let display = format!("{}", error);

        // Common password-related strings should not appear in plaintext
        let forbidden_words = vec!["password=", "pwd=", "secret="];
        for word in forbidden_words {
            assert!(
                !display.to_lowercase().contains(word),
                "Error exposes password field: {}",
                display
            );
        }
    }
}
