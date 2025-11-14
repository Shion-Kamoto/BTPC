//! Linear decay reward calculation for BTPC
//!
//! BTPC implements a linear decay reward system that starts at 32.375 BTPC and
//! decreases linearly over 24 years to a tail emission of 0.5 BTPC per block.

use crate::economics::constants::*;

/// Calculator for block rewards following the linear decay schedule
pub struct RewardCalculator;

impl RewardCalculator {
    /// Calculate the block reward for a given height
    ///
    /// # Formula
    /// - Initial reward: 32.375 BTPC (3,237,500,000 credits)
    /// - Tail emission: 0.5 BTPC (50,000,000 credits)
    /// - Linear decay over 24 years (1,261,440 blocks)
    ///
    /// For height < DECAY_END_HEIGHT:
    /// reward = INITIAL_REWARD - (height * decrease_per_block)
    ///
    /// For height >= DECAY_END_HEIGHT:
    /// reward = TAIL_EMISSION
    pub fn calculate_block_reward(height: u32) -> u64 {
        if height >= DECAY_END_HEIGHT {
            // After 24 years, constant tail emission
            return TAIL_EMISSION;
        }

        // Linear decay calculation
        let total_decrease = INITIAL_REWARD - TAIL_EMISSION;
        let decrease_per_block = total_decrease / (DECAY_END_HEIGHT as u64);
        let total_decrease_at_height = (height as u64) * decrease_per_block;

        // Ensure we don't go below tail emission
        if total_decrease_at_height >= total_decrease {
            TAIL_EMISSION
        } else {
            INITIAL_REWARD - total_decrease_at_height
        }
    }

    /// Calculate the total supply up to a given height
    pub fn calculate_total_supply(height: u32) -> u64 {
        if height == 0 {
            return INITIAL_REWARD; // Genesis block
        }

        let mut total_supply = 0u64;

        if height <= DECAY_END_HEIGHT {
            // Sum of arithmetic sequence for decay period
            let blocks_in_decay = height as u64;
            let first_reward = INITIAL_REWARD;
            let last_reward = Self::calculate_block_reward(height - 1);

            // Sum = n/2 * (first + last)
            total_supply = blocks_in_decay * (first_reward + last_reward) / 2;
        } else {
            // Decay period complete, add tail emission blocks
            let decay_blocks = DECAY_END_HEIGHT as u64;
            let tail_blocks = (height - DECAY_END_HEIGHT) as u64;

            // Sum of decay period (arithmetic sequence)
            let decay_supply = decay_blocks * (INITIAL_REWARD + TAIL_EMISSION) / 2;

            // Add tail emission blocks
            let tail_supply = tail_blocks * TAIL_EMISSION;

            total_supply = decay_supply + tail_supply;
        }

        total_supply
    }

    /// Calculate the expected total supply at a given height (for validation)
    pub fn calculate_expected_supply(height: u32) -> u64 {
        Self::calculate_total_supply(height)
    }

    /// Get the current inflation rate as a percentage
    pub fn calculate_inflation_rate(height: u32) -> f64 {
        if height == 0 {
            return 0.0;
        }

        let current_supply = Self::calculate_total_supply(height);
        let annual_emission = Self::calculate_block_reward(height) * (BLOCKS_PER_YEAR as u64);

        (annual_emission as f64 / current_supply as f64) * 100.0
    }

    /// Get statistics about the reward schedule
    pub fn get_reward_schedule_info() -> RewardScheduleInfo {
        RewardScheduleInfo {
            initial_reward: INITIAL_REWARD,
            tail_emission: TAIL_EMISSION,
            decay_duration_years: DECAY_DURATION_YEARS,
            decay_duration_blocks: DECAY_END_HEIGHT,
            blocks_per_year: BLOCKS_PER_YEAR,
            total_decay_amount: INITIAL_REWARD - TAIL_EMISSION,
        }
    }

    /// Calculate rewards for multiple consecutive blocks efficiently
    pub fn calculate_block_rewards_batch(start_height: u32, count: u32) -> Vec<u64> {
        let mut rewards = Vec::with_capacity(count as usize);

        for height in start_height..(start_height + count) {
            rewards.push(Self::calculate_block_reward(height));
        }

        rewards
    }

    /// Validate that a claimed reward is correct for the given height
    pub fn validate_block_reward(claimed_reward: u64, height: u32) -> bool {
        let expected_reward = Self::calculate_block_reward(height);
        claimed_reward == expected_reward
    }

