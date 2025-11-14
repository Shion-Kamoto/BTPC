//! Integration test for linear decay block reward economics
//! Per data-model.md specification
//!
//! This test MUST FAIL initially (implementation not complete yet)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Block {
    height: u64,
    reward: u64,
    timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlockchainInfo {
    blocks: u64,
    total_supply: u64,
}

// Linear decay constants per data-model.md
const INITIAL_REWARD: u64 = 3_237_500_000; // 32.375 BTPC in satoshis
const TAIL_EMISSION: u64 = 50_000_000;     // 0.5 BTPC in satoshis
const DECAY_PERIOD_YEARS: u64 = 24;
const BLOCKS_PER_YEAR: u64 = 52_560;       // 10 min blocks
const DECAY_PERIOD_BLOCKS: u64 = DECAY_PERIOD_YEARS * BLOCKS_PER_YEAR;
const SATOSHIS_PER_BTPC: u64 = 100_000_000;

#[tokio::test]
#[ignore] // Will fail until block reward implementation is complete
async fn test_initial_block_reward() {
    // Test that block 0 (genesis) has correct initial reward
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8332/api/v1/block/0")
        .send()
        .await;

    assert!(response.is_ok(), "Should get genesis block");

    #[derive(Debug, Deserialize)]
    struct BlockResponse {
        transactions: Vec<Transaction>,
    }

    #[derive(Debug, Deserialize)]
    struct Transaction {
        outputs: Vec<TransactionOutput>,
    }

    #[derive(Debug, Deserialize)]
    struct TransactionOutput {
        amount: u64,
    }

    let block: BlockResponse = response.unwrap().json().await.expect("Should parse");

    // Genesis block coinbase transaction
    let coinbase = &block.transactions[0];
    let reward = coinbase.outputs[0].amount;

    assert_eq!(
        reward, INITIAL_REWARD,
        "Initial reward should be 32.375 BTPC (3,237,500,000 satoshis)"
    );
}

#[tokio::test]
#[ignore] // Will fail until implementation is complete
async fn test_reward_at_mid_decay() {
    // Test reward calculation at midpoint of decay period
    let mid_block = DECAY_PERIOD_BLOCKS / 2; // ~631,680 blocks (12 years)

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://localhost:8332/api/v1/block/{}", mid_block))
        .send()
        .await;

    if response.is_err() {
        // Block may not exist yet in test environment
        return;
    }

    #[derive(Debug, Deserialize)]
    struct BlockResponse {
        transactions: Vec<Transaction>,
    }

    #[derive(Debug, Deserialize)]
    struct Transaction {
        outputs: Vec<TransactionOutput>,
    }

    #[derive(Debug, Deserialize)]
    struct TransactionOutput {
        amount: u64,
    }

    let block: BlockResponse = response.unwrap().json().await.expect("Should parse");
    let coinbase = &block.transactions[0];
    let reward = coinbase.outputs[0].amount;

    // At midpoint: reward should be ~16.4375 BTPC
    // Formula: reward(h) = INITIAL - (INITIAL - TAIL) * h / DECAY_BLOCKS
    let expected = INITIAL_REWARD
        - ((INITIAL_REWARD - TAIL_EMISSION) * mid_block / DECAY_PERIOD_BLOCKS);

    // Allow 1 satoshi tolerance for rounding
    assert!(
        (reward as i64 - expected as i64).abs() <= 1,
        "Mid-decay reward should be ~16.4375 BTPC, got {} satoshis",
        reward
    );
}

