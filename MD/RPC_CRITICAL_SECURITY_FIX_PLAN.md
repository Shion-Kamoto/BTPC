# RPC Module Critical Security Fix Plan

**Document Version:** 1.0
**Created:** 2025-10-12
**Status:** PENDING IMPLEMENTATION
**Priority:** CRITICAL - BLOCKING FOR PRODUCTION

---

## ⚠️ CONSTITUTIONAL COMPLIANCE CHECKPOINT

**MANDATORY REVIEW BEFORE ANY IMPLEMENTATION:**

Before implementing ANY changes in this plan, ALL developers MUST:

1. **Read Constitution.md** (`.specify/memory/constitution.md` Version 1.0.1)
2. **Verify Compliance** with the following articles:
   - **Article I, Section 1.1**: Security-First Principle
     - "Every feature MUST prioritize quantum-resistant security"
     - "No hardcoded secrets or credential exposure"
     - "Constant-time operations for all security-critical code"
   - **Article III**: Test-Driven Development (NON-NEGOTIABLE)
     - "Tests written → Tests fail → Implementation → Tests pass"
     - "Maintain >90% test coverage for new code"
   - **Article V, Section 5.1**: RPC Interface Requirements
     - "JSON-RPC API for wallet and application integration"
     - "Backward-compatible RPC protocols"
   - **Article V, Section 6.2**: Production Readiness
     - "Structured logging with tracing support"
     - "Configurable via TOML files"
     - "Graceful degradation and error recovery"
   - **Article VII, Section 7.1**: Governance
     - "All PRs must verify constitutional compliance"
     - "Security violations are blocking"

3. **Document Constitutional Alignment** in all PR descriptions
4. **No exceptions** - Security violations will be rejected

---

## Executive Summary

This plan addresses **5 CRITICAL security vulnerabilities** in the BTPC RPC module identified during comprehensive security audit. These vulnerabilities create an **immediate security risk** and MUST be resolved before production deployment.

**Current Risk Level:** CRITICAL 🔴
**Target Risk Level:** LOW ✅
**Estimated Timeline:** 2-3 weeks
**Required Resources:** 1-2 senior developers with security expertise

**Critical Issues Summary:**
1. Authentication Bypass (no credential verification)
2. No TLS/SSL Encryption (plaintext communication)
3. Unbounded Request Processing (DoS vulnerability)
4. Plaintext Password Storage (memory exposure)
5. Unvalidated Block Submission (blockchain corruption risk)

---

## Table of Contents

