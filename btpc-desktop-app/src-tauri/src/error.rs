//! Comprehensive error handling for BTPC Desktop Application
//!
//! This module provides centralized error types and handling mechanisms
//! to ensure robust error recovery and user-friendly error messages.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Main error type for the BTPC Desktop Application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BtpcError {
    /// Security-related errors
    Security(SecurityError),
    /// BTPC integration errors
    Integration(IntegrationError),
    /// UTXO management errors
    Utxo(UtxoError),
    /// Transaction-specific errors
    Transaction(TransactionError),
    /// File system operation errors
    FileSystem(FileSystemError),
    /// Network communication errors
    Network(NetworkError),
    /// Configuration errors
    Configuration(ConfigurationError),
    /// Process management errors
    Process(ProcessError),
    /// Validation errors
    Validation(ValidationError),
    /// Generic application errors
    Application(String),
}

/// Security-specific error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityError {
    /// Authentication failed
    AuthenticationFailed { username: String, reason: String },
    /// User account is locked
    AccountLocked { username: String, locked_until: u64 },
    /// Session has expired
    SessionExpired { session_id: String },
    /// Invalid credentials provided
    InvalidCredentials,
    /// Encryption operation failed
    EncryptionFailed { operation: String },
    /// Decryption operation failed
    DecryptionFailed { operation: String },
    /// Key derivation failed
    KeyDerivationFailed,
    /// Invalid recovery phrase
    InvalidRecoveryPhrase,
    /// User already exists
    UserAlreadyExists { username: String },
    /// User not found
    UserNotFound { username: String },
    /// Password does not meet requirements
    PasswordRequirementsNotMet { requirements: String },
}

/// BTPC integration error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrationError {
    /// Binary not found
    BinaryNotFound { binary_name: String, expected_path: String },
    /// Binary execution failed
    ExecutionFailed { binary_name: String, error: String },
    /// Invalid binary output
    InvalidOutput { binary_name: String, output: String },
    /// Binary timeout
    ExecutionTimeout { binary_name: String, timeout_secs: u64 },
    /// Incompatible binary version
    IncompatibleVersion { binary_name: String, version: String, required: String },
    /// Missing BTPC home directory
    MissingBtpcHome { path: String },
    /// Invalid configuration
    InvalidConfiguration { field: String, value: String },
}

/// UTXO management error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UtxoError {
    /// Invalid UTXO data
    InvalidUtxo { txid: String, reason: String },
    /// UTXO not found
    UtxoNotFound { txid: String, vout: u32 },
    /// Insufficient funds
    InsufficientFunds { available: u64, required: u64 },
    /// Invalid address format
    InvalidAddress { address: String },
    /// Database operation failed
    DatabaseError { operation: String, error: String },
    /// Serialization failed
    SerializationError { data_type: String },
    /// Deserialization failed
    DeserializationError { data_type: String, error: String },
}

/// File system operation error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileSystemError {
    /// File not found
    FileNotFound { path: String },
    /// Permission denied
    PermissionDenied { path: String, operation: String },
    /// Directory creation failed
    DirectoryCreationFailed { path: String },
    /// File read failed
    ReadFailed { path: String, error: String },
    /// File write failed
    WriteFailed { path: String, error: String },
    /// Invalid file format
    InvalidFormat { path: String, expected_format: String },
}

/// Network communication error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkError {
    /// Connection failed
    ConnectionFailed { endpoint: String, error: String },
    /// Request timeout
    RequestTimeout { endpoint: String, timeout_ms: u64 },
    /// Invalid response
    InvalidResponse { endpoint: String, status: u16 },
    /// Network unreachable
    NetworkUnreachable,
    /// DNS resolution failed
    DnsResolutionFailed { hostname: String },
    /// TLS handshake failed
    TlsHandshakeFailed { endpoint: String },
}

/// Configuration error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigurationError {
    /// Missing required field
    MissingField { field: String, config_file: String },
    /// Invalid field value
    InvalidValue { field: String, value: String, expected: String },
    /// Configuration file not found
    ConfigFileNotFound { path: String },
    /// Configuration parsing failed
    ParsingFailed { path: String, error: String },
    /// Validation failed
    ValidationFailed { field: String, constraint: String },
}

