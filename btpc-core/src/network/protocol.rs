//! Bitcoin-compatible P2P protocol implementation
//!
//! Implements the Bitcoin wire protocol with quantum-resistant enhancements for BTPC.

use std::{
    net::IpAddr,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    blockchain::{Block, BlockHeader, Transaction},
    crypto::Hash,
};

/// Bitcoin protocol version
pub const PROTOCOL_VERSION: u32 = 70015;

/// BTPC network magic bytes (Issue #13 - Reviewed for collision safety)
///
/// Magic bytes are used to identify BTPC network messages and prevent
/// cross-network connections. These values were chosen to be:
///
/// 1. **Unique from Bitcoin networks:**
///    - Bitcoin Mainnet: 0xF9BEB4D9 (differs significantly)
///    - Bitcoin Testnet:  0x0B110907 (differs significantly)
///
/// 2. **Unique from other major cryptocurrencies:**
///    - Litecoin Mainnet: 0xFBC0B6DB
///    - Dogecoin Mainnet: 0xC0C0C0C0
///    - The chosen values have low collision probability
///
/// 3. **Internally consistent:**
///    - Mainnet (0xB7C01A55): "B7" for BTPC, "C0" for "Co" (quantum-safe)
///    - Testnet (0xF1C0BA55): "F1" variant, maintains "C0BA" pattern
///
/// 4. **Easy to identify in hex dumps:**
///    - Both contain recognizable patterns (0xC0, 0x55)
///    - Distinct enough to avoid accidental confusion
///
/// **Security:** Magic bytes are the first line of defense against
/// accidental cross-network connections. These values prevent BTPC nodes
/// from accidentally connecting to Bitcoin or other cryptocurrency networks.
pub const BTPC_MAINNET_MAGIC: [u8; 4] = [0xB7, 0xC0, 0x1A, 0x55];
pub const BTPC_TESTNET_MAGIC: [u8; 4] = [0xF1, 0xC0, 0xBA, 0x55];

/// Maximum message size (32MB) - deprecated in favor of message-specific limits
pub const MAX_MESSAGE_SIZE: usize = 32 * 1024 * 1024;

/// Message-specific size limits (Issue #7 - DoS prevention)
/// These limits are based on Bitcoin's protocol + BTPC's 1MB block size
/// Maximum block message size: 2MB (allows 1MB block + overhead + quantum signatures)
pub const MAX_BLOCK_MESSAGE_SIZE: usize = 2 * 1024 * 1024;

/// Maximum transaction message size: 100KB per transaction
pub const MAX_TX_MESSAGE_SIZE: usize = 100 * 1024;

/// Maximum inventory message size: 50K inventory items * 36 bytes each
/// (Bitcoin limit: 50,000 items per inv message)
pub const MAX_INV_MESSAGE_SIZE: usize = 50_000 * 36;

/// Maximum headers message size: 2K headers * 81 bytes each
/// (Bitcoin sends max 2,000 headers per message)
pub const MAX_HEADERS_MESSAGE_SIZE: usize = 2_000 * 81;

/// Maximum address message size: 1K addresses * 30 bytes each
/// (Bitcoin limit: 1,000 addresses per addr message)
pub const MAX_ADDR_MESSAGE_SIZE: usize = 1_000 * 30;

/// Maximum version message size: 256 bytes (ample for all fields)
pub const MAX_VERSION_MESSAGE_SIZE: usize = 256;

/// Maximum generic message size: 1MB for unknown/simple message types
pub const MAX_GENERIC_MESSAGE_SIZE: usize = 1024 * 1024;

/// Maximum inventory items per message (Issue #10 - inventory announcement limits)
/// Bitcoin limit: 50,000 items per inv/getdata/notfound message
pub const MAX_INV_ITEMS: usize = 50_000;

/// Node services flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceFlags(pub u64);

impl ServiceFlags {
    /// Node can serve full blocks
    pub const NETWORK: ServiceFlags = ServiceFlags(1);
    /// Node supports bloom filters
    pub const BLOOM: ServiceFlags = ServiceFlags(1 << 2);
    /// Node supports witness data
    pub const WITNESS: ServiceFlags = ServiceFlags(1 << 3);
    /// Node supports network address time
    pub const NETWORK_LIMITED: ServiceFlags = ServiceFlags(1 << 10);
}

/// Network address with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAddress {
    /// Timestamp when address was seen
    pub time: u32,
    /// Services provided by this node
    pub services: u64,
    /// IP address
    pub ip: IpAddr,
    /// Port number
    pub port: u16,
}

impl NetworkAddress {
    /// Create a new network address
    pub fn new(ip: IpAddr, port: u16, services: ServiceFlags) -> Self {
        NetworkAddress {
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as u32,
            services: services.0,
            ip,
            port,
        }
    }
}

