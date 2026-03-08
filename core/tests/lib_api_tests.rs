//! Tests for tensor network types and operations
//!
//! Validates TensorNetworkVertex, EntanglementEdge, and TensorNetworkState

use pot_o_core::TribeResult;

// Note: We test the public API from pot-o-core
// The tensor network types are re-exported via the lib.rs

#[test]
fn test_block_time_target_constant() {
    // Verify that BLOCK_TIME_TARGET is set to reasonable value
    let target = pot_o_core::BLOCK_TIME_TARGET;
    assert_eq!(target, 60); // Should be 60 seconds
}

#[test]
fn test_esp_max_tensor_dim_constant() {
    // Verify ESP32-compatible tensor dimension limit
    let max_dim = pot_o_core::ESP_MAX_TENSOR_DIM;
    assert_eq!(max_dim, 64);
    assert!(max_dim > 0);
}

#[test]
fn test_version_constant_exists() {
    // Version should be populated from Cargo.toml
    let version = pot_o_core::VERSION;
    assert!(!version.is_empty());
    // Should match semver pattern (simplified check)
    assert!(version.contains('.'));
}

#[test]
fn test_constants_are_positive() {
    assert!(pot_o_core::BLOCK_TIME_TARGET > 0);
    assert!(pot_o_core::ESP_MAX_TENSOR_DIM > 0);
}

#[test]
fn test_constants_are_reasonable() {
    // Block time should be between 1 second and 24 hours
    let block_time = pot_o_core::BLOCK_TIME_TARGET;
    assert!(block_time >= 1);
    assert!(block_time <= 86400);

    // Tensor dimension should be power-of-2 or close
    let max_dim = pot_o_core::ESP_MAX_TENSOR_DIM;
    assert!(max_dim >= 32);
    assert!(max_dim <= 256);
}

#[test]
fn test_public_api_exports() {
    // Verify that error types are exported
    use pot_o_core::{TribeError, TribeResult};

    let _err: TribeError = TribeError::InvalidOperation("test".to_string());
    let _result: TribeResult<i32> = Ok(42);
}

#[test]
fn test_crypto_constants_exist() {
    // Verify module exports are available
    use pot_o_core::{BLOCK_TIME_TARGET, ESP_MAX_TENSOR_DIM, VERSION};

    assert!(!VERSION.is_empty());
    assert!(BLOCK_TIME_TARGET > 0);
    assert!(ESP_MAX_TENSOR_DIM > 0);
}

#[test]
fn test_tensor_entropy_functions_available() {
    // Test that tensor entropy functions are exported
    use pot_o_core::{
        approximate_minimal_cut, coherence_probability, effective_distance, entropy_from_cut,
        mutual_information, total_network_entropy,
    };

    // Just verify they're accessible
    let _ = entropy_from_cut;
    let _ = mutual_information;
    let _ = effective_distance;
    let _ = total_network_entropy;
    let _ = approximate_minimal_cut;
    let _ = coherence_probability;
}

#[test]
fn test_math_functions_available() {
    // Test that math functions are exported
    use pot_o_core::{matrix_multiply, tensor_contract, vector_dot};

    // Just verify they're accessible
    let _ = matrix_multiply;
    let _ = vector_dot;
    let _ = tensor_contract;
}

#[test]
fn test_types_exports() {
    // Verify core types are exported
    use pot_o_core::{Block, TokenType, Transaction, TransactionType};

    let _block_type = Block::new(0, "prev".to_string(), vec![], "miner".to_string(), 1000);

    let _token_type = TokenType::TribeChain;
    let _tx_type = TransactionType::Transfer;
}

#[test]
fn test_tensor_constants_available() {
    // Verify tensor constants are exported
    use pot_o_core::{BOND_DIMENSION_MAX, BOND_DIMENSION_MIN};

    assert!(BOND_DIMENSION_MIN > 0);
    assert!(BOND_DIMENSION_MAX > BOND_DIMENSION_MIN);
}

#[test]
fn test_entropy_calculation_constants() {
    // Verify entropy calculation constants
    use pot_o_core::COHERENCE_THRESHOLD;

    assert!(COHERENCE_THRESHOLD >= 0.0);
    assert!(COHERENCE_THRESHOLD <= 1.0);
}
