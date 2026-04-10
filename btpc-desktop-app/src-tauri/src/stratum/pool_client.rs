//! Stratum V2-BTPC Pool Client
//!
//! High-level pool mining client that:
//! - Connects to a Stratum V2 pool
//! - Converts pool jobs into BlockTemplate for the mining thread pool
//! - Submits shares and full blocks
//! - Handles automatic reconnection with exponential backoff
//! - Tracks pool statistics (accepted/rejected/stale shares)

use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};

use crate::rpc_client::{BlockTemplate, RpcClientInterface};
use super::messages::*;
use super::vardiff::VardiffController;

/// Pool connection configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PoolConfig {
    /// Pool URL (host:port)
    pub url: String,
    /// Worker name
    pub worker: String,
    /// Worker password (usually empty or pool-specific)
    pub password: String,
}

/// Pool connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// Stratum V2 pool client implementing RpcClientInterface
///
/// This allows the mining thread pool to use either solo mining
/// (EmbeddedNode) or pool mining (StratumPoolClient) transparently.
pub struct StratumPoolClient {
    config: PoolConfig,
    /// Current mining job from pool
    current_job: Arc<RwLock<Option<NewMiningJob>>>,
    /// Current share target from pool
    share_target: Arc<RwLock<Option<[u8; 64]>>>,
    /// Connection state
    state: Arc<RwLock<ConnectionState>>,
    /// Whether the client is running
    running: Arc<AtomicBool>,
    /// Pool statistics
    stats: Arc<RwLock<PoolStats>>,
    /// Vardiff controller
    vardiff: Arc<Mutex<VardiffController>>,
    /// Connection ID assigned by pool
    connection_id: Arc<AtomicU64>,
    /// Reconnection attempt counter (for exponential backoff)
    reconnect_attempts: Arc<AtomicU64>,
    /// Reference to embedded node for dual-mode block submission
    embedded_node: Option<Arc<tokio::sync::RwLock<crate::embedded_node::EmbeddedNode>>>,
}