impl Default for NetworkAddress {
    fn default() -> Self {
        use std::net::Ipv4Addr;
        NetworkAddress {
            time: 0,
            services: 0,
            ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            port: 0,
        }
    }
}

/// Bitcoin protocol message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Version handshake message
    Version(VersionMessage),
    /// Version acknowledgment
    VerAck,
    /// Ping keep-alive
    Ping(u64),
    /// Pong response
    Pong(u64),
    /// Request peer addresses
    GetAddr,
    /// Peer addresses response
    Addr(Vec<NetworkAddress>),
    /// Inventory vector
    Inv(Vec<InventoryVector>),
    /// Get data request
    GetData(Vec<InventoryVector>),
    /// Block data
    Block(Block),
    /// Transaction data
    Tx(Transaction),
    /// Block headers request
    GetHeaders(GetHeadersMessage),
    /// Block headers response
    Headers(Vec<BlockHeader>),
    /// Reject message
    Reject(RejectMessage),
    /// Memory pool request
    MemPool,
    /// Not found response
    NotFound(Vec<InventoryVector>),
    /// Send headers preference
    SendHeaders,
}

impl Message {
    /// Get the maximum allowed size for this message type
    ///
    /// Returns message-specific size limits to prevent DoS attacks
    /// via oversized messages (Issue #7 - MEDIUM severity)
    pub fn max_size(&self) -> usize {
        match self {
            Message::Block(_) => MAX_BLOCK_MESSAGE_SIZE,
            Message::Tx(_) => MAX_TX_MESSAGE_SIZE,
            Message::Inv(_) | Message::GetData(_) | Message::NotFound(_) => MAX_INV_MESSAGE_SIZE,
            Message::Headers(_) => MAX_HEADERS_MESSAGE_SIZE,
            Message::Addr(_) => MAX_ADDR_MESSAGE_SIZE,
            Message::Version(_) => MAX_VERSION_MESSAGE_SIZE,
            // Simple control messages (small, fixed size)
            Message::VerAck | Message::Ping(_) | Message::Pong(_) |
            Message::GetAddr | Message::MemPool | Message::SendHeaders => 32, // 32 bytes max
            // Complex messages with variable size
            Message::GetHeaders(_) | Message::Reject(_) => MAX_GENERIC_MESSAGE_SIZE,
        }
    }

    /// Get the command name for this message type
    pub fn command(&self) -> &'static str {
        match self {
            Message::Version(_) => "version",
            Message::VerAck => "verack",
            Message::Ping(_) => "ping",
            Message::Pong(_) => "pong",
            Message::GetAddr => "getaddr",
            Message::Addr(_) => "addr",
            Message::Inv(_) => "inv",
            Message::GetData(_) => "getdata",
            Message::Block(_) => "block",
            Message::Tx(_) => "tx",
            Message::GetHeaders(_) => "getheaders",
            Message::Headers(_) => "headers",
            Message::Reject(_) => "reject",
            Message::MemPool => "mempool",
            Message::NotFound(_) => "notfound",
            Message::SendHeaders => "sendheaders",
        }
    }
}

/// Version handshake message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMessage {
    /// Protocol version
    pub version: u32,
    /// Services provided by sender
    pub services: u64,
    /// Timestamp
    pub timestamp: i64,
    /// Receiver's address
    pub addr_recv: NetworkAddress,
    /// Sender's address
    pub addr_from: NetworkAddress,
    /// Random nonce
    pub nonce: u64,
    /// User agent string
    pub user_agent: String,
    /// Latest block height
    pub start_height: u32,
    /// Relay transactions flag
    pub relay: bool,
}

impl VersionMessage {
    /// Create a new version message
    pub fn new(
        services: ServiceFlags,
        addr_recv: NetworkAddress,
        addr_from: NetworkAddress,
        user_agent: String,
        start_height: u32,
        relay: bool,
    ) -> Self {
        VersionMessage {
            version: PROTOCOL_VERSION,
            services: services.0,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            addr_recv,
            addr_from,
            nonce: rand::random(),
            user_agent,
            start_height,
            relay,
        }
    }
}

/// Inventory vector for announcing objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryVector {
    /// Object type
    pub inv_type: InvType,
    /// Object hash
    pub hash: Hash,
}

/// Inventory object types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u32)]
pub enum InvType {
    /// Transaction
    Tx = 1,
    /// Block
    Block = 2,
    /// Filtered block (merkle block)
    FilteredBlock = 3,
    /// Compact block
    CompactBlock = 4,
}

/// Get headers message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetHeadersMessage {
    /// Protocol version
    pub version: u32,
    /// Block locator hashes
    pub block_locator: Vec<Hash>,
    /// Hash stop (zero to get as many as possible)
    pub hash_stop: Hash,
}

