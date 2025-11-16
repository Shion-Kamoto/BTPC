//! Comprehensive Debug Event Listener
//!
//! Captures ALL events for debugging block submission and mining issues:
//! 1. Mining events (block found, submission, success/failure)
//! 2. Node status events (start, stop, sync)
//! 3. GPU stats events (hashrate, blocks found)
//! 4. RPC call events (requests, responses, errors)

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use chrono::Utc;

/// Debug logger for comprehensive event tracking
pub struct DebugLogger {
    log_file: Arc<Mutex<std::fs::File>>,
}

impl DebugLogger {
    /// Create new debug logger
    pub fn new(log_path: &str) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;

        Ok(Self {
            log_file: Arc::new(Mutex::new(file)),
        })
    }

    /// Log mining event
    pub fn log_mining_event(&self, event_type: &str, gpu_id: Option<u32>, details: &str) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let gpu_str = gpu_id.map(|id| format!("GPU {}", id)).unwrap_or_else(|| "CPU".to_string());
        let log_line = format!("[{}] [MINING] [{}] {} - {}\n", timestamp, gpu_str, event_type, details);

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log node status event
    pub fn log_node_event(&self, event_type: &str, details: &str) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!("[{}] [NODE] {} - {}\n", timestamp, event_type, details);

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log GPU stats event
    pub fn log_gpu_stats(&self, gpu_id: u32, hashrate: f64, total_hashes: u64, blocks_found: u64) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!(
            "[{}] [GPU_STATS] GPU {} - Hashrate: {:.2} H/s, Total Hashes: {}, Blocks Found: {}\n",
            timestamp, gpu_id, hashrate, total_hashes, blocks_found
        );

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log RPC request
    pub fn log_rpc_request(&self, method: &str, params: &str) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let params_preview = if params.len() > 200 {
            format!("{}... ({} bytes)", &params[..200], params.len())
        } else {
            params.to_string()
        };
        let log_line = format!("[{}] [RPC_REQUEST] {} - {}\n", timestamp, method, params_preview);

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log RPC response
    pub fn log_rpc_response(&self, method: &str, success: bool, response: &str) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let status = if success { "SUCCESS" } else { "ERROR" };
        let response_preview = if response.len() > 200 {
            format!("{}... ({} bytes)", &response[..200], response.len())
        } else {
            response.to_string()
        };
        let log_line = format!("[{}] [RPC_RESPONSE] {} - {} - {}\n", timestamp, method, status, response_preview);

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log block template request
    pub fn log_block_template(&self, height: u64, prev_hash: &str, coinbase_value: u64, target: &str) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!(
            "[{}] [BLOCK_TEMPLATE] Height: {}, Prev Hash: {}..., Coinbase: {} sats, Target: {}...\n",
            timestamp, height, &prev_hash[..16], coinbase_value, &target[..16]
        );

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log complete block header details for mining
    pub fn log_block_header(&self, gpu_id: Option<u32>, version: u32, prev_hash: &str, merkle_root: &str, timestamp: u64, bits: u32, nonce_start: u32) {
        let timestamp_log = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let gpu_str = gpu_id.map(|id| format!("GPU {}", id)).unwrap_or_else(|| "CPU".to_string());
        let log_line = format!(
            "[{}] [BLOCK_HEADER] {} - Version: {}, Prev Hash: {}, Merkle Root: {}, Timestamp: {}, Bits: 0x{:08x}, Nonce Start: {}\n",
            timestamp_log, gpu_str, version, prev_hash, merkle_root, timestamp, bits, nonce_start
        );

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log mining iteration with hash details
    pub fn log_mining_iteration(&self, gpu_id: Option<u32>, iteration: u64, total_hashes: u64, current_nonce: u32, hash_preview: &str) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let gpu_str = gpu_id.map(|id| format!("GPU {}", id)).unwrap_or_else(|| "CPU".to_string());
        let log_line = format!(
            "[{}] [MINING_ITER] {} - Iteration: {}, Total Hashes: {}, Nonce: {}, Hash: {}...\n",
            timestamp, gpu_str, iteration, total_hashes, current_nonce, &hash_preview[..32]
        );

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log block hash comparison with target
    pub fn log_hash_comparison(&self, gpu_id: Option<u32>, block_hash: &str, target: &str, meets_target: bool) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let gpu_str = gpu_id.map(|id| format!("GPU {}", id)).unwrap_or_else(|| "CPU".to_string());
        let status = if meets_target { "✅ VALID" } else { "❌ INVALID" };
        let log_line = format!(
            "[{}] [HASH_CHECK] {} - {} - Block Hash: {}, Target: {}\n",
            timestamp, gpu_str, status, block_hash, target
        );

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log full block details before submission
    pub fn log_complete_block(&self, gpu_id: Option<u32>, block_height: u64, block_hash: &str, prev_hash: &str, merkle_root: &str, nonce: u32, timestamp: u64, tx_count: usize) {
        let timestamp_log = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let gpu_str = gpu_id.map(|id| format!("GPU {}", id)).unwrap_or_else(|| "CPU".to_string());
        let log_line = format!(
            "[{}] [COMPLETE_BLOCK] {} - Height: {}, Hash: {}, Prev: {}, Merkle: {}, Nonce: {}, Time: {}, TXs: {}\n",
            timestamp_log, gpu_str, block_height, block_hash, prev_hash, merkle_root, nonce, timestamp, tx_count
        );

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log block submission attempt
    pub fn log_block_submission(&self, gpu_id: Option<u32>, nonce: u32, block_hex_len: usize, merkle_root: &str) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let gpu_str = gpu_id.map(|id| format!("GPU {}", id)).unwrap_or_else(|| "CPU".to_string());
        let log_line = format!(
            "[{}] [BLOCK_SUBMIT] {} - Nonce: {}, Block Size: {} bytes, Merkle Root: {}...\n",
            timestamp, gpu_str, nonce, block_hex_len / 2, &merkle_root[..32]
        );

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log raw block hex (first 500 chars for debugging)
    pub fn log_block_hex(&self, gpu_id: Option<u32>, block_hex: &str) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let gpu_str = gpu_id.map(|id| format!("GPU {}", id)).unwrap_or_else(|| "CPU".to_string());
        let preview = if block_hex.len() > 500 {
            format!("{}... ({} total chars)", &block_hex[..500], block_hex.len())
        } else {
            block_hex.to_string()
        };
        let log_line = format!(
            "[{}] [BLOCK_HEX] {} - {}\n",
            timestamp, gpu_str, preview
        );

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log block submission result
    pub fn log_block_result(&self, gpu_id: Option<u32>, success: bool, message: &str) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let gpu_str = gpu_id.map(|id| format!("GPU {}", id)).unwrap_or_else(|| "CPU".to_string());
        let status = if success { "✅ ACCEPTED" } else { "❌ REJECTED" };
        let log_line = format!("[{}] [BLOCK_RESULT] {} - {} - {}\n", timestamp, gpu_str, status, message);

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log critical error
    pub fn log_error(&self, component: &str, error: &str) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!("[{}] [ERROR] {} - {}\n", timestamp, component, error);

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log transaction details (for coinbase debugging)
    pub fn log_transaction(&self, tx_type: &str, tx_id: &str, inputs: usize, outputs: usize, total_output: u64) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!(
            "[{}] [TX] {} - ID: {}..., Inputs: {}, Outputs: {}, Total Output: {} sats\n",
            timestamp, tx_type, &tx_id[..16], inputs, outputs, total_output
        );

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Log merkle root calculation
    pub fn log_merkle_root(&self, num_transactions: usize, merkle_root: &str) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!(
            "[{}] [MERKLE] Calculated from {} txs - Root: {}\n",
            timestamp, num_transactions, merkle_root
        );

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
        eprintln!("{}", log_line.trim());
    }

    /// Flush all pending writes
    pub fn flush(&self) {
        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.flush();
        }
    }
}

