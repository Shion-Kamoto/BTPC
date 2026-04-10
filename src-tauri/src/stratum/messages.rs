//! Stratum V2-BTPC message types
//!
//! All hash/target fields use `[u8; 64]` for SHA-512 compatibility.

use serde::{Deserialize, Serialize};

/// Protocol version for BTPC Stratum V2
pub const STRATUM_PROTOCOL_VERSION: u16 = 1;

/// Message type discriminants (first 2 bytes of each frame)
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    SetupConnection = 0x0000,
    SetupConnectionSuccess = 0x0001,
    SetupConnectionError = 0x0002,
    NewMiningJob = 0x0010,
    SetTarget = 0x0011,
    SetNewPrevHash = 0x0012,
    SubmitSharesStandard = 0x0020,
    SubmitSharesSuccess = 0x0021,
    SubmitSharesError = 0x0022,
    Reconnect = 0x0030,
}

impl MessageType {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            0x0000 => Some(Self::SetupConnection),
            0x0001 => Some(Self::SetupConnectionSuccess),
            0x0002 => Some(Self::SetupConnectionError),
            0x0010 => Some(Self::NewMiningJob),
            0x0011 => Some(Self::SetTarget),
            0x0012 => Some(Self::SetNewPrevHash),
            0x0020 => Some(Self::SubmitSharesStandard),
            0x0021 => Some(Self::SubmitSharesSuccess),
            0x0022 => Some(Self::SubmitSharesError),
            0x0030 => Some(Self::Reconnect),
            _ => None,
        }
    }
}

/// Top-level Stratum V2 message enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StratumMessage {
    SetupConnection(SetupConnection),
    SetupConnectionSuccess(SetupConnectionSuccess),
    SetupConnectionError(SetupConnectionError),
    NewMiningJob(NewMiningJob),
    SetTarget(SetTarget),
    SetNewPrevHash(SetNewPrevHash),
    SubmitSharesStandard(SubmitSharesStandard),
    SubmitSharesSuccess(SubmitSharesSuccess),
    SubmitSharesError(SubmitSharesError),
    Reconnect(ReconnectMsg),
}

/// Setup connection request (miner → pool)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupConnection {
    pub protocol_version: u16,
    pub min_version: u16,
    pub max_version: u16,
    /// Flags: 0x01 = wants work selection, 0x02 = wants version rolling
    pub flags: u32,
    pub endpoint_host: String,
    pub endpoint_port: u16,
    pub vendor: String,
    pub hardware_version: String,
    pub firmware: String,
    pub device_id: String,
    /// Worker name/identifier
    pub worker_name: String,
}

/// Setup connection success (pool → miner)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupConnectionSuccess {
    pub used_version: u16,
    pub flags: u32,
    /// Assigned connection ID
    pub connection_id: u32,
}

/// Setup connection error (pool → miner)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupConnectionError {
    pub flags: u32,
    pub error_code: String,
}

/// New mining job (pool → miner)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMiningJob {
    /// Unique job identifier
    pub job_id: u32,
    /// Block version
    pub version: u32,
    /// Previous block hash (64 bytes, SHA-512)
    #[serde(with = "hex_64")]
    pub prev_hash: [u8; 64],
    /// Merkle root (64 bytes, SHA-512)
    #[serde(with = "hex_64")]
    pub merkle_root: [u8; 64],
    /// Block timestamp
    pub timestamp: u64,
    /// Difficulty target in compact bits
    pub nbits: u32,
    /// Coinbase transaction (hex-encoded)
    pub coinbase_tx: String,
    /// Whether this job replaces all previous jobs
    pub clean_jobs: bool,
    /// Block height
    pub height: u64,
    /// Coinbase reward (crystals)
    pub coinbase_value: u64,
}

/// Set share target (pool → miner)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetTarget {
    /// Share difficulty target (64 bytes, SHA-512)
    #[serde(with = "hex_64")]
    pub target: [u8; 64],
}

/// Set new previous block hash (pool → miner) — sent on new block found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetNewPrevHash {
    /// New previous block hash (64 bytes, SHA-512)
    #[serde(with = "hex_64")]
    pub prev_hash: [u8; 64],
    /// New target nbits
    pub nbits: u32,
    /// Job ID this applies to (0 = all jobs)
    pub job_id: u32,
}

/// Submit shares (miner → pool)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitSharesStandard {
    /// Job ID
    pub job_id: u32,
    /// Nonce that produces valid share
    pub nonce: u64,
    /// Timestamp used in block header
    pub ntime: u64,
    /// Version bits (if version rolling enabled)
    pub version: u32,
}

/// Share accepted (pool → miner)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitSharesSuccess {
    /// Sequence number of accepted share
    pub sequence_number: u64,
    /// New difficulty target if adjusted (optional)
    #[serde(with = "option_hex_64")]
    pub new_target: Option<[u8; 64]>,
}

/// Share rejected (pool → miner)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitSharesError {
    /// Sequence number
    pub sequence_number: u64,
    /// Error code string
    pub error_code: String,
}

/// Reconnect request (pool → miner)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectMsg {
    /// New pool hostname
    pub new_host: String,
    /// New pool port
    pub new_port: u16,
}

// ── Hex serialization helpers for [u8; 64] ────────────────────────

mod hex_64 {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        let mut arr = [0u8; 64];
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom(format!(
                "expected 64 bytes, got {}",
                bytes.len()
            )));
        }
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}

mod option_hex_64 {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(opt: &Option<[u8; 64]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match opt {
            Some(bytes) => serializer.serialize_some(&hex::encode(bytes)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<[u8; 64]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        match opt {
            Some(s) => {
                let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
                let mut arr = [0u8; 64];
                if bytes.len() != 64 {
                    return Err(serde::de::Error::custom(format!(
                        "expected 64 bytes, got {}",
                        bytes.len()
                    )));
                }
                arr.copy_from_slice(&bytes);
                Ok(Some(arr))
            }
            None => Ok(None),
        }
    }
}

/// Pool connection statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PoolStats {
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub stale_shares: u64,
    pub blocks_found: u64,
    pub connection_uptime_secs: u64,
    pub current_difficulty: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_roundtrip() {
        assert_eq!(MessageType::from_u16(0x0010), Some(MessageType::NewMiningJob));
        assert_eq!(MessageType::from_u16(0xFFFF), None);
    }

    #[test]
    fn test_setup_connection_serialize() {
        let msg = SetupConnection {
            protocol_version: STRATUM_PROTOCOL_VERSION,
            min_version: 1,
            max_version: 1,
            flags: 0,
            endpoint_host: "pool.btpc.org".to_string(),
            endpoint_port: 3333,
            vendor: "BTPC Desktop".to_string(),
            hardware_version: "1.0".to_string(),
            firmware: "0.1.0".to_string(),
            device_id: "test-001".to_string(),
            worker_name: "worker1".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let _: SetupConnection = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_new_mining_job_serialize() {
        let msg = NewMiningJob {
            job_id: 1,
            version: 1,
            prev_hash: [0xAB; 64],
            merkle_root: [0xCD; 64],
            timestamp: 1700000000,
            nbits: 0x3c7fffff,
            coinbase_tx: "01000000...".to_string(),
            clean_jobs: true,
            height: 100,
            coinbase_value: 3_237_500_000,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: NewMiningJob = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.job_id, 1);
        assert_eq!(decoded.prev_hash, [0xAB; 64]);
    }

    #[test]
    fn test_pool_stats_default() {
        let stats = PoolStats::default();
        assert_eq!(stats.accepted_shares, 0);
        assert_eq!(stats.rejected_shares, 0);
    }
}