/// Process management error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessError {
    /// Failed to start process
    StartFailed { command: String, error: String },
    /// Process crashed unexpectedly
    ProcessCrashed { pid: u32, exit_code: Option<i32> },
    /// Failed to stop process
    StopFailed { pid: u32, error: String },
    /// Process not found
    ProcessNotFound { pid: u32 },
    /// Resource exhaustion
    ResourceExhaustion { resource: String, limit: String },
    /// Mutex poisoned (thread panicked while holding lock)
    MutexPoisoned { component: String, operation: String },
    /// Database lock conflict
    DatabaseLocked { database: String, holder_info: String },
}

/// Validation error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    /// Field validation failed
    FieldValidation { field: String, value: String, constraint: String },
    /// Format validation failed
    FormatValidation { field: String, format: String },
    /// Range validation failed
    RangeValidation { field: String, value: String, min: String, max: String },
    /// Custom validation failed
    CustomValidation { rule: String, message: String },
}

/// Transaction-specific error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionError {
    // Validation Errors
    InvalidAddress { address: String, reason: String },
    InvalidAmount { amount: u64, min_required: u64 },
    InsufficientFunds { available: u64, required: u64, fee: u64 },
    DustOutput { amount: u64, dust_limit: u64 },
    ValidationFailed { reason: String },

    // UTXO Errors
    UTXOLocked { txid: String, vout: u32, locked_by: String },
    UTXONotFound { txid: String, vout: u32 },
    NoUTXOsAvailable { wallet_id: String },
    ReservationExpired { token: String },

    // Signing Errors
    KeyNotFound { address: String },
    SeedMissing { wallet_id: String, key_index: u32 },
    SignatureFailed { input_index: u32, reason: String },
    WalletLocked { wallet_id: String },
    InvalidPassword,

    // Network Errors
    NodeUnavailable { url: String, error: String },
    BroadcastFailed { tx_id: String, reason: String },
    MempoolFull,
    FeeTooLow { provided: u64, minimum: u64 },

    // System Errors
    StorageError { operation: String, error: String },
    TimeoutError { operation: String, timeout_ms: u64 },
    CorruptionError { file: String, error: String },

    // Transaction State Errors
    TransactionNotFound { tx_id: String },
    InvalidTransactionState { tx_id: String, current_state: String, expected_state: String },
    TransactionAlreadyBroadcast { tx_id: String },
}

impl fmt::Display for BtpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BtpcError::Security(err) => write!(f, "Security error: {}", err),
            BtpcError::Integration(err) => write!(f, "Integration error: {}", err),
            BtpcError::Utxo(err) => write!(f, "UTXO error: {}", err),
            BtpcError::Transaction(err) => write!(f, "Transaction error: {}", err),
            BtpcError::FileSystem(err) => write!(f, "File system error: {}", err),
            BtpcError::Network(err) => write!(f, "Network error: {}", err),
            BtpcError::Configuration(err) => write!(f, "Configuration error: {}", err),
            BtpcError::Process(err) => write!(f, "Process error: {}", err),
            BtpcError::Validation(err) => write!(f, "Validation error: {}", err),
            BtpcError::Application(msg) => write!(f, "Application error: {}", msg),
        }
    }
}

impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityError::AuthenticationFailed { username, reason } => {
                write!(f, "Authentication failed for user '{}': {}", username, reason)
            }
            SecurityError::AccountLocked { username, locked_until } => {
                write!(f, "Account '{}' is locked until timestamp {}", username, locked_until)
            }
            SecurityError::SessionExpired { session_id } => {
                write!(f, "Session '{}' has expired", session_id)
            }
            SecurityError::InvalidCredentials => write!(f, "Invalid credentials provided"),
            SecurityError::EncryptionFailed { operation } => {
                write!(f, "Encryption failed for operation: {}", operation)
            }
            SecurityError::DecryptionFailed { operation } => {
                write!(f, "Decryption failed for operation: {}", operation)
            }
            SecurityError::KeyDerivationFailed => write!(f, "Key derivation failed"),
            SecurityError::InvalidRecoveryPhrase => write!(f, "Invalid recovery phrase"),
            SecurityError::UserAlreadyExists { username } => {
                write!(f, "User '{}' already exists", username)
            }
            SecurityError::UserNotFound { username } => {
                write!(f, "User '{}' not found", username)
            }
            SecurityError::PasswordRequirementsNotMet { requirements } => {
                write!(f, "Password does not meet requirements: {}", requirements)
            }
        }
    }
}

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntegrationError::BinaryNotFound { binary_name, expected_path } => {
                write!(f, "Binary '{}' not found at expected path: {}", binary_name, expected_path)
            }
            IntegrationError::ExecutionFailed { binary_name, error } => {
                write!(f, "Execution of '{}' failed: {}", binary_name, error)
            }
            IntegrationError::InvalidOutput { binary_name, output } => {
                write!(f, "Invalid output from '{}': {}", binary_name, output)
            }
            IntegrationError::ExecutionTimeout { binary_name, timeout_secs } => {
                write!(f, "Execution of '{}' timed out after {} seconds", binary_name, timeout_secs)
            }
            IntegrationError::IncompatibleVersion { binary_name, version, required } => {
                write!(f, "Binary '{}' version {} is incompatible (required: {})", binary_name, version, required)
            }
            IntegrationError::MissingBtpcHome { path } => {
                write!(f, "BTPC home directory not found: {}", path)
            }
            IntegrationError::InvalidConfiguration { field, value } => {
                write!(f, "Invalid configuration for field '{}': {}", field, value)
            }
        }
    }
}

impl fmt::Display for UtxoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UtxoError::InvalidUtxo { txid, reason } => {
                write!(f, "Invalid UTXO {}: {}", txid, reason)
            }
            UtxoError::UtxoNotFound { txid, vout } => {
                write!(f, "UTXO not found: {}:{}", txid, vout)
            }
            UtxoError::InsufficientFunds { available, required } => {
                write!(f, "Insufficient funds: available {} credits, required {} credits", available, required)
            }
            UtxoError::InvalidAddress { address } => {
                write!(f, "Invalid address format: {}", address)
            }
            UtxoError::DatabaseError { operation, error } => {
                write!(f, "Database error during '{}': {}", operation, error)
            }
            UtxoError::SerializationError { data_type } => {
                write!(f, "Failed to serialize {}", data_type)
            }
            UtxoError::DeserializationError { data_type, error } => {
                write!(f, "Failed to deserialize {}: {}", data_type, error)
            }
        }
    }
}

impl fmt::Display for FileSystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileSystemError::FileNotFound { path } => {
                write!(f, "File not found: {}", path)
            }
            FileSystemError::PermissionDenied { path, operation } => {
                write!(f, "Permission denied for {} on path: {}", operation, path)
            }
            FileSystemError::DirectoryCreationFailed { path } => {
                write!(f, "Failed to create directory: {}", path)
            }
            FileSystemError::ReadFailed { path, error } => {
                write!(f, "Failed to read file '{}': {}", path, error)
            }
            FileSystemError::WriteFailed { path, error } => {
                write!(f, "Failed to write file '{}': {}", path, error)
            }
            FileSystemError::InvalidFormat { path, expected_format } => {
                write!(f, "Invalid file format for '{}', expected: {}", path, expected_format)
            }
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::ConnectionFailed { endpoint, error } => {
                write!(f, "Connection to '{}' failed: {}", endpoint, error)
            }
            NetworkError::RequestTimeout { endpoint, timeout_ms } => {
                write!(f, "Request to '{}' timed out after {} ms", endpoint, timeout_ms)
            }
            NetworkError::InvalidResponse { endpoint, status } => {
                write!(f, "Invalid response from '{}': HTTP {}", endpoint, status)
            }
            NetworkError::NetworkUnreachable => {
                write!(f, "Network is unreachable")
            }
            NetworkError::DnsResolutionFailed { hostname } => {
                write!(f, "DNS resolution failed for hostname: {}", hostname)
            }
            NetworkError::TlsHandshakeFailed { endpoint } => {
                write!(f, "TLS handshake failed with endpoint: {}", endpoint)
            }
        }
    }
}

