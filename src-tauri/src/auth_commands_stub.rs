//! Test stubs for authentication commands
//!
//! These stubs provide test-compatible versions of auth commands
//! without requiring Tauri's AppState.

use serde::{Deserialize, Serialize};

/// Response for login command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub session_id: String,
    pub expires_at: String,
}

/// Response for session validation (alias for tests)
pub type CheckSessionResponse = SessionInfo;

/// Response for session validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub valid: bool,
    pub session_id: String,
    pub created_at: String,
    pub expires_at: String,
}

/// Response for create password
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePasswordResponse {
    pub success: bool,
    pub message: String,
}

/// Response for has master password check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasMasterPasswordResponse {
    pub exists: bool,
}

/// Response for logout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutResponse {
    pub success: bool,
}

/// Login with master password (test stub)
pub async fn login(_password: String) -> Result<LoginResponse, String> {
    Ok(LoginResponse {
        success: true,
        session_id: uuid::Uuid::new_v4().to_string(),
        expires_at: (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339(),
    })
}

/// Check if session is valid (test stub)
pub async fn check_session() -> Result<CheckSessionResponse, String> {
    Ok(SessionInfo {
        valid: true,
        session_id: uuid::Uuid::new_v4().to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339(),
    })
}

/// Logout (test stub)
pub async fn logout() -> Result<LogoutResponse, String> {
    Ok(LogoutResponse { success: true })
}

/// Create master password (test stub)
pub async fn create_master_password(_password: String) -> Result<CreatePasswordResponse, String> {
    Ok(CreatePasswordResponse {
        success: true,
        message: "Master password created".to_string(),
    })
}

/// Check if master password exists (test stub)
///
/// This checks if the credentials file actually exists using the real auth_state path.
pub async fn has_master_password() -> Result<HasMasterPasswordResponse, String> {
    use crate::auth_state::{get_credentials_path, MasterCredentials};
    let path = get_credentials_path();
    let exists = MasterCredentials::exists(&path);
    Ok(HasMasterPasswordResponse { exists })
}