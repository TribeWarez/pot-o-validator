use pot_o_core::TribeResult;
use pot_o_mining::{ChallengeGenerator, MMLPathValidator};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::time::Instant;

/// Simple CLI to compare DEFLATE-based MML scores with the ESP-style
/// entropy approximation for a batch of synthetic challenges.
///
/// Usage (from pot-o-validator workspace root):
///   cargo run -p pot-o-validator --bin pot-o-mml-calibrate
fn main() -> TribeResult<()> {
    let difficulty = 2;
    let max_tensor_dim = 64;
    let generator = ChallengeGenerator::new(difficulty, max_tensor_dim);
    let mml = MMLPathValidator::default();

    let mut rng = StdRng::seed_from_u64(42);

    let mut n = 0f64;
    let mut sum_deflate = 0f64;
    let mut sum_entropy = 0f64;
    let mut sum_deflate_sq = 0f64;
    let mut sum_entropy_sq = 0f64;
    let mut sum_cross = 0f64;

    let samples = 100usize;
    let start = Instant::now();

    for _ in 0..samples {
        let slot: u64 = rng.gen_range(1..10_000);
        let slot_bytes = slot.to_le_bytes();
        let slot_hash = hex::encode(slot_bytes);
        let challenge = generator.generate(slot, &format!("{slot_hash:0>64}"))?;

        let task = challenge.to_mining_task("calibration_miner");
        let engine = ai3_lib::AI3Engine::new();
        let output = engine.execute_task(&task)?;

        let deflate_score = mml.compute_mml_score(&challenge.input_tensor, &output)?;
        let entropy_score = mml.compute_entropy_mml_score(&challenge.input_tensor, &output);

        println!(
            "sample,slot={},op={},dim={},mml_deflate={:.6},mml_entropy={:.6}",
            slot,
            challenge.operation_type,
            challenge
                .input_tensor
                .shape
                .dims
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("x"),
            deflate_score,
            entropy_score
        );

        n += 1.0;
        sum_deflate += deflate_score;
        sum_entropy += entropy_score;
        sum_deflate_sq += deflate_score * deflate_score;
        sum_entropy_sq += entropy_score * entropy_score;
        sum_cross += deflate_score * entropy_score;
    }

    let elapsed = start.elapsed();

    if n > 1.0 {
        let mean_d = sum_deflate / n;
        let mean_e = sum_entropy / n;
        let var_d = (sum_deflate_sq / n) - (mean_d * mean_d);
        let var_e = (sum_entropy_sq / n) - (mean_e * mean_e);
        let cov = (sum_cross / n) - (mean_d * mean_e);
        let corr = if var_d > 0.0 && var_e > 0.0 {
            cov / (var_d.sqrt() * var_e.sqrt())
        } else {
            0.0
        };

        println!(
            "summary,samples={},mean_deflate={:.6},mean_entropy={:.6},corr={:.6},elapsed_ms={}",
            samples,
            mean_d,
            mean_e,
            corr,
            elapsed.as_millis()
        );
    }

    Ok(())
}
