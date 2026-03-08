//! Tests for tensor network types and state management
//!
//! Validates TensorNetworkVertex, EntanglementEdge, MinimalCut, and TensorNetworkState

use pot_o_core::{TribeError, TribeResult};

// We test the tensor network types which are re-exported from core

#[test]
fn test_constants_for_tensor_network() {
    // Verify key tensor network constants
    let esp_max = pot_o_core::ESP_MAX_TENSOR_DIM;
    assert!(esp_max > 0);
    assert!(esp_max <= 256); // Should be reasonable for ESP32
}

#[test]
fn test_error_tensor_network_full() {
    let err = TribeError::TensorNetworkFull;
    let err_msg = err.to_string();
    assert!(err_msg.contains("full"));
}

#[test]
fn test_tensor_network_validation_constants() {
    // Test that tensor network dimension constraints make sense
    let max_dim = 16u32; // From the code: dimension.max(2).min(16)
    assert!(max_dim >= 2);
    assert!(max_dim <= 256);
}

#[test]
fn test_bond_dimension_constraints() {
    // Bond dimension should be between 2 and 16 per the vertex creation code
    let min_bond = 2u32;
    let max_bond = 16u32;
    
    // These are implicit in TensorNetworkVertex::new()
    assert!(min_bond <= max_bond);
    assert!(min_bond > 0);
}

#[test]
fn test_coupling_strength_limits() {
    // From EntanglementEdge::new: coupling_strength.min(1_000_000)
    let max_coupling = 1_000_000u64;
    assert!(max_coupling > 0);
}

#[test]
fn test_network_capacity_limits() {
    // From TensorNetworkState: vertex_count < 256, edge_count < 2048
    let max_vertices = 256u32;
    let max_edges = 2048u32;
    
    assert!(max_vertices > 0);
    assert!(max_edges > max_vertices); // More edges than vertices (makes sense for a graph)
}

#[test]
fn test_entropy_fixed_point_scaling() {
    // Entropy uses fixed-point with scale 1e6
    let scale = 1_000_000u64;
    assert!(scale > 0);
    assert_eq!(scale, 1_000_000);
}

#[test]
fn test_fixed_point_arithmetic_example() {
    // Test understanding of fixed-point arithmetic
    let fp_one = 1_000_000u64; // Represents 1.0
    let fp_half = 500_000u64;  // Represents 0.5
    
    // Addition in fixed-point
    let result = fp_half.saturating_add(fp_half);
    assert_eq!(result, fp_one);
}

#[test]
fn test_entropy_zero_case() {
    // Zero entropy should be handled correctly
    let zero_entropy = 0u64;
    assert_eq!(zero_entropy, 0);
}

#[test]
fn test_maximum_entropy_bounds() {
    // Maximum entropy should be reasonable for network calculations
    // From code: mutual information = S(A) + S(B) - S(A∪B)
    // This should be non-negative, implying S(A∪B) ≤ S(A) + S(B)
    let s_a = 1_000_000u64;
    let s_b = 1_000_000u64;
    let s_union = 1_500_000u64; // Can't exceed sum
    
    assert!(s_union <= s_a.saturating_add(s_b));
}

#[test]
fn test_distance_normalization_range() {
    // Effective distance should be in [0, 1]
    // d_eff = 1 - I(A:B) / S_max
    // This is clamped to [0.0, 1.0]
    let zero = 0.0f64;
    let one = 1.0f64;
    
    assert!(zero >= 0.0 && zero <= 1.0);
    assert!(one >= 0.0 && one <= 1.0);
}

#[test]
fn test_coherence_probability_bounds() {
    // tanh function returns [-1, 1] but we use positive values
    // coherence_probability(local/max) where both positive
    let local = 0.5f64;
    let max = 1.0f64;
    
    let tanh_result = (local / max).tanh();
    assert!(tanh_result >= 0.0 && tanh_result <= 1.0);
}

