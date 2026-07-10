pub const TARGET_BLOCK_SECS: u64 = 30;
pub const ADJUSTMENT_WINDOW: usize = 12;
pub const MAX_ADJUST_FACTOR: f64 = 4.0;
pub const MIN_ADJUST_FACTOR: f64 = 0.25;

pub fn adjust_target(base_target: [u8; 32], recent_timestamps: &[u64]) -> [u8; 32] {
    if recent_timestamps.len() < 2 {
        return base_target;
    }

    let mut intervals = Vec::new();
    for i in 1..recent_timestamps.len() {
        let delta = recent_timestamps[i].saturating_sub(recent_timestamps[i - 1]);
        if delta > 0 {
            intervals.push(delta);
        }
    }

    if intervals.is_empty() {
        return base_target;
    }

    let avg_interval = intervals.iter().sum::<u64>() as f64 / intervals.len() as f64;
    let ratio = TARGET_BLOCK_SECS as f64 / avg_interval.max(1.0);
    let clamped = ratio.clamp(MIN_ADJUST_FACTOR, MAX_ADJUST_FACTOR);
    scale_target(base_target, clamped)
}

fn scale_target(target: [u8; 32], factor: f64) -> [u8; 32] {
    let mut scaled: [f64; 32] = [0.0; 32];
    for i in 0..32 {
        scaled[i] = target[i] as f64 * factor;
    }

    let mut result = [0u8; 32];
    let mut carry = 0.0f64;

    for i in 0..32 {
        let val = scaled[i] + carry;
        result[i] = val as u8;
        carry = val - (val as u8) as f64;
        if i < 31 {
            scaled[i + 1] += carry * 256.0;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uint256::Uint256;

    #[test]
    fn test_target_increases_when_blocks_too_fast() {
        let base = [0x0F; 32];
        let timestamps = vec![0, 5, 10, 15, 20];
        let adjusted = adjust_target(base, &timestamps);
        assert!(adjusted >= base);
    }

    #[test]
    fn test_target_decreases_when_blocks_too_slow() {
        let base = [0xFF; 32];
        let timestamps = vec![0, 120, 240, 360];
        let adjusted = adjust_target(base, &timestamps);
        assert!(adjusted <= base);
    }

    #[test]
    fn test_no_adjustment_with_few_timestamps() {
        let base = [0xAB; 32];
        assert_eq!(adjust_target(base, &[]), base);
        assert_eq!(adjust_target(base, &[100]), base);
    }

    #[test]
    fn test_scale_target_identity() {
        let base = [0x10; 32];
        let scaled = scale_target(base, 1.0);
        assert_eq!(scaled, base);
    }

    #[test]
    fn test_scale_target_double() {
        let base = Uint256::from_u64(1000);
        let scaled_bytes = scale_target(*base.as_be_bytes(), 2.0);
        let scaled = Uint256::from_be_bytes(scaled_bytes);
        assert_eq!(scaled, Uint256::from_u64(2000));
    }

    #[test]
    fn test_scale_target_half() {
        let base = Uint256::from_u64(1000);
        let scaled_bytes = scale_target(*base.as_be_bytes(), 0.5);
        let scaled = Uint256::from_be_bytes(scaled_bytes);
        assert_eq!(scaled, Uint256::from_u64(500));
    }

    #[test]
    fn test_adjust_target_exact_target_interval() {
        let base = [0x10; 32];
        let timestamps = vec![0, 30, 60, 90];
        let adjusted = adjust_target(base, &timestamps);
        assert_eq!(adjusted, base);
    }

    #[test]
    fn test_adjust_target_clamps_extreme_fast() {
        let base = [0x10; 32];
        let timestamps = vec![0, 1, 2, 3];
        let adjusted = adjust_target(base, &timestamps);
        let max_expected = scale_target(base, MAX_ADJUST_FACTOR);
        assert_eq!(adjusted, max_expected);
    }

    #[test]
    fn test_adjust_target_clamps_extreme_slow() {
        let base = [0x10; 32];
        let timestamps = vec![0, 10000, 20000, 30000];
        let adjusted = adjust_target(base, &timestamps);
        let min_expected = scale_target(base, MIN_ADJUST_FACTOR);
        assert_eq!(adjusted, min_expected);
    }

    #[test]
    fn test_adjust_target_skips_zero_intervals() {
        let base = [0x10; 32];
        let timestamps = vec![0, 0, 0, 30];
        let adjusted = adjust_target(base, &timestamps);
        assert_eq!(adjusted, base);
    }
}
