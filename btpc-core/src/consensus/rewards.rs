//! Block reward calculation for BTPC linear decay model
//!
//! Implements the constitutional requirement for 24-year linear decay to tail emission.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::consensus::RewardParams;

/// Block reward calculator implementing BTPC's constitutional linear decay model
pub struct RewardCalculator;

impl RewardCalculator {
    /// Calculate block reward for given height using default mainnet parameters
    pub fn calculate_block_reward(height: u32) -> Result<u64, RewardError> {
        let params = crate::consensus::RewardParams::mainnet();
        Self::calculate_block_reward_with_params(height, &params)
    }

    /// Calculate reward (alias for calculate_block_reward for compatibility)
    pub fn calculate_reward(height: u32) -> Option<u64> {
        Self::calculate_block_reward(height).ok()
    }

    /// Calculate block reward for given height using linear decay
    pub fn calculate_block_reward_with_params(
        height: u32,
        params: &RewardParams,
    ) -> Result<u64, RewardError> {
        // Genesis block gets initial reward
        if height == 0 {
            return Ok(params.initial_reward);
        }

        let total_decay_blocks = params.decay_years * params.blocks_per_year;

        // After decay period: tail emission
        if height >= total_decay_blocks {
            return Ok(params.tail_emission);
        }

        // During linear decay period
        // Use integer-only arithmetic for deterministic consensus
        // reward = initial - (total_decrease * height / total_decay_blocks)

        let total_decrease = params.initial_reward.saturating_sub(params.tail_emission);

        // Use u128 to prevent overflow during multiplication
        let decrease_amount = (total_decrease as u128)
            .checked_mul(height as u128)
            .ok_or(RewardError::SupplyOverflow)?
            / (total_decay_blocks as u128);

        // Calculate current reward using checked arithmetic
        let current_reward = (params.initial_reward as u128)
            .checked_sub(decrease_amount)
            .ok_or(RewardError::SupplyOverflow)?;

        // Ensure we don't go below tail emission
        let final_reward = current_reward.max(params.tail_emission as u128);

        // Convert back to u64 (guaranteed to fit since initial_reward is u64)
        Ok(final_reward as u64)
    }

    /// Calculate total supply up to given height
    pub fn calculate_total_supply(height: u32, params: &RewardParams) -> Result<u64, RewardError> {
        let mut total_supply = 0u64;

        if height == 0 {
            return Ok(params.initial_reward);
        }

        let total_decay_blocks = params.decay_years * params.blocks_per_year;

        if height <= total_decay_blocks {
            // During decay phase - sum all individual rewards
            for h in 0..=height {
                let reward = Self::calculate_block_reward_with_params(h, params)?;
                total_supply = total_supply
                    .checked_add(reward)
                    .ok_or(RewardError::SupplyOverflow)?;
            }
        } else {
            // Calculate decay phase total + tail emission
            let mut decay_total = 0u64;
            for h in 0..total_decay_blocks {
                let reward = Self::calculate_block_reward_with_params(h, params)?;
                decay_total = decay_total
                    .checked_add(reward)
                    .ok_or(RewardError::SupplyOverflow)?;
            }

            let tail_blocks = height - total_decay_blocks;
            let tail_total = (tail_blocks as u64)
                .checked_mul(params.tail_emission)
                .ok_or(RewardError::SupplyOverflow)?;

            total_supply = decay_total
                .checked_add(tail_total)
                .ok_or(RewardError::SupplyOverflow)?;
        }

        Ok(total_supply)
    }

    /// Calculate annual inflation rate at given height
    pub fn calculate_inflation_rate(
        height: u32,
        params: &RewardParams,
    ) -> Result<f64, RewardError> {
        let current_supply = Self::calculate_total_supply(height, params)?;
        let current_reward = Self::calculate_block_reward_with_params(height, params)?;
        let annual_emission = current_reward * (params.blocks_per_year as u64);

        if current_supply == 0 {
            return Ok(0.0);
        }

        let inflation_rate = (annual_emission as f64) / (current_supply as f64) * 100.0;
        Ok(inflation_rate)
    }

