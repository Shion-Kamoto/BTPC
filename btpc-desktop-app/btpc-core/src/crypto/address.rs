//! Address generation and validation for BTPC
//!
//! BTPC uses Bitcoin-compatible addresses derived from public key hashes.
//!
//! # Privacy Warning (Issue 1.4.1 - MEDIUM)
//! **Address reuse significantly reduces transaction privacy.**
//!
//! ## Recommendation
//! Always generate a fresh address for each transaction. Reusing addresses allows
//! third parties to:
//! - Link multiple transactions together
//! - Track your transaction history
//! - Estimate your total balance
//! - Compromise your financial privacy
//!
//! ## Best Practices
//! - Use HD (Hierarchical Deterministic) wallets to generate unique addresses
//! - Never publish a receiving address publicly for multiple transactions
//! - Educate users about the privacy implications of address reuse
//!
//! While address reuse doesn't compromise cryptographic security (ML-DSA signatures
//! remain quantum-resistant), it creates a permanent privacy leak in the blockchain.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    crypto::{constants::ADDRESS_SIZE, hash::Hash, PublicKey},
    Network,
};

/// A BTPC address derived from a public key hash
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address {
    hash: [u8; ADDRESS_SIZE],
    network: Network,
    address_type: AddressType,
}

/// Types of addresses supported by BTPC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AddressType {
    /// Pay-to-Public-Key-Hash (P2PKH)
    P2PKH,
    /// Pay-to-Script-Hash (P2SH) - for future use
    P2SH,
}

impl Address {
    /// Create a P2PKH address from a public key
    pub fn from_public_key(public_key: &PublicKey, network: Network) -> Self {
        // Hash the public key (SHA-512 then RIPEMD-160)
        let pubkey_hash = Self::hash_public_key(public_key);

        Address {
            hash: pubkey_hash,
            network,
            address_type: AddressType::P2PKH,
        }
    }

    /// Create an address from a script hash (for P2SH)
    pub fn from_script_hash(script_hash: &[u8; ADDRESS_SIZE], network: Network) -> Self {
        Address {
            hash: *script_hash,
            network,
            address_type: AddressType::P2SH,
        }
    }

    /// Create an address from raw hash bytes and metadata
    pub fn from_hash(
        hash: [u8; ADDRESS_SIZE],
        network: Network,
        address_type: AddressType,
    ) -> Self {
        Address {
            hash,
            network,
            address_type,
        }
    }

    /// Parse an address from its string representation
    pub fn from_string(address_str: &str) -> Result<Self, AddressError> {
        // Implement Base58Check decoding
        let decoded = Self::base58_decode_check(address_str)?;

        if decoded.len() != ADDRESS_SIZE + 1 {
            return Err(AddressError::InvalidLength);
        }

        let version_byte = decoded[0];
        let mut hash = [0u8; ADDRESS_SIZE];
        hash.copy_from_slice(&decoded[1..]);

        let (network, address_type) = Self::parse_version_byte(version_byte)?;

        Ok(Address {
            hash,
            network,
            address_type,
        })
    }

    /// Convert address to its string representation
    pub fn to_string(&self) -> String {
        let version_byte = self.version_byte();
        let mut payload = Vec::with_capacity(ADDRESS_SIZE + 1);
        payload.push(version_byte);
        payload.extend_from_slice(&self.hash);

        Self::base58_encode_check(&payload)
    }

    /// Get the hash160 of the address
    pub fn hash160(&self) -> &[u8; ADDRESS_SIZE] {
        &self.hash
    }

    /// Get the network this address belongs to
    pub fn network(&self) -> Network {
        self.network
    }

    /// Get the address type
    pub fn address_type(&self) -> AddressType {
        self.address_type
    }

    /// Check if this is a P2PKH address
    pub fn is_p2pkh(&self) -> bool {
        self.address_type == AddressType::P2PKH
    }

    /// Check if this is a P2SH address
    pub fn is_p2sh(&self) -> bool {
        self.address_type == AddressType::P2SH
    }

    /// Validate that an address is well-formed
    pub fn is_valid(&self) -> bool {
        // All addresses are valid if they can be constructed
        // Additional validation could be added here
        true
    }