1. [Issue #1: Authentication Bypass](#issue-1-authentication-bypass)
2. [Issue #2: No TLS/SSL Encryption](#issue-2-no-tlsssl-encryption)
3. [Issue #3: Unbounded Request Processing](#issue-3-unbounded-request-processing)
4. [Issue #4: Plaintext Password Storage](#issue-4-plaintext-password-storage)
5. [Issue #5: Unvalidated Block Submission](#issue-5-unvalidated-block-submission)
6. [Implementation Timeline](#implementation-timeline)
7. [Testing Requirements](#testing-requirements)
8. [Constitutional Compliance Checklist](#constitutional-compliance-checklist)

---

## Issue #1: Authentication Bypass

### Problem Description

**Severity:** CRITICAL 🔴
**File:** `btpc-core/src/rpc/server.rs:73-128`
**CVSS Score:** 10.0 (Critical)

The RPC server has authentication configuration (`enable_auth`, `username`, `password`) but **never validates credentials** in the request processing pipeline. The `process_request()` and `handle_request()` methods process all requests without checking authentication.

**Security Impact:**
- Complete unauthorized access to all RPC methods
- Attackers can read blockchain data, submit blocks, send transactions
- No access control whatsoever despite auth configuration existing
- False sense of security from configuration presence

**Constitutional Violation:**
- Article I: "No hardcoded secrets or credential exposure" - Current implementation doesn't protect credentials
- Article I: "Constant-time operations for all security-critical code" - No auth verification implemented

### Detailed Fix Plan

#### Step 1: Add HTTP Basic Auth Parsing

**File:** `btpc-core/src/rpc/server.rs`
**Location:** Add new helper methods

```rust
use base64::{Engine as _, engine::general_purpose};
use subtle::ConstantTimeEq;

impl RpcServer {
    /// Parse HTTP Basic Authentication header
    /// Returns (username, password) if valid format
    fn parse_basic_auth(auth_header: &str) -> Result<(String, String), RpcServerError> {
        // Remove "Basic " prefix
        let auth_header = auth_header.trim_start_matches("Basic ").trim();

        // Decode base64
        let decoded = general_purpose::STANDARD
            .decode(auth_header)
            .map_err(|_| RpcServerError::InvalidRequest("Invalid auth header".to_string()))?;

        let auth_str = String::from_utf8(decoded)
            .map_err(|_| RpcServerError::InvalidRequest("Invalid auth encoding".to_string()))?;

        // Split on ':'
        let parts: Vec<&str> = auth_str.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(RpcServerError::InvalidRequest("Invalid auth format".to_string()));
        }

        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Verify authentication credentials using constant-time comparison
    /// CONSTITUTIONAL REQUIREMENT: Article I - Constant-time operations
    fn verify_auth(&self, provided_username: &str, provided_password: &str) -> bool {
        let config_username = match &self.config.username {
            Some(u) => u,
            None => return false,
        };

        let config_password = match &self.config.password {
            Some(p) => p,
            None => return false,
        };

        // Constant-time comparison to prevent timing attacks
        let username_match = provided_username.as_bytes().ct_eq(config_username.as_bytes());
        let password_match = provided_password.as_bytes().ct_eq(config_password.as_bytes());

        bool::from(username_match & password_match)
    }
}
```

**Dependencies Required:**
```toml
# Add to btpc-core/Cargo.toml
[dependencies]
base64 = "0.21"
subtle = "2.5"  # For constant-time comparison
```

#### Step 2: Modify Request Handler to Enforce Authentication

**File:** `btpc-core/src/rpc/server.rs:145-188`
**Location:** Connection handling loop

```rust
async fn handle_connection(
    stream: TcpStream,
    server: Arc<RpcServer>,
) {
    let mut stream = stream;
    let mut buffer = vec![0; server.config.max_request_size];

    match stream.read(&mut buffer).await {
        Ok(size) if size > 0 => {
            // Parse HTTP request
            let request_str = String::from_utf8_lossy(&buffer[..size]);

            // Extract Authorization header
            let auth_header = request_str
                .lines()
                .find(|line| line.to_lowercase().starts_with("authorization:"))
                .map(|line| line.split_once(':').map(|(_, value)| value.trim()))
                .flatten();

            // ENFORCE AUTHENTICATION (CRITICAL FIX)
            if server.config.enable_auth {
                match auth_header {
                    Some(header) => {
                        // Parse and verify credentials
                        match RpcServer::parse_basic_auth(header) {
                            Ok((username, password)) => {
                                if !server.verify_auth(&username, &password) {
                                    let error_response = r#"{"jsonrpc":"2.0","error":{"code":-32001,"message":"Invalid credentials"},"id":null}"#;
                                    let http_response = format!(
                                        "HTTP/1.1 401 Unauthorized\r\n\
                                         WWW-Authenticate: Basic realm=\"BTPC RPC\"\r\n\
                                         Content-Type: application/json\r\n\
                                         Content-Length: {}\r\n\
                                         \r\n{}",
                                        error_response.len(),
                                        error_response
                                    );
                                    let _ = stream.write_all(http_response.as_bytes()).await;
                                    return;
                                }
                            }
                            Err(_) => {
                                let error_response = r#"{"jsonrpc":"2.0","error":{"code":-32002,"message":"Malformed auth header"},"id":null}"#;
                                let http_response = format!(
                                    "HTTP/1.1 400 Bad Request\r\n\
                                     Content-Type: application/json\r\n\
                                     Content-Length: {}\r\n\
                                     \r\n{}",
                                    error_response.len(),
                                    error_response
                                );
                                let _ = stream.write_all(http_response.as_bytes()).await;
                                return;
                            }
                        }
                    }
                    None => {
                        let error_response = r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"Authentication required"},"id":null}"#;
                        let http_response = format!(
                            "HTTP/1.1 401 Unauthorized\r\n\
                             WWW-Authenticate: Basic realm=\"BTPC RPC\"\r\n\
                             Content-Type: application/json\r\n\
                             Content-Length: {}\r\n\
                             \r\n{}",
                            error_response.len(),
                            error_response
                        );
                        let _ = stream.write_all(http_response.as_bytes()).await;
                        return;
                    }
                }
            }

            // Authentication passed or disabled - continue with request processing
            // Extract JSON body from HTTP request
            // ... existing code ...
        }
        Ok(_) => return,
        Err(e) => {
            eprintln!("Read error: {}", e);
            return;
        }
    }
}
```

#### Step 3: Update Default Configuration

**File:** `btpc-core/src/rpc/server.rs:40-50`

```rust
impl Default for RpcConfig {
    fn default() -> Self {
        RpcConfig {
            bind_address: "127.0.0.1".to_string(),  // Localhost only by default
            port: 8432,  // Use BTPC-specific port (not Bitcoin's 8332)
            max_request_size: 1024 * 1024,
            enable_auth: true,  // FORCE authentication ON by default
            username: None,     // Must be set before server starts
            password: None,     // Must be set before server starts
        }
    }
}

impl RpcServer {
    /// Create new RPC server with validation
    pub fn new(config: RpcConfig) -> Result<Self, RpcServerError> {
        // VALIDATE: If auth enabled, credentials MUST be provided
        if config.enable_auth && (config.username.is_none() || config.password.is_none()) {
            return Err(RpcServerError::InvalidRequest(
                "Authentication enabled but credentials not provided. \
                 Set RpcConfig.username and RpcConfig.password or disable auth \
                 (NOT recommended for production)".to_string()
            ));
        }

        // VALIDATE: Credentials must meet minimum strength requirements
        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            if username.len() < 8 {
                return Err(RpcServerError::InvalidRequest(
                    "Username must be at least 8 characters".to_string()
                ));
            }
            if password.len() < 16 {
                return Err(RpcServerError::InvalidRequest(
                    "Password must be at least 16 characters".to_string()
                ));
            }
        }

        Ok(RpcServer {
            config,
            methods: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}
```

#### Step 4: Add Configuration via TOML

**Constitutional Requirement:** Article V - "Configurable via TOML files"

**File:** Create `btpc-node/config/rpc.toml.example`

```toml
# BTPC RPC Server Configuration
# Constitutional Compliance: Article V - Production Readiness

[rpc]
# Bind address (default: localhost only for security)
bind_address = "127.0.0.1"

# Port number (8432 = BTPC standard, avoid Bitcoin's 8332)
port = 8432

# Maximum request size in bytes
max_request_size = 1048576  # 1MB

# Authentication settings (REQUIRED for production)
enable_auth = true

# Credentials (NEVER commit real credentials to git)
# Generate strong credentials: openssl rand -base64 32
username = "btpc_admin"
password = "CHANGE_THIS_STRONG_PASSWORD_MIN_16_CHARS"

# Security warning
# WARNING: If you disable authentication, ONLY bind to 127.0.0.1
# NEVER expose unauthenticated RPC to public networks
```

**File:** Update `btpc-node/src/main.rs` to load config

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct NodeConfig {
    rpc: RpcConfigToml,
}

#[derive(Deserialize)]
struct RpcConfigToml {
    bind_address: String,
    port: u16,
    max_request_size: usize,
    enable_auth: bool,
    username: Option<String>,
    password: Option<String>,
}

fn load_rpc_config() -> Result<RpcConfig, Box<dyn std::error::Error>> {
    let config_path = std::env::var("BTPC_CONFIG")
        .unwrap_or_else(|_| "config/rpc.toml".to_string());

    let config_str = std::fs::read_to_string(&config_path)?;
    let node_config: NodeConfig = toml::from_str(&config_str)?;

    Ok(RpcConfig {
        bind_address: node_config.rpc.bind_address,
        port: node_config.rpc.port,
        max_request_size: node_config.rpc.max_request_size,
        enable_auth: node_config.rpc.enable_auth,
        username: node_config.rpc.username,
        password: node_config.rpc.password,
    })
}
```

### Testing Requirements (TDD - Article III)

**Constitutional Mandate:** "Tests written → Tests fail → Implementation → Tests pass"

**File:** `btpc-core/src/rpc/tests/auth_tests.rs`

```rust
#[cfg(test)]
mod auth_tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_auth_required_when_enabled() {
        // 1. SETUP: Create server with auth enabled
        let config = RpcConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 0,  // Random port
            max_request_size: 1024,
            enable_auth: true,
            username: Some("testuser".to_string()),
            password: Some("testpassword1234".to_string()),
        };

        let server = RpcServer::new(config).unwrap();

        // 2. TEST: Request without auth header should fail
        let request = r#"{"jsonrpc":"2.0","method":"getblockchaininfo","params":[],"id":1}"#;

        // Send request without Authorization header
        // Expected: 401 Unauthorized

        // 3. VERIFY: Error code -32000 "Authentication required"
        // Implementation will make this pass
    }

    #[tokio::test]
    async fn test_auth_accepts_valid_credentials() {
        // Test valid credentials are accepted
        // Expected: Request processed successfully
    }

    #[tokio::test]
    async fn test_auth_rejects_invalid_credentials() {
        // Test invalid credentials rejected
        // Expected: 401 Unauthorized with "Invalid credentials"
    }

    #[tokio::test]
    async fn test_auth_constant_time_comparison() {
        // Test timing attack resistance
        // Measure response time for correct vs incorrect passwords
        // Difference should be < 1ms to prevent timing attacks
    }

    #[tokio::test]
    async fn test_auth_disabled_allows_access() {
        // Test that disable_auth actually disables authentication
        // Expected: Requests processed without credentials
    }

    #[test]
    fn test_server_creation_requires_credentials_when_auth_enabled() {
        // Test RpcServer::new() validation
        let config = RpcConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 8432,
            max_request_size: 1024,
            enable_auth: true,
            username: None,  // Missing username
            password: None,  // Missing password
        };

        let result = RpcServer::new(config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("credentials not provided"));
    }

    #[test]
    fn test_credential_strength_validation() {
        // Test username minimum length
        let config = RpcConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 8432,
            max_request_size: 1024,
            enable_auth: true,
            username: Some("short".to_string()),  // Too short
            password: Some("validpassword123".to_string()),
        };

        let result = RpcServer::new(config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least 8 characters"));

        // Test password minimum length
        // ... similar test for password
    }
}
```

**Test Coverage Target:** >95% for authentication code paths

### Implementation Checklist

- [ ] Write tests for authentication (TDD - write tests FIRST)
- [ ] Add `base64` and `subtle` dependencies
- [ ] Implement `parse_basic_auth()` helper
- [ ] Implement `verify_auth()` with constant-time comparison
- [ ] Modify connection handler to enforce authentication
- [ ] Update `RpcConfig::default()` to enable auth by default
- [ ] Add validation in `RpcServer::new()`
- [ ] Create TOML configuration example
- [ ] Add config loading to btpc-node
- [ ] Run tests - verify all pass
- [ ] Manual testing with curl
- [ ] Update documentation
- [ ] Security review

### Documentation Updates

**File:** `btpc-core/src/rpc/README.md`

```markdown
# BTPC RPC Module

## Security

**CONSTITUTIONAL REQUIREMENT**: Article I - Security-First

The RPC module implements HTTP Basic Authentication for all requests.

### Configuration

Authentication is **ENABLED BY DEFAULT** for security. Configure via `rpc.toml`:

```toml
[rpc]
enable_auth = true
username = "your_username"  # Minimum 8 characters
password = "your_password"  # Minimum 16 characters
```

### Client Usage

```bash
# Using curl
curl -u username:password http://localhost:8432 \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[],"id":1}'
```

### Security Notes

- Credentials transmitted via HTTP Basic Auth (base64 encoded)
- **USE TLS/SSL** for production (see Issue #2 fix)
- Constant-time comparison prevents timing attacks
- Strong password requirements enforced (minimum 16 characters)
```

---

## Issue #2: No TLS/SSL Encryption

### Problem Description

**Severity:** CRITICAL 🔴
**File:** `btpc-core/src/rpc/server.rs:132-189`
**CVSS Score:** 9.8 (Critical)

The RPC server uses raw TCP sockets without TLS encryption. All communication (including passwords, private keys, transaction data) is transmitted in plaintext.

**Security Impact:**
- Credentials can be intercepted via network sniffing
- Transaction data exposed to eavesdroppers
- Man-in-the-middle attacks possible
- Violates basic security principles for financial applications

**Constitutional Violation:**
- Article I: "Every feature MUST prioritize quantum-resistant security"
- Article I: "No hardcoded secrets or credential exposure"

### Detailed Fix Plan

#### Step 1: Add TLS Dependencies

**File:** `btpc-core/Cargo.toml`

```toml
[dependencies]
# Existing dependencies...

# TLS Support
tokio-rustls = "0.25"
rustls = "0.22"
rustls-pemfile = "2.0"
```

#### Step 2: Extend RpcConfig for TLS

**File:** `btpc-core/src/rpc/server.rs`

```rust
#[derive(Debug, Clone)]
pub struct RpcConfig {
    pub bind_address: String,
    pub port: u16,
    pub max_request_size: usize,
    pub enable_auth: bool,
    pub username: Option<String>,
    pub password: Option<String>,

    // TLS Configuration (CRITICAL SECURITY FEATURE)
    pub enable_tls: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

impl Default for RpcConfig {
    fn default() -> Self {
        RpcConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 8432,
            max_request_size: 1024 * 1024,
            enable_auth: true,
            username: None,
            password: None,
            enable_tls: false,  // Disabled by default (localhost only)
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}
```

#### Step 3: Implement TLS Support

**File:** `btpc-core/src/rpc/server.rs`

```rust
use tokio_rustls::{TlsAcceptor, rustls};
use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc as StdArc;

impl RpcServer {
    /// Load TLS configuration from certificate and key files
    fn load_tls_config(&self) -> Result<StdArc<ServerConfig>, RpcServerError> {
        let cert_path = self.config.tls_cert_path.as_ref()
            .ok_or_else(|| RpcServerError::InvalidRequest("TLS cert path not set".to_string()))?;
        let key_path = self.config.tls_key_path.as_ref()
            .ok_or_else(|| RpcServerError::InvalidRequest("TLS key path not set".to_string()))?;

        // Load certificate chain
        let cert_file = File::open(cert_path)
            .map_err(|e| RpcServerError::Internal(format!("Failed to open cert file: {}", e)))?;
        let mut cert_reader = BufReader::new(cert_file);
        let cert_chain = certs(&mut cert_reader)
            .map(|result| result.map(rustls::Certificate))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| RpcServerError::Internal(format!("Failed to parse cert: {}", e)))?;

        // Load private key
        let key_file = File::open(key_path)
            .map_err(|e| RpcServerError::Internal(format!("Failed to open key file: {}", e)))?;
        let mut key_reader = BufReader::new(key_file);
        let mut keys = pkcs8_private_keys(&mut key_reader)
            .map(|result| result.map(rustls::PrivateKey))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| RpcServerError::Internal(format!("Failed to parse key: {}", e)))?;

        if keys.is_empty() {
            return Err(RpcServerError::Internal("No private keys found".to_string()));
        }

        // Build TLS config
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, keys.remove(0))
            .map_err(|e| RpcServerError::Internal(format!("Invalid TLS config: {}", e)))?;

        Ok(StdArc::new(config))
    }

    /// Start RPC server with TLS support
    pub async fn start(&self) -> Result<(), RpcServerError> {
        let addr = format!("{}:{}", self.config.bind_address, self.config.port);
        let listener = TcpListener::bind(&addr).await
            .map_err(|e| RpcServerError::Internal(format!("Failed to bind: {}", e)))?;

        println!("RPC server listening on {}", addr);
        if self.config.enable_tls {
            println!("TLS enabled - secure connections only");
        } else {
            println!("WARNING: TLS disabled - plaintext communication");
        }

        // Load TLS config if enabled
        let tls_acceptor = if self.config.enable_tls {
            let tls_config = self.load_tls_config()?;
            Some(TlsAcceptor::from(tls_config))
        } else {
            None
        };

        let server = Arc::new(self.clone());

        loop {
            let (stream, peer_addr) = listener.accept().await
                .map_err(|e| RpcServerError::Internal(format!("Accept error: {}", e)))?;

            let server_clone = Arc::clone(&server);
            let tls_acceptor_clone = tls_acceptor.clone();

            tokio::spawn(async move {
                // Wrap stream in TLS if enabled
                if let Some(acceptor) = tls_acceptor_clone {
                    match acceptor.accept(stream).await {
                        Ok(tls_stream) => {
                            Self::handle_tls_connection(tls_stream, server_clone).await;
                        }
                        Err(e) => {
                            eprintln!("TLS handshake failed from {}: {}", peer_addr, e);
                        }
                    }
                } else {
                    // Plaintext connection (localhost only recommended)
                    Self::handle_plaintext_connection(stream, server_clone).await;
                }
            });
        }
    }

    async fn handle_tls_connection(
        stream: tokio_rustls::server::TlsStream<TcpStream>,
        server: Arc<RpcServer>,
    ) {
        // Same handling as plaintext but with TLS stream
        // ... connection handling code ...
    }

    async fn handle_plaintext_connection(
        stream: TcpStream,
        server: Arc<RpcServer>,
    ) {
        // Existing connection handling
        // ... connection handling code ...
    }
}
```

#### Step 4: Generate Development Certificates

**File:** Create `scripts/generate_dev_certs.sh`

```bash
#!/bin/bash
# Generate self-signed certificates for development
# Constitutional Compliance: Article I - Security-First

set -e

CERT_DIR="certs/dev"
mkdir -p "$CERT_DIR"

echo "Generating development TLS certificates..."
echo "WARNING: These are for DEVELOPMENT ONLY"
echo "For production, use certificates from a trusted CA"

# Generate private key
openssl genrsa -out "$CERT_DIR/server.key" 2048

# Generate certificate signing request
openssl req -new -key "$CERT_DIR/server.key" \
    -out "$CERT_DIR/server.csr" \
    -subj "/C=US/ST=State/L=City/O=BTPC Development/CN=localhost"

# Generate self-signed certificate (valid for 1 year)
openssl x509 -req -days 365 \
    -in "$CERT_DIR/server.csr" \
    -signkey "$CERT_DIR/server.key" \
    -out "$CERT_DIR/server.crt"

# Set secure permissions
chmod 600 "$CERT_DIR/server.key"
chmod 644 "$CERT_DIR/server.crt"

echo "✅ Certificates generated:"
echo "   Private Key: $CERT_DIR/server.key"
echo "   Certificate: $CERT_DIR/server.crt"
echo ""
echo "Add to your rpc.toml:"
echo "enable_tls = true"
echo "tls_cert_path = \"$CERT_DIR/server.crt\""
echo "tls_key_path = \"$CERT_DIR/server.key\""
```

#### Step 5: Update Configuration

**File:** `btpc-node/config/rpc.toml.example`

```toml
[rpc]
bind_address = "127.0.0.1"
port = 8432
max_request_size = 1048576

# Authentication (REQUIRED)
enable_auth = true
username = "btpc_admin"
password = "CHANGE_THIS_STRONG_PASSWORD"

# TLS/SSL Configuration (RECOMMENDED for remote access)
enable_tls = false  # Set to true for remote access

# TLS certificate paths (required if enable_tls = true)
# Generate dev certs: ./scripts/generate_dev_certs.sh
# Production: Use certificates from trusted CA (Let's Encrypt, etc.)
tls_cert_path = "certs/dev/server.crt"
tls_key_path = "certs/dev/server.key"

# Security Warning:
# If enable_tls = false, ONLY bind to 127.0.0.1
# NEVER expose plaintext RPC over public networks
# For remote access, ALWAYS enable TLS
```

### Testing Requirements

**File:** `btpc-core/src/rpc/tests/tls_tests.rs`

```rust
#[cfg(test)]
mod tls_tests {
    use super::*;

    #[tokio::test]
    async fn test_tls_server_starts_with_valid_certs() {
        // Test TLS server initialization
    }

    #[tokio::test]
    async fn test_tls_rejects_plaintext_connections() {
        // Verify TLS-enabled server rejects plaintext
    }

    #[tokio::test]
    async fn test_tls_handshake_success() {
        // Test successful TLS handshake
    }

    #[tokio::test]
    async fn test_tls_fails_with_invalid_certs() {
        // Test server fails to start with invalid certificates
    }

    #[tokio::test]
    async fn test_tls_client_connection() {
        // Test client can connect over TLS
    }
}
```

### Implementation Checklist

- [ ] Add TLS dependencies
- [ ] Extend RpcConfig for TLS settings
- [ ] Implement `load_tls_config()`
- [ ] Implement TLS connection handling
- [ ] Create certificate generation script
- [ ] Update TOML configuration example
- [ ] Write TLS tests
- [ ] Test with self-signed certificates
- [ ] Update documentation
- [ ] Security review

### Documentation

**Production Deployment Note:**

```markdown
## TLS for Production

**CONSTITUTIONAL REQUIREMENT**: Article I - Security-First

For production deployments:

1. **Get certificates from trusted CA** (e.g., Let's Encrypt)
2. **Enable TLS** in configuration
3. **Use strong ciphers** (rustls defaults are secure)
4. **Monitor certificate expiration**

DO NOT use self-signed certificates in production.
```

---

## Issue #3: Unbounded Request Processing

### Problem Description

**Severity:** CRITICAL 🔴
**File:** `btpc-core/src/rpc/server.rs:145-188`
**CVSS Score:** 7.5 (High)

The server spawns unlimited concurrent tasks for incoming connections with no rate limiting, connection limits, or timeout controls.

**Security Impact:**
- Denial of Service via connection flooding
- Memory exhaustion from unlimited concurrent requests
- CPU exhaustion from compute-heavy RPC calls
- No protection against slowloris attacks

**Constitutional Violation:**
- Article V: "Graceful degradation and error recovery"
- Article I: Security-First principle

### Detailed Fix Plan

#### Step 1: Add Rate Limiting Dependencies

**File:** `btpc-core/Cargo.toml`

```toml
[dependencies]
# Existing dependencies...

# Rate limiting and concurrency control
tokio = { version = "1.35", features = ["full", "time"] }
governor = "0.6"  # Token bucket rate limiter
dashmap = "5.5"   # Concurrent hashmap for connection tracking
```

#### Step 2: Extend RpcConfig

**File:** `btpc-core/src/rpc/server.rs`

```rust
#[derive(Debug, Clone)]
pub struct RpcConfig {
    // ... existing fields ...

    // Concurrency and Rate Limiting (DoS PROTECTION)
    pub max_concurrent_requests: usize,
    pub request_timeout_secs: u64,
    pub max_connections_per_ip: usize,
    pub rate_limit_per_ip: u32,  // Requests per minute
    pub rate_limit_window_secs: u64,
}

impl Default for RpcConfig {
    fn default() -> Self {
        RpcConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 8432,
            max_request_size: 1024 * 1024,
            enable_auth: true,
            username: None,
            password: None,
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,

            // Conservative defaults for DoS protection
            max_concurrent_requests: 100,
            request_timeout_secs: 30,
            max_connections_per_ip: 10,
            rate_limit_per_ip: 60,  // 60 requests per minute
            rate_limit_window_secs: 60,
        }
    }
}
```

#### Step 3: Implement Rate Limiting

**File:** `btpc-core/src/rpc/rate_limiter.rs` (new file)

```rust
//! Rate limiting for RPC requests
//! Constitutional Compliance: Article I - Security-First, Article V - Graceful degradation

use dashmap::DashMap;
use governor::{Quota, RateLimiter as GovernorRateLimiter, DefaultDirectRateLimiter};
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

pub struct RpcRateLimiter {
    limiters: Arc<DashMap<IpAddr, DefaultDirectRateLimiter>>,
    quota: Quota,
}

impl RpcRateLimiter {
    /// Create new rate limiter with per-IP limits
    pub fn new(requests_per_minute: u32) -> Self {
        let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap());

        RpcRateLimiter {
            limiters: Arc::new(DashMap::new()),
            quota,
        }
    }

    /// Check if request from IP should be allowed
    /// Returns Ok(()) if allowed, Err if rate limited
    pub fn check_rate_limit(&self, ip: IpAddr) -> Result<(), RateLimitError> {
        // Get or create rate limiter for this IP
        let limiter = self.limiters
            .entry(ip)
            .or_insert_with(|| GovernorRateLimiter::direct(self.quota));

        match limiter.check() {
            Ok(_) => Ok(()),
            Err(_) => Err(RateLimitError::TooManyRequests {
                ip,
                retry_after: Duration::from_secs(1),
            }),
        }
    }

    /// Clean up old entries (run periodically)
    pub fn cleanup_old_entries(&self) {
        self.limiters.retain(|_, limiter| {
            // Keep entries that still have capacity remaining
            limiter.check().is_ok()
        });
    }
}

#[derive(Debug)]
pub enum RateLimitError {
    TooManyRequests {
        ip: IpAddr,
        retry_after: Duration,
    },
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::TooManyRequests { ip, retry_after } => {
                write!(f, "Rate limit exceeded for IP {}, retry after {:?}", ip, retry_after)
            }
        }
    }
}

impl std::error::Error for RateLimitError {}
```

#### Step 4: Implement Connection Tracking

**File:** `btpc-core/src/rpc/connection_tracker.rs` (new file)

```rust
//! Connection tracking for per-IP limits
//! Constitutional Compliance: Article V - Graceful degradation

use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;

pub struct ConnectionTracker {
    connections: Arc<DashMap<IpAddr, usize>>,
    max_per_ip: usize,
}

impl ConnectionTracker {
    pub fn new(max_per_ip: usize) -> Self {
        ConnectionTracker {
            connections: Arc::new(DashMap::new()),
            max_per_ip,
        }
    }

    /// Attempt to acquire connection slot for IP
    pub fn try_acquire(&self, ip: IpAddr) -> Result<ConnectionGuard, ConnectionError> {
        let mut entry = self.connections.entry(ip).or_insert(0);

        if *entry >= self.max_per_ip {
            return Err(ConnectionError::TooManyConnections {
                ip,
                current: *entry,
                max: self.max_per_ip,
            });
        }

        *entry += 1;
        Ok(ConnectionGuard {
            connections: Arc::clone(&self.connections),
            ip,
        })
    }
}

/// RAII guard for connection tracking
pub struct ConnectionGuard {
    connections: Arc<DashMap<IpAddr, usize>>,
    ip: IpAddr,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        // Decrement connection count when guard dropped
        if let Some(mut entry) = self.connections.get_mut(&self.ip) {
            *entry = entry.saturating_sub(1);
            if *entry == 0 {
                drop(entry);
                self.connections.remove(&self.ip);
            }
        }
    }
}

#[derive(Debug)]
pub enum ConnectionError {
    TooManyConnections {
        ip: IpAddr,
        current: usize,
        max: usize,
    },
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionError::TooManyConnections { ip, current, max } => {
                write!(f, "Too many connections from IP {} ({}/{})", ip, current, max)
            }
        }
    }
}