    /// Get reward at specific year mark
    pub fn reward_at_year(year: u32, params: &RewardParams) -> Result<u64, RewardError> {
        if year >= params.decay_years {
            return Ok(params.tail_emission);
        }

        let height = year * params.blocks_per_year;
        Self::calculate_block_reward_with_params(height, params)
    }

    /// Calculate when tail emission begins (block height)
    pub fn tail_emission_start_height(params: &RewardParams) -> u32 {
        params.decay_years * params.blocks_per_year
    }

    /// Validate reward parameters for consistency
    pub fn validate_params(params: &RewardParams) -> Result<(), RewardError> {
        if params.initial_reward == 0 {
            return Err(RewardError::InvalidInitialReward);
        }

        if params.tail_emission == 0 {
            return Err(RewardError::InvalidTailEmission);
        }

        if params.initial_reward <= params.tail_emission {
            return Err(RewardError::InvalidDecayRange);
        }

        if params.blocks_per_year == 0 {
            return Err(RewardError::InvalidBlocksPerYear);
        }

        if params.decay_years == 0 {
            return Err(RewardError::InvalidDecayYears);
        }

        // Check for potential overflow in calculations
        let total_blocks = params.decay_years as u64 * params.blocks_per_year as u64;
        if total_blocks > u32::MAX as u64 {
            return Err(RewardError::DecayPeriodTooLong);
        }

        Ok(())
    }

    /// Calculate emission curve statistics
    pub fn calculate_emission_stats(params: &RewardParams) -> Result<EmissionStats, RewardError> {
        Self::validate_params(params)?;

        let tail_height = Self::tail_emission_start_height(params);
        let decay_supply = Self::calculate_total_supply(tail_height - 1, params)?;

        // Annual emission at start and end of decay
        let initial_annual = params.initial_reward * (params.blocks_per_year as u64);
        let tail_annual = params.tail_emission * (params.blocks_per_year as u64);

        Ok(EmissionStats {
            decay_period_blocks: tail_height,
            decay_period_years: params.decay_years,
            total_decay_supply: decay_supply,
            initial_annual_emission: initial_annual,
            tail_annual_emission: tail_annual,
            decay_reduction_ratio: params.initial_reward as f64 / params.tail_emission as f64,
        })
    }
}

/// Emission curve statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmissionStats {
    /// Number of blocks in decay period
    pub decay_period_blocks: u32,
    /// Number of years in decay period
    pub decay_period_years: u32,
    /// Total supply at end of decay period
    pub total_decay_supply: u64,
    /// Initial annual emission (year 0)
    pub initial_annual_emission: u64,
    /// Tail emission annual rate
    pub tail_annual_emission: u64,
    /// Ratio of initial to tail emission
    pub decay_reduction_ratio: f64,
}

/// Error types for reward calculations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewardError {
    /// Invalid initial reward
    InvalidInitialReward,
    /// Invalid tail emission
    InvalidTailEmission,
    /// Invalid decay range (initial <= tail)
    InvalidDecayRange,
    /// Invalid blocks per year
    InvalidBlocksPerYear,
    /// Invalid decay years
    InvalidDecayYears,
    /// Decay period too long (overflow)
    DecayPeriodTooLong,
    /// Supply calculation overflow
    SupplyOverflow,
    /// Height out of range
    InvalidHeight,
}

impl fmt::Display for RewardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RewardError::InvalidInitialReward => write!(f, "Invalid initial reward"),
            RewardError::InvalidTailEmission => write!(f, "Invalid tail emission"),
            RewardError::InvalidDecayRange => write!(f, "Invalid decay range"),
            RewardError::InvalidBlocksPerYear => write!(f, "Invalid blocks per year"),
            RewardError::InvalidDecayYears => write!(f, "Invalid decay years"),
            RewardError::DecayPeriodTooLong => write!(f, "Decay period too long"),
            RewardError::SupplyOverflow => write!(f, "Supply calculation overflow"),
            RewardError::InvalidHeight => write!(f, "Invalid block height"),
        }
    }
}

