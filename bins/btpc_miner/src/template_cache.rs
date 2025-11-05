//! Template Cache for Mining
//!
//! Caches block templates to avoid excessive RPC calls (429 rate limiting).
//! Fetches templates periodically in the background (default: 60 seconds).

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use btpc_core::blockchain::Block;
use serde_json::json;

/// Template cache with background periodic updates
pub struct TemplateCache {
    /// Current cached template (shared across all mining threads)
    current_template: Arc<RwLock<Option<Block>>>,
    /// Last update timestamp
    last_update: Arc<RwLock<Instant>>,
    /// Update interval (default: 60 seconds)
    update_interval: Duration,
    /// RPC URL for fetching templates
    rpc_url: String,
}

impl TemplateCache {
    /// Create a new template cache
    pub fn new(rpc_url: String, update_interval: Duration) -> Self {
        TemplateCache {
            current_template: Arc::new(RwLock::new(None)),
            last_update: Arc::new(RwLock::new(Instant::now())),
            update_interval,
            rpc_url,
        }
    }

    /// Start background template updater
    pub fn start_updater(self: Arc<Self>) {
        std::thread::spawn(move || {
            println!("[TemplateCache] Background updater started (interval: {:?})", self.update_interval);

            loop {
                // Fetch new template
                match self.fetch_template() {
                    Ok(template) => {
                        let mut current = self.current_template.write()
                            .expect("Template cache write lock poisoned");
                        *current = Some(template);

                        let mut last_update = self.last_update.write()
                            .expect("Last update write lock poisoned");
                        *last_update = Instant::now();

                        println!("[TemplateCache] Template updated successfully");
                    }
                    Err(e) => {
                        eprintln!("[TemplateCache] Failed to fetch template: {}", e);
                    }
                }

                // Sleep for update interval
                std::thread::sleep(self.update_interval);
            }
        });
    }

    /// Get current cached template (thread-safe)
    pub fn get_template(&self) -> Option<Block> {
        let template = self.current_template.read()
            .expect("Template cache read lock poisoned");
        template.clone()
    }

    /// Fetch template from RPC (blocking HTTP call)
    fn fetch_template(&self) -> Result<Block, Box<dyn std::error::Error>> {
        use btpc_core::{
            blockchain::{Block, BlockHeader, Transaction, TransactionInput, TransactionOutput, OutPoint},
            consensus::RewardCalculator,
            crypto::{Hash, Script},
        };
        use serde::Deserialize;

        #[derive(Debug, Clone, Deserialize)]
        struct BlockTemplate {
            version: u32,
            height: u32,
            previousblockhash: String,
            #[serde(deserialize_with = "deserialize_bits_from_hex")]
            bits: u32,
            curtime: u64,
        }

        fn deserialize_bits_from_hex<'de, D>(deserializer: D) -> Result<u32, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s: String = serde::Deserialize::deserialize(deserializer)?;
            u32::from_str_radix(&s, 16).map_err(serde::de::Error::custom)
        }

        // Fetch block template from node via RPC
        let client = reqwest::blocking::Client::new();
        let request = json!({
            "jsonrpc": "2.0",
            "id": "template_cache",
            "method": "getblocktemplate",
            "params": []
        });

        let response = client
            .post(&self.rpc_url)
            .header("Content-Type", "application/json")
            .body(request.to_string())
            .timeout(Duration::from_secs(10)) // 10-second timeout
            .send()?;

        if !response.status().is_success() {
            return Err(format!("RPC HTTP error: {}", response.status()).into());
        }

        let response_text = response.text()?;
        let response_json: serde_json::Value = serde_json::from_str(&response_text)?;

        // Check for RPC error
        if let Some(error) = response_json.get("error") {
            if !error.is_null() {
                return Err(format!("RPC error: {}", error).into());
            }
        }

        // Parse template from result
        let result = response_json.get("result")
            .ok_or("Missing result in RPC response")?;

        let template: BlockTemplate = serde_json::from_value(result.clone())?;

        // Parse prev_hash
        let prev_hash = if template.previousblockhash.is_empty()
            || template.previousblockhash == "0000000000000000000000000000000000000000000000000000000000000000" {
            Hash::zero()
        } else {
            Hash::from_hex(&template.previousblockhash)?
        };

        // Create coinbase transaction (simplified - use default address)
        let reward = RewardCalculator::calculate_reward(template.height).unwrap_or(0);

        let coinbase_input = TransactionInput {
            previous_output: OutPoint {
                txid: Hash::zero(),
                vout: 0xffffffff,
            },
            script_sig: Script::from_bytes(b"BTPC Miner Template Cache".to_vec()),
            sequence: 0xffffffff,
        };

        let reward_output = TransactionOutput {
            value: reward,
            script_pubkey: Script::from_bytes(b"template_placeholder".to_vec()),
        };

        let coinbase_tx = Transaction {
            version: 1,
            inputs: vec![coinbase_input],
            outputs: vec![reward_output],
            lock_time: 0,
            fork_id: 2, // Regtest network
        };

        // Calculate merkle root
        let merkle_root = coinbase_tx.hash();

        // Create block header with actual network difficulty
        let header = BlockHeader {
            version: template.version,
            prev_hash,
            merkle_root,
            timestamp: template.curtime,
            bits: template.bits,
            nonce: 0,
        };

        Ok(Block {
            header,
            transactions: vec![coinbase_tx],
        })
    }

    /// Force immediate template refresh (for testing or new block notifications)
    pub fn refresh_now(&self) -> Result<(), Box<dyn std::error::Error>> {
        let template = self.fetch_template()?;

        let mut current = self.current_template.write()
            .expect("Template cache write lock poisoned");
        *current = Some(template);

        let mut last_update = self.last_update.write()
            .expect("Last update write lock poisoned");
        *last_update = Instant::now();

        println!("[TemplateCache] Template refreshed on demand");
        Ok(())
    }

    /// Get age of current template
    /// Reserved for monitoring and metrics
    #[allow(dead_code)]
    pub fn template_age(&self) -> Duration {
        let last_update = self.last_update.read()
            .expect("Last update read lock poisoned");
        Instant::now().duration_since(*last_update)
    }
}