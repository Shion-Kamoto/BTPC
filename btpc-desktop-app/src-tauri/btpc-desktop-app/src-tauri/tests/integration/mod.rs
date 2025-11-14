//! Integration Tests Module - Feature 007
//!
//! Comprehensive integration tests for transaction functionality:
//! - T010: Internal wallet transfers
//! - T011: External address sends
//! - T012: Error handling scenarios
//! - T013: Concurrent transaction handling
//! - T014: Transaction event flow (Article XI compliance)

// Feature 007 integration tests
mod test_internal_transfer;
mod test_external_send;
mod test_error_handling;
mod test_concurrent_tx;
mod test_tx_events;
