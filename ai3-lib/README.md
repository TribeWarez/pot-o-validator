# ai3-lib

[![crates.io](https://img.shields.io/crates/v/ai3-lib.svg)](https://crates.io/crates/ai3-lib)
[![docs.rs](https://img.shields.io/docsrs/ai3-lib)](https://docs.rs/ai3-lib)
[![CI](https://img.shields.io/github/actions/workflow/status/TribeWarez/ai3-lib/ci.yml?branch=main)](https://github.com/TribeWarez/ai3-lib/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

AI3 support library for PoT-O validator and miner components.

**Current Version**: v0.1.6-alpha | **Planned**: v0.2.0 with Tensor Types

- **Crate:** [crates.io/crates/ai3-lib](https://crates.io/crates/ai3-lib)
- **Docs:** [docs.rs/ai3-lib](https://docs.rs/ai3-lib)
- **Repository:** [github.com/TribeWarez/ai3-lib](https://github.com/TribeWarez/ai3-lib)

## Usage

```toml
[dependencies]
ai3-lib = "0.2"
```

Depends on `pot-o-core`. When used inside the pot-o-validator workspace, the workspace member is used automatically.

## Core Components

### Tensor Engine
- Quantum-inspired tensor calculations
- Fixed-point arithmetic (1e6 scale)
- Entropy and mutual information computation
- Entanglement state tracking

### ESP Compatibility Layer
- Support for ESP32/ESP8266 microcontrollers
- Embedded mining firmware compatibility
- Low-power operation modes
- WASM compilation support

### Mining Operations
- Challenge-response protocol
- MML (Minimal Memory Loss) computation
- Neural path signature generation
- Proof assembly and formatting

### AI3 Protocol Support
- Protocol message parsing and validation
- State machine implementation
- Message serialization/deserialization

## Tensor Types (v0.2.0)

```rust
use ai3_lib::tensor::{EntropyValue, CoherenceFactor, TensorState};

// Fixed-point entropy at 1e6 scale
let entropy = EntropyValue::from_u64(500_000); // 0.5

// Device coherence factor
let coherence: CoherenceFactor = 1.0; // ASIC baseline

// Quantum state
let state = TensorState::new(bond_dimension);
```

## Example: Entropy Calculation

```rust
use ai3_lib::tensor::{TensorEngine, EntropyValue};

let engine = TensorEngine::new();
let entropy = engine.calculate_entropy(&pool_state)?;
println!("Pool entropy: {}", entropy.to_f64());

// Fixed-point scale: 1e6 = 1.0
let normalized = entropy.as_u64() as f64 / 1_000_000.0;
```

## ESP32 Usage

```rust
use ai3_lib::esp::EspMiner;

let mut miner = EspMiner::new();
let challenge = miner.receive_challenge()?;
let proof = miner.solve_challenge(&challenge)?;
miner.submit_proof(&proof)?;
```

## Error Handling

```rust
use ai3_lib::error::{Ai3Error, Ai3Result};

fn compute_path() -> Ai3Result<Vec<u64>> {
    // Returns Err(Ai3Error::...) on failure
    Ok(vec![...])
}
```

## Testing

Run tests:
```bash
cargo test --lib ai3_lib --no-default-features  # No ESP
cargo test --lib ai3_lib --features wasm        # WASM support
```

## Documentation

Full API documentation at [docs.rs/ai3-lib](https://docs.rs/ai3-lib)

## Versioning

Releases follow semantic versioning. To publish:

1. Bump `version` in `Cargo.toml`.
2. Create a tag: `git tag v0.1.1 && git push origin v0.1.1`.
3. CI will publish to crates.io and create a GitHub Release.

## License

[MIT](LICENSE)
