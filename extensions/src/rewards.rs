pub const BASE_REWARD: u64 = 50_000_000;

pub fn calculate_mining_reward(difficulty: u64, path_distance: u32) -> u64 {
    let divisor = std::cmp::max(path_distance, 1) as u64;
    BASE_REWARD * difficulty / divisor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_reward_value() {
        assert_eq!(BASE_REWARD, 50_000_000);
    }

    #[test]
    fn test_calculate_mining_reward_basic() {
        assert_eq!(calculate_mining_reward(2, 5), 20_000_000);
    }

    #[test]
    fn test_calculate_mining_reward_path_distance_zero() {
        assert_eq!(calculate_mining_reward(1, 0), 50_000_000);
    }

    #[test]
    fn test_calculate_mining_reward_high_difficulty() {
        assert_eq!(calculate_mining_reward(10, 1), 500_000_000);
    }

    #[test]
    fn test_calculate_mining_reward_high_path_distance() {
        assert_eq!(calculate_mining_reward(1, 100), 500_000);
    }
}