impl std::error::Error for ConnectionError {}
```

#### Step 5: Integrate DoS Protection into Server

**File:** `btpc-core/src/rpc/server.rs`

```rust
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};

pub struct RpcServer {
    config: RpcConfig,
    methods: Arc<RwLock<HashMap<String, RpcMethod>>>,

    // DoS Protection (CRITICAL SECURITY FEATURE)
    concurrency_limiter: Arc<Semaphore>,
    rate_limiter: RpcRateLimiter,
    connection_tracker: ConnectionTracker,
}

impl RpcServer {
    pub fn new(config: RpcConfig) -> Result<Self, RpcServerError> {
        // Validation...

        Ok(RpcServer {
            concurrency_limiter: Arc::new(Semaphore::new(config.max_concurrent_requests)),
            rate_limiter: RpcRateLimiter::new(config.rate_limit_per_ip),
            connection_tracker: ConnectionTracker::new(config.max_connections_per_ip),
            config,
            methods: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn start(&self) -> Result<(), RpcServerError> {
        // ... TLS setup ...

        let server = Arc::new(self.clone());

        // Spawn cleanup task for rate limiter
        let rate_limiter_clone = self.rate_limiter.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                rate_limiter_clone.cleanup_old_entries();
            }
        });

        loop {
            let (stream, peer_addr) = listener.accept().await?;

            // CHECK 1: Per-IP connection limit
            let _connection_guard = match server.connection_tracker.try_acquire(peer_addr.ip()) {
                Ok(guard) => guard,
                Err(e) => {
                    eprintln!("{}", e);
                    continue;  // Drop connection
                }
            };

            // CHECK 2: Rate limit
            if let Err(e) = server.rate_limiter.check_rate_limit(peer_addr.ip()) {
                eprintln!("{}", e);

                // Send 429 Too Many Requests
                let error_response = r#"{"jsonrpc":"2.0","error":{"code":-32005,"message":"Rate limit exceeded"},"id":null}"#;
                let http_response = format!(
                    "HTTP/1.1 429 Too Many Requests\r\n\
                     Retry-After: 1\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: {}\r\n\
                     \r\n{}",
                    error_response.len(),
                    error_response
                );
                let _ = stream.write_all(http_response.as_bytes()).await;
                continue;
            }

            // CHECK 3: Global concurrency limit
            let permit = match server.concurrency_limiter.clone().try_acquire_owned() {
                Ok(permit) => permit,
                Err(_) => {
                    eprintln!("Max concurrent requests reached, dropping connection from {}", peer_addr);

                    let error_response = r#"{"jsonrpc":"2.0","error":{"code":-32006,"message":"Server busy"},"id":null}"#;
                    let http_response = format!(
                        "HTTP/1.1 503 Service Unavailable\r\n\
                         Retry-After: 5\r\n\
                         Content-Type: application/json\r\n\
                         Content-Length: {}\r\n\
                         \r\n{}",
                        error_response.len(),
                        error_response
                    );
                    let _ = stream.write_all(http_response.as_bytes()).await;
                    continue;
                }
            };

            let server_clone = Arc::clone(&server);
            let request_timeout = Duration::from_secs(server.config.request_timeout_secs);

            tokio::spawn(async move {
                let _permit = permit;  // Hold permit for duration of request
                let _guard = _connection_guard;  // Hold connection slot

                // CHECK 4: Request timeout
                match timeout(request_timeout, Self::handle_connection(stream, server_clone)).await {
                    Ok(_) => {},  // Request completed normally
                    Err(_) => {
                        eprintln!("Request timeout from {}", peer_addr);
                    }
                }
            });
        }
    }
}
```

### Testing Requirements

**File:** `btpc-core/src/rpc/tests/dos_protection_tests.rs`

```rust
#[cfg(test)]
mod dos_protection_tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_enforcement() {
        // Send more than rate limit, verify rejection
    }

    #[tokio::test]
    async fn test_concurrent_request_limit() {
        // Spawn max+1 concurrent requests, verify last rejected
    }

    #[tokio::test]
    async fn test_connection_per_ip_limit() {
        // Open max+1 connections from same IP, verify rejection
    }

    #[tokio::test]
    async fn test_request_timeout() {
        // Send slow request, verify timeout after configured duration
    }

    #[tokio::test]
    async fn test_cleanup_rate_limiters() {
        // Verify old rate limiter entries cleaned up
    }
}
```

### Implementation Checklist

- [ ] Add rate limiting dependencies
- [ ] Create `rate_limiter.rs` module
- [ ] Create `connection_tracker.rs` module
- [ ] Integrate into RpcServer
- [ ] Add cleanup task for rate limiters
- [ ] Update configuration
- [ ] Write DoS protection tests
- [ ] Load testing with siege/wrk
- [ ] Documentation
- [ ] Security review

---

## Issue #4: Plaintext Password Storage

### Problem Description

**Severity:** CRITICAL 🔴
**File:** `btpc-core/src/rpc/server.rs:37`
**CVSS Score:** 8.1 (High)

Passwords stored as `Option<String>` in plaintext memory without zeroization or secure memory handling.

**Security Impact:**
- Passwords can leak via memory dumps
- Passwords remain in memory after use
- Vulnerable to memory scraping attacks
- Process core dumps expose credentials

**Constitutional Violation:**
- Article I: "No hardcoded secrets or credential exposure"
- Article I: "All private keys encrypted at rest"

### Detailed Fix Plan

#### Step 1: Add Zeroizing Dependency

**File:** `btpc-core/Cargo.toml`

```toml
[dependencies]
# Existing dependencies...