/// Reject message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectMessage {
    /// Message type being rejected
    pub message: String,
    /// Rejection code
    pub code: RejectCode,
    /// Rejection reason
    pub reason: String,
    /// Extra data (optional)
    pub data: Option<Vec<u8>>,
}

/// Rejection codes
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum RejectCode {
    /// Malformed message
    Malformed = 0x01,
    /// Invalid transaction/block
    Invalid = 0x10,
    /// Obsolete version
    Obsolete = 0x11,
    /// Duplicate
    Duplicate = 0x12,
    /// Non-standard transaction
    Nonstandard = 0x40,
    /// Insufficient fee
    InsufficientFee = 0x42,
    /// Checkpoint mismatch
    Checkpoint = 0x43,
}

/// Message header for Bitcoin protocol
#[derive(Debug, Clone)]
pub struct MessageHeader {
    /// Network magic bytes
    pub magic: [u8; 4],
    /// Command name
    pub command: String,
    /// Payload length
    pub length: u32,
    /// Payload checksum
    pub checksum: [u8; 4],
}

impl MessageHeader {
    /// Create a new message header
    pub fn new(magic: [u8; 4], command: &str, payload: &[u8]) -> Self {
        let checksum = Self::calculate_checksum(payload);
        MessageHeader {
            magic,
            command: command.to_string(),
            length: payload.len() as u32,
            checksum,
        }
    }

    /// Calculate SHA256 double hash checksum
    fn calculate_checksum(payload: &[u8]) -> [u8; 4] {
        use sha2::{Digest, Sha256};

        let first_hash = Sha256::digest(payload);
        let second_hash = Sha256::digest(first_hash);

        let mut checksum = [0u8; 4];
        checksum.copy_from_slice(&second_hash[..4]);
        checksum
    }

    /// Verify checksum matches payload
    pub fn verify_checksum(&self, payload: &[u8]) -> bool {
        let expected_checksum = Self::calculate_checksum(payload);
        self.checksum == expected_checksum
    }
}

/// Protocol errors
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid magic bytes")]
    InvalidMagic,
    #[error("Invalid magic bytes")]
    InvalidMagicBytes,
    #[error("Invalid message format")]
    InvalidFormat,
    #[error("Invalid handshake")]
    InvalidHandshake,
    #[error("Message too large: {0} bytes")]
    MessageTooLarge(usize),
    #[error("Message type '{command}' exceeds size limit: {size} bytes (max: {max} bytes)")]
    MessageTypeTooLarge {
        command: String,
        size: usize,
        max: usize,
    },
    #[error("Invalid checksum")]
    InvalidChecksum,
    #[error("Unknown command: {0}")]
    UnknownCommand(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Protocol version mismatch")]
    VersionMismatch,
    #[error("Too many inventory items: {count} (max: {max})")]
    TooManyInventoryItems { count: usize, max: usize },
}

/// Bitcoin protocol codec for encoding/decoding messages
pub struct ProtocolCodec {
    magic: [u8; 4],
}

impl ProtocolCodec {
    /// Create a new protocol codec
    pub fn new(magic: [u8; 4]) -> Self {
        ProtocolCodec { magic }
    }

    /// Encode a message to bytes
    pub fn encode_message(&self, message: &Message) -> Result<Vec<u8>, ProtocolError> {
        let (command, payload) = self.serialize_message(message)?;
        let header = MessageHeader::new(self.magic, &command, &payload);

        let mut encoded = Vec::new();
        encoded.extend_from_slice(&header.magic);

        // Command name (12 bytes, null-padded)
        let mut cmd_bytes = [0u8; 12];
        let cmd_slice = command.as_bytes();
        if cmd_slice.len() > 12 {
            return Err(ProtocolError::InvalidFormat);
        }
        cmd_bytes[..cmd_slice.len()].copy_from_slice(cmd_slice);
        encoded.extend_from_slice(&cmd_bytes);

        // Length and checksum
        encoded.extend_from_slice(&header.length.to_le_bytes());
        encoded.extend_from_slice(&header.checksum);

        // Payload
        encoded.extend_from_slice(&payload);

        Ok(encoded)
    }

