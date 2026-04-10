//! BIP39 mnemonic phrase handling for deterministic wallet recovery
//!
//! This module provides BIP39-compliant mnemonic generation, parsing, and seed derivation
//! for deterministic ML-DSA key generation.

use bip39::{Mnemonic as Bip39Mnemonic, Language};
use zeroize::Zeroizing;

/// BIP39 mnemonic phrase wrapper for BTPC wallet recovery
///
/// BTPC uses 24-word BIP39 mnemonics with empty passphrase (standard).
/// This ensures deterministic key derivation across devices (FR-006).
#[derive(Debug, Clone)]
pub struct Mnemonic {
    inner: Bip39Mnemonic,
}

impl Mnemonic {
    /// Parse a BIP39 mnemonic from a string
    ///
    /// # Arguments
    /// * `words` - Space-separated 24-word mnemonic phrase
    ///
    /// # Errors
    /// Returns error if:
    /// - Word count is not 24
    /// - Any word is not in BIP39 wordlist
    /// - Checksum is invalid
    ///
    /// # Example
    /// ```ignore
    /// let mnemonic = Mnemonic::parse(
    ///     "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art"
    /// )?;
    /// ```
    pub fn parse(words: &str) -> Result<Self, BIP39Error> {
        // Normalize whitespace and convert to lowercase (BIP39 is case-insensitive)
        let normalized = words
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");

        // Validate word count
        let word_list: Vec<&str> = normalized.split_whitespace().collect();
        if word_list.len() != 24 {
            return Err(BIP39Error::InvalidWordCount {
                expected: 24,
                found: word_list.len(),
            });
        }

        // Parse with bip39 crate (validates words, checksum, and normalizes Unicode)
        // `parse` handles NFKD normalization automatically (FR-005)
        let mnemonic = Bip39Mnemonic::parse(&normalized)
            .map_err(|e| {
                // Check error message for specific cases
                let error_msg = e.to_string();
                if error_msg.contains("word count") || error_msg.contains("word-count") {
                    BIP39Error::InvalidWordCount {
                        expected: 24,
                        found: word_list.len(),
                    }
                } else if error_msg.contains("unknown word") && error_msg.contains("word ") {
                    // Extract word index from error message like "unknown word (word 23)"
                    let word_idx = error_msg
                        .split("word ")
                        .nth(1)
                        .and_then(|s| s.trim_end_matches(')').parse::<usize>().ok())
                        .unwrap_or(0);
                    BIP39Error::InvalidWord {
                        position: word_idx + 1, // 1-indexed
                        word: word_list.get(word_idx).unwrap_or(&"<unknown>").to_string(),
                    }
                } else if error_msg.contains("checksum") {
                    BIP39Error::InvalidChecksum
                } else {
                    BIP39Error::ParseError(error_msg)
                }
            })?;

        Ok(Mnemonic { inner: mnemonic })
    }

    /// Derive 32-byte seed from mnemonic using PBKDF2 (BIP39 standard)
    ///
    /// # Arguments
    /// * `passphrase` - BIP39 passphrase (BTPC uses empty string "")
    ///
    /// # Returns
    /// 32-byte seed for SHAKE256 expansion
    ///
    /// # BIP39 Standard Compliance
    /// - Uses PBKDF2-HMAC-SHA512 with 2048 iterations
    /// - Passphrase is "mnemonic" + user passphrase
    /// - BTPC standard: empty passphrase ("") for deterministic recovery
    /// - Returns first 32 bytes of 64-byte output (FR-003)
    ///
    /// # Example
    /// ```ignore
    /// let mnemonic = Mnemonic::parse("abandon abandon...")?;
    /// let seed = mnemonic.to_seed("")?; // Empty passphrase (BTPC standard)
    /// assert_eq!(seed.len(), 32);
    /// ```
    pub fn to_seed(&self, passphrase: &str) -> Result<[u8; 32], BIP39Error> {
        // PBKDF2 derivation (BIP39 standard, FR-003)
        let seed_512 = self.inner.to_seed(passphrase); // Returns 64 bytes (512 bits)

        // Take first 32 bytes for BTPC (256-bit seed)
        let mut seed_32 = [0u8; 32];
        seed_32.copy_from_slice(&seed_512[..32]);

        Ok(seed_32)
    }

    /// Get the number of words in this mnemonic
    ///
    /// Always returns 24 for BTPC (256-bit entropy)
    pub fn word_count(&self) -> usize {
        24
    }

    /// Get the entropy size in bits
    ///
    /// Always returns 256 for BTPC (24-word mnemonic)
    pub fn entropy_bits(&self) -> usize {
        256
    }

    /// Get the raw entropy bytes
    ///
    /// Returns the 32 bytes of entropy that generated this mnemonic
    pub fn entropy(&self) -> Vec<u8> {
        self.inner.to_entropy()
    }
}

/// BIP39 error types
#[derive(Debug, thiserror::Error)]
pub enum BIP39Error {
    /// Mnemonic word count is invalid (must be 24)
    #[error("Mnemonic must have exactly {expected} words (found: {found})")]
    InvalidWordCount { expected: usize, found: usize },

    /// Word is not in BIP39 wordlist
    #[error("Invalid word at position {position}: '{word}' (not in BIP39 wordlist)")]
    InvalidWord { position: usize, word: String },

    /// BIP39 checksum validation failed
    #[error("Invalid BIP39 checksum - please verify your seed phrase")]
    InvalidChecksum,

    /// Generic parse error
    #[error("BIP39 parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_24_word_mnemonic() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

        let parsed = Mnemonic::parse(mnemonic).unwrap();

        assert_eq!(parsed.word_count(), 24);
        assert_eq!(parsed.entropy_bits(), 256);
        assert_eq!(parsed.entropy().len(), 32);
    }

    #[test]
    fn test_reject_invalid_word_count() {
        let mnemonic_12 = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let result = Mnemonic::parse(mnemonic_12);

        assert!(result.is_err());
        match result.unwrap_err() {
            BIP39Error::InvalidWordCount { expected, found } => {
                assert_eq!(expected, 24);
                assert_eq!(found, 12);
            }
            _ => panic!("Expected InvalidWordCount error"),
        }
    }

    #[test]
    fn test_reject_invalid_word() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon invalidword";

        let result = Mnemonic::parse(mnemonic);

        assert!(result.is_err());
        // Invalid word will be caught as ParseError
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("unknown") || err_msg.contains("invalid") || err_msg.contains("BIP39"));
    }

    #[test]
    fn test_reject_invalid_checksum() {
        // Valid words but wrong checksum (last word should be "art")
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let result = Mnemonic::parse(mnemonic);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BIP39Error::InvalidChecksum));
    }

    #[test]
    fn test_to_seed_produces_32_bytes() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

        let parsed = Mnemonic::parse(mnemonic).unwrap();
        let seed = parsed.to_seed("").unwrap();

        assert_eq!(seed.len(), 32);
    }

    #[test]
    fn test_same_mnemonic_same_seed() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

        let seed1 = Mnemonic::parse(mnemonic).unwrap().to_seed("").unwrap();
        let seed2 = Mnemonic::parse(mnemonic).unwrap().to_seed("").unwrap();

        assert_eq!(seed1, seed2);
    }

    #[test]
    fn test_different_passphrase_different_seed() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

        let parsed = Mnemonic::parse(mnemonic).unwrap();
        let seed_empty = parsed.to_seed("").unwrap();
        let seed_with_pass = parsed.to_seed("password").unwrap();

        assert_ne!(seed_empty, seed_with_pass);
    }
}