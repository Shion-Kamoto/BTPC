//! SHA-512 hashing implementation for BTPC
//!
//! All hashing in BTPC uses SHA-512 for quantum resistance and Bitcoin compatibility.

use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use subtle::{Choice, ConditionallySelectable, ConstantTimeEq, ConstantTimeLess};

use crate::crypto::constants::SHA512_HASH_SIZE;

/// A SHA-512 hash value
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash([u8; SHA512_HASH_SIZE]);

impl Hash {
    /// Create a hash from a byte array
    pub fn from_bytes(bytes: [u8; SHA512_HASH_SIZE]) -> Self {
        Hash(bytes)
    }

    /// Create a hash from a slice, returning error if wrong length
    pub fn from_slice(slice: &[u8]) -> Result<Self, HashError> {
        if slice.len() != SHA512_HASH_SIZE {
            return Err(HashError::InvalidLength);
        }
        let mut bytes = [0u8; SHA512_HASH_SIZE];
        bytes.copy_from_slice(slice);
        Ok(Hash(bytes))
    }

    /// Create a hash from a hex string
    pub fn from_hex(hex: &str) -> Result<Self, HashError> {
        let hex = hex.trim_start_matches("0x");
        if hex.len() != SHA512_HASH_SIZE * 2 {
            return Err(HashError::InvalidHexLength);
        }

        let mut bytes = [0u8; SHA512_HASH_SIZE];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let hex_byte =
                std::str::from_utf8(chunk).map_err(|_| HashError::InvalidHexCharacter)?;
            bytes[i] =
                u8::from_str_radix(hex_byte, 16).map_err(|_| HashError::InvalidHexCharacter)?;
        }

        Ok(Hash(bytes))
    }

    /// Create a hash from an integer (for testing)
    pub fn from_int(value: u64) -> Self {
        let mut hasher = Sha512::new();
        hasher.update(value.to_le_bytes());
        Hash(hasher.finalize().into())
    }

    /// Create a zero hash
    pub fn zero() -> Self {
        Hash([0u8; SHA512_HASH_SIZE])
    }

    /// Create a random hash (for testing)
    pub fn random() -> Self {
        use rand::RngCore;
        let mut bytes = [0u8; SHA512_HASH_SIZE];
        rand::thread_rng().fill_bytes(&mut bytes);
        Hash(bytes)
    }

    /// Get the hash as a byte array
    pub fn as_bytes(&self) -> &[u8; SHA512_HASH_SIZE] {
        &self.0
    }

    /// Get the hash as a slice
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Check if hash is zero (all bytes are 0)
    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|&b| b == 0)
    }

    /// Single SHA-512 hash of data
    pub fn hash(data: &[u8]) -> Self {
        let mut hasher = Sha512::new();
        hasher.update(data);
        Hash(hasher.finalize().into())
    }

    /// Double SHA-512 hash of data (Bitcoin-compatible)
    pub fn double_sha512(data: &[u8]) -> Self {
        let first_hash = Self::hash(data);
        Self::hash(first_hash.as_slice())
    }

    /// Check if this hash meets the given difficulty target
    ///
    /// This uses constant-time comparison to prevent timing attacks.
    /// The comparison is lexicographic: returns true if self <= target.
    pub fn meets_target(&self, target: &[u8; SHA512_HASH_SIZE]) -> bool {
        // Constant-time lexicographic comparison for self <= target
        //
        // Algorithm:
        // - Track if we've found a definitive result (less or greater)
        // - For each byte position:
        //   - If already found difference, maintain that result
        //   - Otherwise, check current byte:
        //     - If self[i] < target[i]: result is true, mark as found
        //     - If self[i] > target[i]: result is false, mark as found
        //     - If self[i] == target[i]: continue to next byte
        // - If all bytes equal, result is true (equality part of <=)

        let mut result = Choice::from(1u8); // Assume true (equal case)
        let mut found_difference = Choice::from(0u8); // Haven't found difference yet

        for i in 0..SHA512_HASH_SIZE {
            let self_byte = self.0[i];
            let target_byte = target[i];

            // Constant-time comparisons
            let less = u8::ct_lt(&self_byte, &target_byte);
            let equal = u8::ct_eq(&self_byte, &target_byte);
            let greater = !(less | equal);

            // Compute what the new result should be if we need to update:
            // If self_byte < target_byte: result should be true (1)
            // If self_byte > target_byte: result should be false (0)
            let new_result = Choice::conditional_select(&Choice::from(0u8), &Choice::from(1u8), less);

            // Only update result if bytes are NOT equal AND we haven't found difference yet
            // This preserves the result when bytes are equal
            let should_update = !equal & !found_difference;
            result = Choice::conditional_select(&result, &new_result, should_update);

            // Mark that we found a difference if bytes are not equal
            found_difference |= !equal;
        }

        bool::from(result)
    }

    /// Calculate the difficulty of this hash
    pub fn difficulty(&self) -> f64 {
        // Count leading zero bits
        let mut leading_zeros = 0;
        for byte in &self.0 {
            if *byte == 0 {
                leading_zeros += 8;
            } else {
                leading_zeros += byte.leading_zeros() as usize;
                break;
            }
        }
        2.0_f64.powi(leading_zeros as i32)
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::LowerHex for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl From<[u8; SHA512_HASH_SIZE]> for Hash {
    fn from(bytes: [u8; SHA512_HASH_SIZE]) -> Self {
        Hash(bytes)
    }
}

impl From<Hash> for [u8; SHA512_HASH_SIZE] {
    fn from(hash: Hash) -> Self {
        hash.0
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Error types for hash operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashError {
    /// Invalid hash length
    InvalidLength,
    /// Invalid hex string length
    InvalidHexLength,
    /// Invalid hex character
    InvalidHexCharacter,
}

impl fmt::Display for HashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashError::InvalidLength => write!(f, "Invalid hash length"),
            HashError::InvalidHexLength => write!(f, "Invalid hex string length"),
            HashError::InvalidHexCharacter => write!(f, "Invalid hex character"),
        }
    }
}

