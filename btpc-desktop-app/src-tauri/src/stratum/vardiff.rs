//! Variable Share Difficulty (Vardiff) Controller
//!
//! Dynamically adjusts share difficulty to target a consistent
//! share submission rate (2 shares/minute by default).

use std::time::{Duration, Instant};

/// Vardiff controller targeting a consistent share rate
pub struct VardiffController {
    /// Target shares per minute
    target_shares_per_min: f64,
    /// Measurement window
    window: Duration,
    /// Minimum adjustment multiplier per window (0.25 = 4× easier max)
    min_adjust: f64,
    /// Maximum adjustment multiplier per window (4.0 = 4× harder max)
    max_adjust: f64,
    /// Current share difficulty (arbitrary units, scaled by pool)
    current_difficulty: f64,
    /// Minimum difficulty floor
    min_difficulty: f64,
    /// Shares received in current window
    shares_this_window: u64,
    /// Window start time
    window_start: Instant,
    /// Grace period after new job (don't count stale shares)
    stale_grace: Duration,
    /// Last job change timestamp
    last_job_change: Instant,
}

impl VardiffController {
    /// Create a new vardiff controller with default settings:
    /// - 2 shares/minute target
    /// - 60-second measurement window
    /// - 0.25× to 4× bounds per window
    /// - 5-second stale grace window
    pub fn new(initial_difficulty: f64) -> Self {
        Self {
            target_shares_per_min: 2.0,
            window: Duration::from_secs(60),
            min_adjust: 0.25,
            max_adjust: 4.0,
            current_difficulty: initial_difficulty,
            min_difficulty: 1.0,
            shares_this_window: 0,
            window_start: Instant::now(),
            stale_grace: Duration::from_secs(5),
            last_job_change: Instant::now(),
        }
    }

    /// Create a vardiff controller with custom parameters
    pub fn with_params(
        initial_difficulty: f64,
        target_shares_per_min: f64,
        window_secs: u64,
        min_difficulty: f64,
    ) -> Self {
        Self {
            target_shares_per_min,
            window: Duration::from_secs(window_secs),
            min_adjust: 0.25,
            max_adjust: 4.0,
            current_difficulty: initial_difficulty,
            min_difficulty,
            shares_this_window: 0,
            window_start: Instant::now(),
            stale_grace: Duration::from_secs(5),
            last_job_change: Instant::now(),
        }
    }

    /// Record a share submission and check if difficulty should be adjusted.
    ///
    /// Returns `Some(new_difficulty)` if the window has elapsed and
    /// difficulty was adjusted, or `None` if no adjustment needed yet.
    pub fn record_share(&mut self) -> Option<f64> {
        self.shares_this_window += 1;

        let elapsed = self.window_start.elapsed();
        if elapsed < self.window {
            return None;
        }

        // Window elapsed — calculate adjustment
        let minutes = elapsed.as_secs_f64() / 60.0;
        let actual_rate = self.shares_this_window as f64 / minutes;

        // Ratio: if actual_rate > target, we need higher difficulty (more work per share)
        let ratio = if self.target_shares_per_min > 0.0 {
            actual_rate / self.target_shares_per_min
        } else {
            1.0
        };

        // Clamp ratio to bounds
        let clamped_ratio = ratio.clamp(self.min_adjust, self.max_adjust);

        // Only adjust if ratio is meaningfully different from 1.0
        if (clamped_ratio - 1.0).abs() > 0.1 {
            self.current_difficulty = (self.current_difficulty * clamped_ratio).max(self.min_difficulty);
            eprintln!(
                "📊 Vardiff: {} shares in {:.1}s ({:.2}/min target {:.1}/min) → diff {:.2} (×{:.2})",
                self.shares_this_window,
                elapsed.as_secs_f64(),
                actual_rate,
                self.target_shares_per_min,
                self.current_difficulty,
                clamped_ratio,
            );
        }

        // Reset window
        self.shares_this_window = 0;
        self.window_start = Instant::now();

        Some(self.current_difficulty)
    }