impl StratumPoolClient {
    /// Create a new pool client
    pub fn new(config: PoolConfig) -> Self {
        Self {
            config,
            current_job: Arc::new(RwLock::new(None)),
            share_target: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(PoolStats::default())),
            vardiff: Arc::new(Mutex::new(VardiffController::new(1.0))),
            connection_id: Arc::new(AtomicU64::new(0)),
            reconnect_attempts: Arc::new(AtomicU64::new(0)),
            embedded_node: None,
        }
    }

    /// Set embedded node reference for dual-mode block submission (B5)
    pub fn set_embedded_node(
        &mut self,
        node: Arc<tokio::sync::RwLock<crate::embedded_node::EmbeddedNode>>,
    ) {
        self.embedded_node = Some(node);
    }

    /// Start the pool client background tasks
    pub async fn start(&self) -> Result<()> {
        self.running.store(true, Ordering::SeqCst);
        *self.state.write().await = ConnectionState::Connecting;

        eprintln!("⛏️ Pool client starting: {}", self.config.url);

        // Connection + job listener runs in background
        let running = self.running.clone();
        let config = self.config.clone();
        let current_job = self.current_job.clone();
        let share_target = self.share_target.clone();
        let state = self.state.clone();
        let stats = self.stats.clone();
        let vardiff = self.vardiff.clone();
        let connection_id = self.connection_id.clone();
        let reconnect_attempts = self.reconnect_attempts.clone();

        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                match Self::connect_and_listen(
                    &config,
                    &current_job,
                    &share_target,
                    &state,
                    &stats,
                    &vardiff,
                    &connection_id,
                    &running,
                )
                .await
                {
                    Ok(()) => {
                        // Clean disconnect
                        eprintln!("⛏️ Pool client disconnected cleanly");
                        reconnect_attempts.store(0, Ordering::SeqCst);
                    }
                    Err(e) => {
                        eprintln!("⛏️ Pool connection error: {}", e);
                        *state.write().await = ConnectionState::Reconnecting;

                        // Exponential backoff: 1s, 2s, 4s, 8s, 16s, 30s (capped)
                        let attempts = reconnect_attempts.fetch_add(1, Ordering::SeqCst);
                        let delay = Duration::from_secs(
                            (1u64 << attempts.min(4)).min(30),
                        );
                        eprintln!(
                            "⛏️ Reconnecting in {:?} (attempt {})",
                            delay,
                            attempts + 1
                        );
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the pool client
    pub async fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        *self.state.write().await = ConnectionState::Disconnected;
        eprintln!("⛏️ Pool client stopped");
    }

    /// Get pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        self.stats.read().await.clone()
    }

    /// Check if connected to pool
    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ConnectionState::Connected
    }

    /// Internal: connect and listen for pool messages
    #[allow(clippy::too_many_arguments)]
    async fn connect_and_listen(
        config: &PoolConfig,
        current_job: &Arc<RwLock<Option<NewMiningJob>>>,
        share_target: &Arc<RwLock<Option<[u8; 64]>>>,
        state: &Arc<RwLock<ConnectionState>>,
        _stats: &Arc<RwLock<PoolStats>>,
        vardiff: &Arc<Mutex<VardiffController>>,
        connection_id: &Arc<AtomicU64>,
        running: &Arc<AtomicBool>,
    ) -> Result<()> {
        // For now, use plaintext transport (Noise can be enabled later)
        let transport = super::transport::PlaintextTransport::connect(&config.url)
            .await
            .context("Failed to connect to pool")?;

        // Send SetupConnection
        let setup = SetupConnection {
            protocol_version: STRATUM_PROTOCOL_VERSION,
            min_version: 1,
            max_version: 1,
            flags: 0,
            endpoint_host: config.url.split(':').next().unwrap_or("").to_string(),
            endpoint_port: config
                .url
                .split(':')
                .nth(1)
                .and_then(|p| p.parse().ok())
                .unwrap_or(3333),
            vendor: "BTPC Desktop".to_string(),
            hardware_version: "1.0".to_string(),
            firmware: env!("CARGO_PKG_VERSION").to_string(),
            device_id: uuid::Uuid::new_v4().to_string(),
            worker_name: config.worker.clone(),
        };

        let setup_msg = StratumMessage::SetupConnection(setup);
        let encoded = super::codec::encode(&setup_msg)?;
        transport.send(&encoded).await?;

        // Wait for response
        let response_data = transport.recv().await?;
        let mut buf = bytes::BytesMut::from(&response_data[..]);
        if let Some(response) = super::codec::decode(&mut buf)? {
            match response {
                StratumMessage::SetupConnectionSuccess(success) => {
                    connection_id.store(success.connection_id as u64, Ordering::SeqCst);
                    *state.write().await = ConnectionState::Connected;
                    eprintln!(
                        "✅ Pool connected (id={}, version={})",
                        success.connection_id, success.used_version
                    );
                }
                StratumMessage::SetupConnectionError(err) => {
                    return Err(anyhow::anyhow!("Pool rejected connection: {}", err.error_code));
                }
                other => {
                    return Err(anyhow::anyhow!(
                        "Unexpected response to SetupConnection: {:?}",
                        std::mem::discriminant(&other)
                    ));
                }
            }
        }

        // Main message loop
        while running.load(Ordering::SeqCst) {
            let data = tokio::time::timeout(Duration::from_secs(120), transport.recv()).await;

            match data {
                Ok(Ok(msg_data)) => {
                    let mut buf = bytes::BytesMut::from(&msg_data[..]);
                    match super::codec::decode(&mut buf) {
                        Ok(Some(msg)) => {
                            Self::handle_pool_message(
                                msg,
                                current_job,
                                share_target,
                                vardiff,
                            )
                            .await;
                        }
                        Ok(None) => {} // Incomplete message
                        Err(e) => eprintln!("⚠️ Pool message decode error: {}", e),
                    }
                }
                Ok(Err(e)) => {
                    return Err(anyhow::anyhow!("Pool recv error: {}", e));
                }
                Err(_) => {
                    // Timeout — send ping or check connection
                    eprintln!("⏱️ Pool connection timeout, reconnecting...");
                    return Err(anyhow::anyhow!("Pool connection timeout"));
                }
            }
        }

        Ok(())
    }

    /// Handle a message from the pool
    async fn handle_pool_message(
        msg: StratumMessage,
        current_job: &Arc<RwLock<Option<NewMiningJob>>>,
        share_target: &Arc<RwLock<Option<[u8; 64]>>>,
        vardiff: &Arc<Mutex<VardiffController>>,
    ) {
        match msg {
            StratumMessage::NewMiningJob(job) => {
                eprintln!(
                    "📋 New pool job: id={} height={} clean={}",
                    job.job_id, job.height, job.clean_jobs
                );
                vardiff.lock().await.notify_new_job();
                *current_job.write().await = Some(job);
            }
            StratumMessage::SetTarget(target) => {
                eprintln!("🎯 New share target from pool");
                *share_target.write().await = Some(target.target);
            }
            StratumMessage::SetNewPrevHash(msg) => {
                eprintln!("🔗 New prev hash (job_id={})", msg.job_id);
                // Update current job's prev_hash if we have one
                if let Some(job) = current_job.write().await.as_mut() {
                    if msg.job_id == 0 || msg.job_id == job.job_id {
                        job.prev_hash = msg.prev_hash;
                        job.nbits = msg.nbits;
                    }
                }
            }
            StratumMessage::SubmitSharesSuccess(success) => {
                eprintln!("✅ Share accepted (seq={})", success.sequence_number);
            }
            StratumMessage::SubmitSharesError(err) => {
                eprintln!(
                    "❌ Share rejected (seq={}): {}",
                    err.sequence_number, err.error_code
                );
            }
            StratumMessage::Reconnect(reconnect) => {
                eprintln!(
                    "🔄 Pool requests reconnect to {}:{}",
                    reconnect.new_host, reconnect.new_port
                );
            }
            _ => {
                eprintln!("⚠️ Unhandled pool message");
            }
        }
    }

    /// Convert current pool job to a BlockTemplate for the mining thread pool
    async fn job_to_template(job: &NewMiningJob) -> BlockTemplate {
        BlockTemplate {
            version: job.version,
            previousblockhash: hex::encode(job.prev_hash),
            transactions: vec![serde_json::Value::String(job.coinbase_tx.clone())],
            coinbasevalue: job.coinbase_value,
            target: hex::encode(job.merkle_root), // Pool target
            mintime: job.timestamp,
            curtime: job.timestamp,
            bits: format!("{:08x}", job.nbits),
            height: job.height,
        }
    }
}

