//! RPC data types and serialization

use serde::{Deserialize, Serialize};

use crate::crypto::Hash;

/// Block information for RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcBlockInfo {
    pub hash: String,
    pub confirmations: u32,
    pub size: u32,
    pub height: u32,
    pub version: u32,
    #[serde(rename = "merkleroot")]
    pub merkle_root: String,
    pub tx: Vec<String>,
    pub time: u32,
    pub nonce: u32,
    pub bits: String,
    pub difficulty: f64,
    #[serde(rename = "previousblockhash")]
    pub previous_block_hash: String,
}

/// Block header information for RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcBlockHeader {
    pub hash: String,
    pub confirmations: u32,
    pub height: u32,
    pub version: u32,
    #[serde(rename = "merkleroot")]
    pub merkle_root: String,
    pub time: u32,
    pub nonce: u32,
    pub bits: String,
    pub difficulty: f64,
    #[serde(rename = "previousblockhash")]
    pub previous_block_hash: String,
}

/// Transaction output information for RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcTxOut {
    #[serde(rename = "bestblock")]
    pub best_block: String,
    pub confirmations: u32,
    pub value: f64,
    #[serde(rename = "scriptPubKey")]
    pub script_pub_key: RpcScriptPubKey,
    pub coinbase: bool,
}

/// Script public key information for RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcScriptPubKey {
    pub asm: String,
    pub hex: String,
    #[serde(rename = "type")]
    pub script_type: String,
    pub addresses: Option<Vec<String>>,
}

/// Blockchain information for RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcBlockchainInfo {
    pub chain: String,
    pub blocks: u32,
    pub headers: u32,
    #[serde(rename = "bestblockhash")]
    pub best_block_hash: String,
    pub difficulty: f64,
    #[serde(rename = "mediantime")]
    pub median_time: u32,
    #[serde(rename = "verificationprogress")]
    pub verification_progress: f64,
    #[serde(rename = "initialblockdownload")]
    pub initial_block_download: bool,
    #[serde(rename = "chainwork")]
    pub chain_work: String,
    #[serde(rename = "size_on_disk")]
    pub size_on_disk: u64,
    pub pruned: bool,
}

/// Network information for RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcNetworkInfo {
    pub version: u32,
    pub subversion: String,
    #[serde(rename = "protocolversion")]
    pub protocol_version: u32,
    #[serde(rename = "localservices")]
    pub local_services: String,
    #[serde(rename = "localservicesnames")]
    pub local_services_names: Vec<String>,
    #[serde(rename = "localrelay")]
    pub local_relay: bool,
    #[serde(rename = "timeoffset")]
    pub time_offset: i32,
    pub connections: u32,
    #[serde(rename = "networkactive")]
    pub network_active: bool,
    pub networks: Vec<RpcNetworkDetails>,
    #[serde(rename = "relayfee")]
    pub relay_fee: f64,
    #[serde(rename = "incrementalfee")]
    pub incremental_fee: f64,
    #[serde(rename = "localaddresses")]
    pub local_addresses: Vec<RpcLocalAddress>,
    pub warnings: String,
}

/// Network details for RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcNetworkDetails {
    pub name: String,
    pub limited: bool,
    pub reachable: bool,
    pub proxy: String,
    #[serde(rename = "proxy_randomize_credentials")]
    pub proxy_randomize_credentials: bool,
}

/// Local address information for RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcLocalAddress {
    pub address: String,
    pub port: u16,
    pub score: u32,
}

