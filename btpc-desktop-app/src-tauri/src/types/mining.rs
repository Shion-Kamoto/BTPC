use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;

use crate::mining_thread_pool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningStatus {
    pub active: bool,
    pub hashrate: u64,
    pub blocks_mined: u32, // Lifetime total
    pub current_difficulty: String,
    pub threads: u32,
    pub current_height: u64, // Current blockchain height for reward calculation
}

impl Default for MiningStatus {
    fn default() -> Self {
        Self {
            active: false,
            hashrate: 0,
            blocks_mined: 0,
            current_difficulty: "0".to_string(),
            threads: 1,
            current_height: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningLogEntry {
    pub timestamp: String,
    pub level: String, // INFO, WARN, ERROR, SUCCESS
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MiningStatsData {
    pub lifetime_blocks_found: u64,
}

#[derive(Clone)]
pub struct MiningStats {
    pub blocks_found: u64, // Lifetime total
    pub hashrate: u64,
    pub start_time: Option<std::time::Instant>,
    pub stats_file: PathBuf,
}

impl MiningStats {
    pub fn new(data_dir: &PathBuf) -> Self {
        let stats_file = data_dir.join("mining_stats.json");

        // Load lifetime blocks_found from disk if it exists
        let blocks_found = if stats_file.exists() {
            match std::fs::read_to_string(&stats_file) {
                Ok(json) => match serde_json::from_str::<MiningStatsData>(&json) {
                    Ok(data) => {
                        println!(
                            "Loaded lifetime mining stats: {} blocks found",
                            data.lifetime_blocks_found
                        );
                        data.lifetime_blocks_found
                    }
                    Err(e) => {
                        println!("Failed to parse mining stats: {}, starting from 0", e);
                        0
                    }
                },
                Err(e) => {
                    println!("Failed to read mining stats: {}, starting from 0", e);
                    0
                }
            }
        } else {
            println!("No existing mining stats found, starting from 0");
            0
        };

        Self {
            blocks_found,
            hashrate: 0,
            start_time: None,
            stats_file,
        }
    }

    pub fn reset(&mut self) {
        // Don't reset blocks_found - it's lifetime persistent
        self.hashrate = 0;
        self.start_time = None;
    }

    pub fn start(&mut self) {
        self.start_time = Some(std::time::Instant::now());
    }

    pub fn increment_blocks(&mut self) {
        self.blocks_found += 1;
        self.save_to_disk();
    }

    pub fn save_to_disk(&self) {
        let data = MiningStatsData {
            lifetime_blocks_found: self.blocks_found,
        };

        match serde_json::to_string_pretty(&data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.stats_file, json) {
                    println!("Failed to save mining stats: {}", e);
                } else {
                    println!("Saved mining stats: {} blocks found", self.blocks_found);
                }
            }
            Err(e) => {
                println!("Failed to serialize mining stats: {}", e);
            }
        }
    }

    pub fn calculate_hashrate(&mut self, estimated_hashes: u64) {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_secs();
            if elapsed > 0 {
                self.hashrate = estimated_hashes / elapsed;
            }
        }
    }
}

pub struct MiningLogBuffer {
    entries: VecDeque<MiningLogEntry>,
    max_entries: usize,
}

impl MiningLogBuffer {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
        }
    }

    pub fn add_entry(&mut self, level: String, message: String) {
        // Format timestamp to match frontend expectations: "YYYY-MM-DD HH:MM:SS"
        // Frontend uses .split(' ')[1] to extract just the time portion
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let entry = MiningLogEntry {
            timestamp,
            level,
            message,
        };

        self.entries.push_back(entry);

        // Keep only the most recent entries
        if self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    pub fn get_entries(&self) -> Vec<MiningLogEntry> {
        self.entries.iter().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

// Implement MiningLogger trait for MiningLogBuffer
impl mining_thread_pool::MiningLogger for MiningLogBuffer {
    fn add_entry(&mut self, level: String, message: String) {
        // Delegate to existing implementation
        MiningLogBuffer::add_entry(self, level, message)
    }
}