    /// Decode a message from bytes
    pub async fn decode_message<R>(&self, reader: &mut R) -> Result<Message, ProtocolError>
    where R: AsyncRead + Unpin {
        use tokio::io::AsyncReadExt;

        // Read header (24 bytes)
        let mut header_bytes = [0u8; 24];
        reader
            .read_exact(&mut header_bytes)
            .await
            .map_err(|e| ProtocolError::Io(e.to_string()))?;

        // Parse header
        let magic: [u8; 4] = header_bytes[0..4].try_into().unwrap();
        if magic != self.magic {
            return Err(ProtocolError::InvalidMagic);
        }

        let command_bytes = &header_bytes[4..16];
        let command = String::from_utf8_lossy(command_bytes)
            .trim_end_matches('\0')
            .to_string();

        let length = u32::from_le_bytes(header_bytes[16..20].try_into().unwrap());
        let checksum: [u8; 4] = header_bytes[20..24].try_into().unwrap();

        if length as usize > MAX_MESSAGE_SIZE {
            return Err(ProtocolError::MessageTooLarge(length as usize));
        }

        // Read payload
        let mut payload = vec![0u8; length as usize];
        if length > 0 {
            reader
                .read_exact(&mut payload)
                .await
                .map_err(|e| ProtocolError::Io(e.to_string()))?;
        }

        // Verify checksum
        let header = MessageHeader::new(magic, &command, &payload);
        if header.checksum != checksum {
            return Err(ProtocolError::InvalidChecksum);
        }

        // Deserialize message
        self.deserialize_message(&command, &payload)
    }

    /// Serialize message to command and payload
    fn serialize_message(&self, message: &Message) -> Result<(String, Vec<u8>), ProtocolError> {
        match message {
            Message::Version(msg) => {
                // Use bincode for version messages to stay under 256-byte limit
                let payload = bincode::serialize(msg)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Ok(("version".to_string(), payload))
            }
            Message::VerAck => Ok(("verack".to_string(), Vec::new())),
            Message::Ping(nonce) => {
                let payload = nonce.to_le_bytes().to_vec();
                Ok(("ping".to_string(), payload))
            }
            Message::Pong(nonce) => {
                let payload = nonce.to_le_bytes().to_vec();
                Ok(("pong".to_string(), payload))
            }
            Message::GetAddr => Ok(("getaddr".to_string(), Vec::new())),
            Message::Addr(addrs) => {
                let payload = serde_json::to_vec(addrs)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Ok(("addr".to_string(), payload))
            }
            Message::Inv(inv) => {
                let payload = serde_json::to_vec(inv)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Ok(("inv".to_string(), payload))
            }
            Message::GetData(inv) => {
                let payload = serde_json::to_vec(inv)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Ok(("getdata".to_string(), payload))
            }
            Message::Block(block) => {
                let payload = serde_json::to_vec(block)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Ok(("block".to_string(), payload))
            }
            Message::Tx(tx) => {
                let payload = serde_json::to_vec(tx)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Ok(("tx".to_string(), payload))
            }
            Message::GetHeaders(msg) => {
                let payload = serde_json::to_vec(msg)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Ok(("getheaders".to_string(), payload))
            }
            Message::Headers(headers) => {
                let payload = serde_json::to_vec(headers)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Ok(("headers".to_string(), payload))
            }
            Message::Reject(msg) => {
                let payload = serde_json::to_vec(msg)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Ok(("reject".to_string(), payload))
            }
            Message::MemPool => Ok(("mempool".to_string(), Vec::new())),
            Message::NotFound(inv) => {
                let payload = serde_json::to_vec(inv)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Ok(("notfound".to_string(), payload))
            }
            Message::SendHeaders => Ok(("sendheaders".to_string(), Vec::new())),
        }
    }