#[tokio::test]
#[ignore] // Will fail until implementation is complete
async fn test_reward_at_tail_emission() {
    // Test that reward reaches tail emission at end of decay period
    let tail_block = DECAY_PERIOD_BLOCKS; // Block 1,261,440 (24 years)

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://localhost:8332/api/v1/block/{}", tail_block))
        .send()
        .await;

    if response.is_err() {
        return;
    }

    #[derive(Debug, Deserialize)]
    struct BlockResponse {
        transactions: Vec<Transaction>,
    }

    #[derive(Debug, Deserialize)]
    struct Transaction {
        outputs: Vec<TransactionOutput>,
    }

    #[derive(Debug, Deserialize)]
    struct TransactionOutput {
        amount: u64,
    }

    let block: BlockResponse = response.unwrap().json().await.expect("Should parse");
    let coinbase = &block.transactions[0];
    let reward = coinbase.outputs[0].amount;

    assert_eq!(
        reward, TAIL_EMISSION,
        "Tail emission should be 0.5 BTPC (50,000,000 satoshis)"
    );
}

#[tokio::test]
#[ignore] // Will fail until implementation is complete
async fn test_reward_after_tail_emission() {
    // Test that reward remains constant after decay period
    let post_tail_block = DECAY_PERIOD_BLOCKS + 100_000; // 100k blocks after tail

    let client = reqwest::Client::new();
    let response = client
        .get(&format!(
            "http://localhost:8332/api/v1/block/{}",
            post_tail_block
        ))
        .send()
        .await;

    if response.is_err() {
        return;
    }

    #[derive(Debug, Deserialize)]
    struct BlockResponse {
        transactions: Vec<Transaction>,
    }

    #[derive(Debug, Deserialize)]
    struct Transaction {
        outputs: Vec<TransactionOutput>,
    }

    #[derive(Debug, Deserialize)]
    struct TransactionOutput {
        amount: u64,
    }

    let block: BlockResponse = response.unwrap().json().await.expect("Should parse");
    let coinbase = &block.transactions[0];
    let reward = coinbase.outputs[0].amount;

    assert_eq!(
        reward, TAIL_EMISSION,
        "Post-tail emission should remain constant at 0.5 BTPC"
    );
}

#[tokio::test]
#[ignore] // Will fail until implementation is complete
async fn test_total_supply_calculation() {
    // Test that total supply matches sum of all block rewards
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8332/api/v1/blockchain/info")
        .send()
        .await;

    assert!(response.is_ok(), "Should get blockchain info");

    let info: BlockchainInfo = response.unwrap().json().await.expect("Should parse");

    if info.blocks == 0 {
        return;
    }

    // Calculate expected total supply based on block height
    let expected_supply = calculate_total_supply(info.blocks);

    // Allow small tolerance for rounding
    let diff = (info.total_supply as i64 - expected_supply as i64).abs();
    assert!(
        diff <= info.blocks as i64,
        "Total supply should match calculated supply (within rounding tolerance)"
    );
}

#[tokio::test]
#[ignore] // Will fail until implementation is complete
async fn test_reward_decreases_linearly() {
    // Test that reward decreases linearly over decay period
    let client = reqwest::Client::new();

    let test_blocks = vec![0, 100_000, 500_000, 1_000_000, DECAY_PERIOD_BLOCKS];

    let mut prev_reward = u64::MAX;
    let mut rewards = Vec::new();

    for block_height in test_blocks {
        let response = client
            .get(&format!("http://localhost:8332/api/v1/block/{}", block_height))
            .send()
            .await;

        if response.is_err() {
            continue;
        }

        #[derive(Debug, Deserialize)]
        struct BlockResponse {
            transactions: Vec<Transaction>,
        }

        #[derive(Debug, Deserialize)]
        struct Transaction {
            outputs: Vec<TransactionOutput>,
        }

        #[derive(Debug, Deserialize)]
        struct TransactionOutput {
            amount: u64,
        }

        let block: BlockResponse = response.unwrap().json().await.expect("Should parse");
        let reward = block.transactions[0].outputs[0].amount;

        rewards.push((block_height, reward));

        if prev_reward != u64::MAX {
            assert!(
                reward <= prev_reward,
                "Reward should decrease or stay constant (tail emission)"
            );
        }

        prev_reward = reward;
    }

    // Verify linear relationship
    if rewards.len() >= 2 {
        let (h1, r1) = rewards[0];
        let (h2, r2) = rewards[rewards.len() - 1];

        // Calculate slope
        let expected_slope = (INITIAL_REWARD as i64 - TAIL_EMISSION as i64) as f64
            / DECAY_PERIOD_BLOCKS as f64;

        let actual_slope = (r1 as i64 - r2 as i64) as f64 / (h2 as i64 - h1 as i64) as f64;

        // Allow 5% tolerance for slope
        let slope_diff = (expected_slope - actual_slope).abs() / expected_slope;
        assert!(
            slope_diff < 0.05,
            "Reward should decrease linearly with slope ~{:.6}",
            expected_slope
        );
    }
}

