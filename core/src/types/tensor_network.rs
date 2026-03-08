//! Tensor network types for REALMS Part IV integration.
//!
//! Implements the quantum-inspired tensor network model of spacetime,
//! where vertices represent quantum subsystems (miners, pools) and edges
//! represent entanglement links.
//!
//! Reference: REALMS Part IV § 3-4 (Tensor Network Entropy)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A vertex in the tensor network (e.g., a miner or pool).
///
/// In REALMS, vertices carry tensor T_{i1,i2,...,ik} mapping incoming indices
/// to outgoing indices. For blockchain, we model this as a labeled quantum subsystem.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TensorNetworkVertex {
    /// Unique identifier (e.g., miner pubkey, pool address)
    pub pubkey: Vec<u8>,
    
    /// Human-readable label for this vertex
    pub label: String,
    
    /// Vector space dimension (typical: 2 for qubits, 4 for ququarts, 8+ for qudits)
    pub dimension: u32,
    
    /// Count of entanglement edges connected to this vertex
    pub entanglement_index: u32,
    
    /// Unix timestamp when this vertex was created
    pub created_at: i64,
}

impl TensorNetworkVertex {
    /// Create a new tensor network vertex
    pub fn new(
        pubkey: Vec<u8>,
        label: String,
        dimension: u32,
        created_at: i64,
    ) -> Self {
        Self {
            pubkey,
            label,
            dimension: dimension.max(2).min(16), // Constrain [2, 16]
            entanglement_index: 0,
            created_at,
        }
    }
}

/// An entanglement edge linking two vertices.
///
/// In REALMS, edges correspond to maximally entangled states |Φ⁺⟩ = (1/√d) Σᵢ |i⟩|i⟩.
/// The bond dimension d determines the entanglement capacity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntanglementEdge {
    /// Unique edge identifier
    pub id: u64,
    
    /// Source vertex pubkey
    pub source: Vec<u8>,
    
    /// Target vertex pubkey
    pub target: Vec<u8>,
    
    /// Bond dimension d in S = |γ| log(d)
    /// Typical: 2 (qubits), 4 (ququarts), up to 16
    pub bond_dimension: u32,
    
    /// Coupling strength α ∈ [0, 1e6] (fixed-point)
    /// Higher coupling = stronger entanglement
    pub coupling_strength: u64,
    
    /// Unix timestamp when edge was created
    pub created_at: i64,
}

impl EntanglementEdge {
    /// Create a new entanglement edge
    pub fn new(
        id: u64,
        source: Vec<u8>,
        target: Vec<u8>,
        bond_dimension: u32,
        coupling_strength: u64,
        created_at: i64,
    ) -> Self {
        Self {
            id,
            source,
            target,
            bond_dimension: bond_dimension.max(2).min(16),
            coupling_strength: coupling_strength.min(1_000_000),
            created_at,
        }
    }
}

/// Minimal cut in the tensor network.
///
/// REALMS Part IV § 3-4: The entropy of a region A is proportional to the minimal cut γ_A:
/// S(A) = |γ_A| * log(d)
///
/// where |γ_A| is the number of edges crossing the cut.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MinimalCut {
    /// Edges that cross the cut
    pub edges: Vec<EntanglementEdge>,
    
    /// Number of edges in the cut
    pub cut_size: u32,
    
    /// Sum of bond dimensions of cut edges
    pub total_bond_dimension: u64,
}

impl MinimalCut {
    /// Create a new minimal cut
    pub fn new(edges: Vec<EntanglementEdge>) -> Self {
        let cut_size = edges.len() as u32;
        let total_bond_dimension = edges.iter()
            .map(|e| e.bond_dimension as u64)
            .sum();
        
        Self {
            edges,
            cut_size,
            total_bond_dimension,
        }
    }
}