    /// Deserialize message from command and payload
    ///
    /// This validates message-specific size limits after deserialization
    /// to prevent DoS attacks via oversized messages (Issue #7)
    fn deserialize_message(&self, command: &str, payload: &[u8]) -> Result<Message, ProtocolError> {
        // Deserialize the message first
        let message = match command {
            "version" => {
                // Use bincode for version messages (compact binary format)
                let msg: VersionMessage = bincode::deserialize(payload)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Message::Version(msg)
            }
            "verack" => Message::VerAck,
            "ping" => {
                if payload.len() != 8 {
                    return Err(ProtocolError::InvalidFormat);
                }
                let nonce = u64::from_le_bytes(payload.try_into().unwrap());
                Message::Ping(nonce)
            }
            "pong" => {
                if payload.len() != 8 {
                    return Err(ProtocolError::InvalidFormat);
                }
                let nonce = u64::from_le_bytes(payload.try_into().unwrap());
                Message::Pong(nonce)
            }
            "getaddr" => Message::GetAddr,
            "addr" => {
                let addrs: Vec<NetworkAddress> = serde_json::from_slice(payload)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Message::Addr(addrs)
            }
            "inv" => {
                let inv: Vec<InventoryVector> = serde_json::from_slice(payload)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                // Issue #10 - Validate inventory item count
                if inv.len() > MAX_INV_ITEMS {
                    return Err(ProtocolError::TooManyInventoryItems {
                        count: inv.len(),
                        max: MAX_INV_ITEMS,
                    });
                }
                Message::Inv(inv)
            }
            "getdata" => {
                let inv: Vec<InventoryVector> = serde_json::from_slice(payload)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                // Issue #10 - Validate inventory item count
                if inv.len() > MAX_INV_ITEMS {
                    return Err(ProtocolError::TooManyInventoryItems {
                        count: inv.len(),
                        max: MAX_INV_ITEMS,
                    });
                }
                Message::GetData(inv)
            }
            "block" => {
                let block: Block = serde_json::from_slice(payload)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Message::Block(block)
            }
            "tx" => {
                let tx: Transaction = serde_json::from_slice(payload)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Message::Tx(tx)
            }
            "getheaders" => {
                let msg: GetHeadersMessage = serde_json::from_slice(payload)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Message::GetHeaders(msg)
            }
            "headers" => {
                let headers: Vec<BlockHeader> = serde_json::from_slice(payload)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Message::Headers(headers)
            }
            "reject" => {
                let msg: RejectMessage = serde_json::from_slice(payload)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                Message::Reject(msg)
            }
            "mempool" => Message::MemPool,
            "notfound" => {
                let inv: Vec<InventoryVector> = serde_json::from_slice(payload)
                    .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
                // Issue #10 - Validate inventory item count
                if inv.len() > MAX_INV_ITEMS {
                    return Err(ProtocolError::TooManyInventoryItems {
                        count: inv.len(),
                        max: MAX_INV_ITEMS,
                    });
                }
                Message::NotFound(inv)
            }
            "sendheaders" => Message::SendHeaders,
            _ => return Err(ProtocolError::UnknownCommand(command.to_string())),
        };

        // Validate message-specific size limit
        let max_size = message.max_size();
        if payload.len() > max_size {
            return Err(ProtocolError::MessageTypeTooLarge {
                command: command.to_string(),
                size: payload.len(),
                max: max_size,
            });
        }

        Ok(message)
    }
}

/// Peer connection handler
pub struct PeerConnection<S> {
    stream: S,
    codec: ProtocolCodec,
    version: Option<VersionMessage>,
    handshake_complete: bool,
}

