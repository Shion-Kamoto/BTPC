# Data Model: Application-Level Authentication

**Feature**: 006-add-application-level
**Date**: 2025-10-28

## Entity Definitions

### 1. MasterCredentials (Persistent)

**Purpose**: Stores encrypted master password for application authentication

**Storage**: ~/.btpc/credentials.enc (encrypted file)

**Fields**:
```rust
pub struct MasterCredentials {
    // File magic bytes
    magic: [u8; 4],              // b"BTPC"

    // Version for future migrations
    version: u32,                // 1

    // Argon2id parameters
    argon2_memory_kb: u32,       // 65536 (64 MB)
    argon2_iterations: u32,      // 3
    argon2_parallelism: u32,     // 4
    argon2_salt: [u8; 16],       // Cryptographically random

    // Encrypted password hash
    encrypted_password_hash: Vec<u8>,  // AES-256-GCM encrypted
    aes_nonce: [u8; 12],              // GCM nonce (96 bits)
    aes_tag: [u8; 16],                // Authentication tag (128 bits)

    // Metadata
    created_at: u64,             // Unix timestamp
    last_used_at: u64,           // Unix timestamp
}
```

**Validation Rules**:
- magic == b"BTPC"
- version == 1 (for v1 format)
- argon2_memory_kb >= 65536 (64 MB minimum)
- argon2_iterations >= 3
- argon2_parallelism >= 1
- argon2_salt.len() >= 16 bytes
- aes_nonce.len() == 12 bytes
- aes_tag.len() == 16 bytes

**State Transitions**:
- Created: First launch, user sets master password
- Updated: Never (immutable after creation until password reset)
- Deleted: User manually deletes file (password reset)

**Serialization Format** (binary):
```
[0-3]   Magic bytes (b"BTPC")
[4-7]   Version (u32 LE)
[8-11]  Argon2 memory KB (u32 LE)
[12-15] Argon2 iterations (u32 LE)
[16-19] Argon2 parallelism (u32 LE)
[20-35] Argon2 salt (16 bytes)
[36-39] Encrypted hash length (u32 LE)
[40-X]  Encrypted password hash (variable length)
[X-X+11] AES nonce (12 bytes)
[X+12-X+27] AES tag (16 bytes)
[X+28-X+35] Created at (u64 LE)
[X+36-X+43] Last used at (u64 LE)
```

---

### 2. SessionState (In-Memory)

**Purpose**: Tracks current authentication status

**Storage**: Arc<RwLock<SessionState>> (Tauri backend state)

**Fields**:
```rust
pub struct SessionState {
    authenticated: bool,         // Is user logged in?
    login_timestamp: Option<u64>, // When did they log in?
    session_token: String,       // Random UUID for this session
}
```

**Validation Rules**:
- If authenticated == true, login_timestamp and session_token must be Some
- If authenticated == false, login_timestamp and session_token must be None
- session_token must be valid UUID v4 format

**State Transitions**:
```
Initial -> Not Authenticated (authenticated=false)
  ↓ (user logs in successfully)
Authenticated (authenticated=true, timestamp set)
  ↓ (user logs out)
Not Authenticated (authenticated=false)
```

**Lifecycle**:
- Created: Application startup (authenticated=false)
- Updated: Login (set authenticated=true) or Logout (set authenticated=false)
- Destroyed: Application shutdown

---

### 3. LoginAttempt (In-Memory, Future Enhancement)

**Purpose**: Track login attempts for rate limiting (not in v1)

**Storage**: Vec<LoginAttempt> in backend state

**Fields**:
```rust
pub struct LoginAttempt {
    timestamp: u64,              // Unix timestamp
    success: bool,               // Was login successful?
    ip_address: Option<String>,  // N/A for desktop (future: remote access)
}
```

**Note**: Not implemented in v1. Placeholder for future rate limiting feature.

---

## Entity Relationships

```
MasterCredentials (file)
    ↓ (validates password)
SessionState (memory) ← Login/Logout actions
    ↓ (emits events)
Frontend (UI state) ← Tauri events
```

**Relationship Rules**:
1. MasterCredentials MUST exist before login can succeed
2. SessionState authenticated=true ONLY if password validated against MasterCredentials
3. Frontend UI MUST reflect SessionState (never maintain own auth state)

---

## Security Properties

### Encryption Chain
```
User Password (plaintext)
    ↓ (Argon2id KDF)
Derived Key (32 bytes)
    ↓ (AES-256-GCM encryption key)
Encrypted Password Hash (stored in MasterCredentials)
```

### Verification Chain
```
User enters password
    ↓ (Argon2id KDF with stored salt)
Derived Key
    ↓ (AES-256-GCM decryption)
Original Password Hash
    ↓ (constant-time comparison with stored hash)
Authentication Decision (true/false)
```

---

## File Permissions

**Unix-like Systems** (Linux, macOS):
- ~/.btpc/credentials.enc: 0600 (owner read/write only)
- ~/. btpc/ directory: 0700 (owner full access only)

**Windows**:
- %APPDATA%\btpc\credentials.enc: ACL restricted to current user
- Inherited from %APPDATA%\btpc directory permissions

---

## Data Migration Strategy

**v1 → v2** (future):
1. Check `version` field in MasterCredentials file
2. If version < 2, read v1 format
3. Prompt user to re-enter password
4. Re-encrypt with v2 parameters
5. Write v2 format file
6. Delete v1 file

**Password Reset** (forgot password):
1. User manually deletes ~/.btpc/credentials.enc
2. Application detects missing file
3. Treats as first launch (password creation screen)
4. User creates new master password
5. New MasterCredentials file created

**Note**: No password recovery mechanism by design (security over convenience)

---

## Implementation Notes

### Constant-Time Comparison

All password comparisons MUST use constant-time equality:
```rust
use subtle::ConstantTimeEq;

// DON'T DO THIS (timing attack vulnerable):
if derived_hash == stored_hash { ... }

// DO THIS (constant-time):
if derived_hash.ct_eq(&stored_hash).unwrap_u8() == 1 { ... }
```

### Zeroizing Sensitive Data

All password-derived keys MUST be zeroized after use:
```rust
use zeroize::Zeroizing;

let password = Zeroizing::new(user_entered_password);
let derived_key = Zeroizing::new(argon2_derive(&password, &salt));
// derived_key automatically zeroed when dropped
```

### Error Messages

All authentication errors MUST use consistent messaging:
- ❌ "Invalid password" (reveals credential file exists)
- ❌ "Credential file not found" (reveals file doesn't exist)
- ✅ "Authentication failed" (consistent for all error cases)

---

## Status

✅ **COMPLETE** - All entities defined with validation rules, state transitions, and security properties specified.