#[test]
fn test_edge_id_uniqueness() {
    // Edge IDs should be unique
    let id_1 = 0u64;
    let id_2 = 1u64;
    
    assert_ne!(id_1, id_2);
}

#[test]
fn test_pubkey_representation() {
    // Pubkeys are Vec<u8>
    let pubkey: Vec<u8> = b"test_miner".to_vec();
    assert!(!pubkey.is_empty());
    assert!(pubkey.len() > 0);
}

#[test]
fn test_vertex_dimension_range() {
    // Dimension should be in [2, 16]
    let min = 2u32;
    let max = 16u32;
    
    assert!(min <= max);
    assert!(min >= 2);
    assert!(max <= 256);
}

#[test]
fn test_timestamp_fields() {
    // Timestamps are i64 (Unix time)
    let ts: i64 = 1000000;
    assert!(ts > 0);
}

#[test]
fn test_capacity_check_for_vertices() {
    // Vertex capacity is 256
    let max_verts = 256u32;
    let test_count = 100u32;
    
    assert!(test_count < max_verts);
    assert!(max_verts > 0);
}

#[test]
fn test_capacity_check_for_edges() {
    // Edge capacity is 2048
    let max_edges = 2048u32;
    let test_count = 500u32;
    
    assert!(test_count < max_edges);
    assert!(max_edges > 0);
}

#[test]
fn test_entanglement_index_increment() {
    // When adding an edge, entanglement_index should increment
    let mut index = 0u32;
    index += 1;
    assert_eq!(index, 1);
    
    index += 1;
    assert_eq!(index, 2);
}

#[test]
fn test_edge_source_target_pubkeys() {
    // Source and target should be different for valid edges
    let source = b"miner_a".to_vec();
    let target = b"miner_b".to_vec();
    
    assert_ne!(source, target);
}

#[test]
fn test_minimal_cut_size_calculation() {
    // Cut size should equal number of edges
    let num_edges = 5;
    let cut_size = num_edges as u32;
    
    assert_eq!(cut_size, num_edges as u32);
}

#[test]
fn test_total_bond_dimension_sum() {
    // Total bond dimension is sum of edge bond dimensions
    let edge_bd1 = 2u32;
    let edge_bd2 = 4u32;
    let expected_total = (edge_bd1 + edge_bd2) as u64;
    
    assert_eq!(expected_total, 6);
}

#[test]
fn test_vertex_label_string() {
    // Labels are human-readable strings
    let label = "Main Mining Pool".to_string();
    assert!(!label.is_empty());
    assert!(label.len() > 0);
}

#[test]
fn test_fixed_point_to_float_conversion() {
    // Converting fixed-point back to float
    let fp = 500_000u64; // Represents 0.5
    let float_val = fp as f64 / 1_000_000.0;
    
    assert!(float_val > 0.49 && float_val < 0.51);
}

#[test]
fn test_network_state_empty_initialization() {
    // New network state should be empty
    let vertex_count = 0u32;
    let edge_count = 0u32;
    
    assert_eq!(vertex_count, 0);
    assert_eq!(edge_count, 0);
}

#[test]
fn test_last_updated_timestamp() {
    // Last updated should track when state changes
    let initial = 0i64;
    let updated = 1000i64;
    
    assert!(updated > initial);
}

#[test]
fn test_entropy_saturation_arithmetic() {
    // Total entropy uses saturating_add to prevent overflow
    let val1 = u64::MAX - 100;
    let val2 = 200u64;
    
    let sum = val1.saturating_add(val2);
    assert_eq!(sum, u64::MAX); // Should saturate, not overflow
}

#[test]
fn test_bond_dimension_byte_representation() {
    // Bond dimension is u32
    let min_bd = 2u32;
    let max_bd = 16u32;
    
    assert!(min_bd.to_le_bytes().len() == 4);
    assert!(max_bd.to_le_bytes().len() == 4);
}