# Secure memory handling (CONSTITUTIONAL REQUIREMENT)
zeroize = { version = "1.7", features = ["derive"] }
secrecy = "0.8"  # Additional security for sensitive data
```

#### Step 2: Replace String with Zeroizing

**File:** `btpc-core/src/rpc/server.rs`

```rust
use zeroize::{Zeroize, Zeroizing};
use secrecy::{Secret, ExposeSecret};

#[derive(Debug, Clone)]
pub struct RpcConfig {
    pub bind_address: String,
    pub port: u16,
    pub max_request_size: usize,
    pub enable_auth: bool,

    // SECURE CREDENTIAL STORAGE (Article I compliance)
    pub username: Option<Secret<String>>,
    pub password: Option<Secret<String>>,

    // ... other fields ...
}

impl RpcServer {
    fn verify_auth(&self, provided_username: &str, provided_password: &str) -> bool {
        let config_username = match &self.config.username {
            Some(u) => u.expose_secret(),
            None => return false,
        };

        let config_password = match &self.config.password {
            Some(p) => p.expose_secret(),
            None => return false,
        };

        // Constant-time comparison
        let username_match = provided_username.as_bytes().ct_eq(config_username.as_bytes());
        let password_match = provided_password.as_bytes().ct_eq(config_password.as_bytes());

        bool::from(username_match & password_match)
    }
}