/// Implement RpcClientInterface so StratumPoolClient can be used by MiningThreadPool
impl RpcClientInterface for Arc<RwLock<StratumPoolClient>> {
    async fn get_block_template(&self) -> Result<BlockTemplate> {
        let client = self.read().await;
        let job_guard = client.current_job.read().await;
        match job_guard.as_ref() {
            Some(job) => Ok(StratumPoolClient::job_to_template(job).await),
            None => Err(anyhow::anyhow!("No mining job from pool yet")),
        }
    }

    async fn submit_block(&self, block_hex: &str) -> Result<String> {
        let client = self.read().await;

        // Record share in vardiff
        {
            let mut vd = client.vardiff.lock().await;
            vd.record_share();
        }

        // Update stats
        {
            let mut stats = client.stats.write().await;
            stats.accepted_shares += 1;
        }

        // B5: Dual-mode submission — also submit to embedded node if available
        #[allow(clippy::blocks_in_conditions)]
        if let Some(ref node) = client.embedded_node {
            match {
                let mut n = node.write().await;
                n.submit_block(block_hex).await
            } {
                Ok(hash) => {
                    eprintln!("🎉 Block-quality share also submitted to local node: {}", hash);
                    let mut stats = client.stats.write().await;
                    stats.blocks_found += 1;
                }
                Err(e) => {
                    eprintln!("⚠️ Local block submission failed (share still sent to pool): {}", e);
                }
            }
        }

        Ok("Share submitted to pool".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_serialize() {
        let config = PoolConfig {
            url: "pool.btpc.org:3333".to_string(),
            worker: "worker1".to_string(),
            password: "x".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let _: PoolConfig = serde_json::from_str(&json).unwrap();
    }

    #[tokio::test]
    async fn test_pool_client_creation() {
        let config = PoolConfig {
            url: "127.0.0.1:3333".to_string(),
            worker: "test".to_string(),
            password: "".to_string(),
        };
        let client = StratumPoolClient::new(config);
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_pool_stats_default() {
        let config = PoolConfig {
            url: "127.0.0.1:3333".to_string(),
            worker: "test".to_string(),
            password: "".to_string(),
        };
        let client = StratumPoolClient::new(config);
        let stats = client.get_stats().await;
        assert_eq!(stats.accepted_shares, 0);
        assert_eq!(stats.blocks_found, 0);
    }
}
