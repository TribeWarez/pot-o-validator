//! Timing harness for PoT-O operations and mining loop on PC. Prints timing metrics to stdout.
//! Exit code: 0 on success, non-zero on error.

use pot_o_core::TribeResult;
use pot_o_mining::PotOConsensus;
use std::time::Instant;

/// Simple timing harness for PoT-O operations and mining loop on PC.
///
/// Usage:
///   cargo run -p pot-o-validator --bin pot-o-timing
fn main() -> TribeResult<()> {
    let difficulty = 2;
    let max_tensor_dim = 64;
    let consensus = PotOConsensus::new(difficulty, max_tensor_dim);

    let slot = 100u64;
    let slot_hash = format!("{:0>64}", hex::encode(slot.to_le_bytes()));
    let challenge = consensus.generate_challenge(slot, &slot_hash)?;

    // Time a single tensor operation via AI3Engine.
    let task = challenge.to_mining_task("timing_miner");
    let op_start = Instant::now();
    let _output = consensus.engine.execute_task(&task)?;
    let op_elapsed = op_start.elapsed();

    println!(
        "op_timing,op={},dims={},time_ms={}",
        challenge.operation_type,
        challenge
            .input_tensor
            .shape
            .dims
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join("x"),
        op_elapsed.as_millis()
    );

    // Time the full mining loop for a modest iteration cap.
    let mine_start = Instant::now();
    let proof = consensus
        .mine(&challenge, "timing_miner", 10_000)?
        .map(|p| p.computation_nonce);
    let mine_elapsed = mine_start.elapsed();

    println!(
        "mine_timing,difficulty={},max_iter={},found_nonce={:?},time_ms={}",
        difficulty,
        10_000,
        proof,
        mine_elapsed.as_millis()
    );

    Ok(())
}