impl<S> PeerConnection<S>
where S: AsyncRead + AsyncWrite + Unpin
{
    /// Create a new peer connection
    pub fn new(stream: S, codec: ProtocolCodec) -> Self {
        PeerConnection {
            stream,
            codec,
            version: None,
            handshake_complete: false,
        }
    }

    /// Send a message to the peer
    pub async fn send_message(&mut self, message: &Message) -> Result<(), ProtocolError> {
        use tokio::io::AsyncWriteExt;

        let encoded = self.codec.encode_message(message)?;
        self.stream
            .write_all(&encoded)
            .await
            .map_err(|e| ProtocolError::Io(e.to_string()))?;

        Ok(())
    }

    /// Receive a message from the peer
    pub async fn receive_message(&mut self) -> Result<Message, ProtocolError> {
        self.codec.decode_message(&mut self.stream).await
    }

    /// Perform handshake with peer
    pub async fn handshake(
        &mut self,
        our_version: VersionMessage,
    ) -> Result<VersionMessage, ProtocolError> {
        // Send our version
        self.send_message(&Message::Version(our_version)).await?;

        // Wait for their version
        let peer_version = match self.receive_message().await? {
            Message::Version(version) => {
                if version.version < PROTOCOL_VERSION {
                    return Err(ProtocolError::VersionMismatch);
                }
                version
            }
            _ => return Err(ProtocolError::InvalidFormat),
        };

        // Send verack
        self.send_message(&Message::VerAck).await?;

        // Wait for their verack
        match self.receive_message().await? {
            Message::VerAck => {}
            _ => return Err(ProtocolError::InvalidFormat),
        }

        self.version = Some(peer_version.clone());
        self.handshake_complete = true;

        Ok(peer_version)
    }

    /// Check if handshake is complete
    pub fn is_handshake_complete(&self) -> bool {
        self.handshake_complete
    }

    /// Get peer version if available
    pub fn peer_version(&self) -> Option<&VersionMessage> {
        self.version.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    #[test]
    fn test_service_flags() {
        let flags = ServiceFlags::NETWORK;
        assert_eq!(flags.0, 1);
    }

    #[test]
    fn test_network_address() {
        let addr = NetworkAddress::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8333,
            ServiceFlags::NETWORK,
        );
        assert_eq!(addr.ip, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(addr.port, 8333);
        assert_eq!(addr.services, 1);
    }

    #[test]
    fn test_message_header_checksum() {
        let payload = b"test payload";
        let header = MessageHeader::new(BTPC_MAINNET_MAGIC, "test", payload);
        assert!(header.verify_checksum(payload));
    }

    #[test]
    fn test_protocol_codec_ping_pong() {
        let codec = ProtocolCodec::new(BTPC_TESTNET_MAGIC);

        let ping_msg = Message::Ping(12345);
        let encoded = codec.encode_message(&ping_msg).unwrap();
        assert!(!encoded.is_empty());

        let pong_msg = Message::Pong(54321);
        let encoded = codec.encode_message(&pong_msg).unwrap();
        assert!(!encoded.is_empty());
    }

    // Issue #7 - Message-specific size limit tests
    #[test]
    fn test_message_max_size_constants() {
        // Verify message-specific size limits are set correctly
        assert_eq!(MAX_BLOCK_MESSAGE_SIZE, 2 * 1024 * 1024); // 2MB
        assert_eq!(MAX_TX_MESSAGE_SIZE, 100 * 1024); // 100KB
        assert_eq!(MAX_INV_MESSAGE_SIZE, 50_000 * 36); // 1.8MB
        assert_eq!(MAX_HEADERS_MESSAGE_SIZE, 2_000 * 81); // 162KB
        assert_eq!(MAX_ADDR_MESSAGE_SIZE, 1_000 * 30); // 30KB
        assert_eq!(MAX_VERSION_MESSAGE_SIZE, 256); // 256 bytes
        assert_eq!(MAX_GENERIC_MESSAGE_SIZE, 1 * 1024 * 1024); // 1MB
    }

    #[test]
    fn test_message_max_size_method() {
        use crate::crypto::Hash;

        // Control messages (small, fixed size)
        assert_eq!(Message::VerAck.max_size(), 32);
        assert_eq!(Message::Ping(12345).max_size(), 32);
        assert_eq!(Message::Pong(54321).max_size(), 32);
        assert_eq!(Message::GetAddr.max_size(), 32);
        assert_eq!(Message::MemPool.max_size(), 32);
        assert_eq!(Message::SendHeaders.max_size(), 32);

        // Version message
        let version_msg = Message::Version(VersionMessage {
            version: PROTOCOL_VERSION,
            services: 1,
            timestamp: 0,
            addr_recv: NetworkAddress::default(),
            addr_from: NetworkAddress::default(),
            nonce: 0,
            user_agent: "test".to_string(),
            start_height: 0,
            relay: true,
        });
        assert_eq!(version_msg.max_size(), MAX_VERSION_MESSAGE_SIZE);

        // Addr message
        assert_eq!(Message::Addr(vec![]).max_size(), MAX_ADDR_MESSAGE_SIZE);

        // Inventory messages
        assert_eq!(Message::Inv(vec![]).max_size(), MAX_INV_MESSAGE_SIZE);
        assert_eq!(Message::GetData(vec![]).max_size(), MAX_INV_MESSAGE_SIZE);
        assert_eq!(Message::NotFound(vec![]).max_size(), MAX_INV_MESSAGE_SIZE);

        // Headers message
        assert_eq!(Message::Headers(vec![]).max_size(), MAX_HEADERS_MESSAGE_SIZE);

        // GetHeaders message
        let getheaders = Message::GetHeaders(GetHeadersMessage {
            version: PROTOCOL_VERSION,
            block_locator: vec![],
            hash_stop: Hash::zero(),
        });
        assert_eq!(getheaders.max_size(), MAX_GENERIC_MESSAGE_SIZE);

        // Reject message
        let reject = Message::Reject(RejectMessage {
            message: "test".to_string(),
            code: RejectCode::Malformed,
            reason: "test".to_string(),
            data: None,
        });
        assert_eq!(reject.max_size(), MAX_GENERIC_MESSAGE_SIZE);
    }

    #[test]
    fn test_message_command_method() {
        use crate::crypto::Hash;

        // Test all message types return correct command names
        assert_eq!(Message::VerAck.command(), "verack");
        assert_eq!(Message::Ping(123).command(), "ping");
        assert_eq!(Message::Pong(456).command(), "pong");
        assert_eq!(Message::GetAddr.command(), "getaddr");
        assert_eq!(Message::MemPool.command(), "mempool");
        assert_eq!(Message::SendHeaders.command(), "sendheaders");

        let version_msg = Message::Version(VersionMessage {
            version: PROTOCOL_VERSION,
            services: 1,
            timestamp: 0,
            addr_recv: NetworkAddress::default(),
            addr_from: NetworkAddress::default(),
            nonce: 0,
            user_agent: "test".to_string(),
            start_height: 0,
            relay: true,
        });
        assert_eq!(version_msg.command(), "version");

        assert_eq!(Message::Addr(vec![]).command(), "addr");
        assert_eq!(Message::Inv(vec![]).command(), "inv");
        assert_eq!(Message::GetData(vec![]).command(), "getdata");
        assert_eq!(Message::NotFound(vec![]).command(), "notfound");
        assert_eq!(Message::Headers(vec![]).command(), "headers");

        let getheaders = Message::GetHeaders(GetHeadersMessage {
            version: PROTOCOL_VERSION,
            block_locator: vec![],
            hash_stop: Hash::zero(),
        });
        assert_eq!(getheaders.command(), "getheaders");

        let reject = Message::Reject(RejectMessage {
            message: "test".to_string(),
            code: RejectCode::Malformed,
            reason: "test".to_string(),
            data: None,
        });
        assert_eq!(reject.command(), "reject");
    }

    #[test]
    fn test_message_type_specific_size_validation() {
        let codec = ProtocolCodec::new(BTPC_TESTNET_MAGIC);

        // Test that oversized version message is rejected
        let oversized_version = VersionMessage {
            version: PROTOCOL_VERSION,
            services: 1,
            timestamp: 0,
            addr_recv: NetworkAddress::default(),
            addr_from: NetworkAddress::default(),
            nonce: 0,
            user_agent: "A".repeat(500), // 500 byte user agent -> exceeds 256 byte limit
            start_height: 0,
            relay: true,
        };

        let version_msg = Message::Version(oversized_version.clone());
        let payload = bincode::serialize(&oversized_version).unwrap(); // Use bincode to match implementation

        // Deserialize should fail due to size limit
        let result = codec.deserialize_message("version", &payload);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProtocolError::MessageTypeTooLarge { command, size, max } => {
                assert_eq!(command, "version");
                assert!(size > max);
                assert_eq!(max, MAX_VERSION_MESSAGE_SIZE);
            }
            _ => panic!("Expected MessageTypeTooLarge error"),
        }
    }

    #[test]
    fn test_oversized_addr_message_rejected() {
        let codec = ProtocolCodec::new(BTPC_TESTNET_MAGIC);

        // Create oversized addr message (more than 1K addresses)
        let oversized_addrs: Vec<NetworkAddress> = (0..2000)
            .map(|i| NetworkAddress {
                time: 0,
                services: 1,
                ip: IpAddr::V4(Ipv4Addr::new(127, 0, (i / 256) as u8, (i % 256) as u8)),
                port: 8333,
            })
            .collect();

        let payload = serde_json::to_vec(&oversized_addrs).unwrap();

        // Payload should exceed MAX_ADDR_MESSAGE_SIZE
        assert!(payload.len() > MAX_ADDR_MESSAGE_SIZE);

        let result = codec.deserialize_message("addr", &payload);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProtocolError::MessageTypeTooLarge { command, size, max } => {
                assert_eq!(command, "addr");
                assert_eq!(size, payload.len());
                assert_eq!(max, MAX_ADDR_MESSAGE_SIZE);
            }
            _ => panic!("Expected MessageTypeTooLarge error"),
        }
    }

    #[test]
    fn test_normal_sized_messages_accepted() {
        let codec = ProtocolCodec::new(BTPC_TESTNET_MAGIC);

        // Test normal-sized version message
        let version = VersionMessage {
            version: PROTOCOL_VERSION,
            services: 1,
            timestamp: 0,
            addr_recv: NetworkAddress::default(),
            addr_from: NetworkAddress::default(),
            nonce: 0,
            user_agent: "/BTPC:0.1.0/".to_string(), // Small, reasonable user agent
            start_height: 0,
            relay: true,
        };

        let payload = bincode::serialize(&version).unwrap(); // Use bincode to match implementation
        assert!(payload.len() <= MAX_VERSION_MESSAGE_SIZE);

        let result = codec.deserialize_message("version", &payload);
        assert!(result.is_ok());
        match result.unwrap() {
            Message::Version(msg) => {
                assert_eq!(msg.user_agent, "/BTPC:0.1.0/");
            }
            _ => panic!("Expected Version message"),
        }

        // Test normal-sized addr message (100 addresses)
        let normal_addrs: Vec<NetworkAddress> = (0..100)
            .map(|i| NetworkAddress {
                time: 0,
                services: 1,
                ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, i as u8)),
                port: 8333,
            })
            .collect();

        let payload = serde_json::to_vec(&normal_addrs).unwrap();
        assert!(payload.len() <= MAX_ADDR_MESSAGE_SIZE);

        let result = codec.deserialize_message("addr", &payload);
        assert!(result.is_ok());
        match result.unwrap() {
            Message::Addr(addrs) => {
                assert_eq!(addrs.len(), 100);
            }
            _ => panic!("Expected Addr message"),
        }
    }

    #[test]
    fn test_control_messages_size_validation() {
        let codec = ProtocolCodec::new(BTPC_TESTNET_MAGIC);

        // Control messages should have small sizes
        let result = codec.deserialize_message("verack", &[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().max_size(), 32);

        let ping_payload = 12345u64.to_le_bytes().to_vec();
        let result = codec.deserialize_message("ping", &ping_payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().max_size(), 32);

        let pong_payload = 54321u64.to_le_bytes().to_vec();
        let result = codec.deserialize_message("pong", &pong_payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().max_size(), 32);

        let result = codec.deserialize_message("getaddr", &[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().max_size(), 32);

        let result = codec.deserialize_message("mempool", &[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().max_size(), 32);

        let result = codec.deserialize_message("sendheaders", &[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().max_size(), 32);
    }

    // Issue #10 - Inventory announcement limit tests
    #[test]
    fn test_inv_item_limit_constant() {
        // Verify the constant is set to Bitcoin's 50K limit
        assert_eq!(MAX_INV_ITEMS, 50_000);
    }

    #[test]
    fn test_inv_message_item_count_limit() {
        use crate::crypto::Hash;

        let codec = ProtocolCodec::new(BTPC_TESTNET_MAGIC);

        // Create oversized inv message (more than 50K items)
        let oversized_inv: Vec<InventoryVector> = (0..60_000)
            .map(|i| InventoryVector {
                inv_type: InvType::Tx,
                hash: Hash::from_bytes([i as u8; 64]),
            })
            .collect();

        let payload = serde_json::to_vec(&oversized_inv).unwrap();

        // Should fail due to item count limit
        let result = codec.deserialize_message("inv", &payload);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProtocolError::TooManyInventoryItems { count, max } => {
                assert_eq!(count, 60_000);
                assert_eq!(max, MAX_INV_ITEMS);
            }
            _ => panic!("Expected TooManyInventoryItems error"),
        }
    }

    #[test]
    fn test_getdata_message_item_count_limit() {
        use crate::crypto::Hash;

        let codec = ProtocolCodec::new(BTPC_TESTNET_MAGIC);

        // Create oversized getdata message
        let oversized_getdata: Vec<InventoryVector> = (0..55_000)
            .map(|_| InventoryVector {
                inv_type: InvType::Block,
                hash: Hash::from_bytes([0u8; 64]),
            })
            .collect();

        let payload = serde_json::to_vec(&oversized_getdata).unwrap();

        let result = codec.deserialize_message("getdata", &payload);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProtocolError::TooManyInventoryItems { count, max } => {
                assert_eq!(count, 55_000);
                assert_eq!(max, MAX_INV_ITEMS);
            }
            _ => panic!("Expected TooManyInventoryItems error"),
        }
    }

    #[test]
    fn test_notfound_message_item_count_limit() {
        use crate::crypto::Hash;

        let codec = ProtocolCodec::new(BTPC_TESTNET_MAGIC);

        // Create oversized notfound message
        let oversized_notfound: Vec<InventoryVector> = (0..51_000)
            .map(|_| InventoryVector {
                inv_type: InvType::Tx,
                hash: Hash::from_bytes([1u8; 64]),
            })
            .collect();

        let payload = serde_json::to_vec(&oversized_notfound).unwrap();

        let result = codec.deserialize_message("notfound", &payload);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProtocolError::TooManyInventoryItems { count, max } => {
                assert_eq!(count, 51_000);
                assert_eq!(max, MAX_INV_ITEMS);
            }
            _ => panic!("Expected TooManyInventoryItems error"),
        }
    }

    #[test]
    fn test_normal_inv_messages_accepted() {
        use crate::crypto::Hash;

        let codec = ProtocolCodec::new(BTPC_TESTNET_MAGIC);

        // Test normal inv message (1000 items - well within limit)
        let normal_inv: Vec<InventoryVector> = (0..1000)
            .map(|i| InventoryVector {
                inv_type: InvType::Tx,
                hash: Hash::from_bytes([i as u8; 64]),
            })
            .collect();

        let payload = serde_json::to_vec(&normal_inv).unwrap();
        let result = codec.deserialize_message("inv", &payload);
        assert!(result.is_ok());
        match result.unwrap() {
            Message::Inv(inv) => {
                assert_eq!(inv.len(), 1000);
            }
            _ => panic!("Expected Inv message"),
        }

        // Test at exact item count limit (50K items)
        // Note: We use a smaller number to stay within the byte size limit
        let large_inv: Vec<InventoryVector> = (0..10_000)
            .map(|_| InventoryVector {
                inv_type: InvType::Block,
                hash: Hash::from_bytes([2u8; 64]),
            })
            .collect();

        let payload = serde_json::to_vec(&large_inv).unwrap();
        let result = codec.deserialize_message("inv", &payload);
        assert!(result.is_ok());
        match result.unwrap() {
            Message::Inv(inv) => {
                assert_eq!(inv.len(), 10_000);
            }
            _ => panic!("Expected Inv message"),
        }
    }
}
