//! Binary codec for BTPC-adapted Stratum V2 messages
//!
//! Frame format:
//! ```text
//! [2 bytes: message type] [4 bytes: payload length (LE)] [N bytes: JSON payload]
//! ```
//!
//! JSON payload is used for simplicity and debuggability during the initial
//! implementation. Future versions may switch to pure binary encoding.

use anyhow::{Context, Result};
use bytes::{Buf, BufMut, BytesMut};

use super::messages::{MessageType, StratumMessage};

/// Minimum frame header size (2 type + 4 length)
const HEADER_SIZE: usize = 6;

/// Maximum payload size (1 MB)
const MAX_PAYLOAD_SIZE: u32 = 1_048_576;

/// Encode a StratumMessage into a binary frame
pub fn encode(msg: &StratumMessage) -> Result<Vec<u8>> {
    let (msg_type, payload) = match msg {
        StratumMessage::SetupConnection(m) => {
            (MessageType::SetupConnection, serde_json::to_vec(m)?)
        }
        StratumMessage::SetupConnectionSuccess(m) => {
            (MessageType::SetupConnectionSuccess, serde_json::to_vec(m)?)
        }
        StratumMessage::SetupConnectionError(m) => {
            (MessageType::SetupConnectionError, serde_json::to_vec(m)?)
        }
        StratumMessage::NewMiningJob(m) => (MessageType::NewMiningJob, serde_json::to_vec(m)?),
        StratumMessage::SetTarget(m) => (MessageType::SetTarget, serde_json::to_vec(m)?),
        StratumMessage::SetNewPrevHash(m) => (MessageType::SetNewPrevHash, serde_json::to_vec(m)?),
        StratumMessage::SubmitSharesStandard(m) => {
            (MessageType::SubmitSharesStandard, serde_json::to_vec(m)?)
        }
        StratumMessage::SubmitSharesSuccess(m) => {
            (MessageType::SubmitSharesSuccess, serde_json::to_vec(m)?)
        }
        StratumMessage::SubmitSharesError(m) => {
            (MessageType::SubmitSharesError, serde_json::to_vec(m)?)
        }
        StratumMessage::Reconnect(m) => (MessageType::Reconnect, serde_json::to_vec(m)?),
    };

    let mut buf = BytesMut::with_capacity(HEADER_SIZE + payload.len());
    buf.put_u16_le(msg_type as u16);
    buf.put_u32_le(payload.len() as u32);
    buf.put_slice(&payload);
    Ok(buf.to_vec())
}

/// Decode a binary frame into a StratumMessage
///
/// Returns `None` if the buffer doesn't contain a complete frame.
/// Advances the buffer past the consumed frame on success.
pub fn decode(buf: &mut BytesMut) -> Result<Option<StratumMessage>> {
    if buf.len() < HEADER_SIZE {
        return Ok(None);
    }

    // Peek at header without consuming
    let msg_type_raw = u16::from_le_bytes([buf[0], buf[1]]);
    let payload_len = u32::from_le_bytes([buf[2], buf[3], buf[4], buf[5]]);

    if payload_len > MAX_PAYLOAD_SIZE {
        return Err(anyhow::anyhow!(
            "Payload too large: {} bytes (max {})",
            payload_len,
            MAX_PAYLOAD_SIZE
        ));
    }

    let total_len = HEADER_SIZE + payload_len as usize;
    if buf.len() < total_len {
        return Ok(None); // Need more data
    }

    // Consume the frame
    buf.advance(HEADER_SIZE);
    let payload = buf.split_to(payload_len as usize);

    let msg_type = MessageType::from_u16(msg_type_raw)
        .context(format!("Unknown message type: 0x{:04x}", msg_type_raw))?;

    let message = match msg_type {
        MessageType::SetupConnection => {
            StratumMessage::SetupConnection(serde_json::from_slice(&payload)?)
        }
        MessageType::SetupConnectionSuccess => {
            StratumMessage::SetupConnectionSuccess(serde_json::from_slice(&payload)?)
        }
        MessageType::SetupConnectionError => {
            StratumMessage::SetupConnectionError(serde_json::from_slice(&payload)?)
        }
        MessageType::NewMiningJob => {
            StratumMessage::NewMiningJob(serde_json::from_slice(&payload)?)
        }
        MessageType::SetTarget => StratumMessage::SetTarget(serde_json::from_slice(&payload)?),
        MessageType::SetNewPrevHash => {
            StratumMessage::SetNewPrevHash(serde_json::from_slice(&payload)?)
        }
        MessageType::SubmitSharesStandard => {
            StratumMessage::SubmitSharesStandard(serde_json::from_slice(&payload)?)
        }
        MessageType::SubmitSharesSuccess => {
            StratumMessage::SubmitSharesSuccess(serde_json::from_slice(&payload)?)
        }
        MessageType::SubmitSharesError => {
            StratumMessage::SubmitSharesError(serde_json::from_slice(&payload)?)
        }
        MessageType::Reconnect => StratumMessage::Reconnect(serde_json::from_slice(&payload)?),
    };

    Ok(Some(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let msg = StratumMessage::SetupConnectionSuccess(
            super::super::messages::SetupConnectionSuccess {
                used_version: 1,
                flags: 0,
                connection_id: 42,
            },
        );

        let encoded = encode(&msg).unwrap();
        assert!(encoded.len() >= HEADER_SIZE);

        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = decode(&mut buf).unwrap().unwrap();

        match decoded {
            StratumMessage::SetupConnectionSuccess(m) => {
                assert_eq!(m.connection_id, 42);
                assert_eq!(m.used_version, 1);
            }
            _ => panic!("Wrong message type decoded"),
        }

        // Buffer should be empty after consuming
        assert!(buf.is_empty());
    }

    #[test]
    fn test_incomplete_frame() {
        let msg = StratumMessage::SetupConnectionSuccess(
            super::super::messages::SetupConnectionSuccess {
                used_version: 1,
                flags: 0,
                connection_id: 1,
            },
        );
        let encoded = encode(&msg).unwrap();

        // Give only partial data
        let mut buf = BytesMut::from(&encoded[..HEADER_SIZE - 1]);
        assert!(decode(&mut buf).unwrap().is_none());

        // Give header but incomplete payload
        let mut buf = BytesMut::from(&encoded[..HEADER_SIZE + 1]);
        assert!(decode(&mut buf).unwrap().is_none());
    }
}