// Custom Drop implementation to ensure cleanup
impl Drop for RpcConfig {
    fn drop(&mut self) {
        // Secret<String> automatically zeroizes on drop
        // Additional cleanup if needed
    }
}
```

#### Step 3: Secure Configuration Loading

**File:** `btpc-node/src/config.rs`

```rust
use secrecy::Secret;
use zeroize::Zeroizing;

pub fn load_rpc_credentials() -> Result<(Option<Secret<String>>, Option<Secret<String>>), ConfigError> {
    // Option 1: Environment variables (zeroized after reading)
    let username = std::env::var("BTPC_RPC_USERNAME")
        .ok()
        .map(|s| Secret::new(s));

    let password = std::env::var("BTPC_RPC_PASSWORD")
        .ok()
        .map(|s| Secret::new(s));

    // Option 2: Read from secure config file with restrictive permissions
    // Only if env vars not set
    if username.is_none() || password.is_none() {
        return load_credentials_from_secure_file();
    }

    Ok((username, password))
}

fn load_credentials_from_secure_file() -> Result<(Option<Secret<String>>, Option<Secret<String>>), ConfigError> {
    let credentials_path = "config/rpc_credentials.toml";

    // Verify file permissions (must be 0600 or 0400)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(credentials_path)?;
        let permissions = metadata.permissions();
        let mode = permissions.mode() & 0o777;

        if mode != 0o600 && mode != 0o400 {
            return Err(ConfigError::InsecureFilePermissions {
                path: credentials_path.to_string(),
                mode,
            });
        }
    }

    // Read file content into zeroizing buffer
    let content = Zeroizing::new(std::fs::read_to_string(credentials_path)?);
    let config: RpcCredentials = toml::from_str(&content)?;

    Ok((
        config.username.map(Secret::new),
        config.password.map(Secret::new),
    ))
}