impl fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigurationError::MissingField { field, config_file } => {
                write!(f, "Missing required field '{}' in config file: {}", field, config_file)
            }
            ConfigurationError::InvalidValue { field, value, expected } => {
                write!(f, "Invalid value '{}' for field '{}', expected: {}", value, field, expected)
            }
            ConfigurationError::ConfigFileNotFound { path } => {
                write!(f, "Configuration file not found: {}", path)
            }
            ConfigurationError::ParsingFailed { path, error } => {
                write!(f, "Failed to parse config file '{}': {}", path, error)
            }
            ConfigurationError::ValidationFailed { field, constraint } => {
                write!(f, "Validation failed for field '{}': {}", field, constraint)
            }
        }
    }
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessError::StartFailed { command, error } => {
                write!(f, "Failed to start process '{}': {}", command, error)
            }
            ProcessError::ProcessCrashed { pid, exit_code } => {
                write!(f, "Process {} crashed with exit code: {:?}", pid, exit_code)
            }
            ProcessError::StopFailed { pid, error } => {
                write!(f, "Failed to stop process {}: {}", pid, error)
            }
            ProcessError::ProcessNotFound { pid } => {
                write!(f, "Process not found: {}", pid)
            }
            ProcessError::ResourceExhaustion { resource, limit } => {
                write!(f, "Resource exhaustion for '{}': {}", resource, limit)
            }
            ProcessError::MutexPoisoned { component, operation } => {
                write!(f, "Internal error: {} mutex poisoned during {}", component, operation)
            }
            ProcessError::DatabaseLocked { database, holder_info } => {
                write!(f, "Database '{}' is locked by {}", database, holder_info)
            }
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::FieldValidation { field, value, constraint } => {
                write!(f, "Field '{}' with value '{}' violates constraint: {}", field, value, constraint)
            }
            ValidationError::FormatValidation { field, format } => {
                write!(f, "Field '{}' has invalid format, expected: {}", field, format)
            }
            ValidationError::RangeValidation { field, value, min, max } => {
                write!(f, "Field '{}' value '{}' is out of range [{}, {}]", field, value, min, max)
            }
            ValidationError::CustomValidation { rule, message } => {
                write!(f, "Validation rule '{}' failed: {}", rule, message)
            }
        }
    }
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Validation Errors
            TransactionError::InvalidAddress { address, reason } => {
                write!(f, "Invalid BTPC address '{}': {}", address, reason)
            }
            TransactionError::InvalidAmount { amount, min_required } => {
                write!(f, "Invalid amount {} satoshis (minimum: {})", amount, min_required)
            }
            TransactionError::InsufficientFunds { available, required, fee } => {
                write!(
                    f,
                    "Insufficient funds. Available: {} BTPC, Required: {} BTPC + {} fee",
                    *available as f64 / 100_000_000.0,
                    *required as f64 / 100_000_000.0,
                    *fee as f64 / 100_000_000.0
                )
            }
            TransactionError::DustOutput { amount, dust_limit } => {
                write!(f, "Output {} is below dust limit of {} satoshis", amount, dust_limit)
            }
            TransactionError::ValidationFailed { reason } => {
                write!(f, "Transaction validation failed: {}", reason)
            }

            // UTXO Errors
            TransactionError::UTXOLocked { txid, vout, locked_by } => {
                write!(f, "UTXO {}:{} is locked by transaction {}", txid, vout, locked_by)
            }
            TransactionError::UTXONotFound { txid, vout } => {
                write!(f, "UTXO {}:{} not found", txid, vout)
            }
            TransactionError::NoUTXOsAvailable { wallet_id } => {
                write!(f, "No UTXOs available for wallet {}", wallet_id)
            }
            TransactionError::ReservationExpired { token } => {
                write!(f, "UTXO reservation {} has expired", token)
            }

            // Signing Errors
            TransactionError::KeyNotFound { address } => {
                write!(f, "Private key not found for address {}", address)
            }
            TransactionError::SeedMissing { wallet_id, key_index } => {
                write!(f, "Seed missing for wallet {} key index {}. Restore from backup.", wallet_id, key_index)
            }
            TransactionError::SignatureFailed { input_index, reason } => {
                write!(f, "Failed to sign input {}: {}", input_index, reason)
            }
            TransactionError::WalletLocked { wallet_id } => {
                write!(f, "Wallet {} is locked. Enter password to unlock.", wallet_id)
            }
            TransactionError::InvalidPassword => {
                write!(f, "Invalid password. Please try again.")
            }

            // Network Errors
            TransactionError::NodeUnavailable { url, error } => {
                write!(f, "Cannot connect to node at {}: {}", url, error)
            }
            TransactionError::BroadcastFailed { tx_id, reason } => {
                write!(f, "Failed to broadcast transaction {}: {}", tx_id, reason)
            }
            TransactionError::MempoolFull => {
                write!(f, "Mempool is full. Transaction queued for later broadcast.")
            }
            TransactionError::FeeTooLow { provided, minimum } => {
                write!(f, "Fee too low. Provided: {} sat/byte, Minimum: {} sat/byte", provided, minimum)
            }

            // System Errors
            TransactionError::StorageError { operation, error } => {
                write!(f, "Storage error during {}: {}", operation, error)
            }
            TransactionError::TimeoutError { operation, timeout_ms } => {
                write!(f, "Operation '{}' timed out after {}ms", operation, timeout_ms)
            }
            TransactionError::CorruptionError { file, error } => {
                write!(f, "File '{}' is corrupted: {}", file, error)
            }

            // Transaction State Errors
            TransactionError::TransactionNotFound { tx_id } => {
                write!(f, "Transaction {} not found", tx_id)
            }
            TransactionError::InvalidTransactionState { tx_id, current_state, expected_state } => {
                write!(
                    f,
                    "Transaction {} in invalid state '{}', expected '{}'",
                    tx_id, current_state, expected_state
                )
            }
            TransactionError::TransactionAlreadyBroadcast { tx_id } => {
                write!(f, "Transaction {} has already been broadcast", tx_id)
            }
        }
    }
}

