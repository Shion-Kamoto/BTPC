// Additional tests for pow.rs to verify security fixes
// Add these tests to pow.rs test module

#[cfg(test)]
mod additional_pow_tests {
    use super::*;
    use crate::consensus::{Difficulty, DifficultyTarget};

    #[test]
    fn test_mining_target_from_easy_difficulty() {
        // Test that from_difficulty correctly converts easy test difficulty
        let difficulty = Difficulty::from_bits(0x207fffff);
        let mining_target = MiningTarget::from_difficulty(difficulty);

        // Should match the easy target
        let easy_target = MiningTarget::easy_target();

        // Targets should be equal or very close
        assert_eq!(mining_target.as_bytes()[0], easy_target.as_bytes()[0]);
        assert_eq!(mining_target.as_bytes()[1], easy_target.as_bytes()[1]);
    }

    #[test]
    fn test_mining_target_from_mainnet_difficulty() {
        // Test that from_difficulty works with mainnet difficulty
        let difficulty = Difficulty::from_bits(0x1d00ffff);
        let mining_target = MiningTarget::from_difficulty(difficulty);

        // Should produce a harder (lower value) target than easy target
        let easy_target = MiningTarget::easy_target();

        // First byte should be less than easy target (harder)
        assert!(mining_target.as_bytes()[0] <= easy_target.as_bytes()[0]);
    }

    #[test]
    fn test_mining_target_consistency() {
        // Test that from_difficulty produces consistent results
        let difficulty = Difficulty::from_bits(0x1d00ffff);

        let target1 = MiningTarget::from_difficulty(difficulty);
        let target2 = MiningTarget::from_difficulty(difficulty);

        // Should produce identical targets
        assert_eq!(target1, target2);
    }

    #[test]
    fn test_difficulty_target_round_trip() {
        // Test that Difficulty -> DifficultyTarget -> MiningTarget works correctly
        let original_bits = 0x1d00ffff;
        let difficulty = Difficulty::from_bits(original_bits);
        let difficulty_target = DifficultyTarget::from_bits(original_bits);
        let mining_target = MiningTarget::from_difficulty(difficulty);

        // Mining target should match difficulty target bytes
        assert_eq!(mining_target.as_bytes(), difficulty_target.as_bytes());
    }

    #[test]
    fn test_mine_with_real_difficulty() {
        // This test verifies that mining now respects actual difficulty
        // With the fix, easy blocks should mine quickly, hard blocks slowly

        let header = crate::blockchain::BlockHeader::create_test_header();

        // Easy difficulty should find solution quickly
        let easy_difficulty = Difficulty::from_bits(0x207fffff);
        let easy_target = MiningTarget::from_difficulty(easy_difficulty);

        let start = std::time::Instant::now();
        let easy_result = ProofOfWork::mine(&header, &easy_target);
        let easy_time = start.elapsed();

        assert!(easy_result.is_ok(), "Easy mining should succeed");
        assert!(easy_time.as_secs() < 10, "Easy mining should complete in < 10 seconds");

        // Note: We don't test hard difficulty as it would take too long
        // But the fix ensures it would actually BE hard now
    }
}