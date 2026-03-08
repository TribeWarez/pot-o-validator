//! Entropy calculations from REALMS Part IV § 3-4
//!
//! Implements the tensor network entropy formula:
//! S(A) = |γ_A| * log(d)
//!
//! where:
//! - γ_A is the minimal cut separating region A from the rest
//! - |γ_A| is the number of edges in the cut
//! - d is the bond dimension of the entanglement

use crate::types::tensor_network::{EntanglementEdge, MinimalCut, TensorNetworkState};
use crate::TribeResult;

/// Calculate entropy from a minimal cut
///
/// REALMS Part IV Formula: S(A) = |γ_A| * ln(d)
///
/// This represents the entanglement entropy at the boundary of region A.
/// In holographic duality, this entropy is proportional to the area of the boundary,
/// recovering the Bekenstein-Hawking formula S = A/(4ℓ_P²).
///
/// # Arguments
/// * `cut` - The minimal cut separating a region
///
/// # Returns
/// Entropy in fixed-point format (scale 1e6)
pub fn entropy_from_cut(cut: &MinimalCut) -> TribeResult<u64> {
    if cut.edges.is_empty() {
        return Ok(0);
    }

    // High-precision calculation
    let cut_size = cut.cut_size as f64;
    let avg_bond_dim = if cut.edges.is_empty() {
        2.0
    } else {
        (cut.total_bond_dimension as f64) / (cut.edges.len() as f64)
    };

    // S = |γ| * ln(d)
    let bond_dim_safe = avg_bond_dim.max(2.0);
    let entropy_f64 = cut_size * bond_dim_safe.ln();

    // Convert to fixed-point: u64 with scale 1e6
    let entropy_fixed = (entropy_f64 * 1_000_000.0) as u64;

    Ok(entropy_fixed)
}

/// Calculate mutual information between two regions
///
/// REALMS Part IV Formula: I(A:B) = S(A) + S(B) - S(A ∪ B)
///
/// This measures the amount of entanglement shared between regions A and B.
/// Regions with high mutual information are "close" in the entanglement geometry.
///
/// # Arguments
/// * `entropy_a` - Entropy of region A (fixed-point)
/// * `entropy_b` - Entropy of region B (fixed-point)
/// * `entropy_union` - Entropy of the union A ∪ B (fixed-point)
///
/// # Returns
/// Mutual information I(A:B) ≥ 0 (fixed-point)
pub fn mutual_information(entropy_a: u64, entropy_b: u64, entropy_union: u64) -> TribeResult<u64> {
    // Convert from fixed-point to f64
    let s_a = entropy_a as f64 / 1_000_000.0;
    let s_b = entropy_b as f64 / 1_000_000.0;
    let s_union = entropy_union as f64 / 1_000_000.0;

    // I(A:B) = S(A) + S(B) - S(A∪B)
    let mutual_info = (s_a + s_b - s_union).max(0.0);

    // Back to fixed-point
    Ok((mutual_info * 1_000_000.0) as u64)
}

/// Calculate effective distance from mutual information
///
/// REALMS Part IV Formula: d_eff(A, B) = 1 - I(A:B) / S_max
///
/// This defines a pseudo-distance in the entanglement geometry.
/// - d_eff = 0: Perfectly entangled (zero distance)
/// - d_eff = 1: Unentangled (maximum distance)
///
/// This recovers Ryu-Takayanagi duality: geometric distance ↔ entanglement entropy
///
/// # Arguments
/// * `mutual_info` - Mutual information I(A:B) (fixed-point)
/// * `max_entropy` - Maximum possible entropy for normalization (fixed-point)
///
/// # Returns
/// Effective distance d_eff ∈ [0, 1] (fixed-point, scale 1e6)
pub fn effective_distance(mutual_info: u64, max_entropy: u64) -> TribeResult<u64> {
    if max_entropy == 0 {
        return Ok(1_000_000); // Unentangled
    }

    let mutual_f64 = mutual_info as f64 / 1_000_000.0;
    let max_f64 = max_entropy as f64 / 1_000_000.0;

    let distance = (1.0 - (mutual_f64 / max_f64)).clamp(0.0, 1.0);

    Ok((distance * 1_000_000.0) as u64)
}