impl std::error::Error for HashError {}

// Custom serialization implementation for Hash
impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_hex())
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        if deserializer.is_human_readable() {
            let hex_str = String::deserialize(deserializer)?;
            Hash::from_hex(&hex_str).map_err(serde::de::Error::custom)
        } else {
            use serde::de::{self, Visitor};

            struct HashVisitor;

            impl<'de> Visitor<'de> for HashVisitor {
                type Value = [u8; SHA512_HASH_SIZE];

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a 64-byte array")
                }

                fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
                where E: de::Error {
                    if value.len() == SHA512_HASH_SIZE {
                        let mut bytes = [0u8; SHA512_HASH_SIZE];
                        bytes.copy_from_slice(value);
                        Ok(bytes)
                    } else {
                        Err(E::custom(format!(
                            "expected {} bytes, got {}",
                            SHA512_HASH_SIZE,
                            value.len()
                        )))
                    }
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where A: de::SeqAccess<'de> {
                    let mut bytes = [0u8; SHA512_HASH_SIZE];
                    for i in 0..SHA512_HASH_SIZE {
                        bytes[i] = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(i, &self))?;
                    }
                    Ok(bytes)
                }
            }

            let bytes = deserializer.deserialize_bytes(HashVisitor)?;
            Ok(Hash(bytes))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_creation() {
        let data = b"Hello, BTPC!";
        let hash = Hash::hash(data);

        // SHA-512 should produce 64-byte hash
        assert_eq!(hash.as_bytes().len(), 64);

        // Same data should produce same hash
        let hash2 = Hash::hash(data);
        assert_eq!(hash, hash2);

        // Different data should produce different hash
        let hash3 = Hash::hash(b"Different data");
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_double_sha512() {
        let data = b"BTPC double hash test";
        let single_hash = Hash::hash(data);
        let double_hash = Hash::double_sha512(data);

        // Double hash should be different from single hash
        assert_ne!(single_hash, double_hash);

        // Double hash should equal hash of hash
        let manual_double = Hash::hash(single_hash.as_slice());
        assert_eq!(double_hash, manual_double);
    }

    #[test]
    fn test_zero_hash() {
        let zero = Hash::zero();
        assert_eq!(zero.as_bytes(), &[0u8; 64]);
    }

    #[test]
    fn test_hex_conversion() {
        let hash = Hash::from_int(12345);
        let hex_str = hash.to_hex();

        // Should be 128 hex characters (64 bytes * 2)
        assert_eq!(hex_str.len(), 128);

        // Should be able to parse back
        let parsed_hash = Hash::from_hex(&hex_str).unwrap();
        assert_eq!(hash, parsed_hash);
    }

    #[test]
    fn test_invalid_hex() {
        // Too short
        assert!(Hash::from_hex("abc123").is_err());

        // Invalid characters
        assert!(Hash::from_hex(&"z".repeat(128)).is_err());

        // Wrong length
        assert!(Hash::from_hex(&"a".repeat(126)).is_err());
    }

    #[test]
    fn test_difficulty_calculation() {
        // Hash with many leading zeros should have high difficulty
        let easy_target = [0u8; 64];
        let hash_with_zeros = Hash::from_bytes({
            let mut bytes = [0xffu8; 64];
            bytes[0] = 0;
            bytes[1] = 0;
            bytes
        });

        let difficulty = hash_with_zeros.difficulty();
        assert!(difficulty >= 16.0); // At least 16 leading zero bits
    }

    #[test]
    fn test_target_comparison() {
        let target = [0x0fu8; 64]; // High target (easy)
        let easy_hash = Hash::from_bytes([0x01u8; 64]);
        let hard_hash = Hash::from_bytes([0xffu8; 64]);

        assert!(easy_hash.meets_target(&target));
        assert!(!hard_hash.meets_target(&target));
    }

    #[test]
    fn test_serialization() {
        let hash = Hash::from_int(54321);

        // Test serde serialization
        let serialized = serde_json::to_string(&hash).unwrap();
        let deserialized: Hash = serde_json::from_str(&serialized).unwrap();
        assert_eq!(hash, deserialized);
    }

    // Constant-time comparison tests
    #[test]
    fn test_constant_time_meets_target_equal() {
        // Test that equal hashes meet the target
        let hash = Hash::from_bytes([0x42u8; 64]);
        let target = [0x42u8; 64];
        assert!(hash.meets_target(&target));
    }

    #[test]
    fn test_constant_time_meets_target_less() {
        // Test that smaller hash meets target
        let hash = Hash::from_bytes([0x41u8; 64]);
        let target = [0x42u8; 64];
        assert!(hash.meets_target(&target));
    }

    #[test]
    fn test_constant_time_meets_target_greater() {
        // Test that larger hash does not meet target
        let hash = Hash::from_bytes([0x43u8; 64]);
        let target = [0x42u8; 64];
        assert!(!hash.meets_target(&target));
    }

    #[test]
    fn test_constant_time_first_byte_difference() {
        // Test difference in first byte
        let mut hash_bytes = [0x42u8; 64];
        hash_bytes[0] = 0x41; // First byte less
        let hash = Hash::from_bytes(hash_bytes);
        let target = [0x42u8; 64];
        assert!(hash.meets_target(&target));

        let mut hash_bytes2 = [0x42u8; 64];
        hash_bytes2[0] = 0x43; // First byte greater
        let hash2 = Hash::from_bytes(hash_bytes2);
        assert!(!hash2.meets_target(&target));
    }

    #[test]
    fn test_constant_time_last_byte_difference() {
        // Test difference in last byte
        let mut hash_bytes = [0x42u8; 64];
        hash_bytes[63] = 0x41; // Last byte less
        let hash = Hash::from_bytes(hash_bytes);
        let target = [0x42u8; 64];
        assert!(hash.meets_target(&target));

        let mut hash_bytes2 = [0x42u8; 64];
        hash_bytes2[63] = 0x43; // Last byte greater
        let hash2 = Hash::from_bytes(hash_bytes2);
        assert!(!hash2.meets_target(&target));
    }

    #[test]
    fn test_constant_time_middle_byte_difference() {
        // Test difference in middle byte (32nd)
        let mut hash_bytes = [0x42u8; 64];
        hash_bytes[32] = 0x41; // Middle byte less
        let hash = Hash::from_bytes(hash_bytes);
        let target = [0x42u8; 64];
        assert!(hash.meets_target(&target));

        let mut hash_bytes2 = [0x42u8; 64];
        hash_bytes2[32] = 0x43; // Middle byte greater
        let hash2 = Hash::from_bytes(hash_bytes2);
        assert!(!hash2.meets_target(&target));
    }

    #[test]
    fn test_constant_time_zero_hash() {
        // Zero hash should meet any non-zero target
        let zero = Hash::zero();
        let target = [0x01u8; 64];
        assert!(zero.meets_target(&target));

        // Zero hash should meet zero target (equal)
        let zero_target = [0x00u8; 64];
        assert!(zero.meets_target(&zero_target));
    }

    #[test]
    fn test_constant_time_max_hash() {
        // Max hash (all 0xff) should not meet most targets
        let max_hash = Hash::from_bytes([0xffu8; 64]);
        let target = [0x7fu8; 64];
        assert!(!max_hash.meets_target(&target));

        // But should meet itself
        let max_target = [0xffu8; 64];
        assert!(max_hash.meets_target(&max_target));
    }

    #[test]
    fn test_constant_time_lexicographic_order() {
        // Test lexicographic ordering: earlier bytes take precedence
        let mut hash1 = [0x00u8; 64];
        hash1[0] = 0x01; // First byte = 0x01
        hash1[63] = 0xff; // Last byte = 0xff

        let mut hash2 = [0x00u8; 64];
        hash2[0] = 0x02; // First byte = 0x02
        hash2[63] = 0x00; // Last byte = 0x00

        // hash1 < hash2 because first byte is smaller, even though last byte is larger
        let hash_obj1 = Hash::from_bytes(hash1);
        let hash_obj2 = Hash::from_bytes(hash2);
        assert!(hash_obj1.meets_target(&hash2));
        assert!(!hash_obj2.meets_target(&hash1));
    }

    #[test]
    fn test_constant_time_edge_case_0xff_vs_0x00() {
        // Test boundary between 0xff and 0x00
        let hash_ff = Hash::from_bytes([0xffu8; 64]);
        let target_00 = [0x00u8; 64];
        assert!(!hash_ff.meets_target(&target_00)); // 0xff > 0x00

        let hash_00 = Hash::from_bytes([0x00u8; 64]);
        let target_ff = [0xffu8; 64];
        assert!(hash_00.meets_target(&target_ff)); // 0x00 < 0xff
    }

    #[test]
    fn test_constant_time_sequential_bytes() {
        // Test with sequential byte values
        let hash_seq = Hash::from_bytes({
            let mut bytes = [0u8; 64];
            for i in 0..64 {
                bytes[i] = i as u8;
            }
            bytes
        });

        let target_seq = {
            let mut bytes = [0u8; 64];
            for i in 0..64 {
                bytes[i] = i as u8;
            }
            bytes
        };

        assert!(hash_seq.meets_target(&target_seq)); // Equal
    }

    #[test]
    fn test_constant_time_realistic_mining() {
        // Simulate realistic mining scenario
        // Easy target (high value = easier to meet)
        let easy_target = {
            let mut bytes = [0xffu8; 64];
            bytes[0] = 0x00;
            bytes[1] = 0x00;
            bytes[2] = 0xff;
            bytes
        };

        // Hash that meets easy target
        let good_hash = Hash::from_bytes({
            let mut bytes = [0x00u8; 64];
            bytes[2] = 0x42;
            bytes
        });
        assert!(good_hash.meets_target(&easy_target));

        // Hash that doesn't meet easy target
        let bad_hash = Hash::from_bytes({
            let mut bytes = [0xffu8; 64];
            bytes[0] = 0x00;
            bytes[1] = 0x01; // Slightly above target
            bytes
        });
        assert!(!bad_hash.meets_target(&easy_target));
    }
}