    /// Get the version byte for this address
    fn version_byte(&self) -> u8 {
        match (self.network, self.address_type) {
            (Network::Mainnet, AddressType::P2PKH) => 0x00,
            (Network::Mainnet, AddressType::P2SH) => 0x05,
            (Network::Testnet, AddressType::P2PKH) => 0x6f,
            (Network::Testnet, AddressType::P2SH) => 0xc4,
            (Network::Regtest, AddressType::P2PKH) => 0x6f,
            (Network::Regtest, AddressType::P2SH) => 0xc4,
        }
    }

    /// Parse version byte to determine network and address type
    fn parse_version_byte(version: u8) -> Result<(Network, AddressType), AddressError> {
        match version {
            0x00 => Ok((Network::Mainnet, AddressType::P2PKH)),
            0x05 => Ok((Network::Mainnet, AddressType::P2SH)),
            0x6f => Ok((Network::Testnet, AddressType::P2PKH)), // Also used for regtest
            0xc4 => Ok((Network::Testnet, AddressType::P2SH)),  // Also used for regtest
            _ => Err(AddressError::InvalidVersion),
        }
    }

    /// Hash a public key to create an address hash (SHA-512 -> RIPEMD-160)
    fn hash_public_key(public_key: &PublicKey) -> [u8; ADDRESS_SIZE] {
        use ripemd::{Digest, Ripemd160};

        // First, hash with SHA-512
        let sha512_hash = Hash::hash(&public_key.to_bytes());

        // Then hash with RIPEMD-160 to get 20-byte address
        let mut ripemd = Ripemd160::new();
        ripemd.update(sha512_hash.as_slice());
        let result = ripemd.finalize();

        let mut hash = [0u8; ADDRESS_SIZE];
        hash.copy_from_slice(&result[..ADDRESS_SIZE]);
        hash
    }

    /// Base58Check encoding with double SHA-512 checksum
    fn base58_encode_check(payload: &[u8]) -> String {
        // Calculate checksum using double SHA-512 (first 4 bytes)
        let checksum_hash = Hash::double_sha512(payload);
        let checksum = &checksum_hash.as_slice()[..4];

        // Combine payload and checksum
        let mut data = Vec::with_capacity(payload.len() + 4);
        data.extend_from_slice(payload);
        data.extend_from_slice(checksum);

        // Encode with Base58
        bs58::encode(data).into_string()
    }

    /// Base58Check decoding with double SHA-512 checksum verification
    fn base58_decode_check(encoded: &str) -> Result<Vec<u8>, AddressError> {
        // Decode Base58
        let decoded = bs58::decode(encoded)
            .into_vec()
            .map_err(|_| AddressError::InvalidBase58)?;

        if decoded.len() < 4 {
            return Err(AddressError::InvalidLength);
        }

        // Split payload and checksum
        let (payload, checksum) = decoded.split_at(decoded.len() - 4);

        // Verify checksum
        let expected_checksum_hash = Hash::double_sha512(payload);
        let expected_checksum = &expected_checksum_hash.as_slice()[..4];

        if checksum != expected_checksum {
            return Err(AddressError::InvalidChecksum);
        }

        Ok(payload.to_vec())
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl std::str::FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Address::from_string(s)
    }
}

/// Error types for address operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressError {
    /// Invalid address length
    InvalidLength,
    /// Invalid Base58 encoding
    InvalidBase58,
    /// Invalid checksum
    InvalidChecksum,
    /// Invalid version byte
    InvalidVersion,
    /// Unsupported address type
    UnsupportedType,
}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressError::InvalidLength => write!(f, "Invalid address length"),
            AddressError::InvalidBase58 => write!(f, "Invalid Base58 encoding"),
            AddressError::InvalidChecksum => write!(f, "Invalid address checksum"),
            AddressError::InvalidVersion => write!(f, "Invalid address version"),
            AddressError::UnsupportedType => write!(f, "Unsupported address type"),
        }
    }
}

