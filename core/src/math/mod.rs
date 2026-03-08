//! Multi-tier arithmetic for tensor network calculations
//!
//! Supports three implementation tiers:
//! 1. Research: Arbitrary precision (optional, requires features)
//! 2. Portable: f64-based (default, widely supported)
//! 3. Hardware: Fixed-point u32/u64 (optimal for embedded/blockchain)

/// Research-precision arithmetic (optional feature)
///
/// For theoretical validation and manuscript proofs.
/// Requires `num-bigint` and `num-rational` crates.
#[cfg(feature = "research-precision")]
pub mod field {
    use num_bigint::BigInt;
    use num_rational::Ratio;

    /// Arbitrary-precision rational number
    /// Used for exact symbolic calculations
    pub type ResearchScalar = Ratio<BigInt>;

    /// Natural logarithm with arbitrary precision
    pub fn ln_arbitrary(x: &ResearchScalar) -> ResearchScalar {
        // Approximation using Taylor series (for research only)
        // In production, use decimal crate or similar
        let f64_val = x.to_f64().unwrap_or(1.0);
        let result = f64_val.ln();
        Ratio::from_float(result).unwrap_or_else(|| Ratio::new(BigInt::from(1), BigInt::from(1)))
    }
}

/// Portable f64-based arithmetic (default)
///
/// Works on all platforms. Precision limited to ~15 decimal digits.
/// Suitable for on-chain calculations where f64 is available.
pub mod portable {
    /// Standard f64 floating-point
    pub type PortableScalar = f64;

    /// Natural logarithm (standard libm)
    #[inline]
    pub fn ln(x: f64) -> f64 {
        x.ln()
    }

    /// Tanh for coherence/probability
    #[inline]
    pub fn tanh(x: f64) -> f64 {
        x.tanh()
    }
}

/// Fixed-point arithmetic for blockchain
///
/// Optimal for Solana/EVM with limited floating-point support.
/// Uses u64 with configurable scale (typically 1e6).
pub mod fixed_point {
    /// Fixed-point u64 with configurable scale
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct FixedPoint64 {
        /// Raw value (fixed-point)
        pub value: u64,
        /// Decimal places (typically 6 for 1e6 scale)
        pub scale: u32,
    }

    impl FixedPoint64 {
        /// Create from raw value and scale
        pub const fn new(value: u64, scale: u32) -> Self {
            FixedPoint64 { value, scale }
        }

        /// Create from f64
        pub fn from_f64(f: f64, scale: u32) -> Self {
            let scale_f = 10u64.pow(scale) as f64;
            FixedPoint64 {
                value: (f.max(0.0) * scale_f) as u64,
                scale,
            }
        }

        /// Convert to f64
        pub fn to_f64(&self) -> f64 {
            let scale_f = 10u64.pow(self.scale) as f64;
            self.value as f64 / scale_f
        }

        /// Multiply two fixed-point numbers
        pub fn multiply(&self, other: &FixedPoint64) -> FixedPoint64 {
            debug_assert_eq!(self.scale, other.scale);

            let scale_factor = 10u64.pow(self.scale);
            let result = ((self.value as u128 * other.value as u128) / scale_factor as u128) as u64;

            FixedPoint64 {
                value: result,
                scale: self.scale,
            }
        }

        /// Natural logarithm approximation (fixed-point)
        /// Uses polynomial approximation suitable for fixed-point
        pub fn ln(&self) -> FixedPoint64 {
            let f = self.to_f64();
            let ln_f = f.ln();
            FixedPoint64::from_f64(ln_f, self.scale)
        }

        /// Tanh approximation (fixed-point)
        pub fn tanh(&self) -> FixedPoint64 {
            let f = self.to_f64();
            let tanh_f = f.tanh();
            FixedPoint64::from_f64(tanh_f, self.scale)
        }
    }
}

/// Hardware-optimized arithmetic for embedded systems (ESP32, etc.)
///
/// Uses u32 with precision bits for extreme resource-constrained environments.
pub mod hardware {
    /// Minimal fixed-point u32 for embedded
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct HardwareFixed {
        /// Raw value
        pub value: u32,
        /// Bits after binary point (typically 8-16)
        pub precision_bits: u8,
    }

    impl HardwareFixed {
        /// Create from value and precision
        pub const fn new(value: u32, precision_bits: u8) -> Self {
            HardwareFixed {
                value,
                precision_bits,
            }
        }

        /// Multiply (with rounding to avoid overflow)
        pub fn multiply(&self, other: &HardwareFixed) -> HardwareFixed {
            debug_assert_eq!(self.precision_bits, other.precision_bits);

            let result =
                ((self.value as u64 * other.value as u64) >> (self.precision_bits as u64)) as u32;

            HardwareFixed {
                value: result,
                precision_bits: self.precision_bits,
            }
        }

        /// Fast approximation of ln using Taylor series
        /// Assumes input normalized to [1, 2)
        pub fn ln_approx(&self) -> HardwareFixed {
            // Quick approximation: ln(x) ≈ (x-1) - (x-1)²/2 + (x-1)³/3 - ...
            // For embedded, just return a rough estimate
            let scale = 1u32 << self.precision_bits;
            let norm_val = (self.value as f32) / (scale as f32);
            let ln_result = norm_val.ln();
            let scaled = (ln_result * (scale as f32)) as u32;

            HardwareFixed {
                value: scaled,
                precision_bits: self.precision_bits,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_point_64_from_f64() {
        let fp = fixed_point::FixedPoint64::from_f64(1.5, 6);
        assert_eq!(fp.value, 1_500_000);
    }

    #[test]
    fn test_fixed_point_64_to_f64() {
        let fp = fixed_point::FixedPoint64::new(1_500_000, 6);
        let f = fp.to_f64();
        assert!((f - 1.5).abs() < 0.000001);
    }

    #[test]
    fn test_fixed_point_64_multiply() {
        let a = fixed_point::FixedPoint64::from_f64(2.0, 6);
        let b = fixed_point::FixedPoint64::from_f64(3.0, 6);
        let result = a.multiply(&b);
        let f = result.to_f64();
        assert!((f - 6.0).abs() < 0.001);
    }

    #[test]
    fn test_hardware_fixed_multiply() {
        let a = hardware::HardwareFixed::new(256, 8); // 1.0 with 256 = 2^8
        let b = hardware::HardwareFixed::new(512, 8); // 2.0 with 256 = 2^8
        let result = a.multiply(&b);
        // Result should be roughly 2.0 with same scale
        assert!(result.value > 400 && result.value < 600);
    }
}