#[derive(Deserialize, Zeroize)]
#[zeroize(drop)]
struct RpcCredentials {
    username: Option<String>,
    password: Option<String>,
}
```

#### Step 4: Add Credential Generation Tool

**File:** `scripts/generate_rpc_credentials.sh`

```bash
#!/bin/bash
# Generate secure RPC credentials
# Constitutional Compliance: Article I - No hardcoded secrets

set -e

CREDS_FILE="config/rpc_credentials.toml"

# Generate strong random credentials
USERNAME=$(openssl rand -base64 24 | tr -d '/+=' | cut -c1-16)
PASSWORD=$(openssl rand -base64 48 | tr -d '/+=' | cut -c1-32)

# Create credentials file with secure permissions
umask 077  # Ensure file created with 0600 permissions

cat > "$CREDS_FILE" << EOF
# BTPC RPC Credentials
# GENERATED: $(date)
# Constitutional Compliance: Article I - Security-First
#
# WARNING: Keep this file secure!
# - Permissions: 0600 (owner read/write only)
# - Do NOT commit to git
# - Rotate regularly

username = "$USERNAME"
password = "$PASSWORD"
EOF

chmod 600 "$CREDS_FILE"

echo "✅ Credentials generated and saved to $CREDS_FILE"
echo ""
echo "Username: $USERNAME"
echo "Password: $PASSWORD"
echo ""
echo "Permissions: $(ls -l $CREDS_FILE | awk '{print $1}')"
echo ""
echo "Add to .gitignore:"
echo "echo 'config/rpc_credentials.toml' >> .gitignore"
```

#### Step 5: Update .gitignore

**File:** `.gitignore`

```gitignore
# RPC Credentials (NEVER commit)
config/rpc_credentials.toml
certs/*/server.key
*.pem
*.key
```

### Testing Requirements

**File:** `btpc-core/src/rpc/tests/secure_storage_tests.rs`

```rust
#[cfg(test)]
mod secure_storage_tests {
    use super::*;

    #[test]
    fn test_credentials_zeroized_on_drop() {
        // Create config with credentials
        // Drop config
        // Verify memory zeroized (difficult to test directly)
    }

    #[test]
    fn test_secure_credential_loading() {
        // Test loading from environment variables
        // Test loading from secure file
        // Test permission validation
    }

    #[test]
    fn test_insecure_file_permissions_rejected() {
        // Create file with wrong permissions (0644)
        // Attempt to load
        // Verify rejected with error
    }

    #[test]
    fn test_credential_expose_limited() {
        // Verify credentials not exposed in Debug output
        // Verify credentials not exposed in error messages
    }
}
```

### Implementation Checklist

- [ ] Add zeroize and secrecy dependencies
- [ ] Replace String with Secret<String>
- [ ] Update verify_auth() to use exposed secrets
- [ ] Implement secure config loading
- [ ] Add file permission validation
- [ ] Create credential generation script
- [ ] Update .gitignore
- [ ] Write secure storage tests
- [ ] Security review
- [ ] Documentation

---

## Issue #5: Unvalidated Block Submission

### Problem Description

**Severity:** CRITICAL 🔴
**File:** `btpc-core/src/rpc/handlers.rs:401-520`
**CVSS Score:** 9.1 (Critical)

`submit_block` accepts blocks but validation can be bypassed if consensus validation fails. The block is stored even on validation errors in some code paths.

**Security Impact:**
- Invalid blocks could be stored in database
- Blockchain state corruption
- Consensus failures could be ignored
- Fork creation from malicious blocks

**Constitutional Violation:**
- Article IV, Section 4.2: "Block Validation: Bitcoin-compatible block structure validation"
- Article I: Security-First principle

### Detailed Fix Plan

#### Step 1: Analyze Current Implementation

**File:** `btpc-core/src/rpc/handlers.rs:401-520`

Current problematic flow:
```rust
fn submit_block(...) {
    // 1. Parse block
    // 2. Get previous block
    // 3. Validate (but errors not always fatal)
    // 4. Store block (happens even if validation questionable)
}
```

#### Step 2: Implement Strict Validation-First Pattern

**File:** `btpc-core/src/rpc/handlers.rs`

```rust
/// Submit a new block to the blockchain
/// CONSTITUTIONAL REQUIREMENT: Article IV, Section 4.2 - Strict block validation
pub fn submit_block(
    blockchain_db: Arc<RwLock<BlockchainDatabase>>,
    utxo_db: Arc<RwLock<UtxoDatabase>>,
    consensus: Arc<ConsensusEngine>,
    params: Option<Value>,
) -> Result<Value, RpcServerError> {
    // STEP 1: Parse and validate block structure
    let block = parse_block_from_params(params)?;

    // STEP 2: Acquire locks atomically (prevent TOCTOU)
    let (mut blockchain_guard, mut utxo_guard) = acquire_write_locks(
        &blockchain_db,
        &utxo_db,
    )?;

    // STEP 3: Retrieve previous block within lock
    let prev_block = (*blockchain_guard)
        .get_block(&block.header.prev_hash)
        .map_err(|e| RpcServerError::Internal(format!("Failed to get previous block: {}", e)))?;

    let height = if let Some(ref prev) = prev_block {
        (*blockchain_guard).get_block_height(&prev.hash())
            .map_err(|e| RpcServerError::Internal(format!("Failed to get block height: {}", e)))?
    } else {
        0
    };

    // STEP 4: CRITICAL - Validate BEFORE storage
    // Article IV, Section 4.2: Bitcoin-compatible block structure validation
    validate_block_strict(&block, prev_block.as_ref(), &consensus, height)?;

    // STEP 5: Only store if validation passed completely
    (*blockchain_guard)
        .store_block(&block)
        .map_err(|e| RpcServerError::Internal(format!("Failed to store block: {}", e)))?;

    // STEP 6: Process UTXOs (atomic with block storage)
    process_block_utxos(&mut utxo_guard, &block)?;

    // STEP 7: Update chain tip if necessary
    update_chain_tip(&mut blockchain_guard, &block, height)?;

    Ok(json!({
        "status": "accepted",
        "height": height + 1,
        "hash": block.hash().to_hex(),
    }))
}

/// Strict block validation with comprehensive checks
/// Returns Err on ANY validation failure
fn validate_block_strict(
    block: &Block,
    prev_block: Option<&Block>,
    consensus: &ConsensusEngine,
    height: u64,
) -> Result<(), RpcServerError> {
    // Check 1: Genesis block validation
    if height == 0 {
        if block.header.prev_hash != Hash::zero() {
            return Err(RpcServerError::InvalidParams(
                "Genesis block must have zero prev_hash".to_string()
            ));
        }
    } else {
        if prev_block.is_none() {
            return Err(RpcServerError::InvalidParams(
                "Previous block not found".to_string()
            ));
        }
    }

    // Check 2: Proof of Work validation
    if !consensus.verify_proof_of_work(&block.header) {
        return Err(RpcServerError::InvalidParams(
            "Invalid proof of work".to_string()
        ));
    }

    // Check 3: Block timestamp validation
    validate_block_timestamp(&block, prev_block)?;

    // Check 4: Merkle root validation
    let calculated_merkle = block.calculate_merkle_root();
    if calculated_merkle != block.header.merkle_root {
        return Err(RpcServerError::InvalidParams(
            format!("Invalid merkle root: expected {}, got {}",
                calculated_merkle.to_hex(),
                block.header.merkle_root.to_hex())
        ));
    }

    // Check 5: Transaction validation
    validate_block_transactions(block, consensus)?;

    // Check 6: Consensus validation (difficulty, reward, etc.)
    consensus.validate_block(block, prev_block)
        .map_err(|e| RpcServerError::InvalidParams(
            format!("Consensus validation failed: {}", e)
        ))?;

    Ok(())
}

/// Validate block timestamp (2-hour future limit, after previous block)
fn validate_block_timestamp(
    block: &Block,
    prev_block: Option<&Block>,
) -> Result<(), RpcServerError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Future timestamp limit: 2 hours (Bitcoin compatibility)
    const MAX_FUTURE_BLOCK_TIME: u64 = 2 * 60 * 60;

    if block.header.timestamp > now + MAX_FUTURE_BLOCK_TIME {
        return Err(RpcServerError::InvalidParams(
            format!("Block timestamp too far in future: {} > {} + {}",
                block.header.timestamp, now, MAX_FUTURE_BLOCK_TIME)
        ));
    }

    // Must be after previous block
    if let Some(prev) = prev_block {
        if block.header.timestamp <= prev.header.timestamp {
            return Err(RpcServerError::InvalidParams(
                "Block timestamp must be after previous block".to_string()
            ));
        }
    }

    Ok(())
}

/// Validate all transactions in block
fn validate_block_transactions(
    block: &Block,
    consensus: &ConsensusEngine,
) -> Result<(), RpcServerError> {
    if block.transactions.is_empty() {
        return Err(RpcServerError::InvalidParams(
            "Block must contain at least coinbase transaction".to_string()
        ));
    }

    // Validate coinbase transaction
    let coinbase = &block.transactions[0];
    if !coinbase.is_coinbase() {
        return Err(RpcServerError::InvalidParams(
            "First transaction must be coinbase".to_string()
        ));
    }

    // Validate regular transactions
    for (i, tx) in block.transactions.iter().enumerate().skip(1) {
        if tx.is_coinbase() {
            return Err(RpcServerError::InvalidParams(
                format!("Non-first transaction {} is coinbase", i)
            ));
        }

        // Validate transaction signatures (ML-DSA)
        consensus.validate_transaction(tx)
            .map_err(|e| RpcServerError::InvalidParams(
                format!("Transaction {} validation failed: {}", i, e)
            ))?;
    }

    Ok(())
}

/// Acquire both database locks atomically
fn acquire_write_locks<'a>(
    blockchain_db: &'a Arc<RwLock<BlockchainDatabase>>,
    utxo_db: &'a Arc<RwLock<UtxoDatabase>>,
) -> Result<(
    std::sync::RwLockWriteGuard<'a, BlockchainDatabase>,
    std::sync::RwLockWriteGuard<'a, UtxoDatabase>,
), RpcServerError> {
    // Use try_write with timeout to prevent deadlock
    let blockchain_guard = blockchain_db
        .try_write()
        .map_err(|_| RpcServerError::Internal("Failed to acquire blockchain lock".to_string()))?;

    let utxo_guard = utxo_db
        .try_write()
        .map_err(|_| RpcServerError::Internal("Failed to acquire UTXO lock".to_string()))?;

    Ok((blockchain_guard, utxo_guard))
}
```

#### Step 3: Add Comprehensive Validation Tests

**File:** `btpc-core/src/rpc/tests/block_submission_tests.rs`

```rust
#[cfg(test)]
mod block_submission_tests {
    use super::*;

    #[test]
    fn test_submit_valid_block() {
        // Test normal valid block submission
    }

    #[test]
    fn test_reject_invalid_pow() {
        // Block with invalid proof of work
        // Expected: Rejected with "Invalid proof of work"
    }

    #[test]
    fn test_reject_invalid_merkle_root() {
        // Block with tampered merkle root
        // Expected: Rejected with merkle root error
    }

    #[test]
    fn test_reject_future_timestamp() {
        // Block with timestamp > 2 hours in future
        // Expected: Rejected with timestamp error
    }

    #[test]
    fn test_reject_past_timestamp() {
        // Block with timestamp <= previous block
        // Expected: Rejected with timestamp error
    }

    #[test]
    fn test_reject_invalid_transaction_signature() {
        // Block with invalid ML-DSA signature
        // Expected: Rejected with transaction validation error
    }

    #[test]
    fn test_reject_missing_coinbase() {
        // Block without coinbase transaction
        // Expected: Rejected
    }

    #[test]
    fn test_reject_multiple_coinbase() {
        // Block with multiple coinbase transactions
        // Expected: Rejected
    }

    #[test]
    fn test_atomic_block_storage() {
        // Test that block and UTXO updates are atomic
        // If UTXO update fails, block should not be stored
    }

    #[test]
    fn test_orphan_block_handling() {
        // Submit block with unknown prev_hash
        // Expected: Rejected with "Previous block not found"
    }
}
```

### Implementation Checklist

- [ ] Refactor submit_block() to validation-first pattern
- [ ] Implement validate_block_strict()
- [ ] Add timestamp validation
- [ ] Add merkle root validation
- [ ] Add transaction validation
- [ ] Implement atomic lock acquisition
- [ ] Write comprehensive tests
- [ ] Test with malformed blocks
- [ ] Test with invalid signatures
- [ ] Security review
- [ ] Documentation

---

## Implementation Timeline

### Phase 1: Foundation (Week 1)
**Goal:** Establish testing framework and address authentication

**Monday-Tuesday:**
- [ ] Constitutional compliance review
- [ ] Set up test infrastructure (TDD - Article III)
- [ ] Write all tests for Issue #1 (Authentication)
- [ ] Write all tests for Issue #4 (Secure storage)

**Wednesday-Thursday:**
- [ ] Implement Issue #1: Authentication enforcement
- [ ] Implement Issue #4: Secure password storage
- [ ] All tests passing for Issues #1 and #4

**Friday:**
- [ ] Code review for Issues #1 and #4
- [ ] Security review
- [ ] Update documentation

### Phase 2: Network Security (Week 2)
**Goal:** Add TLS and DoS protection

**Monday-Tuesday:**
- [ ] Write all tests for Issue #2 (TLS)
- [ ] Implement Issue #2: TLS/SSL support
- [ ] Generate development certificates
- [ ] Test TLS with self-signed certs

**Wednesday-Thursday:**
- [ ] Write all tests for Issue #3 (DoS protection)
- [ ] Implement Issue #3: Rate limiting and concurrency control
- [ ] Load testing with siege/wrk

**Friday:**
- [ ] Integration testing (Auth + TLS + Rate limiting)
- [ ] Code review for Issues #2 and #3
- [ ] Security review

### Phase 3: Consensus Security (Week 3)
**Goal:** Harden block validation

**Monday-Tuesday:**
- [ ] Write all tests for Issue #5 (Block validation)
- [ ] Implement Issue #5: Strict block validation
- [ ] Test with malformed blocks

**Wednesday-Thursday:**
- [ ] Integration testing (all issues combined)
- [ ] End-to-end testing
- [ ] Performance testing

**Friday:**
- [ ] Final security review
- [ ] Documentation updates
- [ ] Deployment preparation

---

## Testing Requirements

### Constitutional Mandate (Article III)

**"TDD mandatory for all code: Tests written → Tests fail → Implementation → Tests pass"**

### Test Coverage Requirements

- **Minimum:** >90% code coverage (Article III)
- **Security-critical paths:** 100% coverage
- **Integration tests:** All 5 issues working together

### Test Categories

#### 1. Unit Tests
- Authentication parsing and verification
- TLS certificate loading
- Rate limiting logic
- Credential zeroization
- Block validation rules

#### 2. Integration Tests
- Auth + TLS combined
- DoS protection under load
- Block submission with validation
- Error handling across modules

#### 3. Security Tests
- Timing attack resistance (constant-time comparison)
- Memory leak tests (credential zeroization)
- Fuzzing for input validation
- Load testing for DoS protection

#### 4. Performance Tests
- Request throughput with rate limiting
- TLS handshake overhead
- Block validation performance
- Concurrent request handling

### Testing Tools

```bash
# Unit tests
cargo test --lib --package btpc-core rpc::

# Integration tests
cargo test --test rpc_integration

# Coverage
cargo tarpaulin --out Html --output-dir coverage

# Load testing
wrk -t12 -c400 -d30s --latency https://localhost:8432/

# Fuzzing
cargo fuzz run rpc_request_parser
```

---

## Constitutional Compliance Checklist

Before merging any changes, verify compliance:

### Article I: Security-First ✅
- [ ] Authentication enforced with constant-time comparison
- [ ] TLS encryption for network communication
- [ ] No hardcoded credentials
- [ ] Secure credential storage with zeroization
- [ ] DoS protection implemented

### Article III: Test-Driven Development ✅
- [ ] All tests written BEFORE implementation
- [ ] >90% code coverage achieved
- [ ] All tests passing
- [ ] Integration tests cover all 5 issues

### Article V: Production Readiness ✅
- [ ] Configuration via TOML files
- [ ] Structured logging with tracing
- [ ] Graceful degradation (rate limiting, timeouts)
- [ ] Backward-compatible RPC protocol

### Article IV: Consensus Mechanism ✅
- [ ] Block validation follows Bitcoin-compatible rules
- [ ] ML-DSA signature verification enforced
- [ ] Proof-of-work validation strict

### Article VII: Governance ✅
- [ ] Constitutional compliance documented in PR
- [ ] Security violations resolved
- [ ] No prohibited changes made

---

## Documentation Updates

### Files to Update

1. **btpc-core/src/rpc/README.md**
   - Add security section
   - Document authentication setup
   - Document TLS configuration
   - Add troubleshooting guide

2. **CLAUDE.md**
   - Update RPC security section
   - Add deployment checklist

3. **SECURITY.md** (create if not exists)
   - Document security features
   - Responsible disclosure policy
   - Security best practices

4. **CHANGELOG.md**
   - Document all security fixes
   - Breaking changes (auth required by default)

---

## Risk Assessment

### Before Fixes

**Risk Level:** CRITICAL 🔴
- No authentication enforcement
- Plaintext communication
- No DoS protection
- Credentials in plaintext memory
- Invalid blocks could be stored

**Blockers:** Cannot deploy to production

### After Fixes

**Risk Level:** LOW ✅
- Authentication enforced
- TLS encryption available
- Comprehensive DoS protection
- Secure credential storage
- Strict block validation

**Status:** Production-ready for testnet

---

## Deployment Checklist

### Pre-deployment

- [ ] All 5 critical issues resolved
- [ ] All tests passing (>90% coverage)
- [ ] Security review completed
- [ ] Documentation updated
- [ ] Constitutional compliance verified

### Configuration

- [ ] Generate strong credentials
- [ ] Generate TLS certificates
- [ ] Configure rate limits appropriately
- [ ] Set request timeouts
- [ ] Verify file permissions (0600 for credentials)

### Monitoring

- [ ] Set up logging for auth failures
- [ ] Monitor rate limit events
- [ ] Track connection counts
- [ ] Alert on invalid block submissions

### Incident Response

- [ ] Document security incident procedures
- [ ] Contact information for security issues
- [ ] Procedure for credential rotation

---

## Success Criteria

Fix plan is considered successful when:

1. ✅ All 5 CRITICAL issues resolved
2. ✅ All tests passing with >90% coverage
3. ✅ Security review approved
4. ✅ Constitutional compliance verified
5. ✅ Documentation complete
6. ✅ Load testing successful
7. ✅ No security regressions

**Final Status:** CLEARED FOR TESTNET DEPLOYMENT

---

**Document Version:** 1.0
**Next Review:** After implementation completion
**Constitutional Compliance:** Article I, III, IV, V, VII
**Security Review Required:** YES

---

*End of Critical Security Fix Plan*