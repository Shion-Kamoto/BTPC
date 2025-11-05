# Tauri Auth Commands Contract

**Feature**: 006-add-application-level
**Date**: 2025-10-28

## Command: `create_master_password`

**Purpose**: Create master password on first launch

**Input**:
```typescript
{
  password: string,           // User-entered master password (min 8 chars)
  password_confirm: string    // Password confirmation (must match)
}
```

**Output** (Success):
```typescript
{
  success: true,
  message: "Master password created successfully"
}
```

**Output** (Error):
```typescript
{
  success: false,
  error: "PASSWORD_TOO_SHORT" | "PASSWORDS_DONT_MATCH" | "CREDENTIALS_ALREADY_EXIST" | "ENCRYPTION_FAILED"
}
```

**Side Effects**:
- Creates ~/.btpc/credentials.enc file
- Emits `session:login` event
- Updates SessionState (authenticated=true)

**Validations**:
- password.length >= 8
- password == password_confirm
- credentials.enc must NOT already exist

---

## Command: `login`

**Purpose**: Authenticate user with existing master password

**Input**:
```typescript
{
  password: string  // User-entered password
}
```

**Output** (Success):
```typescript
{
  success: true,
  message: "Login successful"
}
```

**Output** (Error):
```typescript
{
  success: false,
  error: "AUTHENTICATION_FAILED" | "CREDENTIALS_NOT_FOUND" | "DECRYPTION_FAILED"
}
```

**Side Effects**:
- Emits `session:login` event
- Updates SessionState (authenticated=true, login_timestamp=now)
- Updates MasterCredentials (last_used_at=now)

**Validations**:
- credentials.enc must exist
- Password must match stored hash (constant-time comparison)

**Security Notes**:
- All errors return "AUTHENTICATION_FAILED" to prevent timing attacks
- Constant-time password comparison used

---

## Command: `logout`

**Purpose**: End user session

**Input**: None

**Output**:
```typescript
{
  success: true,
  message: "Logged out successfully"
}
```

**Side Effects**:
- Emits `session:logout` event
- Updates SessionState (authenticated=false)
- Clears session token

**Validations**: None (logout always succeeds)

---

## Command: `check_session`

**Purpose**: Check if user is currently authenticated (navigation guard)

**Input**: None

**Output**:
```typescript
{
  authenticated: boolean,
  session_token: string | null  // UUID v4 if authenticated
}
```

**Side Effects**: None (read-only)

**Performance**: <50ms (reads Arc<RwLock> in-memory state)

---

## Command: `has_master_password`

**Purpose**: Check if master password exists (first launch detection)

**Input**: None

**Output**:
```typescript
{
  exists: boolean  // true if credentials.enc exists
}
```

**Side Effects**: None (read-only, checks file existence)

**Use Case**: Application startup routing logic

---

## Events

### Event: `session:login`

**Emitted By**:
- `create_master_password` (after successful creation)
- `login` (after successful authentication)

**Payload**:
```typescript
{
  session_token: string,  // UUID v4
  timestamp: number       // Unix timestamp
}
```

**Frontend Action**: Navigate to dashboard, show authenticated UI

---

### Event: `session:logout`

**Emitted By**:
- `logout` (after clearing session)

**Payload**:
```typescript
{
  timestamp: number  // Unix timestamp
}
```

**Frontend Action**: Navigate to login screen, hide authenticated UI

---

## Error Handling Contract

**All errors follow consistent structure**:
```typescript
{
  success: false,
  error: string  // Error code (SCREAMING_SNAKE_CASE)
}
```

**Error Codes**:
- `PASSWORD_TOO_SHORT`: Password < 8 characters
- `PASSWORDS_DONT_MATCH`: password != password_confirm
- `CREDENTIALS_ALREADY_EXIST`: Attempted create when credentials.enc exists
- `CREDENTIALS_NOT_FOUND`: Attempted login but credentials.enc doesn't exist
- `AUTHENTICATION_FAILED`: Password doesn't match (constant-time failure)
- `ENCRYPTION_FAILED`: Argon2id or AES-256-GCM operation failed
- `DECRYPTION_FAILED`: AES-256-GCM decryption failed (corrupted file or wrong password)

**Security Requirement**: Timing attacks prevented by:
1. Constant-time password comparison
2. Consistent error messages ("AUTHENTICATION_FAILED" for all auth failures)
3. Fixed-time error responses (don't reveal whether file exists)

---

## Contract Tests

**Test File**: `btpc-desktop-app/src-tauri/tests/auth_contract_test.rs`

**Test Cases**:
```rust
#[test]
fn test_create_master_password_success() {
    // Valid password creation
    assert!(create_master_password("password123", "password123").success);
}

#[test]
fn test_create_master_password_too_short() {
    // Password < 8 chars
    assert_eq!(create_master_password("pass", "pass").error, "PASSWORD_TOO_SHORT");
}

#[test]
fn test_create_master_password_mismatch() {
    // Passwords don't match
    assert_eq!(create_master_password("password123", "password456").error, "PASSWORDS_DONT_MATCH");
}

#[test]
fn test_login_success() {
    // Setup: create password
    create_master_password("password123", "password123");
    // Test: login with correct password
    assert!(login("password123").success);
}

#[test]
fn test_login_wrong_password() {
    // Setup: create password
    create_master_password("password123", "password123");
    // Test: login with wrong password
    assert_eq!(login("wrongpassword").error, "AUTHENTICATION_FAILED");
}

#[test]
fn test_logout() {
    // Always succeeds
    assert!(logout().success);
}

#[test]
fn test_check_session_authenticated() {
    // Setup: create and login
    create_master_password("password123", "password123");
    login("password123");
    // Test: check session
    assert!(check_session().authenticated);
}

#[test]
fn test_check_session_not_authenticated() {
    // Test: check session after logout
    logout();
    assert!(!check_session().authenticated);
}

#[test]
fn test_has_master_password_false() {
    // Fresh install
    assert!(!has_master_password().exists);
}

#[test]
fn test_has_master_password_true() {
    // After creating password
    create_master_password("password123", "password123");
    assert!(has_master_password().exists);
}
```

**Note**: These tests must FAIL initially (RED phase), then pass after implementation (GREEN phase).

---

## Status

✅ **COMPLETE** - All Tauri command contracts defined with input/output schemas, error codes, and contract tests.