/// Global debug logger instance
static DEBUG_LOGGER: once_cell::sync::Lazy<Option<DebugLogger>> = once_cell::sync::Lazy::new(|| {
    match DebugLogger::new("/home/bob/.btpc/logs/debug_events.log") {
        Ok(logger) => {
            eprintln!("📝 Debug logger initialized: /home/bob/.btpc/logs/debug_events.log");
            Some(logger)
        }
        Err(e) => {
            eprintln!("⚠️ Failed to initialize debug logger: {}", e);
            None
        }
    }
});

/// Get global debug logger
pub fn get_debug_logger() -> Option<&'static DebugLogger> {
    DEBUG_LOGGER.as_ref()
}

/// Macro for easy logging
#[macro_export]
macro_rules! debug_log_mining {
    ($event:expr, $gpu_id:expr, $details:expr) => {
        if let Some(logger) = $crate::debug_logger::get_debug_logger() {
            logger.log_mining_event($event, $gpu_id, $details);
        }
    };
}

#[macro_export]
macro_rules! debug_log_node {
    ($event:expr, $details:expr) => {
        if let Some(logger) = $crate::debug_logger::get_debug_logger() {
            logger.log_node_event($event, $details);
        }
    };
}

#[macro_export]
macro_rules! debug_log_error {
    ($component:expr, $error:expr) => {
        if let Some(logger) = $crate::debug_logger::get_debug_logger() {
            logger.log_error($component, $error);
        }
    };
}