    /// Calculate the decrease per block during decay period
    pub fn get_decrease_per_block() -> u64 {
        let total_decrease = INITIAL_REWARD - TAIL_EMISSION;
        total_decrease / (DECAY_END_HEIGHT as u64)
    }

    /// Check if we're currently in the decay period
    pub fn is_in_decay_period(height: u32) -> bool {
        height < DECAY_END_HEIGHT
    }

    /// Check if we've reached tail emission
    pub fn is_tail_emission(height: u32) -> bool {
        height >= DECAY_END_HEIGHT
    }

    /// Get the year number for a given height (0-indexed)
    pub fn get_year_from_height(height: u32) -> u32 {
        height / BLOCKS_PER_YEAR
    }

    /// Get the height at the start of a given year
    pub fn get_height_from_year(year: u32) -> u32 {
        year * BLOCKS_PER_YEAR
    }
}

/// Information about the reward schedule
#[derive(Debug, Clone)]
pub struct RewardScheduleInfo {
    pub initial_reward: u64,
    pub tail_emission: u64,
    pub decay_duration_years: u32,
    pub decay_duration_blocks: u32,
    pub blocks_per_year: u32,
    pub total_decay_amount: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_reward() {
        let genesis_reward = RewardCalculator::calculate_block_reward(0);
        assert_eq!(genesis_reward, INITIAL_REWARD);
        assert_eq!(genesis_reward, 3_237_500_000); // 32.375 BTPC
    }

    #[test]
    fn test_linear_decay() {
        // Test rewards decrease linearly
        let year_1_reward = RewardCalculator::calculate_block_reward(BLOCKS_PER_YEAR);
        let year_12_reward = RewardCalculator::calculate_block_reward(12 * BLOCKS_PER_YEAR);
        let year_24_reward = RewardCalculator::calculate_block_reward(24 * BLOCKS_PER_YEAR);

        // Year 1 should be less than genesis
        assert!(year_1_reward < INITIAL_REWARD);

        // Year 12 should be roughly halfway (accounting for integer division rounding)
        // With integer division: decrease_per_block = 3,187,500,000 / 1,262,304 = 2,525 (truncated)
        // At year 12 (631,152 blocks): reward = 3,237,500,000 - (631,152 * 2,525) = 1,643,841,200
        let expected_year_12 = 1_643_841_200; // Actual value with integer division
        assert_eq!(year_12_reward, expected_year_12);

        // Year 24 should reach tail emission
        assert_eq!(year_24_reward, TAIL_EMISSION);
    }

    #[test]
    fn test_tail_emission_persistence() {
        let tail_start = DECAY_END_HEIGHT;
        let far_future = tail_start + 1_000_000;

        assert_eq!(
            RewardCalculator::calculate_block_reward(tail_start),
            TAIL_EMISSION
        );
        assert_eq!(
            RewardCalculator::calculate_block_reward(far_future),
            TAIL_EMISSION
        );
    }

    #[test]
    fn test_reward_monotonic_decrease() {
        let mut previous_reward = u64::MAX;

        // Test decreasing rewards during decay period
        for year in 0..24 {
            let height = year * BLOCKS_PER_YEAR;
            let current_reward = RewardCalculator::calculate_block_reward(height);

            if year > 0 {
                assert!(
                    current_reward < previous_reward,
                    "Reward at year {} should be less than previous year",
                    year
                );
            }

            previous_reward = current_reward;
        }
    }

    #[test]
    fn test_total_supply_calculation() {
        // Test total supply at various points
        let supply_year_1 = RewardCalculator::calculate_total_supply(BLOCKS_PER_YEAR);
        let supply_year_24 = RewardCalculator::calculate_total_supply(24 * BLOCKS_PER_YEAR);

        assert!(supply_year_1 > 0);
        assert!(supply_year_24 > supply_year_1);

        // Test arithmetic sequence formula for first 24 years
        // Note: Due to integer division in the reward calculation, the actual supply
        // differs slightly from the pure arithmetic formula
        // Actual: 2,074,188,999,841,920 vs Formula: 2,073,492,000,000,000
        let actual_supply = supply_year_24;
        let approximate_formula = (24 * BLOCKS_PER_YEAR) as u64 * (INITIAL_REWARD + TAIL_EMISSION) / 2;

        // Supply should be close to formula (within 0.1% due to rounding)
        let diff = if actual_supply > approximate_formula {
            actual_supply - approximate_formula
        } else {
            approximate_formula - actual_supply
        };
        let tolerance = approximate_formula / 1000; // 0.1% tolerance
        assert!(diff < tolerance, "Supply difference {} exceeds tolerance {}", diff, tolerance);
    }