    /// Notify that a new mining job was received (for stale grace window)
    pub fn notify_new_job(&mut self) {
        self.last_job_change = Instant::now();
    }

    /// Check if a share might be stale (submitted within grace period of a job change)
    pub fn is_within_stale_grace(&self) -> bool {
        self.last_job_change.elapsed() < self.stale_grace
    }

    /// Get current share difficulty
    pub fn current_difficulty(&self) -> f64 {
        self.current_difficulty
    }

    /// Manually set difficulty (e.g., from pool SetTarget message)
    pub fn set_difficulty(&mut self, difficulty: f64) {
        self.current_difficulty = difficulty.max(self.min_difficulty);
        // Reset window when difficulty is externally set
        self.shares_this_window = 0;
        self.window_start = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vardiff_initial_state() {
        let vd = VardiffController::new(1.0);
        assert_eq!(vd.current_difficulty(), 1.0);
        assert_eq!(vd.shares_this_window, 0);
    }

    #[test]
    fn test_vardiff_no_adjustment_within_window() {
        let mut vd = VardiffController::new(1.0);
        // Submit share immediately — window hasn't elapsed
        assert!(vd.record_share().is_none());
        assert_eq!(vd.shares_this_window, 1);
    }

    #[test]
    fn test_vardiff_increases_difficulty_on_fast_shares() {
        let mut vd = VardiffController::with_params(1.0, 2.0, 1, 0.1); // 1-second window
        // Simulate window elapsed by faking the start time
        vd.window_start = Instant::now() - Duration::from_secs(2);
        // Submit 10 shares (way over 2/min target)
        for _ in 0..10 {
            vd.shares_this_window += 1;
        }
        let result = vd.record_share(); // 11th share triggers window check
        assert!(result.is_some());
        // Difficulty should have increased
        assert!(result.unwrap() > 1.0);
    }

    #[test]
    fn test_vardiff_decreases_difficulty_on_slow_shares() {
        let mut vd = VardiffController::with_params(10.0, 2.0, 1, 0.1); // 1-second window
        vd.window_start = Instant::now() - Duration::from_secs(120); // 2 minutes elapsed
        // Only 1 share in 2 minutes (target is 2/min = 4 shares expected)
        let result = vd.record_share();
        assert!(result.is_some());
        // Difficulty should have decreased
        assert!(result.unwrap() < 10.0);
    }

    #[test]
    fn test_vardiff_respects_minimum_difficulty() {
        let mut vd = VardiffController::with_params(0.5, 2.0, 1, 1.0); // min_difficulty = 1.0
        vd.window_start = Instant::now() - Duration::from_secs(600); // Very slow
        let result = vd.record_share();
        if let Some(diff) = result {
            assert!(diff >= 1.0, "Difficulty went below minimum: {}", diff);
        }
    }

    #[test]
    fn test_vardiff_stale_grace_window() {
        let mut vd = VardiffController::new(1.0);
        vd.notify_new_job();
        assert!(vd.is_within_stale_grace());
    }

    #[test]
    fn test_vardiff_manual_set_resets_window() {
        let mut vd = VardiffController::new(1.0);
        vd.shares_this_window = 50;
        vd.set_difficulty(5.0);
        assert_eq!(vd.current_difficulty(), 5.0);
        assert_eq!(vd.shares_this_window, 0);
    }

    #[test]
    fn test_vardiff_clamp_bounds() {
        let mut vd = VardiffController::with_params(1.0, 2.0, 1, 0.01);
        vd.window_start = Instant::now() - Duration::from_secs(2);

        // Submit 1000 shares in 2 seconds — would want 500× increase, but clamped to 4×
        for _ in 0..1000 {
            vd.shares_this_window += 1;
        }
        let result = vd.record_share();
        assert!(result.is_some());
        let new_diff = result.unwrap();
        assert!(new_diff <= 4.0 + 0.01, "Should be clamped to 4×: {}", new_diff);
    }
}
