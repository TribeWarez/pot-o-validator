use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Uint256 {
    data: [u8; 32],
}

impl Uint256 {
    pub const fn new() -> Self {
        Self { data: [0u8; 32] }
    }

    pub const fn from_be_bytes(bytes: [u8; 32]) -> Self {
        Self { data: bytes }
    }

    pub const fn as_be_bytes(&self) -> &[u8; 32] {
        &self.data
    }

    pub fn from_u64(v: u64) -> Self {
        let mut out = Self::new();
        let be = v.to_be_bytes();
        out.data[24..32].copy_from_slice(&be);
        out
    }

    pub fn mul_div(&mut self, mul: u32, div: u32) {
        if div == 0 {
            return;
        }

        let mut a = [0u32; 8];
        for (chunk, limb) in self.data.chunks_exact(4).zip(a.iter_mut()) {
            *limb = u32::from_be_bytes(chunk.try_into().unwrap());
        }

        let mut carry: u64 = 0;
        for limb in a.iter_mut().rev() {
            let v = (*limb as u64) * (mul as u64) + carry;
            *limb = (v & 0xFFFF_FFFF) as u32;
            carry = v >> 32;
        }
        if carry != 0 {
            *self = Self::max_value();
            return;
        }

        let mut rem: u64 = 0;
        for limb in a.iter_mut() {
            let cur = (rem << 32) | (*limb as u64);
            *limb = (cur / div as u64) as u32;
            rem = cur % div as u64;
        }

        for (chunk, &limb) in self.data.chunks_exact_mut(4).zip(a.iter()) {
            let be = limb.to_be_bytes();
            chunk.copy_from_slice(&be);
        }
    }

    pub const fn max_value() -> Self {
        Self { data: [0xFFu8; 32] }
    }
}

impl Default for Uint256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Ord for Uint256 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.data.cmp(&other.data)
    }
}

impl PartialOrd for Uint256 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_is_zero() {
        let u = Uint256::new();
        assert_eq!(u.as_be_bytes(), &[0u8; 32]);
    }

    #[test]
    fn test_from_u64() {
        let u = Uint256::from_u64(1);
        let bytes = u.as_be_bytes();
        assert_eq!(bytes[31], 1);
        for i in 0..31 {
            assert_eq!(bytes[i], 0);
        }
    }

    #[test]
    fn test_from_u64_large() {
        let val: u64 = 0xDEAD_BEEF_CAFE_BABE;
        let u = Uint256::from_u64(val);
        let be = val.to_be_bytes();
        assert_eq!(u.as_be_bytes()[24..32], be);
    }

    #[test]
    fn test_equality() {
        let a = Uint256::from_u64(42);
        let b = Uint256::from_u64(42);
        let c = Uint256::from_u64(43);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_ordering() {
        let small = Uint256::from_u64(1);
        let large = Uint256::from_u64(2);
        assert!(small < large);
        assert!(large > small);
    }

    #[test]
    fn test_max_value() {
        let max = Uint256::max_value();
        assert_eq!(max.as_be_bytes(), &[0xFFu8; 32]);
    }

    #[test]
    fn test_max_value_ordering() {
        let max = Uint256::max_value();
        let zero = Uint256::new();
        assert!(max > zero);
        assert!(zero < max);
    }

    #[test]
    fn test_mul_div_zero_mul() {
        let mut u = Uint256::from_u64(100);
        u.mul_div(0, 1);
        assert_eq!(u, Uint256::new());
    }

    #[test]
    fn test_mul_div_div_one() {
        let mut u = Uint256::from_u64(42);
        u.mul_div(1, 1);
        assert_eq!(u, Uint256::from_u64(42));
    }

    #[test]
    fn test_mul_div_noop_on_div_zero() {
        let mut u = Uint256::from_u64(42);
        u.mul_div(1, 0);
        assert_eq!(u, Uint256::from_u64(42));
    }

    #[test]
    fn test_mul_div_simple() {
        let mut u = Uint256::from_u64(30);
        u.mul_div(5, 3);
        assert_eq!(u, Uint256::from_u64(50));
    }

    #[test]
    fn test_mul_div_truncation() {
        let mut u = Uint256::from_u64(10);
        u.mul_div(1, 3);
        assert_eq!(u, Uint256::from_u64(3));
    }

    #[test]
    fn test_mul_div_overflow_saturates() {
        let mut u = Uint256::max_value();
        u.mul_div(2, 1);
        assert_eq!(u, Uint256::max_value());
    }

    #[test]
    fn test_mul_div_increases_target() {
        let mut u = Uint256::from_u64(1000);
        let before = u;
        u.mul_div(115, 100);
        assert!(u > before);
        assert!(u <= Uint256::max_value());
    }

    #[test]
    fn test_mul_div_decreases_target() {
        let mut u = Uint256::from_u64(1000);
        let before = u;
        u.mul_div(85, 100);
        assert!(u < before);
    }

    #[test]
    fn test_mul_div_identity_twelve_steps() {
        let mut u = Uint256::from_u64(1 << 20);
        for _ in 0..12 {
            u.mul_div(115, 100);
        }
        assert!(u > Uint256::from_u64(1 << 20));
    }

    #[test]
    fn test_mul_div_max_value_stays_max() {
        let mut u = Uint256::max_value();
        u.mul_div(115, 100);
        assert_eq!(u, Uint256::max_value());
    }

    #[test]
    fn test_from_be_bytes_roundtrip() {
        let bytes = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C,
            0x1D, 0x1E, 0x1F, 0x20,
        ];
        let u = Uint256::from_be_bytes(bytes);
        assert_eq!(u.as_be_bytes(), &bytes);
    }

    #[test]
    fn test_mul_div_known_answer() {
        let mut u = Uint256::from_u64(123456789);
        u.mul_div(115, 100);
        let expected = 123456789u64 * 115 / 100;
        assert_eq!(
            u,
            Uint256::from_u64(expected),
            "123456789 * 115 / 100 should equal {}",
            expected
        );
    }
}
