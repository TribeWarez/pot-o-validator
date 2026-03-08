//! Tests for pot-o-mining module
//!
//! Validates challenge generation, mining operations, and proof types

use pot_o_mining::{Challenge, ChallengeGenerator};

#[test]
fn test_challenge_generator_creation() {
    let gen = ChallengeGenerator::default();

    assert!(gen.base_difficulty > 0);
    assert!(gen.base_mml_threshold >= 0.0);
    assert!(gen.base_path_distance > 0);
    assert!(gen.max_tensor_dim > 0);
    assert!(gen.challenge_ttl_secs > 0);
}

#[test]
fn test_challenge_generator_with_custom_params() {
    let gen = ChallengeGenerator::new(5000, 128);

    assert_eq!(gen.base_difficulty, 5000);
    assert_eq!(gen.max_tensor_dim, 128);
}

#[test]
fn test_challenge_generator_default_values() {
    let gen = ChallengeGenerator::default();

    // Verify defaults are reasonable
    assert!(gen.base_difficulty >= 100);
    assert!(gen.base_difficulty <= 10000);
    assert!(gen.base_mml_threshold > 0.0);
    assert!(gen.base_mml_threshold <= 1.0);
}

#[test]
fn test_challenge_creation_from_slot() {
    let gen = ChallengeGenerator::default();
    let slot = 100u64;
    let slot_hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    let challenge = gen.generate(slot, slot_hash);
    assert!(challenge.is_ok());

    let ch = challenge.unwrap();
    assert_eq!(ch.slot, slot);
    assert_eq!(ch.slot_hash, slot_hash);
}

#[test]
fn test_challenge_id_uniqueness() {
    let gen = ChallengeGenerator::default();

    let ch1 = gen.generate(100, "hash1").unwrap();
    let ch2 = gen.generate(101, "hash2").unwrap();

    // Different slots should generate different challenge IDs
    assert_ne!(ch1.id, ch2.id);
}

#[test]
fn test_challenge_contains_operation_type() {
    let gen = ChallengeGenerator::default();
    let challenge = gen
        .generate(
            100,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();

    assert!(!challenge.operation_type.is_empty());
}

#[test]
fn test_challenge_has_valid_difficulty() {
    let gen = ChallengeGenerator::default();
    let challenge = gen.generate(100, "hash").unwrap();

    assert!(challenge.difficulty > 0);
    assert_eq!(challenge.difficulty, gen.base_difficulty);
}

#[test]
fn test_challenge_mml_threshold() {
    let gen = ChallengeGenerator::default();
    let challenge = gen.generate(100, "hash").unwrap();

    assert!(challenge.mml_threshold >= 0.0);
    assert!(challenge.mml_threshold <= 1.0);
}

#[test]
fn test_challenge_path_distance_max() {
    let gen = ChallengeGenerator::default();
    let challenge = gen.generate(100, "hash").unwrap();

    assert!(challenge.path_distance_max > 0);
}

#[test]
fn test_challenge_max_tensor_dim() {
    let gen = ChallengeGenerator::new(1000, 64);
    let challenge = gen.generate(100, "hash").unwrap();

    assert_eq!(challenge.max_tensor_dim, 64);
}

#[test]
fn test_challenge_expiration_logic() {
    let gen = ChallengeGenerator::default();
    let challenge = gen.generate(100, "hash").unwrap();

    // Challenge should not be expired immediately
    assert!(!challenge.is_expired());
}

#[test]
fn test_challenge_has_timestamps() {
    let gen = ChallengeGenerator::default();
    let challenge = gen.generate(100, "hash").unwrap();

    // Expiry should be after creation
    assert!(challenge.expires_at > challenge.created_at);
}

#[test]
fn test_challenge_ttl_applied() {
    let gen = ChallengeGenerator::default();
    let challenge = gen.generate(100, "hash").unwrap();

    let expected_ttl = gen.challenge_ttl_secs as i64;
    let actual_ttl = (challenge.expires_at - challenge.created_at).num_seconds();

    assert_eq!(actual_ttl, expected_ttl);
}

#[test]
fn test_challenge_to_mining_task() {
    let gen = ChallengeGenerator::default();
    let challenge = gen.generate(100, "hash").unwrap();

    let task = challenge.to_mining_task("requester1");

    assert_eq!(task.operation_type, challenge.operation_type);
    assert_eq!(task.difficulty, challenge.difficulty);
}

#[test]
fn test_challenge_input_tensor_not_empty() {
    let gen = ChallengeGenerator::default();
    let challenge = gen.generate(100, "hash").unwrap();

    // Input tensor should be valid
    let dims = challenge.input_tensor.shape().dims();
    assert!(!dims.is_empty());
}

#[test]
fn test_different_slots_produce_different_challenges() {
    let gen = ChallengeGenerator::default();

    let ch_slot_0 = gen.generate(0, "same_hash").unwrap();
    let ch_slot_1 = gen.generate(1, "same_hash").unwrap();

    // Different slots should produce different challenge data
    assert_ne!(ch_slot_0.id, ch_slot_1.id);
}

#[test]
fn test_same_slot_different_hash_produces_different_challenge() {
    let gen = ChallengeGenerator::default();

    let ch1 = gen.generate(100, "hash_a").unwrap();
    let ch2 = gen.generate(100, "hash_b").unwrap();

    assert_ne!(ch1.id, ch2.id);
}

#[test]
fn test_challenge_generator_max_tensor_dim_constraint() {
    // Test that max_tensor_dim is respected
    let gen1 = ChallengeGenerator::new(1000, 32);
    let gen2 = ChallengeGenerator::new(1000, 128);

    assert_eq!(gen1.max_tensor_dim, 32);
    assert_eq!(gen2.max_tensor_dim, 128);
}

#[test]
fn test_challenge_generator_difficulty_affect() {
    // Challenges with different difficulties
    let gen_easy = ChallengeGenerator::new(100, 64);
    let gen_hard = ChallengeGenerator::new(5000, 64);

    let ch_easy = gen_easy.generate(100, "hash").unwrap();
    let ch_hard = gen_hard.generate(100, "hash").unwrap();

    assert!(ch_easy.difficulty < ch_hard.difficulty);
}

#[test]
fn test_challenge_deterministic_generation() {
    let gen = ChallengeGenerator::default();

    let ch1 = gen.generate(100, "specific_hash_value").unwrap();
    let ch2 = gen.generate(100, "specific_hash_value").unwrap();

    // Same inputs should produce same challenge
    assert_eq!(ch1.id, ch2.id);
    assert_eq!(ch1.slot, ch2.slot);
    assert_eq!(ch1.slot_hash, ch2.slot_hash);
}

#[test]
fn test_mining_task_conversion_preserves_fields() {
    let gen = ChallengeGenerator::default();
    let challenge = gen.generate(100, "hash").unwrap();

    let task = challenge.to_mining_task("miner_1");

    assert_eq!(task.operation_type, challenge.operation_type);
    assert_eq!(task.difficulty, challenge.difficulty);
    assert!(task.reward > 0); // Should have reward
}

#[test]
fn test_challenge_slot_range() {
    let gen = ChallengeGenerator::default();

    // Test various slot numbers
    let slots = vec![0u64, 1, 100, 1000, u64::MAX - 1];

    for slot in slots {
        let ch = gen.generate(slot, "hash");
        assert!(ch.is_ok());
        assert_eq!(ch.unwrap().slot, slot);
    }
}
