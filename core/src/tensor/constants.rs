//! Physical constants from REALMS Part IV: Information-Theoretic Foundation of Spacetime
//!
//! These constants establish the connection between tensor network geometry and
//! thermodynamic/quantum properties of spacetime.

/// Planck length (ℓ_P) in meters
/// Characteristic scale at which quantum effects of spacetime dominate
pub const PLANCK_LENGTH: f64 = 1.616255e-35;

/// Planck time (t_P) in seconds
/// Characteristic temporal scale for quantum gravity
pub const PLANCK_TIME: f64 = 5.391247e-44;

/// Planck energy (E_P) in joules
pub const PLANCK_ENERGY: f64 = 1.95616e9;

/// Planck mass (m_P) in kilograms
pub const PLANCK_MASS: f64 = 2.176434e-8;

/// Boltzmann constant (k_B) in J/K
/// Relates entropy to thermodynamic quantities
pub const BOLTZMANN_CONSTANT: f64 = 1.380649e-23;

/// Speed of light (c) in m/s
pub const SPEED_OF_LIGHT: f64 = 299792458.0;

/// Newton's gravitational constant (G) in m³/(kg·s²)
pub const GRAVITATIONAL_CONSTANT: f64 = 6.67430e-11;

/// Reduced Planck constant (ℏ = h / 2π) in J·s
pub const REDUCED_PLANCK: f64 = 1.054571817e-34;

/// Black hole entropy coefficient
/// S = (A/4) * (c³/(hbar·G)) = (k_B*c³)/(4*G*hbar)
pub const BH_ENTROPY_COEFF: f64 = 0.25;

/// Bond dimension for qubits (d=2)
/// Minimal quantum system: two-level system
pub const DEFAULT_BOND_DIM_QUBIT: u32 = 2;

/// Bond dimension for ququarts (d=4)
/// Four-level quantum system
pub const DEFAULT_BOND_DIM_QUQUART: u32 = 4;

/// Bond dimension for qudits (d=8)
/// Eight-level quantum system
pub const DEFAULT_BOND_DIM_QUDIT: u32 = 8;

/// Maximum number of vertices in on-chain tensor network
/// (Limited by account size and Solana transaction size)
pub const MAX_TENSOR_VERTICES: u32 = 256;

/// Maximum number of edges in on-chain tensor network
pub const MAX_TENSOR_EDGES: u32 = 2048;

/// Maximum bond dimension (practical limit for on-chain computation)
pub const MAX_BOND_DIMENSION: u32 = 16;

/// Minimum bond dimension (must be at least 2 for meaningful entanglement)
pub const MIN_BOND_DIMENSION: u32 = 2;

/// Default coupling strength (fixed-point, scale 1e6)
/// Represents neutral/moderate entanglement
pub const DEFAULT_COUPLING_STRENGTH: u64 = 500_000;

/// Maximum entropy threshold for on-chain validation
/// (prevents overflow in fixed-point arithmetic)
pub const MAX_ENTROPY_FIXED: u64 = u64::MAX / 2;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_constants_are_positive() {
        assert!(PLANCK_LENGTH > 0.0);
        assert!(PLANCK_TIME > 0.0);
        assert!(BOLTZMANN_CONSTANT > 0.0);
        assert!(SPEED_OF_LIGHT > 0.0);
        assert!(GRAVITATIONAL_CONSTANT > 0.0);
    }
    
    #[test]
    fn test_bond_dimensions() {
        assert_eq!(DEFAULT_BOND_DIM_QUBIT, 2);
        assert_eq!(DEFAULT_BOND_DIM_QUQUART, 4);
        assert!(MAX_BOND_DIMENSION >= DEFAULT_BOND_DIM_QUDIT);
    }
    
    #[test]
    fn test_network_limits() {
        assert!(MAX_TENSOR_VERTICES > 0);
        assert!(MAX_TENSOR_EDGES > 0);
        assert!(MAX_TENSOR_EDGES >= MAX_TENSOR_VERTICES);
    }
}
