//! JSON-RPC server for BTPC
//!
//! Provides Bitcoin-compatible JSON-RPC interface for blockchain operations.

pub mod handlers;
pub mod integrated_handlers;
pub mod methods;
pub mod server;
pub mod types;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    /// JSON-RPC version (should be "2.0")
    pub jsonrpc: String,
    /// Method name
    pub method: String,
    /// Parameters (can be array or object)
    pub params: Option<serde_json::Value>,
    /// Request ID
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Result (present on success)
    pub result: Option<serde_json::Value>,
    /// Error (present on failure)
    pub error: Option<RpcError>,
    /// Request ID
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional data
    pub data: Option<serde_json::Value>,
}

impl RpcError {
    /// Create a new RPC error
    pub fn new(code: i32, message: String) -> Self {
        RpcError {
            code,
            message,
            data: None,
        }
    }

    /// Parse error (-32700)
    pub fn parse_error() -> Self {
        RpcError::new(-32700, "Parse error".to_string())
    }

    /// Invalid request (-32600)
    pub fn invalid_request() -> Self {
        RpcError::new(-32600, "Invalid Request".to_string())
    }

    /// Method not found (-32601)
    pub fn method_not_found() -> Self {
        RpcError::new(-32601, "Method not found".to_string())
    }

    /// Invalid parameters (-32602)
    pub fn invalid_params() -> Self {
        RpcError::new(-32602, "Invalid params".to_string())
    }

    /// Internal error (-32603)
    pub fn internal_error() -> Self {
        RpcError::new(-32603, "Internal error".to_string())
    }
}

/// RPC server errors
#[derive(Error, Debug)]
pub enum RpcServerError {
    #[error("JSON parse error: {0}")]
    JsonParse(String),
    #[error("Invalid request format")]
    InvalidRequest,
    #[error("Method not found: {0}")]
    MethodNotFound(String),
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    #[error("Internal server error: {0}")]
    Internal(String),
    #[error("I/O error: {0}")]
    Io(String),
}

impl From<RpcServerError> for RpcError {
    fn from(err: RpcServerError) -> Self {
        match err {
            RpcServerError::JsonParse(_) => RpcError::parse_error(),
            RpcServerError::InvalidRequest => RpcError::invalid_request(),
            RpcServerError::MethodNotFound(_) => RpcError::method_not_found(),
            RpcServerError::InvalidParams(_) => RpcError::invalid_params(),
            RpcServerError::Internal(_) => RpcError::internal_error(),
            RpcServerError::Io(_) => RpcError::internal_error(),
        }
    }
}