/// Calculate total entropy of entire network
///
/// S_total = Σ_{edges} |γ_i| * ln(d_i)
///
/// This sums up entanglement entropy over all edges in the network.
///
/// # Arguments
/// * `network_state` - The complete tensor network state
///
/// # Returns
/// Total entropy (fixed-point)
pub fn total_network_entropy(network_state: &TensorNetworkState) -> u64 {
    network_state
        .edges
        .iter()
        .map(|edge| {
            let bond_dim = edge.bond_dimension.max(2) as f64;
            let edge_entropy = (bond_dim.ln() * 1_000_000.0) as u64;
            edge_entropy
        })
        .fold(0u64, |acc, val| acc.saturating_add(val))
}

/// Approximate minimal cut using greedy algorithm
///
/// For a complete solution, finding the minimal cut is NP-hard.
/// This function provides a polynomial-time approximation using
/// a greedy edge removal strategy.
///
/// WARNING: Not optimal; suitable for on-chain approximation only
pub fn approximate_minimal_cut(network_state: &TensorNetworkState) -> MinimalCut {
    // Simplified: all edges form the cut
    // (In production, implement proper min-cut algorithm)
    MinimalCut::new(network_state.edges.clone())
}

/// Tanh-based coherence function for probabilistic unlock
///
/// P(unlock) = tanh(S_A / S_max)
///
/// Used in staking: determines probability of unlock based on
/// local entanglement entropy relative to network maximum.
pub fn coherence_probability(local_entropy: f64, max_entropy: f64) -> f64 {
    if max_entropy <= 0.0 {
        return 0.0;
    }
    let normalized = local_entropy / max_entropy;
    normalized.tanh()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_from_empty_cut() {
        let cut = MinimalCut::new(vec![]);
        let entropy = entropy_from_cut(&cut).unwrap();
        assert_eq!(entropy, 0);
    }

    #[test]
    fn test_entropy_from_single_edge() {
        let edge = EntanglementEdge::new(0, b"src".to_vec(), b"dst".to_vec(), 2, 500_000, 1000);
        let cut = MinimalCut::new(vec![edge]);
        let entropy = entropy_from_cut(&cut).unwrap();

        // S = 1 * ln(2) ≈ 0.693 → 693_000 (fixed-point)
        assert!(entropy > 600_000 && entropy < 800_000);
    }

    #[test]
    fn test_mutual_information() {
        let s_a = 1_000_000; // 1.0
        let s_b = 1_000_000; // 1.0
        let s_union = 1_500_000; // 1.5

        let mi = mutual_information(s_a, s_b, s_union).unwrap();
        // I = 1 + 1 - 1.5 = 0.5 → 500_000
        assert_eq!(mi, 500_000);
    }

    #[test]
    fn test_effective_distance() {
        let mi = 500_000; // I(A:B) = 0.5
        let max = 1_000_000; // S_max = 1.0

        let d = effective_distance(mi, max).unwrap();
        // d_eff = 1 - 0.5/1.0 = 0.5 → 500_000
        assert_eq!(d, 500_000);
    }

    #[test]
    fn test_coherence_probability() {
        let local = 0.5;
        let max = 1.0;

        let p = coherence_probability(local, max);
        // tanh(0.5) ≈ 0.46
        assert!(p > 0.45 && p < 0.47);
    }

    #[test]
    fn test_total_network_entropy() {
        let mut state = TensorNetworkState::new();

        let edge1 = EntanglementEdge::new(0, b"a".to_vec(), b"b".to_vec(), 2, 500_000, 1000);
        let edge2 = EntanglementEdge::new(1, b"b".to_vec(), b"c".to_vec(), 2, 500_000, 1000);

        state.edges.push(edge1);
        state.edges.push(edge2);

        let total = total_network_entropy(&state);
        // 2 edges * ln(2) ≈ 1.386 → 1_386_000
        assert!(total > 1_300_000 && total < 1_500_000);
    }
}