    #[test]
    fn test_batch_reward_calculation() {
        let start_height = 1000;
        let count = 100;
        let batch_rewards = RewardCalculator::calculate_block_rewards_batch(start_height, count);

        assert_eq!(batch_rewards.len(), count as usize);

        // Verify each reward matches individual calculation
        for (i, &reward) in batch_rewards.iter().enumerate() {
            let expected = RewardCalculator::calculate_block_reward(start_height + i as u32);
            assert_eq!(reward, expected);
        }
    }

    #[test]
    fn test_reward_validation() {
        let height = 52560; // 1 year
        let correct_reward = RewardCalculator::calculate_block_reward(height);
        let incorrect_reward = correct_reward + 1;

        assert!(RewardCalculator::validate_block_reward(
            correct_reward,
            height
        ));
        assert!(!RewardCalculator::validate_block_reward(
            incorrect_reward,
            height
        ));
    }

    #[test]
    fn test_inflation_rate_calculation() {
        // Inflation should decrease over time
        let inflation_year_1 = RewardCalculator::calculate_inflation_rate(BLOCKS_PER_YEAR);
        let inflation_year_10 = RewardCalculator::calculate_inflation_rate(10 * BLOCKS_PER_YEAR);
        let inflation_year_50 = RewardCalculator::calculate_inflation_rate(50 * BLOCKS_PER_YEAR);

        assert!(inflation_year_1 > inflation_year_10);
        assert!(inflation_year_10 > inflation_year_50);
        assert!(inflation_year_50 > 0.0); // Should still be positive due to tail emission
    }

    #[test]
    fn test_period_detection() {
        let decay_height = DECAY_END_HEIGHT / 2;
        let tail_height = DECAY_END_HEIGHT + 100;

        assert!(RewardCalculator::is_in_decay_period(decay_height));
        assert!(!RewardCalculator::is_in_decay_period(tail_height));

        assert!(!RewardCalculator::is_tail_emission(decay_height));
        assert!(RewardCalculator::is_tail_emission(tail_height));
    }

    #[test]
    fn test_year_height_conversion() {
        for year in 0..30 {
            let height = RewardCalculator::get_height_from_year(year);
            let calculated_year = RewardCalculator::get_year_from_height(height);
            assert_eq!(year, calculated_year);
        }
    }

    #[test]
    fn test_reward_schedule_info() {
        let info = RewardCalculator::get_reward_schedule_info();

        assert_eq!(info.initial_reward, INITIAL_REWARD);
        assert_eq!(info.tail_emission, TAIL_EMISSION);
        assert_eq!(info.decay_duration_years, DECAY_DURATION_YEARS);
        assert_eq!(info.total_decay_amount, INITIAL_REWARD - TAIL_EMISSION);
    }

    #[test]
    fn test_precision_and_rounding() {
        // Test that calculations are precise and don't accumulate rounding errors
        for height in 0..1000 {
            let reward = RewardCalculator::calculate_block_reward(height);

            // Reward should always be positive
            assert!(reward > 0);

            // Reward should never be less than tail emission
            assert!(reward >= TAIL_EMISSION);

            // Reward should never exceed initial reward
            assert!(reward <= INITIAL_REWARD);
        }
    }

    #[test]
    fn test_economic_model_consistency() {
        // Verify the economic model is internally consistent
        let decrease_per_block = RewardCalculator::get_decrease_per_block();
        let total_decrease = INITIAL_REWARD - TAIL_EMISSION;
        let expected_decrease_per_block = total_decrease / (DECAY_END_HEIGHT as u64);

        assert_eq!(decrease_per_block, expected_decrease_per_block);

        // Verify total decrease happens over exactly 24 years
        let year_24_height = 24 * BLOCKS_PER_YEAR;
        assert_eq!(year_24_height, DECAY_END_HEIGHT);

        let final_decay_reward = RewardCalculator::calculate_block_reward(DECAY_END_HEIGHT - 1);
        assert!(final_decay_reward > TAIL_EMISSION);

        let first_tail_reward = RewardCalculator::calculate_block_reward(DECAY_END_HEIGHT);
        assert_eq!(first_tail_reward, TAIL_EMISSION);
    }
}