/// Peer information for RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcPeerInfo {
    pub id: u32,
    pub addr: String,
    #[serde(rename = "addrlocal")]
    pub addr_local: String,
    pub services: String,
    #[serde(rename = "servicesnames")]
    pub services_names: Vec<String>,
    #[serde(rename = "relaytxes")]
    pub relay_txes: bool,
    #[serde(rename = "lastsend")]
    pub last_send: u64,
    #[serde(rename = "lastrecv")]
    pub last_recv: u64,
    #[serde(rename = "bytessent")]
    pub bytes_sent: u64,
    #[serde(rename = "bytesrecv")]
    pub bytes_recv: u64,
    #[serde(rename = "conntime")]
    pub conn_time: u64,
    #[serde(rename = "timeoffset")]
    pub time_offset: i32,
    #[serde(rename = "pingtime")]
    pub ping_time: f64,
    #[serde(rename = "minping")]
    pub min_ping: f64,
    pub version: u32,
    pub subver: String,
    pub inbound: bool,
    #[serde(rename = "startingheight")]
    pub starting_height: u32,
    #[serde(rename = "banscore")]
    pub ban_score: u32,
    #[serde(rename = "synced_headers")]
    pub synced_headers: u32,
    #[serde(rename = "synced_blocks")]
    pub synced_blocks: u32,
}

/// Transaction information for RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcTransactionInfo {
    pub txid: String,
    pub hash: String,
    pub version: u32,
    pub size: u32,
    pub vsize: u32,
    pub weight: u32,
    #[serde(rename = "locktime")]
    pub lock_time: u32,
    pub vin: Vec<RpcTxInput>,
    pub vout: Vec<RpcTxOutput>,
    pub hex: String,
    #[serde(rename = "blockhash")]
    pub block_hash: Option<String>,
    pub confirmations: Option<u32>,
    pub time: Option<u32>,
    #[serde(rename = "blocktime")]
    pub block_time: Option<u32>,
}

/// Transaction input for RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcTxInput {
    pub txid: String,
    pub vout: u32,
    #[serde(rename = "scriptSig")]
    pub script_sig: RpcScriptSig,
    pub sequence: u32,
}

/// Script signature for RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcScriptSig {
    pub asm: String,
    pub hex: String,
}

/// Transaction output for RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcTxOutput {
    pub value: f64,
    pub n: u32,
    #[serde(rename = "scriptPubKey")]
    pub script_pub_key: RpcScriptPubKey,
}

/// Memory pool information for RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcMempoolInfo {
    pub size: u32,
    pub bytes: u32,
    pub usage: u32,
    #[serde(rename = "maxmempool")]
    pub max_mempool: u32,
    #[serde(rename = "mempoolminfee")]
    pub mempool_min_fee: f64,
    #[serde(rename = "minrelaytxfee")]
    pub min_relay_tx_fee: f64,
}

#[cfg(test)]
mod tests {
    use serde_json;

    use super::*;

    #[test]
    fn test_block_info_serialization() {
        let block_info = RpcBlockInfo {
            hash: "test_hash".to_string(),
            confirmations: 1,
            size: 1000,
            height: 100,
            version: 1,
            merkle_root: "merkle_root".to_string(),
            tx: vec!["tx1".to_string(), "tx2".to_string()],
            time: 1234567890,
            nonce: 12345,
            bits: "1d00ffff".to_string(),
            difficulty: 1.0,
            previous_block_hash: "prev_hash".to_string(),
        };

        let json = serde_json::to_string(&block_info).unwrap();
        let deserialized: RpcBlockInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(block_info.hash, deserialized.hash);
        assert_eq!(block_info.height, deserialized.height);
    }

    #[test]
    fn test_network_info_serialization() {
        let network_info = RpcNetworkInfo {
            version: 1000000,
            subversion: "/BTPC:0.1.0/".to_string(),
            protocol_version: 70015,
            local_services: "0000000000000001".to_string(),
            local_services_names: vec!["NETWORK".to_string()],
            local_relay: true,
            time_offset: 0,
            connections: 8,
            network_active: true,
            networks: vec![],
            relay_fee: 0.00001,
            incremental_fee: 0.00001,
            local_addresses: vec![],
            warnings: "".to_string(),
        };

        let json = serde_json::to_string(&network_info).unwrap();
        let deserialized: RpcNetworkInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(network_info.version, deserialized.version);
        assert_eq!(network_info.subversion, deserialized.subversion);
    }
}
