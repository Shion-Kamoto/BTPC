//! Thermal Throttling Service
//!
//! Implements automatic GPU mining intensity reduction when temperature exceeds threshold.
//! Uses incremental throttling (10% per interval) with hysteresis to prevent oscillation.
//!
//! Feature: 012-create-an-new

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Thermal throttling state for a single GPU
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrottleState {
    pub gpu_device_index: u32,
    pub current_intensity: u8,        // 0-100%, 100% = full power
    pub is_throttled: bool,
    pub temperature: f32,
    pub threshold: f32,
}

/// Thermal throttling manager
pub struct ThermalThrottle {
    /// Per-GPU throttle states
    states: HashMap<u32, ThrottleState>,
    /// Temperature threshold (°C)
    temperature_threshold: f32,
    /// Hysteresis (°C below threshold to restore)
    hysteresis: f32,
}

impl ThermalThrottle {
    /// Create a new thermal throttle manager
    ///
    /// # Arguments
    /// * `temperature_threshold` - Temperature threshold in °C (default: 80.0)
    pub fn new(temperature_threshold: f32) -> Self {
        Self {
            states: HashMap::new(),
            temperature_threshold,
            hysteresis: 5.0,  // 5°C hysteresis
        }
    }

    /// Update temperature threshold
    pub fn set_threshold(&mut self, threshold: f32) {
        self.temperature_threshold = threshold;
    }

    /// Check if throttling is needed for a GPU
    ///
    /// Implements incremental throttling algorithm with hysteresis:
    /// - If temp > threshold: Reduce intensity by 10% (min 10%)
    /// - If temp < (threshold - hysteresis): Restore to 100%
    /// - Uses 5°C hysteresis to prevent oscillation
    ///
    /// # Arguments
    /// * `device_index` - GPU device index
    /// * `current_temp` - Current GPU temperature in °C
    ///
    /// # Returns
    /// * `u8` - New mining intensity (10-100%)
    ///
    /// # Implementation (T020)
    /// Thermal throttling algorithm with incremental reduction and hysteresis
    pub fn check_throttle(&mut self, device_index: u32, current_temp: f32) -> u8 {
        // Get or create state for this GPU
        let state = self.states.entry(device_index).or_insert(ThrottleState {
            gpu_device_index: device_index,
            current_intensity: 100,
            is_throttled: false,
            temperature: current_temp,
            threshold: self.temperature_threshold,
        });

        // Update temperature reading
        state.temperature = current_temp;
        state.threshold = self.temperature_threshold;

        // Calculate restore threshold (threshold - hysteresis)
        let restore_threshold = self.temperature_threshold - self.hysteresis;

        // Determine throttling action
        if current_temp > self.temperature_threshold {
            // Temperature exceeds threshold - reduce intensity by 10%
            if state.current_intensity > 10 {
                state.current_intensity = state.current_intensity.saturating_sub(10);
            } else {
                // Minimum intensity is 10% (never stop completely)
                state.current_intensity = 10;
            }
            state.is_throttled = true;
        } else if current_temp < restore_threshold && state.is_throttled {
            // Temperature dropped below restore threshold - restore to 100%
            state.current_intensity = 100;
            state.is_throttled = false;
        }
        // else: Temperature in hysteresis zone - maintain current intensity

        state.current_intensity
    }

    /// Get current throttle state for a GPU
    pub fn get_state(&self, device_index: u32) -> Option<&ThrottleState> {
        self.states.get(&device_index)
    }

    /// Get all throttle states
    pub fn get_all_states(&self) -> Vec<ThrottleState> {
        self.states.values().cloned().collect()
    }

    /// Reset throttle state for a specific GPU (T020)
    ///
    /// Sets intensity back to 100% and clears throttle flag.
    /// Useful when mining stops or GPU is replaced.
    ///
    /// # Arguments
    /// * `device_index` - GPU device index to reset
    pub fn reset_gpu(&mut self, device_index: u32) {
        if let Some(state) = self.states.get_mut(&device_index) {
            state.current_intensity = 100;
            state.is_throttled = false;
        }
    }

    /// Reset all throttle states (T020)
    ///
    /// Sets all GPUs back to 100% intensity.
    /// Useful when mining stops globally.
    pub fn reset_all(&mut self) {
        for state in self.states.values_mut() {
            state.current_intensity = 100;
            state.is_throttled = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_throttle_creation() {
        let throttle = ThermalThrottle::new(80.0);
        assert_eq!(throttle.temperature_threshold, 80.0);
        assert_eq!(throttle.hysteresis, 5.0);
    }

    #[test]
    fn test_threshold_update() {
        let mut throttle = ThermalThrottle::new(80.0);
        throttle.set_threshold(75.0);
        assert_eq!(throttle.temperature_threshold, 75.0);
    }

    #[test]
    fn test_thermal_throttle_reduces_intensity() {
        let mut throttle = ThermalThrottle::new(80.0);

        // Initial check at normal temp - should be 100%
        let intensity = throttle.check_throttle(0, 75.0);
        assert_eq!(intensity, 100);

        // Temperature exceeds threshold - should reduce by 10% to 90%
        let intensity = throttle.check_throttle(0, 85.0);
        assert_eq!(intensity, 90);

        // Another check at high temp - should reduce by another 10% to 80%
        let intensity = throttle.check_throttle(0, 86.0);
        assert_eq!(intensity, 80);
    }

    #[test]
    fn test_thermal_throttle_hysteresis() {
        let mut throttle = ThermalThrottle::new(80.0);

        // Heat up - trigger throttling
        throttle.check_throttle(0, 85.0);  // 90%
        let intensity = throttle.check_throttle(0, 85.0);  // 80%
        assert_eq!(intensity, 80);

        // Cool down slightly (still above restore threshold of 75°C) - should stay at 80%
        let intensity = throttle.check_throttle(0, 78.0);
        assert_eq!(intensity, 80, "Should maintain throttle in hysteresis zone");

        // Cool down below restore threshold (75°C) - should restore to 100%
        let intensity = throttle.check_throttle(0, 74.0);
        assert_eq!(intensity, 100, "Should restore to 100% below hysteresis threshold");
    }

    #[test]
    fn test_thermal_throttle_minimum_intensity() {
        let mut throttle = ThermalThrottle::new(80.0);

        // Reduce intensity all the way down
        for _ in 0..15 {
            throttle.check_throttle(0, 95.0);
        }

        // Should never go below 10%
        let intensity = throttle.check_throttle(0, 95.0);
        assert_eq!(intensity, 10, "Intensity should not drop below 10%");
    }

    #[test]
    fn test_reset_gpu() {
        let mut throttle = ThermalThrottle::new(80.0);

        // Throttle GPU
        throttle.check_throttle(0, 85.0);
        throttle.check_throttle(0, 85.0);

        // Reset GPU
        throttle.reset_gpu(0);

        let state = throttle.get_state(0).unwrap();
        assert_eq!(state.current_intensity, 100);
        assert!(!state.is_throttled);
    }
}