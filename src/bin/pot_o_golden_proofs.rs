use pot_o_mining::PotOConsensus;
use pot_o_core::TribeResult;
use serde_json::json;
use std::time::SystemTime;

/// Generate a small set of "golden" PoT-O proofs for a fixed slot/seed.
///
/// This is intended to be used alongside ESP firmware proofs and on-chain
/// `ProofRecord` data to cross-check hashes, path signatures, and thresholds.
///
/// Usage:
///   cargo run -p pot-o-validator --bin pot-o-golden-proofs > golden_proofs.jsonl
fn main() -> TribeResult<()> {
    let difficulty = 1;
    let max_tensor_dim = 16;
    let consensus = PotOConsensus::new(difficulty, max_tensor_dim);

    for slot in [100u64, 101u64, 102u64] {
        let slot_hash = format!("{:0>64}", hex::encode(slot.to_le_bytes()));
        let challenge = consensus.generate_challenge(slot, &slot_hash)?;

        if let Some(proof) = consensus
            .mine(&challenge, "golden_miner_pubkey", 5_000)?
        {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let line = json!({
                "generated_at": now,
                "challenge": {
                    "id": challenge.id,
                    "slot": challenge.slot,
                    "slot_hash": challenge.slot_hash,
                    "operation_type": challenge.operation_type,
                    "difficulty": challenge.difficulty,
                    "mml_threshold": challenge.mml_threshold,
                    "path_distance_max": challenge.path_distance_max,
                },
                "proof": proof,
            });
            println!("{}", line);
        }
    }

    Ok(())
}