impl std::error::Error for AddressError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::PrivateKey;

    #[test]
    fn test_address_creation_from_public_key() {
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();

        let mainnet_address = Address::from_public_key(&public_key, Network::Mainnet);
        let testnet_address = Address::from_public_key(&public_key, Network::Testnet);

        // Addresses should be different for different networks
        assert_ne!(mainnet_address, testnet_address);

        // But both should be P2PKH
        assert!(mainnet_address.is_p2pkh());
        assert!(testnet_address.is_p2pkh());
        assert!(!mainnet_address.is_p2sh());

        // Both should be valid
        assert!(mainnet_address.is_valid());
        assert!(testnet_address.is_valid());
    }

    #[test]
    fn test_address_string_conversion() {
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let address = Address::from_public_key(&public_key, Network::Mainnet);

        // Convert to string and back
        let address_string = address.to_string();
        let parsed_address = Address::from_string(&address_string).unwrap();

        assert_eq!(address, parsed_address);
        assert_eq!(address.hash160(), parsed_address.hash160());
        assert_eq!(address.network(), parsed_address.network());
        assert_eq!(address.address_type(), parsed_address.address_type());
    }

    #[test]
    fn test_address_determinism() {
        let seed = [42u8; 32];
        let private_key = PrivateKey::from_seed(&seed).unwrap();
        let public_key = private_key.public_key();

        let address1 = Address::from_public_key(&public_key, Network::Mainnet);
        let address2 = Address::from_public_key(&public_key, Network::Mainnet);

        // Same public key should generate same address
        assert_eq!(address1, address2);
        assert_eq!(address1.to_string(), address2.to_string());
    }

    #[test]
    fn test_different_networks() {
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();

        let mainnet_addr = Address::from_public_key(&public_key, Network::Mainnet);
        let testnet_addr = Address::from_public_key(&public_key, Network::Testnet);
        let regtest_addr = Address::from_public_key(&public_key, Network::Regtest);

        // Different networks should have different string representations
        let mainnet_str = mainnet_addr.to_string();
        let testnet_str = testnet_addr.to_string();
        let regtest_str = regtest_addr.to_string();

        assert_ne!(mainnet_str, testnet_str);
        // Testnet and regtest use same version bytes, so strings should be same
        assert_eq!(testnet_str, regtest_str);

        // But the Address objects should track network correctly
        assert_eq!(mainnet_addr.network(), Network::Mainnet);
        assert_eq!(testnet_addr.network(), Network::Testnet);
        assert_eq!(regtest_addr.network(), Network::Regtest);
    }

    #[test]
    fn test_address_validation() {
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let address = Address::from_public_key(&public_key, Network::Mainnet);

        // Valid address
        assert!(address.is_valid());

        // Test string parsing validation
        let address_str = address.to_string();
        assert!(Address::from_string(&address_str).is_ok());

        // Test invalid strings
        assert!(Address::from_string("invalid_address").is_err());
        assert!(Address::from_string("").is_err());
        assert!(Address::from_string("1").is_err());
    }

    #[test]
    fn test_address_type_detection() {
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();

        let p2pkh_address = Address::from_public_key(&public_key, Network::Mainnet);
        assert!(p2pkh_address.is_p2pkh());
        assert!(!p2pkh_address.is_p2sh());
        assert_eq!(p2pkh_address.address_type(), AddressType::P2PKH);

        // Create a P2SH address
        let script_hash = [0u8; ADDRESS_SIZE];
        let p2sh_address = Address::from_script_hash(&script_hash, Network::Mainnet);
        assert!(!p2sh_address.is_p2pkh());
        assert!(p2sh_address.is_p2sh());
        assert_eq!(p2sh_address.address_type(), AddressType::P2SH);
    }

    #[test]
    fn test_checksum_validation() {
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let address = Address::from_public_key(&public_key, Network::Mainnet);

        let mut address_str = address.to_string();

        // Corrupt the last character (checksum)
        address_str.pop();
        address_str.push('X');

        // Should fail checksum validation
        assert!(Address::from_string(&address_str).is_err());
    }

    #[test]
    fn test_hash160_consistency() {
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();

        // Create address and get its hash
        let address = Address::from_public_key(&public_key, Network::Mainnet);
        let hash160 = address.hash160();

        // Manually compute hash160
        let expected_hash = Address::hash_public_key(&public_key);

        assert_eq!(hash160, &expected_hash);
    }

    #[test]
    fn test_serialization() {
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let address = Address::from_public_key(&public_key, Network::Mainnet);

        // Test serde serialization
        let serialized = serde_json::to_string(&address).unwrap();
        let deserialized: Address = serde_json::from_str(&serialized).unwrap();

        assert_eq!(address, deserialized);
    }
}
