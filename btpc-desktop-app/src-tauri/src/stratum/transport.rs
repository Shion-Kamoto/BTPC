//! Noise-encrypted transport for Stratum V2-BTPC
//!
//! Uses Noise_NX_25519_ChaChaPoly_SHA256 for encrypted communication
//! between miner and pool, as specified by Stratum V2.

use anyhow::{Context, Result};
use snow::{Builder, TransportState};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

/// Noise protocol pattern for Stratum V2
const NOISE_PATTERN: &str = "Noise_NX_25519_ChaChaPoly_SHA256";

/// Maximum message size (64 KB)
const MAX_MESSAGE_SIZE: usize = 65535;

/// Noise-encrypted TCP transport
pub struct NoiseTransport {
    stream: Arc<Mutex<TcpStream>>,
    noise: Arc<Mutex<TransportState>>,
}

impl NoiseTransport {
    /// Perform Noise NX handshake as initiator (miner side)
    ///
    /// The NX pattern:
    /// - Initiator (miner) has no static key
    /// - Responder (pool) authenticates with its static key
    /// - Miner verifies pool identity, pool doesn't verify miner
    pub async fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr)
            .await
            .context("Failed to connect to pool")?;

        let builder = Builder::new(NOISE_PATTERN.parse().context("Invalid noise pattern")?);
        let mut handshake = builder
            .build_initiator()
            .context("Failed to build noise initiator")?;

        let mut buf = [0u8; MAX_MESSAGE_SIZE];

        // → e (send ephemeral key)
        let len = handshake
            .write_message(&[], &mut buf)
            .context("Noise handshake write failed")?;
        Self::send_raw(&stream, &buf[..len]).await?;

        // ← e, ee, s, es (receive pool's response)
        let received = Self::recv_raw(&stream).await?;
        handshake
            .read_message(&received, &mut buf)
            .context("Noise handshake read failed")?;

        let noise = handshake
            .into_transport_mode()
            .context("Failed to enter transport mode")?;

        Ok(NoiseTransport {
            stream: Arc::new(Mutex::new(stream)),
            noise: Arc::new(Mutex::new(noise)),
        })
    }

    /// Send an encrypted message
    pub async fn send(&self, plaintext: &[u8]) -> Result<()> {
        let mut buf = vec![0u8; plaintext.len() + 64]; // Extra for AEAD tag
        let len = {
            let mut noise = self.noise.lock().await;
            noise
                .write_message(plaintext, &mut buf)
                .context("Noise encrypt failed")?
        };
        let stream = self.stream.lock().await;
        Self::send_raw(&stream, &buf[..len]).await
    }

    /// Receive and decrypt a message
    pub async fn recv(&self) -> Result<Vec<u8>> {
        let ciphertext = {
            let stream = self.stream.lock().await;
            Self::recv_raw(&stream).await?
        };
        let mut buf = vec![0u8; ciphertext.len()];
        let len = {
            let mut noise = self.noise.lock().await;
            noise
                .read_message(&ciphertext, &mut buf)
                .context("Noise decrypt failed")?
        };
        buf.truncate(len);
        Ok(buf)
    }

    /// Send raw bytes with 2-byte length prefix
    async fn send_raw(stream: &TcpStream, data: &[u8]) -> Result<()> {
        let len = data.len() as u16;
        let len_bytes = len.to_le_bytes();

        // TcpStream requires &mut for write but we have shared ref via Mutex guard
        // Use the underlying socket directly
        stream.writable().await.context("Stream not writable")?;
        let mut written = 0;
        // Write length prefix
        while written < 2 {
            match stream.try_write(&len_bytes[written..]) {
                Ok(n) => written += n,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    stream.writable().await?;
                }
                Err(e) => return Err(e.into()),
            }
        }
        // Write payload
        written = 0;
        while written < data.len() {
            match stream.try_write(&data[written..]) {
                Ok(n) => written += n,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    stream.writable().await?;
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    /// Receive raw bytes with 2-byte length prefix
    async fn recv_raw(stream: &TcpStream) -> Result<Vec<u8>> {
        let mut len_buf = [0u8; 2];
        let mut read = 0;
        while read < 2 {
            stream.readable().await?;
            match stream.try_read(&mut len_buf[read..]) {
                Ok(0) => return Err(anyhow::anyhow!("Connection closed")),
                Ok(n) => read += n,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(e) => return Err(e.into()),
            }
        }
        let len = u16::from_le_bytes(len_buf) as usize;
        if len > MAX_MESSAGE_SIZE {
            return Err(anyhow::anyhow!("Message too large: {} bytes", len));
        }

        let mut buf = vec![0u8; len];
        read = 0;
        while read < len {
            stream.readable().await?;
            match stream.try_read(&mut buf[read..]) {
                Ok(0) => return Err(anyhow::anyhow!("Connection closed")),
                Ok(n) => read += n,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(buf)
    }

    /// Check if the connection is still alive
    pub async fn is_connected(&self) -> bool {
        let stream = self.stream.lock().await;
        stream.peer_addr().is_ok()
    }
}

/// Plaintext transport (for testing or when encryption is not needed)
pub struct PlaintextTransport {
    stream: Arc<Mutex<TcpStream>>,
}

impl PlaintextTransport {
    pub async fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr)
            .await
            .context("Failed to connect to pool")?;
        Ok(PlaintextTransport {
            stream: Arc::new(Mutex::new(stream)),
        })
    }

    pub async fn send(&self, data: &[u8]) -> Result<()> {
        let mut stream = self.stream.lock().await;
        let len = (data.len() as u16).to_le_bytes();
        stream.write_all(&len).await?;
        stream.write_all(data).await?;
        Ok(())
    }

    pub async fn recv(&self) -> Result<Vec<u8>> {
        let mut stream = self.stream.lock().await;
        let mut len_buf = [0u8; 2];
        stream.read_exact(&mut len_buf).await?;
        let len = u16::from_le_bytes(len_buf) as usize;
        if len > MAX_MESSAGE_SIZE {
            return Err(anyhow::anyhow!("Message too large: {} bytes", len));
        }
        let mut buf = vec![0u8; len];
        stream.read_exact(&mut buf).await?;
        Ok(buf)
    }
}