#[test]
fn test_linear_decay_formula() {
    // Test reward calculation formula
    fn calculate_reward(height: u64) -> u64 {
        if height >= DECAY_PERIOD_BLOCKS {
            return TAIL_EMISSION;
        }

        let decay = (INITIAL_REWARD - TAIL_EMISSION) * height / DECAY_PERIOD_BLOCKS;
        INITIAL_REWARD - decay
    }

    // Test known values
    assert_eq!(calculate_reward(0), INITIAL_REWARD);
    assert_eq!(calculate_reward(DECAY_PERIOD_BLOCKS), TAIL_EMISSION);

    // Test midpoint
    let mid_reward = calculate_reward(DECAY_PERIOD_BLOCKS / 2);
    let expected_mid = (INITIAL_REWARD + TAIL_EMISSION) / 2;
    assert!((mid_reward as i64 - expected_mid as i64).abs() <= 1);

    // Test post-tail
    assert_eq!(calculate_reward(DECAY_PERIOD_BLOCKS + 1), TAIL_EMISSION);
    assert_eq!(
        calculate_reward(DECAY_PERIOD_BLOCKS + 1_000_000),
        TAIL_EMISSION
    );
}

#[test]
fn test_total_supply_formula() {
    // Calculate total supply after n blocks
    let n = DECAY_PERIOD_BLOCKS;

    // During decay: sum of arithmetic sequence
    // Sum = n/2 * (first + last)
    let decay_supply = n * (INITIAL_REWARD + TAIL_EMISSION) / 2;

    // Expected: ~20.65 billion BTPC after 24 years
    let expected_btpc = decay_supply / SATOSHIS_PER_BTPC;

    assert!(
        expected_btpc > 20_000_000_000,
        "Should produce >20B BTPC during decay period"
    );
    assert!(
        expected_btpc < 21_000_000_000,
        "Should produce <21B BTPC during decay period"
    );
}

// Helper function to calculate total supply
fn calculate_total_supply(height: u64) -> u64 {
    if height == 0 {
        return INITIAL_REWARD;
    }

    if height <= DECAY_PERIOD_BLOCKS {
        // During decay: sum of arithmetic sequence
        // Sum = n/2 * (first + last)
        let last_reward =
            INITIAL_REWARD - (INITIAL_REWARD - TAIL_EMISSION) * height / DECAY_PERIOD_BLOCKS;
        return height * (INITIAL_REWARD + last_reward) / 2;
    }

    // After decay: decay supply + tail emission
    let decay_supply = DECAY_PERIOD_BLOCKS * (INITIAL_REWARD + TAIL_EMISSION) / 2;
    let tail_blocks = height - DECAY_PERIOD_BLOCKS;
    decay_supply + tail_blocks * TAIL_EMISSION
}

#[test]
fn test_calculate_total_supply_helper() {
    // Test helper function
    assert_eq!(calculate_total_supply(0), INITIAL_REWARD);

    // After 1 block
    assert_eq!(calculate_total_supply(1), INITIAL_REWARD * 2 - (INITIAL_REWARD - TAIL_EMISSION) / DECAY_PERIOD_BLOCKS);

    // At tail emission
    let tail_supply = calculate_total_supply(DECAY_PERIOD_BLOCKS);
    assert!(tail_supply > 20_000_000_000 * SATOSHIS_PER_BTPC);

    // After tail emission
    let post_tail = calculate_total_supply(DECAY_PERIOD_BLOCKS + 1);
    assert_eq!(post_tail, tail_supply + TAIL_EMISSION);
}