impl std::error::Error for BtpcError {}

/// Result type alias for BTPC operations
pub type BtpcResult<T> = Result<T, BtpcError>;

/// Error conversion traits for seamless error handling
impl From<std::io::Error> for BtpcError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => {
                BtpcError::FileSystem(FileSystemError::FileNotFound {
                    path: "unknown".to_string(),
                })
            }
            std::io::ErrorKind::PermissionDenied => {
                BtpcError::FileSystem(FileSystemError::PermissionDenied {
                    path: "unknown".to_string(),
                    operation: "unknown".to_string(),
                })
            }
            _ => BtpcError::Application(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for BtpcError {
    fn from(_err: serde_json::Error) -> Self {
        BtpcError::Utxo(UtxoError::SerializationError {
            data_type: "JSON".to_string(),
        })
    }
}

impl From<anyhow::Error> for BtpcError {
    fn from(err: anyhow::Error) -> Self {
        BtpcError::Application(err.to_string())
    }
}

// Convert anyhow::Error to TransactionError
impl From<anyhow::Error> for TransactionError {
    fn from(err: anyhow::Error) -> Self {
        TransactionError::ValidationFailed {
            reason: err.to_string(),
        }
    }
}

/// Helper macros for error creation
#[macro_export]
macro_rules! security_error {
    ($variant:ident { $($field:ident: $value:expr),* }) => {
        BtpcError::Security(SecurityError::$variant { $($field: $value),* })
    };
}

#[macro_export]
macro_rules! integration_error {
    ($variant:ident { $($field:ident: $value:expr),* }) => {
        BtpcError::Integration(IntegrationError::$variant { $($field: $value),* })
    };
}

#[macro_export]
macro_rules! utxo_error {
    ($variant:ident { $($field:ident: $value:expr),* }) => {
        BtpcError::Utxo(UtxoError::$variant { $($field: $value),* })
    };
}

/// Error sanitization for security (NFR-003, NFR-004)
pub trait ErrorSanitization {
    /// Get user-friendly message (no sensitive data)
    fn user_message(&self) -> String;

    /// Get technical details (sanitized for logging/debugging)
    fn technical_details(&self) -> Option<String>;

    /// Check if error contains sensitive information
    fn contains_sensitive_data(&self) -> bool;
}

impl ErrorSanitization for BtpcError {
    fn user_message(&self) -> String {
        match self {
            BtpcError::Security(_) => "Authentication or security error occurred".to_string(),
            BtpcError::Process(ProcessError::MutexPoisoned { component, .. }) => {
                format!("Internal error in {}. Please restart the application", component)
            }
            BtpcError::Process(ProcessError::DatabaseLocked { .. }) => {
                "Database is currently locked. Please try again in a moment".to_string()
            }
            _ => self.to_string(),
        }
    }

    fn technical_details(&self) -> Option<String> {
        // Sanitize paths and sensitive information
        let details = format!("{:?}", self);

        // Remove potential private key patterns
        let sanitized = details
            .replace(|c: char| c.is_ascii_hexdigit() && details.len() > 64, "X");

        Some(sanitized)
    }

    fn contains_sensitive_data(&self) -> bool {
        matches!(self, BtpcError::Security(_))
    }
}

/// Error recovery strategies
pub trait ErrorRecovery {
    /// Attempt to recover from the error
    fn recover(&self) -> BtpcResult<()>;

    /// Check if the error is recoverable
    fn is_recoverable(&self) -> bool;

    /// Get suggested user action
    fn user_action(&self) -> Option<String>;
}

impl ErrorRecovery for BtpcError {
    fn recover(&self) -> BtpcResult<()> {
        match self {
            BtpcError::Network(_) => {
                // Network errors might be temporary, suggest retry
                Err(BtpcError::Application("Retry the operation".to_string()))
            }
            BtpcError::Process(_) => {
                // Process errors might require restart
                Err(BtpcError::Application("Restart the component".to_string()))
            }
            _ => Err(BtpcError::Application("Manual intervention required".to_string())),
        }
    }

    fn is_recoverable(&self) -> bool {
        matches!(
            self,
            BtpcError::Network(_) | BtpcError::Process(_) | BtpcError::Integration(_)
        )
    }

    fn user_action(&self) -> Option<String> {
        match self {
            BtpcError::Security(SecurityError::AccountLocked { .. }) => {
                Some("Wait for the lockout period to expire or use recovery phrase".to_string())
            }
            BtpcError::Integration(IntegrationError::BinaryNotFound { .. }) => {
                Some("Ensure BTPC binaries are installed and accessible".to_string())
            }
            BtpcError::Network(_) => {
                Some("Check your internet connection and try again".to_string())
            }
            BtpcError::FileSystem(FileSystemError::PermissionDenied { .. }) => {
                Some("Check file permissions and run with appropriate privileges".to_string())
            }
            BtpcError::Process(ProcessError::MutexPoisoned { .. }) => {
                Some("Restart the application to recover from this internal error".to_string())
            }
            BtpcError::Process(ProcessError::DatabaseLocked { .. }) => {
                Some("Wait a moment and try again, or restart the application".to_string())
            }
            _ => None,
        }
    }
}

/// Helper functions for creating common error types
impl BtpcError {
    /// Create a mutex poison error
    pub fn mutex_poison(component: impl Into<String>, operation: impl Into<String>) -> Self {
        BtpcError::Process(ProcessError::MutexPoisoned {
            component: component.into(),
            operation: operation.into(),
        })
    }

    /// Create a database locked error
    pub fn database_locked(database: impl Into<String>, holder_info: impl Into<String>) -> Self {
        BtpcError::Process(ProcessError::DatabaseLocked {
            database: database.into(),
            holder_info: holder_info.into(),
        })
    }

    /// Create a process crash error
    pub fn process_crashed(pid: u32, exit_code: Option<i32>) -> Self {
        BtpcError::Process(ProcessError::ProcessCrashed { pid, exit_code })
    }
}