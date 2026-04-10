use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkType {
    Mainnet,
    Testnet,
    Regtest,
}

impl NetworkType {
    /// Get fork ID for replay protection
    /// Mainnet=0, Testnet=1, Regtest=2
    pub fn fork_id(&self) -> u8 {
        match self {
            NetworkType::Mainnet => 0,
            NetworkType::Testnet => 1,
            NetworkType::Regtest => 2,
        }
    }
}

impl std::fmt::Display for NetworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkType::Mainnet => write!(f, "mainnet"),
            NetworkType::Testnet => write!(f, "testnet"),
            NetworkType::Regtest => write!(f, "regtest"),
        }
    }
}

/// FIX 2025-12-01: Convert NetworkType to btpc_core::Network for address generation
impl From<&NetworkType> for btpc_core::Network {
    fn from(network_type: &NetworkType) -> Self {
        match network_type {
            NetworkType::Mainnet => btpc_core::Network::Mainnet,
            NetworkType::Testnet => btpc_core::Network::Testnet,
            NetworkType::Regtest => btpc_core::Network::Regtest,
        }
    }
}

impl From<NetworkType> for btpc_core::Network {
    fn from(network_type: NetworkType) -> Self {
        (&network_type).into()
    }
}