impl std::error::Error for RewardError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_params() -> RewardParams {
        RewardParams {
            initial_reward: 3_237_500_000, // 32.375 BTPC
            tail_emission: 50_000_000,     // 0.5 BTPC
            blocks_per_year: 52_560,       // ~10 minute blocks
            decay_years: 24,
        }
    }

    #[test]
    fn test_genesis_reward() {
        let params = test_params();
        let genesis_reward =
            RewardCalculator::calculate_block_reward_with_params(0, &params).unwrap();

        assert_eq!(genesis_reward, params.initial_reward);
    }

    #[test]
    fn test_linear_decay_progression() {
        let params = test_params();

        // Test rewards decrease monotonically during decay period
        let mut previous_reward = params.initial_reward;

        for year in 1..params.decay_years {
            let height = year * params.blocks_per_year;
            let current_reward =
                RewardCalculator::calculate_block_reward_with_params(height, &params).unwrap();

            assert!(
                current_reward < previous_reward,
                "Reward should decrease monotonically at year {}",
                year
            );

            assert!(
                current_reward >= params.tail_emission,
                "Reward should not go below tail emission at year {}",
                year
            );

            previous_reward = current_reward;
        }
    }

    #[test]
    fn test_tail_emission_transition() {
        let params = test_params();
        let tail_height = RewardCalculator::tail_emission_start_height(&params);

        // Last block of decay period
        let pre_tail_reward =
            RewardCalculator::calculate_block_reward_with_params(tail_height - 1, &params).unwrap();
        assert!(pre_tail_reward >= params.tail_emission);

        // First block of tail emission
        let tail_reward =
            RewardCalculator::calculate_block_reward_with_params(tail_height, &params).unwrap();
        assert_eq!(tail_reward, params.tail_emission);

        // Blocks after tail emission should remain constant
        for offset in 1..=1000 {
            let future_reward =
                RewardCalculator::calculate_block_reward_with_params(tail_height + offset, &params)
                    .unwrap();
            assert_eq!(future_reward, params.tail_emission);
        }
    }

    #[test]
    fn test_total_supply_calculation() {
        let params = test_params();

        // Genesis supply
        let genesis_supply = RewardCalculator::calculate_total_supply(0, &params).unwrap();
        assert_eq!(genesis_supply, params.initial_reward);

        // Supply should always increase
        let mut previous_supply = genesis_supply;
        for height in 1..=100 {
            let current_supply = RewardCalculator::calculate_total_supply(height, &params).unwrap();
            assert!(current_supply > previous_supply);
            previous_supply = current_supply;
        }
    }

    #[test]
    fn test_inflation_rate_calculation() {
        let params = test_params();

        // Inflation rate should generally decrease over time (during decay)
        let initial_rate =
            RewardCalculator::calculate_inflation_rate(params.blocks_per_year, &params).unwrap();
        let later_rate =
            RewardCalculator::calculate_inflation_rate(10 * params.blocks_per_year, &params)
                .unwrap();

        assert!(
            later_rate < initial_rate,
            "Inflation rate should decrease during decay"
        );

        // Tail emission period should have decreasing inflation (due to growing supply)
        let tail_height = RewardCalculator::tail_emission_start_height(&params);
        let tail_rate_1 = RewardCalculator::calculate_inflation_rate(
            tail_height + params.blocks_per_year,
            &params,
        )
        .unwrap();
        let tail_rate_2 = RewardCalculator::calculate_inflation_rate(
            tail_height + 2 * params.blocks_per_year,
            &params,
        )
        .unwrap();

        assert!(
            tail_rate_2 < tail_rate_1,
            "Tail emission inflation should decrease due to growing supply"
        );
    }

    #[test]
    fn test_reward_at_year() {
        let params = test_params();

        // Year 0 should match initial reward
        let year_0 = RewardCalculator::reward_at_year(0, &params).unwrap();
        assert_eq!(year_0, params.initial_reward);

        // Year 24+ should be tail emission
        let year_24 = RewardCalculator::reward_at_year(24, &params).unwrap();
        let year_30 = RewardCalculator::reward_at_year(30, &params).unwrap();
        assert_eq!(year_24, params.tail_emission);
        assert_eq!(year_30, params.tail_emission);

        // Intermediate years should be between initial and tail
        for year in 1..24 {
            let reward = RewardCalculator::reward_at_year(year, &params).unwrap();
            assert!(reward < params.initial_reward);
            assert!(reward >= params.tail_emission);
        }
    }

    #[test]
    fn test_parameter_validation() {
        let mut params = test_params();

        // Valid params should pass
        assert!(RewardCalculator::validate_params(&params).is_ok());

        // Invalid initial reward
        params.initial_reward = 0;
        assert!(RewardCalculator::validate_params(&params).is_err());
        params = test_params();

        // Invalid tail emission
        params.tail_emission = 0;
        assert!(RewardCalculator::validate_params(&params).is_err());
        params = test_params();

        // Initial <= tail (invalid decay)
        params.tail_emission = params.initial_reward + 1;
        assert!(RewardCalculator::validate_params(&params).is_err());
        params = test_params();

        // Zero blocks per year
        params.blocks_per_year = 0;
        assert!(RewardCalculator::validate_params(&params).is_err());
        params = test_params();

        // Zero decay years
        params.decay_years = 0;
        assert!(RewardCalculator::validate_params(&params).is_err());
    }

    #[test]
    fn test_emission_statistics() {
        let params = test_params();
        let stats = RewardCalculator::calculate_emission_stats(&params).unwrap();

        assert_eq!(stats.decay_period_years, 24);
        assert_eq!(stats.decay_period_blocks, 24 * 52_560);
        assert!(stats.total_decay_supply > 0);
        assert_eq!(
            stats.initial_annual_emission,
            params.initial_reward * (params.blocks_per_year as u64)
        );
        assert_eq!(
            stats.tail_annual_emission,
            params.tail_emission * (params.blocks_per_year as u64)
        );
        assert!(stats.decay_reduction_ratio > 1.0);
    }

    #[test]
    fn test_constitutional_compliance() {
        let params = test_params();

        // Verify constitutional requirements
        assert_eq!(params.initial_reward, 3_237_500_000); // 32.375 BTPC
        assert_eq!(params.tail_emission, 50_000_000); // 0.5 BTPC
        assert_eq!(params.decay_years, 24); // 24 years

        // Verify linear decay math
        let year_12_height = 12 * params.blocks_per_year;
        let year_12_reward =
            RewardCalculator::calculate_block_reward_with_params(year_12_height, &params).unwrap();

        // At 12 years (halfway), reward should be approximately halfway between initial and tail
        let expected_halfway = (params.initial_reward + params.tail_emission) / 2;
        let tolerance = params.initial_reward / 100; // 1% tolerance

        assert!(
            year_12_reward.abs_diff(expected_halfway) < tolerance,
            "Year 12 reward should be approximately halfway: got {}, expected ~{}",
            year_12_reward,
            expected_halfway
        );
    }

    #[test]
    fn test_precision_and_rounding() {
        let params = test_params();

        // Test that reward calculations are consistent and don't have rounding errors
        for height in 0..1000 {
            let reward1 =
                RewardCalculator::calculate_block_reward_with_params(height, &params).unwrap();
            let reward2 =
                RewardCalculator::calculate_block_reward_with_params(height, &params).unwrap();
            assert_eq!(reward1, reward2, "Reward calculation must be deterministic");
        }

        // Test edge cases around tail emission transition
        let tail_height = RewardCalculator::tail_emission_start_height(&params);

        let pre_tail =
            RewardCalculator::calculate_block_reward_with_params(tail_height - 1, &params).unwrap();
        let at_tail =
            RewardCalculator::calculate_block_reward_with_params(tail_height, &params).unwrap();

        assert!(pre_tail >= params.tail_emission);
        assert_eq!(at_tail, params.tail_emission);
    }
}