/// Global tensor network state.
///
/// Represents the complete tensor network for a PoT-O system:
/// - Vertices: miners, pools, or other quantum subsystems
/// - Edges: entanglement links representing coupling
/// - Entropy: cumulative S_total = Σ |γ_i| log(d_i)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TensorNetworkState {
    /// All vertices in the network (keyed by pubkey for O(1) lookup)
    pub vertices: HashMap<Vec<u8>, TensorNetworkVertex>,
    
    /// All entanglement edges
    pub edges: Vec<EntanglementEdge>,
    
    /// Total entropy S_total (fixed-point u64, scale 1e6)
    pub total_entropy: u64,
    
    /// Maximum possible entropy for normalization
    pub max_entropy: u64,
    
    /// Number of vertices
    pub vertex_count: u32,
    
    /// Number of edges
    pub edge_count: u32,
    
    /// Last update timestamp
    pub last_updated: i64,
}

impl TensorNetworkState {
    /// Create a new empty tensor network state
    pub fn new() -> Self {
        Self {
            vertices: HashMap::new(),
            edges: Vec::new(),
            total_entropy: 0,
            max_entropy: 0,
            vertex_count: 0,
            edge_count: 0,
            last_updated: 0,
        }
    }
    
    /// Add a vertex to the network
    pub fn add_vertex(&mut self, vertex: TensorNetworkVertex) -> crate::TribeResult<()> {
        if self.vertex_count >= 256 {
            return Err(crate::TribeError::TensorNetworkFull);
        }
        
        self.vertices.insert(vertex.pubkey.clone(), vertex);
        self.vertex_count += 1;
        
        Ok(())
    }
    
    /// Add an edge to the network
    pub fn add_edge(&mut self, edge: EntanglementEdge) -> crate::TribeResult<()> {
        if self.edge_count >= 2048 {
            return Err(crate::TribeError::TensorNetworkFull);
        }
        
        // Update entanglement indices for both vertices
        if let Some(source_vertex) = self.vertices.get_mut(&edge.source) {
            source_vertex.entanglement_index += 1;
        }
        if let Some(target_vertex) = self.vertices.get_mut(&edge.target) {
            target_vertex.entanglement_index += 1;
        }
        
        self.edges.push(edge);
        self.edge_count += 1;
        
        Ok(())
    }
    
    /// Get a vertex by pubkey
    pub fn get_vertex(&self, pubkey: &[u8]) -> Option<&TensorNetworkVertex> {
        self.vertices.get(pubkey)
    }
    
    /// Get all edges incident to a vertex
    pub fn incident_edges(&self, pubkey: &[u8]) -> Vec<&EntanglementEdge> {
        self.edges.iter()
            .filter(|e| e.source == pubkey || e.target == pubkey)
            .collect()
    }
}

impl Default for TensorNetworkState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vertex_creation() {
        let pubkey = b"miner_1".to_vec();
        let vertex = TensorNetworkVertex::new(pubkey.clone(), "Miner 1".to_string(), 4, 1000);
        
        assert_eq!(vertex.label, "Miner 1");
        assert_eq!(vertex.dimension, 4);
        assert_eq!(vertex.created_at, 1000);
    }
    
    #[test]
    fn test_edge_creation() {
        let edge = EntanglementEdge::new(
            0,
            b"miner_1".to_vec(),
            b"miner_2".to_vec(),
            2,
            500_000,
            1000,
        );
        
        assert_eq!(edge.bond_dimension, 2);
        assert_eq!(edge.coupling_strength, 500_000);
    }
    
    #[test]
    fn test_network_state() {
        let mut state = TensorNetworkState::new();
        
        let v1 = TensorNetworkVertex::new(b"m1".to_vec(), "M1".to_string(), 2, 1000);
        let v2 = TensorNetworkVertex::new(b"m2".to_vec(), "M2".to_string(), 2, 1000);
        
        assert!(state.add_vertex(v1).is_ok());
        assert!(state.add_vertex(v2).is_ok());
        assert_eq!(state.vertex_count, 2);
    }
}
